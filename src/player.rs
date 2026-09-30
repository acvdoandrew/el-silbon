//! First-person body: camera, flashlight, device input → intent, and the
//! look/light handling. Position is owned by the session; this module only
//! turns the head and lifts the eye.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::{FogVolume, NotShadowCaster, NotShadowReceiver, VolumetricFog, VolumetricLight};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::effect_stack::{ChromaticAberration, Vignette};
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};

use crate::app::{Flow, GameSet, Launch, LayoutRes, RunReset, Settings, TuningRes};
use crate::control::{Intent, Pose};
use crate::world::mesh::{MeshBuilder, srgb};
use crate::world::{CarriedSatchel, Palette, SatchelAsset};

/// Luminous power of the flashlight's hot core (lumens, as Bevy measures
/// spot lights: the cone's width does not concentrate it).
pub const FLASHLIGHT_LUMENS: f32 = 380_000.0;
/// The wide, dim spill around the core: the soft ring of a real torch.
const SPILL_LUMENS: f32 = 70_000.0;
/// Bloom between lightning strikes (the flash adds to it).
pub const BLOOM: f32 = 0.12;
/// Where the torch's tail is held (camera space) and the point it aims at:
/// nearly parallel to the view, so it sits low right and the beam meets the
/// centre of view well out.
const TORCH_GRIP: Vec3 = Vec3::new(0.16, -0.24, -0.2);
const TORCH_AIM: Vec3 = Vec3::new(0.0, 0.0, -12.0);
/// Torch body length (m) along its axis; the lens and the beams sit at its head.
const TORCH_LENGTH: f32 = 0.235;
/// Side of the fog volume that travels with the camera (m).
const FOG_VOLUME: f32 = 60.0;

#[derive(Component)]
pub struct Player {
    pub pose: Pose,
}

/// The torch in hand: its on/off state, and how it sways and bobs.
#[derive(Component)]
pub struct Flashlight {
    pub on: bool,
    /// Smoothed look-sway offset (yaw, pitch; radians).
    sway: Vec2,
}

/// A light of the torch and the luminous power it has when on.
#[derive(Component)]
struct Beam(f32);

/// The torch lens: glows while the torch is on.
#[derive(Resource)]
struct TorchLens(Handle<StandardMaterial>);

/// The walking gait, shared by the head bob and the torch: the stride phase
/// (radians, one step per half turn) and how strongly it shows (0–1).
#[derive(Resource, Default)]
struct Gait {
    phase: f32,
    amount: f32,
}

/// Radians of stride phase per metre walked: a step about every 0.7 m.
const STRIDE_PER_M: f32 = 4.5;
/// Head bob at full pace: rise and fall, side sway (m) and roll (radians).
/// Kept small: a sense of footfall, not a camera shake.
const BOB_RISE: f32 = 0.028;
const BOB_SWAY: f32 = 0.012;
const BOB_ROLL: f32 = 0.005;

/// Fog for the torch beam to scatter in, kept centred on the eye.
#[derive(Component)]
struct TravellingFog;

/// This frame's intent (from devices, or from the debug route).
#[derive(Resource, Default)]
pub struct CurrentIntent(pub Intent);

/// Whether the flashlight is on (the host counts it: light makes you easier to see).
#[derive(Resource)]
pub struct LightOn(pub bool);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentIntent>()
            .init_resource::<TorchHand>()
            .init_resource::<Gait>()
            .insert_resource(LightOn(true))
            .add_systems(Startup, spawn_player.after(crate::world::spawn_world))
            .add_systems(
                Update,
                (view_settings, head_bob, torch_in_hand, torch_light, carry_fog)
                    .chain()
                    .in_set(GameSet::Present)
                    .after(apply_motion),
            )
            .add_systems(
                Update,
                (
                    reset_player.in_set(GameSet::Control),
                    read_devices
                        .in_set(GameSet::Control)
                        .after(reset_player)
                        .run_if(in_state(Flow::Playing))
                        .run_if(|launch: Res<Launch>| !launch.smoke && !launch.net_smoke && !launch.photos),
                    apply_motion.in_set(GameSet::Motion).run_if(in_state(Flow::Playing)),
                ),
            );
    }
}

pub(crate) fn eye_transform(
    pose: &Pose,
    tuning: &crate::tuning::Tuning,
    layout: &crate::geometry::Layout,
) -> Transform {
    Transform::from_translation(pose.eye(tuning, layout)).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        pose.yaw,
        pose.pitch,
        0.0,
    ))
}

/// Watching a friend: how far behind, to the right of and above their eye
/// the view rides (m), and how far it keeps from any wall.
const SHOULDER_BACK: f32 = 2.2;
const SHOULDER_SIDE: f32 = 0.45;
const SHOULDER_RISE: f32 = 0.45;
const SHOULDER_CLEARANCE: f32 = 0.25;

/// Where a watched friend is seen from: behind and over their shoulder,
/// looking where they look. Pulled in toward them wherever a wall would come
/// between, so the view never stands on the far side of a wall from them.
/// (What of him shows is still only what their own eye sees: the session
/// decides that, not this camera.)
pub(crate) fn shoulder_transform(
    pose: &Pose,
    tuning: &crate::tuning::Tuning,
    layout: &crate::geometry::Layout,
) -> Transform {
    let eye = pose.eye(tuning, layout);
    let back = -pose.forward2() * SHOULDER_BACK + pose.right2() * SHOULDER_SIDE;
    let at = [1.0, 0.75, 0.5, 0.3, 0.15]
        .into_iter()
        .map(|k| layout.move_circle(pose.pos, back * k, SHOULDER_CLEARANCE))
        .find(|&spot| layout.line_of_sight(pose.pos, spot))
        .unwrap_or(pose.pos);
    let y = (eye.y + SHOULDER_RISE).max(layout.surface_height(at) + 0.5);
    Transform::from_translation(Vec3::new(at.x, y, at.y)).looking_at(eye + pose.look_dir() * 6.0, Vec3::Y)
}

fn spawn_player(
    mut commands: Commands,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    satchel: Res<SatchelAsset>,
    palette: Res<Palette>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pose = Pose::spawn(&layout.0);
    let fog = crate::world::land::HORIZON;
    let (body, lens) = torch_meshes();
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.75, 0.77),
        metallic: 0.25,
        perceptual_roughness: 0.42,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let lens_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.88, 0.8),
        emissive: torch_glow(true),
        perceptual_roughness: 0.1,
        ..default()
    });
    commands.insert_resource(TorchLens(lens_mat.clone()));
    commands.spawn((
        Name::new("travelling fog"),
        TravellingFog,
        FogVolume {
            fog_color: Color::srgb(0.62, 0.68, 0.8),
            density_factor: 0.012,
            absorption: 0.2,
            scattering: 0.3,
            scattering_asymmetry: 0.55,
            light_intensity: 0.25,
            ..default()
        },
        Transform::from_scale(Vec3::splat(FOG_VOLUME)),
    ));
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
            night_grading(crate::display::grade(0.0, 1.0, fog_level())),
            Bloom {
                intensity: BLOOM,
                ..Bloom::NATURAL
            },
            DistanceFog {
                color: Color::linear_rgba(fog[0], fog[1], fog[2], 1.0),
                directional_light_color: Color::srgba(0.42, 0.48, 0.66, 0.3),
                directional_light_exponent: 22.0,
                falloff: FogFalloff::from_visibility_squared(crate::world::land::fog_visibility(&layout.0)),
            },
            // The torch beam's haze; the fog volume is lit only by volumetric
            // lights, never by the sky fill.
            VolumetricFog {
                ambient_intensity: 0.0,
                // No jitter: without TAA to resolve it, it reads as grain
                // crawling along the beam.
                step_count: 64,
                jitter: 0.0,
                ..default()
            },
            Vignette {
                intensity: 0.55,
                radius: 0.9,
                smoothness: 1.6,
                ..default()
            },
            ChromaticAberration {
                intensity: 0.006,
                ..default()
            },
            eye_transform(&pose, &tuning.0, &layout.0),
        ))
        .with_children(|cam| {
            cam.spawn((
                Name::new("torch in hand"),
                Flashlight {
                    on: true,
                    sway: Vec2::ZERO,
                },
                torch_rest(),
                Visibility::Inherited,
            ))
            .with_children(|torch| {
                // The body lies along the torch's forward (-Z) axis.
                let along = Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
                torch.spawn((
                    Mesh3d(meshes.add(body)),
                    MeshMaterial3d(body_mat),
                    along,
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                torch.spawn((
                    Mesh3d(meshes.add(lens)),
                    MeshMaterial3d(lens_mat),
                    along,
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                let head = Transform::from_xyz(0.0, 0.0, -TORCH_LENGTH - 0.01);
                // A hot core that reaches, scatters in the rain haze...
                torch.spawn((
                    Name::new("flashlight"),
                    Beam(FLASHLIGHT_LUMENS),
                    SpotLight {
                        color: Color::srgb(1.0, 0.93, 0.82),
                        intensity: FLASHLIGHT_LUMENS,
                        range: 38.0,
                        radius: 0.02,
                        inner_angle: 0.1,
                        outer_angle: 0.27,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    VolumetricLight,
                    head,
                ));
                // ...and a wide dim spill around it.
                torch.spawn((
                    Name::new("flashlight spill"),
                    Beam(SPILL_LUMENS),
                    SpotLight {
                        color: Color::srgb(1.0, 0.9, 0.76),
                        intensity: SPILL_LUMENS,
                        range: 20.0,
                        radius: 0.03,
                        inner_angle: 0.22,
                        outer_angle: 0.62,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    head,
                ));
            });
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

/// The level the night's contrast turns about: the fog's (and the sky's at
/// the horizon) linear luminance.
pub(crate) fn fog_level() -> f32 {
    crate::display::luminance(crate::world::land::HORIZON)
}

/// The camera's grade for a picture setting: a dark, cool night that lets
/// the practicals stay warm against it, slightly under-saturated, so colour
/// belongs to the light. Brightness and contrast live in every section's
/// gamma and the exposure (see `display`), never in the sections' contrast,
/// whose pivot at linear 0.5 crushes linear-HDR darks.
pub(crate) fn night_grading(grade: crate::display::Grade) -> ColorGrading {
    ColorGrading::with_identical_sections(
        ColorGradingGlobal {
            exposure: grade.exposure,
            temperature: -0.03,
            post_saturation: 0.9,
            ..default()
        },
        ColorGradingSection {
            gamma: grade.gamma,
            ..default()
        },
    )
}

/// A plain metal torch along +Y from its tail at the origin: ribbed grip,
/// flared head and bezel, plus the lens disc as its own mesh.
fn torch_meshes() -> (Mesh, Mesh) {
    let dark = srgb(0.55, 0.55, 0.57);
    let grip = srgb(0.32, 0.32, 0.33);
    let mut body = MeshBuilder::new();
    let l = TORCH_LENGTH;
    body.lathe(
        Vec3::ZERO,
        &[(0.0, 0.0), (0.013, 0.0), (0.016, 0.006), (0.016, 0.02)],
        20,
        4.0,
        dark,
    );
    // Knurled grip: alternating narrow rings.
    let mut rings = vec![(0.016, 0.02)];
    for i in 0..9 {
        let y = 0.024 + i as f32 * 0.011;
        rings.push((0.0172, y));
        rings.push((0.0172, y + 0.005));
        rings.push((0.0162, y + 0.0062));
        rings.push((0.0162, y + 0.0105));
    }
    body.lathe(Vec3::ZERO, &rings, 20, 4.0, grip);
    body.lathe(
        Vec3::ZERO,
        &[
            (0.0162, 0.123),
            (0.017, 0.14),
            (0.021, l - 0.07),
            (0.026, l - 0.035),
            (0.027, l - 0.008),
            (0.029, l - 0.006),
            (0.029, l),
            (0.024, l),
        ],
        24,
        4.0,
        dark,
    );
    let mut lens = MeshBuilder::new();
    lens.lathe(
        Vec3::ZERO,
        &[(0.024, l - 0.002), (0.0, l - 0.001)],
        24,
        1.0,
        srgb(1.0, 1.0, 1.0),
    );
    (body.build(), lens.build())
}

/// Lens glow for a torch that is on or off.
fn torch_glow(on: bool) -> LinearRgba {
    if on {
        LinearRgba::rgb(40.0, 36.0, 30.0)
    } else {
        LinearRgba::BLACK
    }
}

/// The torch at rest: held low right, aimed across the centre of view.
fn torch_rest() -> Transform {
    Transform::from_translation(TORCH_GRIP).looking_at(TORCH_AIM, Vec3::Y)
}

fn read_devices(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    net: Res<crate::net::Network>,
    mut intent: ResMut<CurrentIntent>,
    mut lock: ResMut<crate::encounter::LockPanel>,
    mut naming: ResMut<crate::encounter::NamePanel>,
) {
    let mut axis = Vec2::ZERO;
    // At the key box, 1, 2 and 3 turn the dials (Shift turns them back) and
    // Enter tries the combination.
    let mut code = None;
    if lock.open {
        let back = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .enumerate()
        {
            if keys.just_pressed(key) {
                lock.dials[i] = if back {
                    (lock.dials[i] + 9) % 10
                } else {
                    (lock.dials[i] + 1) % 10
                };
            }
        }
        if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
            code = Some(lock.dials);
        }
    }
    // At the ceiba: 1, 2 or 3 chooses which of him walks; Enter names him.
    let mut name = None;
    if naming.open {
        for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .enumerate()
        {
            if keys.just_pressed(key) {
                naming.choice = i as u8;
            }
        }
        if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
            name = Some(naming.choice);
            naming.open = false;
        }
    }
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
    let v = keys.just_pressed(KeyCode::KeyV) || mouse.just_pressed(MouseButton::Middle);
    // Down, V is no mark: it is a hoarse cry for help from where you lie
    // (and from his sack, nothing at all).
    let (down, hauled) = net.me().map_or((false, false), |p| (p.status == 1, p.hauled));
    intent.0 = Intent {
        move_axis: axis.normalize_or_zero(),
        look_delta: Vec2::new(
            motion.delta.x * k,
            if settings.invert_y {
                motion.delta.y * k
            } else {
                -motion.delta.y * k
            },
        ),
        interact_pressed: keys.just_pressed(KeyCode::KeyE) || mouse.just_pressed(MouseButton::Left),
        interact_held: keys.pressed(KeyCode::KeyE) || mouse.pressed(MouseButton::Left),
        toggle_flashlight: keys.just_pressed(KeyCode::KeyF),
        crouch: keys.any_pressed([KeyCode::ControlLeft, KeyCode::KeyC]),
        sprint: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        drop: keys.just_pressed(KeyCode::KeyG),
        use_aji: keys.just_pressed(KeyCode::KeyQ),
        drive_off: keys.just_pressed(KeyCode::KeyX),
        ping: v && !down,
        call: (v && down && !hauled).then_some(crate::net::protocol::CallKind::Help),
        skill: if keys.just_pressed(KeyCode::Space) {
            net.needle(&tuning.0)
        } else {
            None
        },
        code,
        name,
    };
}

fn apply_motion(
    intent: Res<CurrentIntent>,
    tuning: Res<TuningRes>,
    layout: Res<LayoutRes>,
    player: Single<(&mut Player, &mut Transform)>,
    net: Res<crate::net::Network>,
    mut light: ResMut<LightOn>,
    mut torch: Single<&mut Flashlight>,
) {
    let (mut player, mut tf) = player.into_inner();
    // Even the frozen and the downed can turn their heads; the dead only watch.
    if net.status() != 2 {
        player.pose.look(intent.0.look_delta, &tuning.0);
    }
    *tf = eye_transform(&player.pose, &tuning.0, &layout.0);

    if intent.0.toggle_flashlight && net.status() != 2 {
        torch.on = !torch.on;
        light.0 = torch.on;
    }
}

/// The player's view settings: field of view, brightness and contrast (the
/// photo driver sets its own field of view per shot).
fn view_settings(
    settings: Res<Settings>,
    launch: Res<Launch>,
    fright: Res<crate::world::omen::Fright>,
    camera: Single<(&mut Projection, &mut ColorGrading), With<Player>>,
    mut applied: Local<bool>,
    mut punching: Local<bool>,
) {
    // The catch: the world dims in its silence; each time the torch bursts
    // back on him the view flares and punches in (never in a photo, which
    // holds its own view).
    let lunge = if launch.photos && !launch.trailer {
        None
    } else {
        fright.lunge
    };
    if *applied && !settings.is_changed() && lunge.is_none() && !*punching {
        return;
    }
    *applied = true;
    *punching = lunge.is_some();
    let (fov_add, exposure_add) = lunge.map_or((0.0, 0.0), |s| {
        use crate::world::omen::{CUT_AT, LUNGE_GAP};
        if s < 0.0 {
            (0.0, -0.9 * ((s + LUNGE_GAP) / 0.3).clamp(0.0, 1.0))
        } else if s < CUT_AT {
            let flare = [0.0, 0.2, 0.4, 0.62]
                .iter()
                .map(|&at| if s >= at { (-(s - at) * 22.0).exp() } else { 0.0 })
                .fold(0.0, f32::max);
            (-12.0 * flare, 1.1 * flare)
        } else {
            (0.0, 0.0)
        }
    });
    let (mut projection, mut grading) = camera.into_inner();
    if !launch.photos
        && let Projection::Perspective(p) = &mut *projection
    {
        p.fov = (settings.fov + fov_add).to_radians();
    }
    let grade = crate::display::grade(settings.brightness, settings.contrast, fog_level());
    // The trailer's video is a touch brighter than play.
    let video = if launch.trailer {
        crate::trailer::EXPOSURE_LIFT
    } else {
        0.0
    };
    *grading = night_grading(grade);
    grading.global.exposure = grade.exposure + video + exposure_add;
}

/// The torch's lights and lens follow its switch and its charge: a dead
/// battery gives nothing, a weak one gutters and catches.
/// Whether the torch is seen in hand (the trailer's cinematic shots hide it).
#[derive(Resource)]
pub struct TorchHand(pub bool);

impl Default for TorchHand {
    fn default() -> Self {
        Self(true)
    }
}

#[allow(clippy::too_many_arguments)]
fn torch_light(
    time: Res<Time>,
    state: Res<State<Flow>>,
    hand: Res<TorchHand>,
    fright: Res<crate::world::omen::Fright>,
    layout: Res<LayoutRes>,
    torch: Single<(&Flashlight, &mut Visibility)>,
    net: Res<crate::net::Network>,
    tuning: Res<TuningRes>,
    lens: Res<TorchLens>,
    mut beams: Query<(&Beam, &mut SpotLight)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shown: Local<Option<u16>>,
) {
    let (torch, mut vis) = torch.into_inner();
    // On the title screen the camera drifts over the llano: no torch in hand
    // (nor in a trailer's cinematic shots, nor for the fallen watching a
    // friend, whose own beam lights the way).
    let unheld = *state.get() == Flow::Title || net.spectating();
    let want = if unheld || !hand.0 {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    if *vis != want {
        *vis = want;
    }
    let charge = net.snapshot().map_or(1.0, |s| s.me.battery);
    let low = tuning.0.battery_low;
    let level = if unheld || !torch.on || charge <= 0.0 {
        0.0
    } else if charge >= low {
        1.0
    } else {
        // Weaker as it drains, with a stutter that gets worse: two beats
        // that rarely line up, and now and then a drop to almost nothing.
        let t = time.elapsed_secs();
        let weak = charge / low;
        let beat = (t * 13.7).sin() * (t * 5.3 + 1.1).sin();
        let drop = if (t * 1.9).sin() > 0.55 + 0.4 * weak { 0.12 } else { 1.0 };
        (0.35 + 0.55 * weak + 0.1 * beat).clamp(0.05, 1.0) * drop
    };
    // The catch has the torch: it dies in the silence, strobes back on him.
    let level = match (fright.lunge, fright.catch) {
        (Some(s), Some(c)) => {
            let t = &tuning.0;
            let f = crate::world::silbon::lunge_frame(s, &c, t.eye_height - t.downed_lower, &|p| {
                layout.0.surface_height(p)
            });
            // Whatever it had left, it has now.
            f.torch
        }
        _ => level,
    };
    // Quantized so a steady beam writes nothing.
    let key = (level * 200.0).round() as u16;
    if *shown == Some(key) {
        return;
    }
    *shown = Some(key);
    for (beam, mut spot) in &mut beams {
        spot.intensity = beam.0 * level;
    }
    if let Some(mut m) = materials.get_mut(&lens.0) {
        m.emissive = torch_glow(true) * level;
    }
}

/// The head rises and falls with each footfall. Presentation only: aim,
/// sight and targeting read the pose, never the camera. Whoever last placed
/// the camera (the session each frame, a photo or debug view) is respected:
/// the bob is laid on top of that placement and taken off again next frame.
#[allow(clippy::too_many_arguments)]
fn head_bob(
    time: Res<Time>,
    settings: Res<Settings>,
    state: Res<State<Flow>>,
    fright: Res<crate::world::omen::Fright>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    net: Res<crate::net::Network>,
    mut gait: ResMut<Gait>,
    mut camera: Single<&mut Transform, With<Player>>,
    mut placed: Local<Option<(Transform, Transform)>>,
) {
    let dt = time.delta_secs();
    // The camera without last frame's bob: unchanged since we wrote it, or
    // freshly placed by someone else.
    let base = match *placed {
        Some((base, written)) if **camera == written => base,
        _ => **camera,
    };
    let last = placed.map(|(b, _)| b.translation);
    let moved = match last {
        Some(p) if dt > 0.0 => {
            let step = Vec2::new(base.translation.x - p.x, base.translation.z - p.z).length();
            // A teleport (photo, restart) is not a stride.
            if step > 1.0 { 0.0 } else { step }
        }
        _ => 0.0,
    };
    if dt > 0.0 {
        let pace = (moved / dt / 3.5).clamp(0.0, 1.3);
        gait.amount += (pace - gait.amount) * (dt * 6.0).min(1.0);
        gait.phase = (gait.phase + moved * STRIDE_PER_M) % std::f32::consts::TAU;
    }
    // Watching a friend, their stride is theirs, not a head of your own.
    let a = if settings.head_bob && *state.get() != Flow::Title && !net.spectating() {
        gait.amount
    } else {
        0.0
    };
    let mut tf = base;
    if a > 1e-3 {
        let right = base.rotation * Vec3::X;
        let rise = -(1.0 - (gait.phase * 2.0).cos()) * 0.5 * BOB_RISE * a;
        let sway = gait.phase.sin() * BOB_SWAY * a;
        tf.translation += Vec3::Y * rise + right * sway;
        tf.rotation *= Quat::from_rotation_z(gait.phase.sin() * BOB_ROLL * a);
    }
    // Caught: the catch owns the view until its black falls. From the eye
    // that was caught it falls to the ground and finds his face, shaking.
    // (Replaced, not laid on, so a haul moving the body underneath never
    // shows; the black covers the hand-back.)
    if let (Some(s), Some(c)) = (fright.lunge, fright.catch)
        && s < crate::world::omen::CUT_AT
    {
        let t = &tuning.0;
        let f =
            crate::world::silbon::lunge_frame(s, &c, t.eye_height - t.downed_lower, &|p| layout.0.surface_height(p));
        let jitter = |k: f32| (s * k).sin() * (s * k * 0.37 + 1.3).sin();
        let shake = Vec3::new(jitter(71.0), jitter(59.0), 0.0) * 0.025 * f.shake;
        tf = Transform::from_translation(f.eye + shake).looking_at(f.look, Vec3::Y);
        tf.rotation *= Quat::from_rotation_z(0.18 * f.shake * jitter(23.0) + 0.12 * (s / 0.5).clamp(0.0, 1.0));
    }
    **camera = tf;
    *placed = Some((base, tf));
}

/// The torch lags the look a little and bobs with the stride. Presentation
/// only: the beam's gameplay effect is its on/off state.
fn torch_in_hand(
    time: Res<Time>,
    gait: Res<Gait>,
    player: Single<&Transform, (With<Player>, Without<Flashlight>)>,
    torch: Single<(&mut Flashlight, &mut Transform)>,
    mut last: Local<Option<(f32, f32)>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let (mut hold, mut tf) = torch.into_inner();
    let (yaw, pitch, _) = player.rotation.to_euler(EulerRot::YXZ);
    let turn = match *last {
        Some((y, x)) => {
            let dy = (yaw - y + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            Vec2::new(dy, pitch - x)
        }
        None => Vec2::ZERO,
    };
    *last = Some((yaw, pitch));
    // Sway: pushed against the turn, eased back to rest.
    let k = (dt * 9.0).min(1.0);
    let push = (-turn * 0.6).clamp(Vec2::splat(-0.08), Vec2::splat(0.08));
    hold.sway = (hold.sway + push) * (1.0 - k);
    // Bob: the hand swings a little behind the head's footfall.
    let p = gait.phase - 0.5;
    let bob = Vec3::new(p.sin() * 0.006, -(p * 2.0).sin().abs() * 0.008, 0.0) * gait.amount;
    let rest = torch_rest();
    tf.translation = rest.translation + bob;
    tf.rotation = Quat::from_euler(EulerRot::YXZ, hold.sway.x, hold.sway.y, 0.0) * rest.rotation;
}

/// Keeps the beam's fog volume centred on the eye.
fn carry_fog(
    player: Single<&Transform, (With<Player>, Without<TravellingFog>)>,
    mut fog: Single<&mut Transform, With<TravellingFog>>,
) {
    fog.translation = player.translation;
}

pub(crate) fn reset_player(
    mut requests: MessageReader<RunReset>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    player: Single<(&mut Player, &mut Transform)>,
    mut torch: Single<&mut Flashlight>,
    mut light: ResMut<LightOn>,
    mut intent: ResMut<CurrentIntent>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let (mut player, mut tf) = player.into_inner();
    player.pose = Pose::spawn(&layout.0);
    *tf = eye_transform(&player.pose, &tuning.0, &layout.0);
    torch.on = true;
    light.0 = true;
    intent.0 = Intent::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_grade(settings: &Settings) -> ColorGrading {
        night_grading(crate::display::grade(
            settings.brightness,
            settings.contrast,
            fog_level(),
        ))
    }

    #[test]
    fn the_default_picture_leaves_the_camera_as_it_was_graded() {
        // The grade the camera had before brightness and contrast moved
        // into the sections: global only, every section at its default.
        let g = camera_grade(&Settings::default());
        assert_eq!(g.global.exposure, 0.3);
        assert_eq!(g.global.temperature, -0.03);
        assert_eq!(g.global.tint, 0.0);
        assert_eq!(g.global.hue, 0.0);
        assert_eq!(g.global.post_saturation, 0.9);
        assert_eq!(g.global.midtones_range, 0.2..0.7);
        assert!(g.all_sections().all(|s| *s == ColorGradingSection::default()));
        // Any other picture reaches every section alike.
        for (brightness, contrast) in [(0.5, 1.0), (0.0, 1.1), (-0.3, 0.9)] {
            let g = camera_grade(&Settings {
                brightness,
                contrast,
                ..Settings::default()
            });
            assert!(g.shadows != ColorGradingSection::default());
            assert!(g.shadows == g.midtones && g.midtones == g.highlights);
        }
    }

    #[test]
    fn a_friend_is_watched_from_their_own_side_of_every_wall() {
        let layout = crate::geometry::Layout::new();
        let tuning = crate::tuning::Tuning::default();
        let (mut pulled_in, mut open) = (0, 0);
        // Everywhere a friend can stand in and around the house, facing
        // every way, the view behind them stays on their side of the walls.
        for i in 0..60 {
            for j in 0..60 {
                let p = layout.spawn + Vec2::new(i as f32 - 30.0, j as f32 - 30.0) * 0.5;
                if layout.resolve(p, tuning.player_radius) != p {
                    continue;
                }
                for turn in 0..8 {
                    let pose = Pose {
                        pos: p,
                        yaw: turn as f32 * std::f32::consts::FRAC_PI_4,
                        pitch: -0.1,
                        lower: 0.0,
                    };
                    let tf = shoulder_transform(&pose, &tuning, &layout);
                    let at = crate::geometry::ground(tf.translation);
                    assert!(layout.line_of_sight(p, at), "watched from beyond a wall at {p:?}");
                    assert!(tf.forward().dot(pose.look_dir()) > 0.8, "looking where they look");
                    let back = at.distance(p);
                    if back < 1.5 {
                        pulled_in += 1;
                    } else if back > 2.2 {
                        open += 1;
                    }
                }
            }
        }
        assert!(pulled_in > 0, "a wall at their back pulls the view in");
        assert!(open > 0, "in the open it stands back over the shoulder");
    }
}
