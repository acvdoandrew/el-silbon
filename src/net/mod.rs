//! Two-player development session. Transport and hidden simulation are separate
//! from this application's local camera, UI and audio presentation.
pub mod protocol;
mod render_smoke;
pub mod session;
pub mod smoke;
pub mod transport;

use crate::{
    app::{EncounterMsg, Flow, GameSet, Launch, LayoutRes, RestartRequest, Truth, TuningRes, WhistleMsg},
    encounter::CurrentTarget,
    perception::{WhistlePhrase, WhistleVariant},
    player::{CurrentIntent, Player},
    sim::{Encounter, Presence, Stats, ThreatState},
    world::{Palette, SatchelAsset},
};
use bevy::prelude::*;
use protocol::{Action, HOST, Satchel, ServerMessage};
use transport::Endpoint;

#[derive(Resource, Default)]
pub struct Network {
    pub enabled: bool,
    pub endpoint: Option<Endpoint>,
    pub error: String,
    applied_run: u64,
    last_serial: u64,
    roster: std::collections::BTreeMap<u64, Entity>,
}
impl Network {
    pub fn snapshot(&self) -> Option<&protocol::Snapshot> {
        self.endpoint.as_ref().and_then(|e| e.snapshot.as_ref())
    }
    pub fn id(&self) -> Option<u64> {
        self.endpoint.as_ref().and_then(|e| e.id)
    }
    pub fn carrying(&self) -> bool {
        self.snapshot().is_some_and(|s| {
            Some(match s.satchel {
                Satchel::Carried(id) => id,
                _ => 0,
            }) == self.id()
        })
    }
    pub fn caught(&self) -> bool {
        self.snapshot()
            .is_some_and(|s| s.players.iter().any(|p| Some(p.id) == self.id() && p.caught))
    }
    pub fn active(&self) -> bool {
        self.snapshot()
            .is_some_and(|s| s.started && !protocol::objective(s.objective).is_over())
            && !self.caught()
    }
}
#[derive(Message)]
pub enum NetControl {
    Action(Action),
    Leave,
}
#[derive(Component)]
struct RemotePlayer(u64);
#[derive(Component)]
struct RemoteBag(u64);
#[derive(Component)]
struct NetBanner;
#[derive(Resource)]
struct AvatarAssets {
    body: Handle<Mesh>,
    head: Handle<Mesh>,
    amber: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
}

pub struct NetworkPlugin;
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let launch = app.world().resource::<Launch>().clone();
        let mut net = Network {
            enabled: launch.network.is_some(),
            ..default()
        };
        if let Some(mode) = launch.network {
            match Endpoint::new(
                mode,
                &app.world().resource::<LayoutRes>().0,
                &app.world().resource::<TuningRes>().0,
            ) {
                Ok(e) => net.endpoint = Some(e),
                Err(e) => net.error = e,
            }
        }
        app.insert_resource(net).add_message::<NetControl>();
        if app.world().resource::<Network>().enabled {
            app.add_systems(Startup, setup)
                .add_systems(Update, net_keys.in_set(GameSet::Control))
                .add_systems(Update, update.in_set(GameSet::Simulate))
                .add_systems(Update, (avatars, banner).in_set(GameSet::Present));
            if launch.net_smoke {
                app.add_plugins(render_smoke::NetworkSmokePlugin);
            }
        }
    }
}
pub fn offline(launch: Res<Launch>) -> bool {
    launch.network.is_none()
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    mut next: ResMut<NextState<Flow>>,
) {
    commands.insert_resource(AvatarAssets {
        body: meshes.add(Capsule3d::new(0.25, 0.95)),
        head: meshes.add(Sphere::new(0.19)),
        amber: mats.add(Color::srgb(0.8, 0.43, 0.12)),
        blue: mats.add(Color::srgb(0.15, 0.65, 0.82)),
    });
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
    ));
    next.set(Flow::Playing);
}
fn net_keys(keys: Res<ButtonInput<KeyCode>>, mut controls: MessageWriter<NetControl>) {
    if keys.just_pressed(KeyCode::Enter) {
        controls.write(NetControl::Action(Action::Start));
    }
    if keys.just_pressed(KeyCode::KeyG) {
        controls.write(NetControl::Action(Action::Drop));
    }
    if keys.just_pressed(KeyCode::F6) || keys.just_pressed(KeyCode::KeyR) {
        controls.write(NetControl::Action(Action::Restart));
    }
    if keys.just_pressed(KeyCode::F10) {
        controls.write(NetControl::Leave);
    }
}
fn update(
    time: Res<Time<Real>>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    state: Res<State<Flow>>,
    mut net: ResMut<Network>,
    mut controls: MessageReader<NetControl>,
    intent: Res<CurrentIntent>,
    target: Res<CurrentTarget>,
    mut player: Single<(&mut Player, &mut Transform)>,
    mut truth: ResMut<Truth>,
    mut next: ResMut<NextState<Flow>>,
    mut reset: MessageWriter<RestartRequest>,
    mut events: MessageWriter<EncounterMsg>,
    mut whistles: MessageWriter<WhistleMsg>,
) {
    let active = net.active() && *state.get() == Flow::Playing;
    let Network {
        endpoint,
        applied_run,
        last_serial,
        ..
    } = &mut *net;
    let Some(endpoint) = endpoint else {
        next.set(Flow::Paused);
        return;
    };
    let pose = player.0.pose;
    endpoint.input(
        if active {
            intent.0.move_axis.to_array()
        } else {
            [0.0; 2]
        },
        pose.yaw,
        pose.pitch,
        active && intent.0.interact_held,
    );
    if active
        && intent.0.interact_pressed
        && target
            .0
            .is_some_and(|t| t.ready() && t.kind == crate::control::TargetKind::Satchel)
    {
        endpoint.command(Action::Take, &layout.0, &tuning.0);
    }
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
    endpoint.update(time.delta_secs(), &layout.0, &tuning.0);
    if endpoint.closed {
        if *applied_run != 0 {
            *applied_run = 0;
            reset.write(RestartRequest);
        }
        truth.encounter = Encounter::new(&layout.0);
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
        reset.write(RestartRequest);
        truth.encounter = Encounter::new(&layout.0);
        next.set(Flow::Playing);
        info!(
            "NET applied run {} local={:?} roster={}",
            s.run,
            endpoint.id,
            s.players.len()
        );
    }
    let enc = &mut truth.encounter;
    enc.objective = protocol::objective(s.objective);
    enc.restitution = s.restitution;
    enc.restituting = s.restituting;
    enc.elapsed = s.elapsed;
    enc.stats = Stats {
        warnings: s.stats[0],
        hunts: s.stats[1],
        recoveries: s.stats[2],
    };
    enc.threat.presence = Presence::Hidden;
    enc.threat.pos = Vec2::ZERO;
    enc.threat.speed = 0.0;
    enc.threat.state = match s.danger {
        1 => ThreatState::Warning,
        2 | 3 => ThreatState::Hunting,
        _ => ThreatState::Dormant,
    };
    enc.threat.has_sight = matches!(s.danger, 1 | 2);
    enc.threat.exposure = s.exposure;
    if let Some(visible) = s.threat {
        enc.threat.pos = Vec2::from_array(visible.position);
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
    if let Some(local) = s.players.iter().find(|p| Some(p.id) == endpoint.id) {
        let position = Vec2::from_array(local.position);
        let k = if changed {
            1.0
        } else {
            (time.delta_secs() * 24.0).min(1.0)
        };
        player.0.pose.pos = player.0.pose.pos.lerp(position, k);
        if changed {
            player.0.pose.yaw = local.yaw;
            player.0.pose.pitch = local.pitch;
        }
        *player.1 = crate::player::eye_transform(&player.0.pose, &tuning.0);
    }
    if enc.objective.is_over() {
        next.set(Flow::Outcome);
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
                    gain: variant.gain(&tuning.0),
                    speed: *speed,
                    seeming_closeness: 0.0,
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
fn avatars(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut net: ResMut<Network>,
    assets: Res<AvatarAssets>,
    satchel: Res<SatchelAsset>,
    palette: Res<Palette>,
    mut roots: Query<(&RemotePlayer, &mut Transform, &mut Visibility)>,
    mut bags: Query<(&RemoteBag, &mut Visibility), Without<RemotePlayer>>,
) {
    let Network { endpoint, roster, .. } = &mut *net;
    let local = endpoint.as_ref().and_then(|e| e.id);
    let snapshot = endpoint.as_ref().and_then(|e| e.snapshot.as_ref());
    let players = snapshot.map_or(&[][..], |s| s.players.as_slice());
    let carried = snapshot.map(|s| s.satchel);
    roster.retain(|id, entity| {
        let keep = players.iter().any(|p| p.id == *id && Some(p.id) != local);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for p in players.iter().filter(|p| Some(p.id) != local) {
        roster.entry(p.id).or_insert_with(|| {
            let material = if p.id == HOST { &assets.amber } else { &assets.blue };
            commands
                .spawn((
                    RemotePlayer(p.id),
                    Name::new(format!("remote player {}", p.id)),
                    Mesh3d(assets.body.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(p.position[0], 0.85, p.position[1]),
                    Visibility::Inherited,
                ))
                .with_children(|root| {
                    root.spawn((
                        Mesh3d(assets.head.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.0, 0.85, 0.0),
                    ));
                    root.spawn((
                        RemoteBag(p.id),
                        Mesh3d(satchel.sack.clone()),
                        MeshMaterial3d(palette.burlap.clone()),
                        Transform::from_xyz(0.4, -0.2, -0.2).with_scale(Vec3::splat(0.7)),
                        Visibility::Hidden,
                    ));
                })
                .id()
        });
    }
    for (id, mut transform, mut vis) in &mut roots {
        if let Some(p) = players.iter().find(|p| p.id == id.0) {
            let to = Vec3::new(p.position[0], 0.85, p.position[1]);
            transform.translation = if transform.translation.distance(to) > 4.0 {
                to
            } else {
                transform.translation.lerp(to, (time.delta_secs() * 18.0).min(1.0))
            };
            transform.rotation = transform
                .rotation
                .slerp(Quat::from_rotation_y(p.yaw), (time.delta_secs() * 18.0).min(1.0));
            *vis = if p.caught {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
    for (bag, mut vis) in &mut bags {
        *vis = if carried == Some(Satchel::Carried(bag.0)) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn banner(
    net: Res<Network>,
    flow: Res<State<Flow>>,
    banner: Single<(&mut Text, &mut Node), With<NetBanner>>,
    mut buffer: Local<String>,
) {
    use std::fmt::Write;
    let (mut text, mut node) = banner.into_inner();
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
    let role = if e.mode.is_host() {
        "HOST P1 (amber)"
    } else {
        "CLIENT (blue)"
    };
    if e.closed {
        buffer.push_str(&e.status);
    } else if let Some(s) = &e.snapshot {
        let state = if !s.started {
            "Lobby: host presses Enter when both players are connected"
        } else if s.objective == 3 {
            "SHARED VICTORY: the bones are home and a survivor reached the road."
        } else if s.objective == 4 {
            "SHARED FAILURE: everyone remaining was caught. The host can restart."
        } else if net.caught() {
            "CAUGHT: inactive until host restarts. Your teammate can recover the satchel."
        } else {
            "G: put satchel down | Esc: local menu (world continues)"
        };
        let _ = write!(
            &mut *buffer,
            "{role} | player {} | {}/2 connected | run {}\n{state}\nF6/R: host restart | F10: {}",
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
