//! Renet/netcode direct-address transport. Deliberately independent of Bevy:
//! the same endpoint runs in rendered applications and real-process smoke runs.
use super::{protocol::*, session::Session};
use crate::{geometry::Layout, survivor::Survivor, tuning::Tuning};
use renet::{DefaultChannel, RenetClient, RenetServer, ServerEvent};
use renet_netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// One player, no sockets: the same session, run in-process.
    Solo,
    Host(SocketAddr),
    Join(SocketAddr),
}
impl Mode {
    /// Runs the authoritative session (solo or hosted).
    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host(_) | Self::Solo)
    }
    pub fn is_solo(&self) -> bool {
        matches!(self, Self::Solo)
    }
}

pub fn local_address(address: SocketAddr) -> Result<SocketAddr, String> {
    let allowed = match address.ip() {
        // Loopback, a private LAN, or a private overlay network (Tailscale
        // and other CGNAT-range VPNs, 100.64.0.0/10) for friends far away.
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || shared_space(ip),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    };
    if !allowed || address.port() == 0 {
        return Err(
            "Use an explicit loopback, private LAN or VPN (100.64.x.x) address with a nonzero port; \
                    public/wildcard hosting is disabled."
                .into(),
        );
    }
    Ok(address)
}

/// The shared address space (RFC 6598), where private overlay networks
/// such as Tailscale give each machine its address.
fn shared_space(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    a == 100 && (64..128).contains(&b)
}

struct HostSocket {
    server: RenetServer,
    transport: NetcodeServerTransport,
    session: Session,
    peers: BTreeMap<u64, PlayerId>,
    pending: BTreeMap<u64, f32>,
    closing: BTreeMap<u64, f32>,
    next_id: PlayerId,
}
struct ClientSocket {
    client: RenetClient,
    transport: NetcodeClientTransport,
    hello: bool,
}
enum SocketSide {
    Solo(Box<Session>),
    Host(Box<HostSocket>),
    Client(Box<ClientSocket>),
}

pub struct Endpoint {
    pub mode: Mode,
    side: SocketSide,
    pub id: Option<PlayerId>,
    pub snapshot: Option<Snapshot>,
    pub notices: Vec<ServerMessage>,
    pub status: String,
    pub last_error: String,
    pub closed: bool,
    /// Set when the host answered with its own night (seed, night code):
    /// the adapter reconnects with it.
    pub adopt: Option<(u64, u8)>,
    latest: Input,
    input_sequence: u64,
    action_sequence: u64,
    send_in: f32,
    accumulator: f32,
    elapsed: f32,
    last_snapshot_at: f32,
    error_until: f32,
    /// Who this player asked to be.
    survivor: Survivor,
}
impl Endpoint {
    pub fn new(mode: Mode, layout: &Layout, tuning: &Tuning) -> Result<Self, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;
        let (side, id, status) = match mode {
            Mode::Solo => (
                SocketSide::Solo(Box::new(Session::new(layout, tuning))),
                Some(HOST),
                "Solo run".to_owned(),
            ),
            Mode::Host(addr) => {
                local_address(addr)?;
                let socket = UdpSocket::bind(addr).map_err(|e| format!("Cannot host on {addr}: {e}"))?;
                let config = ServerConfig {
                    current_time: now,
                    max_clients: 4,
                    protocol_id: PROTOCOL,
                    public_addresses: vec![addr],
                    authentication: ServerAuthentication::Unsecure,
                };
                let transport = NetcodeServerTransport::new(config, socket).map_err(|e| e.to_string())?;
                (
                    SocketSide::Host(Box::new(HostSocket {
                        server: RenetServer::new(Default::default()),
                        transport,
                        session: Session::new(layout, tuning),
                        peers: BTreeMap::new(),
                        pending: BTreeMap::new(),
                        closing: BTreeMap::new(),
                        next_id: 2,
                    })),
                    Some(HOST),
                    format!("Hosting {addr}; waiting for player 2"),
                )
            }
            Mode::Join(addr) => {
                local_address(addr)?;
                let bind = if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
                let socket = UdpSocket::bind(bind).map_err(|e| format!("Cannot create client socket: {e}"))?;
                let random = renet_netcode::generate_random_bytes::<8>();
                let authentication = ClientAuthentication::Unsecure {
                    server_addr: addr,
                    client_id: u64::from_le_bytes(random),
                    user_data: None,
                    protocol_id: PROTOCOL,
                };
                let transport = NetcodeClientTransport::new(now, authentication, socket).map_err(|e| e.to_string())?;
                (
                    SocketSide::Client(Box::new(ClientSocket {
                        client: RenetClient::new(Default::default()),
                        transport,
                        hello: false,
                    })),
                    None,
                    format!("Connecting to {addr}…"),
                )
            }
        };
        if !mode.is_solo() {
            eprintln!("NET fingerprint {:#018x}", fingerprint());
        }
        Ok(Self {
            mode,
            side,
            id,
            snapshot: None,
            notices: Vec::new(),
            status,
            last_error: String::new(),
            closed: false,
            adopt: None,
            latest: Input::default(),
            input_sequence: 0,
            action_sequence: 0,
            send_in: 0.0,
            accumulator: 0.0,
            elapsed: 0.0,
            last_snapshot_at: 0.0,
            error_until: 0.0,
            survivor: Survivor::default(),
        })
    }
    /// Ask to be `survivor`: set at once where this process holds the
    /// session, sent in the handshake when joining.
    pub fn with_survivor(mut self, survivor: Survivor) -> Self {
        self.survivor = survivor;
        match &mut self.side {
            SocketSide::Solo(session) => {
                let _ = session.choose_survivor(HOST, survivor);
            }
            SocketSide::Host(host) => {
                let _ = host.session.choose_survivor(HOST, survivor);
            }
            SocketSide::Client(_) => {}
        }
        self
    }
    pub fn run(&self) -> u64 {
        self.snapshot.as_ref().map_or(1, |s| s.run)
    }
    /// DEBUG ONLY: the authoritative session, when this process runs it
    /// (solo or host). Presentation must never read hidden truth from here.
    pub fn debug_session(&self) -> Option<&Session> {
        match &self.side {
            SocketSide::Solo(s) => Some(s),
            SocketSide::Host(h) => Some(&h.session),
            SocketSide::Client(_) => None,
        }
    }
    pub fn snapshot_age(&self) -> f32 {
        self.elapsed - self.last_snapshot_at
    }
    /// Record the newest input frame; the endpoint stamps epoch and sequence.
    pub fn input(&mut self, mut input: Input) {
        self.input_sequence += 1;
        input.run = self.run();
        input.sequence = self.input_sequence;
        self.latest = input;
    }
    pub fn command(&mut self, action: Action, layout: &Layout, tuning: &Tuning) {
        if self.closed {
            return;
        }
        self.action_sequence += 1;
        let run = self.run();
        match &mut self.side {
            SocketSide::Solo(session) => {
                let _ = session.input(HOST, self.latest, tuning);
                if let Err(e) = session.command(HOST, run, self.action_sequence, action, layout, tuning) {
                    self.last_error = e;
                    self.error_until = self.elapsed + 4.0;
                } else {
                    self.last_error.clear();
                }
            }
            SocketSide::Host(socket) => {
                let session = &mut socket.session;
                let _ = session.input(HOST, self.latest, tuning);
                if let Err(e) = session.command(HOST, run, self.action_sequence, action, layout, tuning) {
                    self.last_error = e;
                    self.error_until = self.elapsed + 4.0;
                } else {
                    self.last_error.clear();
                }
            }
            SocketSide::Client(socket) => {
                let client = &mut socket.client;
                // Orientation precedes actions on the same ordered channel.
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    encode(&ClientMessage::Input(self.latest)),
                );
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    encode(&ClientMessage::Action {
                        run,
                        sequence: self.action_sequence,
                        action,
                    }),
                );
            }
        }
    }
    pub fn close(&mut self, reason: &str) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.status = reason.to_owned();
        self.notices.clear();
        self.snapshot = None;
        match &mut self.side {
            SocketSide::Solo(_) => {}
            SocketSide::Host(socket) => {
                let HostSocket { server, transport, .. } = &mut **socket;
                server.broadcast_message(
                    DefaultChannel::ReliableOrdered,
                    encode(&ServerMessage::Ended(reason.to_owned())),
                );
                transport.send_packets(server);
                transport.disconnect_all(server);
            }
            SocketSide::Client(socket) => {
                let ClientSocket { client, transport, .. } = &mut **socket;
                client.send_message(DefaultChannel::ReliableOrdered, encode(&ClientMessage::Leave));
                let _ = transport.send_packets(client);
                transport.disconnect();
            }
        }
    }
    pub fn update(&mut self, dt: f32, layout: &Layout, tuning: &Tuning) {
        if self.closed {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.elapsed += dt;
        if self.elapsed >= self.error_until {
            self.last_error.clear();
        }
        self.send_in -= dt;
        let send = self.send_in <= 0.0;
        if send {
            self.send_in = SEND_INTERVAL;
        }
        let duration = Duration::from_secs_f32(dt);
        let mut incoming = Vec::new();
        let mut failure = None;
        match &mut self.side {
            SocketSide::Solo(session) => {
                if !session.started {
                    // A solo run starts at once; the briefing screen is its lobby.
                    self.action_sequence += 1;
                    let _ = session.command(HOST, session.run, self.action_sequence, Action::Start, layout, tuning);
                }
                let _ = session.input(HOST, self.latest, tuning);
                self.accumulator += dt;
                while self.accumulator >= STEP {
                    session.step(layout, tuning, STEP);
                    self.accumulator -= STEP;
                }
                for (id, message) in session.outbox.drain(..) {
                    if id == HOST {
                        incoming.push(message);
                    }
                }
                incoming.push(ServerMessage::Snapshot(session.snapshot(HOST, layout, tuning)));
            }
            SocketSide::Host(socket) => {
                let HostSocket {
                    server,
                    transport,
                    session,
                    peers,
                    pending,
                    closing,
                    next_id,
                } = &mut **socket;
                server.update(duration);
                if let Err(e) = transport.update(duration, server) {
                    failure = Some(format!("Host transport failed: {e}"));
                }
                while let Some(event) = server.get_event() {
                    match event {
                        ServerEvent::ClientConnected { client_id } => {
                            pending.insert(client_id, 0.0);
                        }
                        ServerEvent::ClientDisconnected { client_id, reason } => {
                            if let Some(id) = peers.remove(&client_id) {
                                session.remove_player(id);
                                self.status = format!("Player {id} left ({reason}); the host can continue.");
                                eprintln!("NET disconnected player={id}; bundles={}", session.relic_summary());
                            }
                            pending.remove(&client_id);
                            closing.remove(&client_id);
                        }
                    }
                }
                for client_id in server.clients_id() {
                    if closing.contains_key(&client_id) {
                        continue;
                    }
                    for channel in [
                        u8::from(DefaultChannel::ReliableOrdered),
                        u8::from(DefaultChannel::Unreliable),
                    ] {
                        for _ in 0..64 {
                            let Some(bytes) = server.receive_message(client_id, channel) else {
                                break;
                            };
                            let message = match decode::<ClientMessage>(&bytes) {
                                Ok(m) => m,
                                Err(e) => {
                                    server.send_message(
                                        client_id,
                                        DefaultChannel::ReliableOrdered,
                                        encode(&ServerMessage::Rejected(e)),
                                    );
                                    closing.insert(client_id, 0.0);
                                    break;
                                }
                            };
                            if matches!(message, ClientMessage::Leave) {
                                if let Some(id) = peers.remove(&client_id) {
                                    session.remove_player(id);
                                }
                                server.disconnect(client_id);
                                break;
                            }
                            if let Some(&id) = peers.get(&client_id) {
                                match message {
                                    ClientMessage::Input(input) => {
                                        let _ = session.input(id, input, tuning);
                                    }
                                    ClientMessage::Action { run, sequence, action } => {
                                        let error = session.command(id, run, sequence, action, layout, tuning).err();
                                        server.send_message(
                                            client_id,
                                            DefaultChannel::ReliableOrdered,
                                            encode(&ServerMessage::ActionResult { run, sequence, error }),
                                        );
                                    }
                                    _ => {}
                                }
                            } else if let ClientMessage::Hello {
                                fingerprint: version,
                                seed,
                                night,
                                survivor,
                            } = message
                            {
                                let result = if version != fingerprint() {
                                    Err(
                                        "Different gameplay build/configuration; use the same checkout and Cargo.lock."
                                            .to_owned(),
                                    )
                                } else if seed != tuning.seed || night != tuning.night.code() {
                                    // Not refused: told which night this is.
                                    server.send_message(
                                        client_id,
                                        DefaultChannel::ReliableOrdered,
                                        encode(&ServerMessage::Tonight {
                                            seed: tuning.seed,
                                            night: tuning.night.code(),
                                        }),
                                    );
                                    closing.insert(client_id, 0.0);
                                    break;
                                } else {
                                    session.add_player(*next_id, layout, tuning)
                                };
                                match result {
                                    Ok(()) => {
                                        let id = *next_id;
                                        *next_id += 1;
                                        let _ = session.choose_survivor(id, Survivor::from_code(survivor));
                                        peers.insert(client_id, id);
                                        pending.remove(&client_id);
                                        server.send_message(
                                            client_id,
                                            DefaultChannel::ReliableOrdered,
                                            encode(&ServerMessage::Welcome { id }),
                                        );
                                        eprintln!("NET accepted player={id} run={}", session.run);
                                    }
                                    Err(e) => {
                                        server.send_message(
                                            client_id,
                                            DefaultChannel::ReliableOrdered,
                                            encode(&ServerMessage::Rejected(e)),
                                        );
                                        closing.insert(client_id, 0.0);
                                    }
                                }
                            }
                        }
                    }
                }
                for (&id, age) in pending.iter_mut().chain(closing.iter_mut()) {
                    *age += dt;
                    if *age > 3.0 {
                        server.disconnect(id);
                    }
                }
                let _ = session.input(HOST, self.latest, tuning);
                self.accumulator += dt;
                while self.accumulator >= STEP {
                    session.step(layout, tuning, STEP);
                    self.accumulator -= STEP;
                }
                for (id, message) in session.outbox.drain(..) {
                    if id == HOST {
                        incoming.push(message);
                    } else if let Some((&wire_id, _)) = peers.iter().find(|(_, player)| **player == id) {
                        server.send_message(wire_id, DefaultChannel::ReliableOrdered, encode(&message));
                    }
                }
                incoming.push(ServerMessage::Snapshot(session.snapshot(HOST, layout, tuning)));
                if send {
                    for (&wire_id, &id) in peers.iter() {
                        server.send_message(
                            wire_id,
                            DefaultChannel::Unreliable,
                            encode(&ServerMessage::Snapshot(session.snapshot(id, layout, tuning))),
                        );
                    }
                }
                transport.send_packets(server);
            }
            SocketSide::Client(socket) => {
                let ClientSocket {
                    client,
                    transport,
                    hello,
                } = &mut **socket;
                client.update(duration);
                if let Err(e) = transport.update(duration, client) {
                    failure = Some(format!("Disconnected from host: {e}"));
                }
                if client.is_connected() {
                    if !*hello {
                        client.send_message(
                            DefaultChannel::ReliableOrdered,
                            encode(&ClientMessage::Hello {
                                fingerprint: fingerprint(),
                                seed: tuning.seed,
                                night: tuning.night.code(),
                                survivor: self.survivor.code(),
                            }),
                        );
                        *hello = true;
                    }
                    if send && self.id.is_some() {
                        client.send_message(DefaultChannel::Unreliable, encode(&ClientMessage::Input(self.latest)));
                    }
                    for channel in [
                        u8::from(DefaultChannel::ReliableOrdered),
                        u8::from(DefaultChannel::Unreliable),
                    ] {
                        for _ in 0..128 {
                            let Some(bytes) = client.receive_message(channel) else {
                                break;
                            };
                            match decode::<ServerMessage>(&bytes) {
                                Ok(m) => incoming.push(m),
                                Err(e) => {
                                    failure = Some(e);
                                    break;
                                }
                            }
                        }
                    }
                }
                if let Err(e) = transport.send_packets(client) {
                    failure = Some(format!("Disconnected from host: {e}"));
                }
            }
        }
        for message in incoming {
            self.receive(message);
        }
        if !self.closed {
            if let Some(e) = failure {
                self.close(&e);
            } else if !self.mode.is_host() && self.elapsed - self.last_snapshot_at > 10.0 {
                self.close("Connection timed out: no host snapshots for 10 seconds. Check address and that the host is running.");
            }
        }
    }
    fn receive(&mut self, message: ServerMessage) {
        if self.closed {
            return;
        }
        match message {
            ServerMessage::Welcome { id } => {
                self.id = Some(id);
                self.status = format!("Connected as player {id}; waiting for host to start");
            }
            ServerMessage::Rejected(reason) | ServerMessage::Ended(reason) => self.close(&reason),
            ServerMessage::Tonight { seed, night } => {
                self.adopt = Some((seed, night));
                self.close("The host's night is another; joining it.");
            }
            ServerMessage::Snapshot(snapshot) => {
                // Unreliable snapshots may arrive before reliable admission.
                // Do not render our own player as a remote before Welcome.
                if self.id.is_none() {
                    return;
                }
                if self
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| snapshot.run < s.run || (snapshot.run == s.run && snapshot.tick <= s.tick))
                {
                    return;
                }
                if self.snapshot.as_ref().is_some_and(|s| snapshot.run > s.run) {
                    self.input_sequence = 0;
                    self.action_sequence = 0;
                    self.latest = Input::default();
                    self.last_error.clear();
                    self.notices
                        .retain(|m| message_run(m).is_some_and(|r| r >= snapshot.run));
                }
                self.last_snapshot_at = self.elapsed;
                self.snapshot = Some(snapshot);
            }
            ServerMessage::ActionResult { run, error, .. } if run == self.run() => {
                self.last_error = error.unwrap_or_default();
                self.error_until = self.elapsed + 4.0;
            }
            m if message_run(&m).is_some_and(|r| r >= self.run()) => self.notices.push(m),
            _ => {}
        }
    }
}
fn message_run(m: &ServerMessage) -> Option<u64> {
    match m {
        ServerMessage::Cue { run, .. } | ServerMessage::Events { run, .. } => Some(*run),
        _ => None,
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.close("Session ended: host or client closed the application.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tuning::Night;

    #[test]
    fn only_loopback_lan_and_private_vpn_addresses_may_host() {
        for ok in [
            "127.0.0.1:5000",
            "192.168.1.20:5000",
            "10.0.0.4:5000",
            "172.20.1.1:5000",
            "100.101.7.9:5000",
        ] {
            assert!(local_address(ok.parse().unwrap()).is_ok(), "{ok}");
        }
        for bad in [
            "0.0.0.0:5000",
            "8.8.8.8:5000",
            "100.20.0.1:5000",
            "100.128.0.1:5000",
            "192.168.1.20:0",
        ] {
            assert!(local_address(bad.parse().unwrap()).is_err(), "{bad}");
        }
    }

    /// Real loopback UDP: a joiner knocking with another night is told the
    /// host's, and admitted once it knocks again with it.
    #[test]
    fn a_joiner_with_another_night_is_told_the_hosts_and_admitted_with_it() {
        let addr: SocketAddr = "127.0.0.1:38917".parse().unwrap();
        let host_tuning = Tuning::with_seed(7).with_night(Night::Hard);
        let host_layout = Layout::with_seed(7);
        let mut host = Endpoint::new(Mode::Host(addr), &host_layout, &host_tuning).expect("host");
        let knock = |tuning: &Tuning, layout: &Layout, host: &mut Endpoint| {
            let mut client = Endpoint::new(Mode::Join(addr), layout, tuning).expect("client");
            for _ in 0..300 {
                host.update(0.016, &host_layout, &host_tuning);
                client.update(0.016, layout, tuning);
                if client.closed || client.id.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(4));
            }
            client
        };
        let wrong = Tuning::with_seed(3).with_night(Night::Gentle);
        let first = knock(&wrong, &Layout::with_seed(3), &mut host);
        assert!(first.closed, "a mismatched joiner is not admitted: {}", first.status);
        assert_eq!(first.adopt, Some((7, Night::Hard.code())));
        assert!(first.id.is_none());
        drop(first);
        let second = knock(&host_tuning, &host_layout, &mut host);
        assert!(!second.closed, "{}", second.status);
        assert!(second.id.is_some(), "admitted with the host's night");
    }
}
