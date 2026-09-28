//! First-person body: camera, flashlight, device input → intent, and the
//! shared look/collision movement from `control`.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::NotShadowCaster;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::app::{Flow, GameSet, Launch, LayoutRes, RestartRequest, Settings, Truth, TuningRes};
use crate::control::{Intent, Pose};
use crate::world::{CarriedSatchel, Palette, SatchelAsset};

/// Luminous power of the flashlight (lumens, as Bevy measures spot lights).
pub const FLASHLIGHT_LUMENS: f32 = 380_000.0;

#[derive(Component)]
pub struct Player {
    pub pose: Pose,
}

#[derive(Component)]
pub struct Flashlight {
    pub on: bool,
}

/// This frame's intent (from devices, or from the debug route).
#[derive(Resource, Default)]
pub struct CurrentIntent(pub Intent);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentIntent>()
            .add_systems(Startup, spawn_player.after(crate::world::spawn_world))
            .add_systems(
                Update,
                (
                    reset_player.in_set(GameSet::Control),
                    read_devices
                        .in_set(GameSet::Control)
                        .after(reset_player)
                        .run_if(in_state(Flow::Playing))
                        .run_if(|launch: Res<Launch>| !launch.smoke && !launch.net_smoke),
                    apply_motion.in_set(GameSet::Motion).run_if(in_state(Flow::Playing)),
                ),
            );
    }
}

pub(crate) fn eye_transform(pose: &Pose, tuning: &crate::tuning::Tuning) -> Transform {
    Transform::from_translation(pose.eye(tuning)).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        pose.yaw,
        pose.pitch,
        0.0,
    ))
}

fn spawn_player(
    mut commands: Commands,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    satchel: Res<SatchelAsset>,
    palette: Res<Palette>,
) {
    let pose = Pose::spawn(&layout.0);
    let fog = crate::world::land::HORIZON;
    commands
        .spawn((
            Name::new("player camera"),
            Player { pose },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 68.0_f32.to_radians(),
                near: 0.05,
                far: 1000.0,
                ..default()
            }),
            bevy::camera::Hdr,
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: 0.1,
                ..Bloom::NATURAL
            },
            DistanceFog {
                color: Color::linear_rgba(fog[0], fog[1], fog[2], 1.0),
                directional_light_color: Color::srgba(0.42, 0.48, 0.66, 0.3),
                directional_light_exponent: 22.0,
                falloff: FogFalloff::from_visibility_squared(82.0),
            },
            eye_transform(&pose, &tuning.0),
        ))
        .with_children(|cam| {
            cam.spawn((
                Name::new("flashlight"),
                Flashlight { on: true },
                SpotLight {
                    color: Color::srgb(1.0, 0.93, 0.82),
                    intensity: FLASHLIGHT_LUMENS,
                    range: 38.0,
                    radius: 0.02,
                    inner_angle: 0.2,
                    outer_angle: 0.42,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(0.18, -0.14, -0.05),
            ));
            cam.spawn((
                Name::new("carried satchel"),
                CarriedSatchel,
                Mesh3d(satchel.sack.clone()),
                MeshMaterial3d(palette.burlap.clone()),
                Transform::from_xyz(-0.34, -0.43, -0.6)
                    .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.9, -0.35, 0.2))
                    .with_scale(Vec3::splat(0.9)),
                Visibility::Hidden,
                NotShadowCaster,
                children![(
                    Mesh3d(satchel.bones.clone()),
                    MeshMaterial3d(palette.bone.clone()),
                    Transform::IDENTITY,
                    NotShadowCaster,
                )],
            ));
        });
}

fn read_devices(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    mut intent: ResMut<CurrentIntent>,
) {
    let mut axis = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        axis.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        axis.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        axis.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        axis.x -= 1.0;
    }
    let k = tuning.0.mouse_radians_per_count * settings.sensitivity;
    intent.0 = Intent {
        move_axis: axis.normalize_or_zero(),
        look_delta: Vec2::new(motion.delta.x * k, -motion.delta.y * k),
        interact_pressed: keys.just_pressed(KeyCode::KeyE) || mouse.just_pressed(MouseButton::Left),
        interact_held: keys.pressed(KeyCode::KeyE) || mouse.pressed(MouseButton::Left),
        toggle_flashlight: keys.just_pressed(KeyCode::KeyF),
    };
}

fn apply_motion(
    time: Res<Time>,
    intent: Res<CurrentIntent>,
    truth: Res<Truth>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    player: Single<(&mut Player, &mut Transform)>,
    net: Res<crate::net::Network>,
    flashlight: Single<(&mut Flashlight, &mut SpotLight)>,
) {
    let tuning = &tuning.0;
    let dt = time.delta_secs().min(tuning.max_step);
    let (mut player, mut tf) = player.into_inner();
    let speed = truth.encounter.player_speed(tuning);
    if !net.enabled || !net.caught() {
        player.pose.look(intent.0.look_delta, tuning);
    }
    if !net.enabled {
        player.pose.walk(&layout.0, tuning, intent.0.move_axis, speed, dt);
    }
    *tf = eye_transform(&player.pose, tuning);

    if intent.0.toggle_flashlight {
        let (mut torch, mut spot) = flashlight.into_inner();
        torch.on = !torch.on;
        spot.intensity = if torch.on { FLASHLIGHT_LUMENS } else { 0.0 };
    }
}

pub(crate) fn reset_player(
    mut requests: MessageReader<RestartRequest>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    player: Single<(&mut Player, &mut Transform)>,
    flashlight: Single<(&mut Flashlight, &mut SpotLight)>,
    mut intent: ResMut<CurrentIntent>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let (mut player, mut tf) = player.into_inner();
    player.pose = Pose::spawn(&layout.0);
    *tf = eye_transform(&player.pose, &tuning.0);
    let (mut torch, mut spot) = flashlight.into_inner();
    torch.on = true;
    spot.intensity = FLASHLIGHT_LUMENS;
    intent.0 = Intent::default();
}
