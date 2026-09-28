//! Renet/netcode direct-address transport. Deliberately independent of Bevy:
//! the same endpoint runs in rendered applications and real-process smoke runs.
use super::{protocol::*, session::Session};
use crate::{geometry::Layout, tuning::Tuning};
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
    Host(SocketAddr),
    Join(SocketAddr),
}
impl Mode {
    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host(_))
    }
}

pub fn local_address(address: SocketAddr) -> Result<SocketAddr, String> {
    let allowed = match address.ip() {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    };
    if !allowed || address.port() == 0 {
        return Err(
            "Use an explicit loopback or private LAN address with a nonzero port; public/wildcard hosting is disabled."
                .into(),
        );
    }
    Ok(address)
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
    latest: Input,
    input_sequence: u64,
    action_sequence: u64,
    send_in: f32,
    accumulator: f32,
    elapsed: f32,
    last_snapshot_at: f32,
    error_until: f32,
}
impl Endpoint {
    pub fn new(mode: Mode, layout: &Layout, tuning: &Tuning) -> Result<Self, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;
        let (side, id, status) = match mode {
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
        Ok(Self {
            mode,
            side,
            id,
            snapshot: None,
            notices: Vec::new(),
            status,
            last_error: String::new(),
            closed: false,
            latest: Input::default(),
            input_sequence: 0,
            action_sequence: 0,
            send_in: 0.0,
            accumulator: 0.0,
            elapsed: 0.0,
            last_snapshot_at: 0.0,
            error_until: 0.0,
        })
    }
    pub fn run(&self) -> u64 {
        self.snapshot.as_ref().map_or(1, |s| s.run)
    }
    pub fn snapshot_age(&self) -> f32 {
        self.elapsed - self.last_snapshot_at
    }
    pub fn input(&mut self, axis: [f32; 2], yaw: f32, pitch: f32, hold: bool) {
        self.input_sequence += 1;
        self.latest = Input {
            run: self.run(),
            sequence: self.input_sequence,
            axis,
            yaw,
            pitch,
            hold,
        };
    }
    pub fn command(&mut self, action: Action, layout: &Layout, tuning: &Tuning) {
        if self.closed {
            return;
        }
        self.action_sequence += 1;
        let run = self.run();
        match &mut self.side {
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
                                eprintln!("NET disconnected player={id}; satchel={:?}", session.satchel);
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
                            } = message
                            {
                                let result = if version != fingerprint() {
                                    Err(
                                        "Different gameplay build/configuration; use the same checkout and Cargo.lock."
                                            .to_owned(),
                                    )
                                } else if seed != tuning.seed {
                                    Err(format!("Seed mismatch: restart with --seed {}", tuning.seed))
                                } else {
                                    session.add_player(*next_id, layout, tuning)
                                };
                                match result {
                                    Ok(()) => {
                                        let id = *next_id;
                                        *next_id += 1;
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
                if send {
                    incoming.push(ServerMessage::Snapshot(session.snapshot(HOST, layout, tuning)));
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
