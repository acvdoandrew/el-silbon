//! The client adapter: one path for solo, hosted and joined play. The
//! transport and the hidden simulation stay independent of this
//! application's camera, HUD and audio; this module feeds local input to the
//! endpoint and mirrors each snapshot into what the presentation may know.
//!
//! The rules of one client frame live here as plain functions so the game,
//! the smoke drivers and the session tests all play by them: `controls_live`
//! gates the controls, `Wire` is what a frame sends, `follow_body` is how the
//! local body trails its snapshot row and `mirror` is what the presentation
//! may know of the encounter.
pub mod protocol;
mod render_smoke;
pub mod session;
pub mod smoke;
pub mod transport;

use crate::{
    app::{EncounterMsg, Flow, GameSet, Launch, LayoutRes, RunReset, Truth, TuningRes, WhistleMsg},
    control::{Intent, Pose, Target, TargetKind},
    encounter::CurrentTarget,
    geometry::Layout,
    perception::{WhistlePhrase, WhistleVariant},
    player::{CurrentIntent, LightOn, Player},
    sim::{Encounter, Outcome, Presence, Stats, ThreatState},
    tuning::{Night, Tuning},
};
use bevy::prelude::*;
use protocol::{Action, HOST, Input, PlayerId, PlayerView, ServerMessage, Snapshot};
use transport::{Endpoint, Mode};

/// Begin a night from the menu: how (solo, hosting, joining), which night
/// (the seed) and how hard. Applied at once, in this process.
#[derive(Message, Clone, Debug)]
pub struct StartRun {
    pub mode: Mode,
    pub seed: u64,
    pub night: crate::tuning::Night,
}

/// Leave the night for the title screen (a shared session is left, or ended
/// for everyone by its host).
#[derive(Message, Clone, Copy, Debug)]
pub struct LeaveRun;

/// 0 active, 1 downed, 2 dead (active until the first snapshot).
pub fn status_of(snapshot: Option<&Snapshot>, me: Option<PlayerId>) -> u8 {
    snapshot
        .zip(me)
        .and_then(|(s, id)| s.player(id))
        .map_or(0, |p| p.status)
}

/// Whether this player's controls reach the world: the run is on, they are
/// not dead, the game is playing and no susto holds them frozen. When not,
/// the packet a frame sends carries no movement and it sends no commands.
pub fn controls_live(snapshot: Option<&Snapshot>, me: Option<PlayerId>, playing: bool) -> bool {
    snapshot.is_some_and(|s| {
        playing && s.started && !s.outcome().is_over() && status_of(Some(s), me) != 2 && s.me.stun <= 0.0
    })
}

/// What one client frame puts on the wire: the input packet (orientation as
/// this frame left it), then the presses of the intent, in order: interact
/// only where the crosshair is on a bundle or pepper, drop, pepper, and a
/// mark of what the crosshair holds. The game and every smoke driver build
/// their frames here; only explicit session commands (start, restart, a
/// scripted mark) follow them. Run and sequence are stamped by the sender.
pub struct Wire {
    pub input: Input,
    commands: [Option<Action>; 7],
}

impl Wire {
    pub fn new(
        intent: &Intent,
        live: bool,
        pose: &Pose,
        light: bool,
        target: Option<Target>,
        layout: &Layout,
        tuning: &Tuning,
    ) -> Self {
        let input = Input {
            axis: if live { intent.move_axis.to_array() } else { [0.0; 2] },
            yaw: pose.yaw,
            pitch: pose.pitch,
            hold: live && intent.interact_held,
            crouch: live && intent.crouch,
            sprint: live && intent.sprint,
            light,
            ..default()
        };
        if !live {
            return Self {
                input,
                commands: [None; 7],
            };
        }
        let pickup = intent.interact_pressed
            && target.filter(|t| t.usable()).is_some_and(|t| {
                matches!(
                    t.kind,
                    TargetKind::Relic(_) | TargetKind::Aji(_) | TargetKind::Batteries(_) | TargetKind::Panel
                )
            });
        let mark = intent.ping.then(|| {
            let at = target
                .map(|t| t.center)
                .unwrap_or_else(|| layout.ray_ground(pose.eye(tuning, layout), pose.look_dir(), tuning.ping_range));
            Action::Ping { at: at.to_array() }
        });
        Self {
            input,
            commands: [
                pickup.then_some(Action::Interact),
                intent.drop.then_some(Action::Drop),
                intent.use_aji.then_some(Action::UseAji),
                mark,
                intent.skill.map(|(id, needle)| Action::Skill { id, needle }),
                intent.code.map(|code| Action::TryCode { code }),
                intent.name.map(|variant| Action::Name { variant }),
            ],
        }
    }

    /// The commands that follow the input packet, in order.
    pub fn commands(&self) -> impl Iterator<Item = Action> + '_ {
        self.commands.iter().flatten().copied()
    }

    /// Hand the frame to an endpoint, which stamps run and sequence.
    pub fn send(&self, endpoint: &mut Endpoint, layout: &Layout, tuning: &Tuning) {
        endpoint.input(self.input);
        for action in self.commands() {
            endpoint.command(action, layout, tuning);
        }
    }
}

/// The local body follows its own row of the newest snapshot as the eye does:
/// position and stance ease toward the authority's (`dt` seconds on the
/// session's clock); the player's own look is only taken from it when a new
/// run begins (`fresh`).
pub fn follow_body(pose: &mut Pose, local: &PlayerView, fresh: bool, dt: f32, tuning: &Tuning) {
    let position = Vec2::from_array(local.position);
    let k = if fresh { 1.0 } else { (dt * 24.0).min(1.0) };
    pose.pos = pose.pos.lerp(position, k);
    if fresh {
        pose.yaw = local.yaw;
        pose.pitch = local.pitch;
    }
    let want_lower = match local.status {
        0 if local.crouch => tuning.crouch_lower,
        0 => 0.0,
        _ => tuning.downed_lower,
    };
    pose.lower += (want_lower - pose.lower) * (dt * 9.0).min(1.0);
    if fresh {
        pose.lower = want_lower;
    }
}

/// The skill check this player is being asked for, and where its needle is
/// now: the host's needle in the newest snapshot, carried on by the seconds
/// since it arrived.
pub fn needle(snap: &Snapshot, age: f32, tuning: &Tuning) -> Option<(u32, f32)> {
    snap.me.check.map(|c| (c.id, c.needle + age / tuning.check_sweep))
}

/// The presentation's mirror of the encounter, from a snapshot alone: the
/// outcome, the clock, the tallies and the danger this player feels. Where
/// the threat stands comes only from what the player can see.
pub fn mirror(enc: &mut Encounter, s: &Snapshot) {
    enc.outcome = s.outcome();
    enc.elapsed = s.elapsed;
    enc.stats = Stats {
        warnings: s.stats[0],
        hunts: s.stats[1],
        recoveries: s.stats[2],
        downs: s.stats[3],
        revives: s.stats[4],
    };
    enc.threat.presence = Presence::Hidden;
    enc.threat.speed = 0.0;
    enc.threat.state = match s.danger {
        1 => ThreatState::Warning,
        2 | 3 => ThreatState::Hunting,
        4 => ThreatState::Counting,
        _ => ThreatState::Dormant,
    };
    enc.threat.has_sight = matches!(s.danger, 1 | 2);
    enc.threat.exposure = s.exposure;
}

#[derive(Resource, Default)]
pub struct Network {
    pub endpoint: Option<Endpoint>,
    pub error: String,
    applied_run: u64,
    last_serial: u64,
}

impl Network {
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.endpoint.as_ref().and_then(|e| e.snapshot.as_ref())
    }
    pub fn id(&self) -> Option<u64> {
        self.endpoint.as_ref().and_then(|e| e.id)
    }
    /// This player's row in the latest snapshot.
    pub fn me(&self) -> Option<&protocol::PlayerView> {
        let id = self.id()?;
        self.snapshot()?.player(id)
    }
    /// 0 active, 1 downed, 2 dead (active until the first snapshot).
    pub fn status(&self) -> u8 {
        status_of(self.snapshot(), self.id())
    }
    pub fn carrying(&self) -> usize {
        self.me().map_or(0, |p| p.carrying as usize)
    }
    /// The run is on: started and not over.
    pub fn running(&self) -> bool {
        self.snapshot().is_some_and(|s| s.started && !s.outcome().is_over())
    }
    /// On their feet and the run is on.
    pub fn active(&self) -> bool {
        self.running() && self.status() == 0
    }
    pub fn is_solo(&self) -> bool {
        self.endpoint.as_ref().is_some_and(|e| e.mode.is_solo())
    }
    pub fn is_shared(&self) -> bool {
        self.endpoint.as_ref().is_some_and(|e| !e.mode.is_solo())
    }
    pub fn stunned(&self) -> bool {
        self.snapshot().is_some_and(|s| s.me.stun > 0.0)
    }
    /// The skill check in flight for this player and its needle now.
    pub fn needle(&self, tuning: &Tuning) -> Option<(u32, f32)> {
        let e = self.endpoint.as_ref()?;
        needle(e.snapshot.as_ref()?, e.snapshot_age(), tuning)
    }
}

#[derive(Message)]
pub enum NetControl {
    Action(Action),
    Leave,
}

#[derive(Component)]
pub(crate) struct RemotePlayer(u64);
#[derive(Component)]
struct NetBanner;

pub struct NetworkPlugin;
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let launch = app.world().resource::<Launch>().clone();
        let mut net = Network::default();
        // From the title screen, the menu opens the session when a night
        // begins; otherwise the command line has already chosen it.
        if !launch.menu {
            match Endpoint::new(
                launch.network.clone(),
                &app.world().resource::<LayoutRes>().0,
                &app.world().resource::<TuningRes>().0,
            ) {
                Ok(e) => net.endpoint = Some(e),
                Err(e) => net.error = e,
            }
        }
        app.insert_resource(net)
            .add_message::<NetControl>()
            .add_message::<StartRun>()
            .add_message::<LeaveRun>()
            .add_systems(Startup, setup)
            .add_systems(Update, (net_keys, begin_or_leave).chain().in_set(GameSet::Control))
            .add_systems(Update, update.in_set(GameSet::Simulate))
            .add_systems(Update, (avatars, banner).in_set(GameSet::Present));
        if launch.net_smoke {
            app.add_plugins(render_smoke::NetworkSmokePlugin);
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, launch: Res<Launch>, mut next: ResMut<NextState<Flow>>) {
    // The session banner exists always; it shows only in shared play.
    commands.spawn((
        NetBanner,
        Text::new("Connecting…"),
        TextFont {
            font: assets.load::<Font>("fonts/NotoSans-Regular.ttf").into(),
            font_size: bevy::text::FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(80),
            left: px(24),
            max_width: percent(85),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.015, 0.02, 0.92)),
        GlobalZIndex(100),
        Visibility::Hidden,
    ));
    if !launch.menu && !launch.network.is_solo() {
        next.set(Flow::Playing);
    }
}

/// A night begins (from the menu) or is left (back to the title).
#[allow(clippy::too_many_arguments)]
fn begin_or_leave(
    mut starts: MessageReader<StartRun>,
    mut leaves: MessageReader<LeaveRun>,
    mut net: ResMut<Network>,
    mut launch: ResMut<Launch>,
    mut layout: ResMut<LayoutRes>,
    mut tuning: ResMut<TuningRes>,
    mut truth: ResMut<Truth>,
    mut next: ResMut<NextState<Flow>>,
    mut reset: MessageWriter<RunReset>,
) {
    let fresh = |net: &mut Network, truth: &mut Truth, layout: &Layout, reset: &mut MessageWriter<RunReset>| {
        net.applied_run = 0;
        net.last_serial = 0;
        truth.encounter = Encounter::new(layout);
        reset.write(RunReset);
    };
    if leaves.read().count() > 0 {
        if let Some(e) = &mut net.endpoint {
            e.close(if e.mode.is_host() {
                "The host ended the night."
            } else {
                "You left the session."
            });
        }
        net.endpoint = None;
        net.error.clear();
        launch.network = Mode::Solo;
        fresh(&mut net, &mut truth, &layout.0, &mut reset);
        next.set(Flow::Title);
    }
    // The host answered with its own night: take it and knock again, once.
    let adopt = net.endpoint.as_ref().and_then(|e| match (&e.mode, e.adopt) {
        (Mode::Join(_), Some((seed, code))) if (seed, code) != (tuning.0.seed, tuning.0.night.code()) => {
            Night::from_code(code).map(|night| StartRun {
                mode: e.mode.clone(),
                seed,
                night,
            })
        }
        _ => None,
    });
    let Some(start) = starts.read().last().cloned().or(adopt) else {
        return;
    };
    if let Some(e) = &mut net.endpoint {
        e.close("You left the session.");
    }
    // The night: its seed moves the bundles, sets the padlock and chooses
    // which of him walks; its difficulty scales the rules.
    launch.seed = start.seed;
    launch.night = start.night;
    launch.network = start.mode.clone();
    tuning.0 = Tuning::with_seed(start.seed).with_night(start.night);
    layout.0 = Layout::with_seed(start.seed);
    match Endpoint::new(start.mode.clone(), &layout.0, &tuning.0) {
        Ok(e) => {
            net.endpoint = Some(e);
            net.error.clear();
        }
        Err(e) => {
            net.endpoint = None;
            net.error = e;
            launch.network = Mode::Solo;
            return;
        }
    }
    fresh(&mut net, &mut truth, &layout.0, &mut reset);
    next.set(if start.mode.is_solo() {
        Flow::Briefing
    } else {
        Flow::Playing
    });
}

fn net_keys(keys: Res<ButtonInput<KeyCode>>, launch: Res<Launch>, mut controls: MessageWriter<NetControl>) {
    if launch.smoke || launch.net_smoke || launch.photos {
        return;
    }
    if keys.just_pressed(KeyCode::Enter) && launch.network.is_host() && !launch.network.is_solo() {
        controls.write(NetControl::Action(Action::Start));
    }
    // R on the outcome screen is the app's (`app::pause_keys`).
    if keys.just_pressed(KeyCode::F6) && launch.network.is_host() {
        controls.write(NetControl::Action(Action::Restart));
    }
    if keys.just_pressed(KeyCode::F10) && !launch.network.is_solo() {
        controls.write(NetControl::Leave);
    }
}

fn update(
    (time, real): (Res<Time>, Res<Time<Real>>),
    mut outcome_wait: Local<f32>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    state: Res<State<Flow>>,
    mut net: ResMut<Network>,
    mut controls: MessageReader<NetControl>,
    intent: Res<CurrentIntent>,
    light: Res<LightOn>,
    target: Res<CurrentTarget>,
    mut player: Single<(&mut Player, &mut Transform)>,
    mut truth: ResMut<Truth>,
    mut next: ResMut<NextState<Flow>>,
    mut reset: MessageWriter<RunReset>,
    mut events: MessageWriter<EncounterMsg>,
    mut whistles: MessageWriter<WhistleMsg>,
) {
    let playing = *state.get() == Flow::Playing;
    let solo = net.is_solo();
    let live = controls_live(net.snapshot(), net.id(), playing);
    let Network {
        endpoint,
        applied_run,
        last_serial,
        ..
    } = &mut *net;
    let Some(endpoint) = endpoint else {
        // The title screen has no night yet (nor a night just left for it);
        // anywhere else, no session is a fault.
        let title = *state.get() == Flow::Title || matches!(*next, NextState::Pending(Flow::Title));
        if !title {
            next.set(Flow::Paused);
        }
        return;
    };
    let pose = player.0.pose;
    let wire = Wire::new(&intent.0, live, &pose, light.0, target.0, &layout.0, &tuning.0);
    wire.send(endpoint, &layout.0, &tuning.0);
    for command in controls.read() {
        match command {
            NetControl::Action(a) => endpoint.command(*a, &layout.0, &tuning.0),
            NetControl::Leave => endpoint.close(if endpoint.mode.is_host() {
                "Session ended by host. Relaunch to host again."
            } else {
                "You left the session. Relaunch to join another session."
            }),
        }
    }
    // A paused solo run stands still; shared worlds keep going.
    let dt = if solo {
        if playing { time.delta_secs() } else { 0.0 }
    } else {
        real.delta_secs()
    };
    endpoint.update(dt, &layout.0, &tuning.0);
    if endpoint.closed {
        if *applied_run != 0 {
            *applied_run = 0;
            reset.write(RunReset);
        }
        truth.encounter.outcome = Outcome::Running;
        next.set(Flow::Paused);
        return;
    }
    let Some(s) = &endpoint.snapshot else {
        return;
    };
    let changed = *applied_run != s.run;
    if changed {
        *applied_run = s.run;
        *last_serial = 0;
        reset.write(RunReset);
        truth.encounter = Encounter::new(&layout.0);
        // A new run is played at once, restarted from the outcome screen or
        // the menu alike; only solo's opening briefing (its lobby) waits.
        let briefing = *state.get() == Flow::Briefing || matches!(*next, NextState::Pending(Flow::Briefing));
        if !(solo && briefing) {
            next.set(Flow::Playing);
        }
        info!(
            "NET applied run {} local={:?} roster={}",
            s.run,
            endpoint.id,
            s.players.len()
        );
    }
    mirror(&mut truth.encounter, s);
    let enc = &mut truth.encounter;
    if let Some(visible) = s.threat {
        let target_pos = Vec2::from_array(visible.position);
        // The wire runs at 20 Hz; smooth what the eye sees.
        enc.threat.pos = if changed || enc.threat.pos.distance(target_pos) > 6.0 {
            target_pos
        } else {
            enc.threat.pos.lerp(target_pos, (real.delta_secs() * 14.0).min(1.0))
        };
        enc.threat.facing = Vec2::from_array(visible.facing);
        enc.threat.speed = visible.speed;
        if s.danger == 0 {
            enc.threat.state = protocol::threat_state(visible.state);
        }
        enc.threat.presence = if visible.visibility >= 1.0 {
            Presence::Present
        } else {
            Presence::Rising {
                t: visible.visibility * tuning.0.rise_time,
            }
        };
    }
    if let Some(local) = s.player(endpoint.id.unwrap_or(HOST)) {
        // The body eases on the clock its session runs on: simulated time in
        // solo (so a fixed-step smoke replays exactly), the wall clock shared.
        follow_body(&mut player.0.pose, local, changed, dt, &tuning.0);
        *player.1 = crate::player::eye_transform(&player.0.pose, &tuning.0, &layout.0);
    }
    // The run is over: the outcome screen, once a player who was just
    // caught has had the moment he lunges at them.
    if enc.outcome.is_over() {
        *outcome_wait += real.delta_secs();
        let caught = status_of(Some(s), endpoint.id) != 0;
        let hold = if caught && enc.outcome == crate::sim::Outcome::Failed {
            crate::world::omen::LUNGE + 0.2
        } else {
            0.0
        };
        if *outcome_wait >= hold {
            next.set(Flow::Outcome);
        }
    } else {
        *outcome_wait = 0.0;
    }
    let run = s.run;
    endpoint.notices.retain(|message| {
        if matches!(message, ServerMessage::Cue { run: r, .. } | ServerMessage::Events { run: r, .. } if *r > run) {
            return true;
        }
        match message {
            ServerMessage::Cue {
                run: r,
                serial,
                variant,
                speed,
                phantom,
            } if *r == run && *serial > *last_serial => {
                *last_serial = *serial;
                let variant = match *variant {
                    0 => WhistleVariant::Loud,
                    1 => WhistleVariant::Middling,
                    _ => WhistleVariant::Faint,
                };
                // Deliberately categorical: do not transmit invertible continuous distance.
                whistles.write(WhistleMsg(WhistlePhrase {
                    variant,
                    gain: variant.gain(&tuning.0) * if *phantom { 0.7 } else { 1.0 },
                    speed: *speed,
                    seeming_closeness: 0.0,
                    phantom: *phantom,
                }));
            }
            ServerMessage::Events {
                run: r,
                serial,
                events: codes,
            } if *r == run && *serial > *last_serial => {
                *last_serial = *serial;
                for &code in codes {
                    if let Some(event) = protocol::event(code) {
                        events.write(EncounterMsg(event));
                    }
                }
            }
            _ => {}
        }
        false
    });
}

/// Remote players, drawn from the snapshot: position, yaw, stance, torch and
/// the bundle on their back.
fn avatars(
    mut commands: Commands,
    time: Res<Time<Real>>,
    net: Res<Network>,
    kit: Res<crate::world::avatar::AvatarKit>,
    layout: Res<LayoutRes>,
    mut roster: Local<std::collections::BTreeMap<u64, Entity>>,
    mut roots: Query<(&RemotePlayer, &mut Transform, &mut Visibility, &Children)>,
    mut parts: Query<
        (
            &mut Visibility,
            Has<crate::world::avatar::AvatarBag>,
            Has<crate::world::avatar::AvatarTorch>,
        ),
        Without<RemotePlayer>,
    >,
) {
    let local = net.id();
    let players = net.snapshot().map_or(&[][..], |s| s.players.as_slice());
    roster.retain(|id, entity| {
        let keep = players.iter().any(|p| p.id == *id && Some(p.id) != local);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for (slot, p) in players.iter().enumerate().filter(|(_, p)| Some(p.id) != local) {
        roster.entry(p.id).or_insert_with(|| {
            let at = Vec3::new(
                p.position[0],
                layout.0.surface_height(Vec2::from_array(p.position)),
                p.position[1],
            );
            kit.spawn(
                &mut commands,
                slot,
                format!("remote player {}", p.id),
                RemotePlayer(p.id),
                at,
            )
        });
    }
    for (id, mut transform, mut vis, children) in &mut roots {
        let Some(p) = players.iter().find(|p| p.id == id.0) else {
            continue;
        };
        let pos = Vec2::from_array(p.position);
        let ground = layout.0.surface_height(pos);
        // Standing, crouched, or down on the ground.
        let (lift, squash, roll) = match p.status {
            0 if p.crouch => (0.0, 0.72, 0.0),
            0 => (0.0, 1.0, if p.sprint { 0.16 } else { 0.0 }),
            _ => (0.16, 1.0, 1.35),
        };
        let to = Vec3::new(pos.x, ground + lift, pos.y);
        let k = (time.delta_secs() * 18.0).min(1.0);
        transform.translation = if transform.translation.distance(to) > 4.0 {
            to
        } else {
            transform.translation.lerp(to, k)
        };
        let want = Quat::from_rotation_y(p.yaw) * Quat::from_rotation_x(-roll);
        transform.rotation = transform.rotation.slerp(want, k);
        transform.scale = transform.scale.lerp(Vec3::new(1.0, squash, 1.0), k);
        // The dead are gone, and the one in his sack is inside it.
        *vis = if p.status == 2 || p.hauled {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        for &child in children {
            if let Ok((mut child_vis, bag, torch)) = parts.get_mut(child) {
                if bag {
                    *child_vis = if p.carrying > 0 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                } else if torch {
                    *child_vis = if p.light && p.status == 0 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
}

fn banner(
    net: Res<Network>,
    flow: Res<State<Flow>>,
    banner: Option<Single<(&mut Text, &mut Node, &mut Visibility), With<NetBanner>>>,
    mut buffer: Local<String>,
) {
    use std::fmt::Write;
    let Some(banner) = banner else {
        return;
    };
    let (mut text, mut node, mut vis) = banner.into_inner();
    // Only shared play has a session to report on; the title has its menus.
    let shown = *flow.get() != Flow::Title && net.endpoint.as_ref().is_some_and(|e| !e.mode.is_solo());
    let want = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
    if !shown {
        return;
    }
    // Keep session status visible without covering pause/outcome menu buttons.
    let menu = *flow.get() != Flow::Playing;
    let top = if menu { px(8) } else { Val::Auto };
    let bottom = if menu { Val::Auto } else { px(80) };
    if node.top != top {
        node.top = top;
    }
    if node.bottom != bottom {
        node.bottom = bottom;
    }
    buffer.clear();
    let Some(e) = &net.endpoint else {
        let _ = write!(&mut *buffer, "Cannot open session: {}\nEsc: menu / Quit", net.error);
        if text.0 != *buffer {
            text.0.clone_from(&buffer);
        }
        return;
    };
    let role = if e.mode.is_host() { "HOST" } else { "CLIENT" };
    if e.closed {
        buffer.push_str(&e.status);
    } else if let Some(s) = &e.snapshot {
        let state = if !s.started {
            "Lobby: the host presses Enter when everyone is connected"
        } else if s.outcome == 1 {
            "SHARED VICTORY: the truck is away with everyone still standing."
        } else if s.outcome == 2 {
            "SHARED FAILURE: nobody is left on their feet. The host can restart."
        } else if net.status() == 1 {
            "DOWN: a teammate can revive you (hold E beside you)."
        } else if net.status() == 2 {
            "You bled out. Watch over your teammates."
        } else {
            "V: mark | Q: pepper | G: put a bundle down | Esc: local menu (world continues)"
        };
        let _ = write!(
            &mut *buffer,
            "{role} | player {} | {} connected | run {}\n{state}\nF6: host restart | F10: {}",
            e.id.unwrap_or(0),
            s.players.len(),
            s.run,
            if e.mode.is_host() {
                "end session"
            } else {
                "leave session"
            }
        );
        if !e.last_error.is_empty() {
            let _ = write!(&mut *buffer, "\n{}", e.last_error);
        }
    } else {
        buffer.push_str(&e.status);
    }
    if text.0 != *buffer {
        text.0.clone_from(&buffer);
    }
}
