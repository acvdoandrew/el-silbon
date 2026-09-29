//! Frights: the director's omens and the jump scares, as this client sees
//! and hears them. Every omen is placed near this player's own eye from what
//! the player already has (the map and the snapshot); nothing here reads
//! where he truly is, and nothing changes the rules.
//!
//! - Lamps die: the lamps and lanterns near you gutter out and come back.
//! - Silence: insects, frogs and rain stop; then one clack of bone.
//! - Bones: a clatter somewhere off in the dark.
//! - Drag marks: two furrows through the mud, as if a heavy sack was hauled.
//! - The hat: his broad hat lying on the trail ahead; gone when you get close.
//! - Phantom: a tall, hatted shape at the edge of sight, for a heartbeat.
//! - Stolen light: a torch moving far off that belongs to nobody.
//! - Caught: he lunges into your face (the outcome screen waits for it).
//! - Reveal: a lightning strike shows him close, with a stinger.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::mesh::{MeshBuilder, Ring, WHITE, srgb};
use crate::app::{EncounterMsg, LayoutRes, RunReset, StormClock, TuningRes};
use crate::geometry::Layout;
use crate::net::Network;
use crate::player::Player;
use crate::sim::Event;

/// Seconds the lamps stay out, the llano stays silent, and each shown omen
/// lasts (at most).
const LAMPS_DARK: f32 = 7.0;
const SILENCE: f32 = 7.0;
const DRAG_LIFE: f32 = 120.0;
const HAT_LIFE: f32 = 60.0;
const PHANTOM_LIFE: f32 = 0.7;
const STOLEN_LIFE: f32 = 28.0;
/// The caught lunge: how long it lasts, and how fast he closes.
pub const LUNGE: f32 = 1.4;
pub const LUNGE_IN: f32 = 0.28;
/// Seconds between two lightning reveals.
const REVEAL_COOL: f32 = 45.0;

/// A sound the frights ask for.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sting {
    Caught,
    Reveal,
    Phantom,
    Bones,
    Lamps,
    Swell,
    Clack,
}

#[derive(Resource, Default)]
pub struct Fright {
    /// Seconds into the lamps' death.
    lamps: Option<f32>,
    /// Seconds into a silence.
    silence: Option<f32>,
    /// Seconds into the caught lunge.
    pub lunge: Option<f32>,
    reveal_cool: f32,
    last_flash: f32,
    /// This player's status in the last snapshot (0 on their feet).
    last_status: u8,
    /// A small counter that varies where omens are placed.
    turn: u32,
}

impl Fright {
    /// Multiplier for lamps and lanterns near the eye.
    pub fn lamp_level(&self, t: f32) -> f32 {
        let Some(s) = self.lamps else {
            return 1.0;
        };
        let stutter = if (t * 23.0).sin() * (t * 9.7).sin() > 0.1 {
            1.0
        } else {
            0.15
        };
        if s < 0.9 {
            stutter * (1.0 - s / 0.9)
        } else if s > LAMPS_DARK - 1.0 {
            stutter * ((s - (LAMPS_DARK - 1.0)) / 1.0).min(1.0)
        } else {
            0.0
        }
    }

    /// Multiplier for the night's ambience and rain during a silence.
    pub fn hush(&self) -> f32 {
        match self.silence {
            Some(s) if s < 0.6 => 1.0 - 0.92 * s / 0.6,
            Some(s) if s < SILENCE - 0.8 => 0.08,
            Some(s) => 0.08 + 0.92 * ((s - (SILENCE - 0.8)) / 0.8).min(1.0),
            None => 1.0,
        }
    }
}

#[derive(Component)]
pub struct DragMarks(f32);
/// The light that catches him when he lunges.
#[derive(Component)]
pub struct LungeLight;
#[derive(Component)]
pub struct OmenHat(f32);
#[derive(Component)]
pub struct PhantomFigure(f32);
#[derive(Component)]
pub struct StolenTorch {
    left: f32,
    drift: Vec2,
}

fn furrows() -> Mesh {
    // Two shallow grooves, three metres long, with churned edges.
    let mut m = MeshBuilder::new();
    let dark = srgb(0.07, 0.05, 0.035);
    let rim = srgb(0.2, 0.15, 0.1);
    for x in [-0.22_f32, 0.22] {
        for (w, y, c) in [(0.16, 0.012, rim), (0.09, 0.016, dark)] {
            let pts: Vec<Vec3> = (0..=12)
                .map(|i| {
                    let z = -1.5 + i as f32 * 0.25;
                    Vec3::new(x + 0.05 * (i as f32 * 1.7).sin(), y, z)
                })
                .collect();
            let widths = vec![w; pts.len()];
            m.ribbon(&pts, &widths, Vec3::X, &[c]);
        }
    }
    m.build()
}

fn hat() -> Mesh {
    // A broad, frayed brim around a low crown, lying on its side a little.
    let mut m = MeshBuilder::new();
    let straw = srgb(0.52, 0.42, 0.28);
    let band = srgb(0.12, 0.09, 0.07);
    m.lathe(
        Vec3::ZERO,
        &[(0.0, 0.03), (0.2, 0.035), (0.45, 0.03), (0.62, 0.0), (0.6, -0.01)],
        28,
        2.0,
        straw,
    );
    m.lathe(
        Vec3::ZERO,
        &[(0.2, 0.035), (0.19, 0.1), (0.19, 0.13), (0.16, 0.2), (0.0, 0.22)],
        20,
        2.0,
        straw,
    );
    m.lathe(Vec3::ZERO, &[(0.2, 0.04), (0.202, 0.08)], 20, 1.0, band);
    m.build()
}

fn figure() -> Mesh {
    // Tall, thin, hat-brimmed, arms too long: a shape, not a body.
    let mut m = MeshBuilder::new();
    let c = WHITE;
    m.lathe(
        Vec3::ZERO,
        &[
            (0.0, 0.0),
            (0.2, 0.05),
            (0.24, 0.9),
            (0.2, 1.5),
            (0.26, 1.85),
            (0.12, 2.05),
            (0.1, 2.2),
            (0.14, 2.35),
            (0.0, 2.42),
        ],
        12,
        1.0,
        c,
    );
    m.lathe(
        Vec3::new(0.0, 2.36, 0.0),
        &[(0.0, 0.02), (0.55, 0.0), (0.5, -0.02)],
        16,
        1.0,
        c,
    );
    for side in [-1.0_f32, 1.0] {
        m.tube(
            &[
                Ring {
                    center: Vec3::new(0.24 * side, 1.82, 0.0),
                    radius: 0.05,
                    color: c,
                },
                Ring {
                    center: Vec3::new(0.34 * side, 1.2, 0.08),
                    radius: 0.04,
                    color: c,
                },
                Ring {
                    center: Vec3::new(0.36 * side, 0.55, 0.14),
                    radius: 0.025,
                    color: c,
                },
            ],
            6,
            1.0,
            1.0,
            true,
            &|_, _| 1.0,
        );
    }
    m.build()
}

pub fn spawn_frights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mud = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.45,
        ..default()
    });
    commands.spawn((
        Name::new("omen: drag marks"),
        DragMarks(0.0),
        Mesh3d(meshes.add(furrows())),
        MeshMaterial3d(mud),
        NotShadowCaster,
        Transform::IDENTITY,
        Visibility::Hidden,
    ));
    let straw = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Name::new("omen: his hat"),
        OmenHat(0.0),
        Mesh3d(meshes.add(hat())),
        MeshMaterial3d(straw),
        Transform::IDENTITY,
        Visibility::Hidden,
    ));
    // Unlit near-black: against the fog it reads only as a shape.
    let shadow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.004, 0.004, 0.006),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Name::new("omen: phantom"),
        PhantomFigure(0.0),
        Mesh3d(meshes.add(figure())),
        MeshMaterial3d(shadow),
        NotShadowCaster,
        NotShadowReceiver,
        Transform::IDENTITY,
        Visibility::Hidden,
    ));
    commands.spawn((
        Name::new("omen: lunge light"),
        LungeLight,
        PointLight {
            color: Color::srgb(1.0, 0.86, 0.72),
            intensity: 0.0,
            range: 6.0,
            radius: 0.05,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::IDENTITY,
    ));
    let glow = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(30.0, 26.0, 18.0),
        unlit: true,
        ..default()
    });
    commands
        .spawn((
            Name::new("omen: stolen torch"),
            StolenTorch {
                left: 0.0,
                drift: Vec2::ZERO,
            },
            Mesh3d(meshes.add(Sphere::new(0.05).mesh().ico(2).expect("small sphere"))),
            MeshMaterial3d(glow),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .with_children(|t| {
            t.spawn((
                SpotLight {
                    color: Color::srgb(1.0, 0.9, 0.75),
                    intensity: 90_000.0,
                    range: 22.0,
                    radius: 0.03,
                    inner_angle: 0.15,
                    outer_angle: 0.4,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::default().looking_to(Vec3::new(0.2, -0.35, 1.0), Vec3::Y),
            ));
        });
}

/// Somewhere on open, dry, visible ground `dist` metres from the eye, within
/// `spread` radians of `dir` (either side), varied by `turn`.
fn spot(layout: &Layout, eye: Vec2, dir: Vec2, dist: (f32, f32), spread: (f32, f32), turn: u32) -> Option<Vec2> {
    let base = dir.y.atan2(dir.x);
    for k in 0..40u32 {
        let h = (turn.wrapping_mul(2654435761) ^ k.wrapping_mul(40503)) as f32 / u32::MAX as f32;
        let h2 = ((turn + 7).wrapping_mul(97531) ^ k.wrapping_mul(2246822519)) as f32 / u32::MAX as f32;
        let side = if (turn + k).is_multiple_of(2) { 1.0 } else { -1.0 };
        let a = base + side * (spread.0 + (spread.1 - spread.0) * h);
        let r = dist.0 + (dist.1 - dist.0) * h2;
        let p = eye + Vec2::new(a.cos(), a.sin()) * r;
        if layout.bounds.contains(p)
            && layout.is_free(p, 0.6)
            && !layout.wading(p)
            && !layout.district.water.iter().any(|w| w.contains(p))
            && layout.line_of_sight(eye, p)
        {
            return Some(p);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub fn frights(
    time: Res<Time>,
    net: Res<Network>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    clock: Res<StormClock>,
    mut fright: ResMut<Fright>,
    mut events: MessageReader<EncounterMsg>,
    mut resets: MessageReader<RunReset>,
    mut stings: MessageWriter<Sting>,
    camera: Single<
        &Transform,
        (
            With<Player>,
            Without<DragMarks>,
            Without<OmenHat>,
            Without<PhantomFigure>,
            Without<StolenTorch>,
        ),
    >,
    mut drag: Single<
        (&mut DragMarks, &mut Transform, &mut Visibility),
        (Without<OmenHat>, Without<PhantomFigure>, Without<StolenTorch>),
    >,
    mut hat: Single<
        (&mut OmenHat, &mut Transform, &mut Visibility),
        (Without<DragMarks>, Without<PhantomFigure>, Without<StolenTorch>),
    >,
    mut phantom: Single<
        (&mut PhantomFigure, &mut Transform, &mut Visibility),
        (Without<DragMarks>, Without<OmenHat>, Without<StolenTorch>),
    >,
    mut torch: Single<
        (&mut StolenTorch, &mut Transform, &mut Visibility),
        (Without<DragMarks>, Without<OmenHat>, Without<PhantomFigure>),
    >,
    lunge_light: Single<
        (&mut PointLight, &mut Transform),
        (
            With<LungeLight>,
            Without<Player>,
            Without<DragMarks>,
            Without<OmenHat>,
            Without<PhantomFigure>,
            Without<StolenTorch>,
        ),
    >,
) {
    let dt = time.delta_secs();
    let layout = &layout.0;
    let eye3 = camera.translation;
    let eye = Vec2::new(eye3.x, eye3.z);
    let f3 = camera.rotation * Vec3::NEG_Z;
    let fwd = Vec2::new(f3.x, f3.z).normalize_or(Vec2::NEG_Y);
    let ground = |p: Vec2| Vec3::new(p.x, layout.surface_height(p), p.y);

    if resets.read().count() > 0 {
        *fright = Fright::default();
        *drag.2 = Visibility::Hidden;
        *hat.2 = Visibility::Hidden;
        *phantom.2 = Visibility::Hidden;
        *torch.2 = Visibility::Hidden;
    }

    for EncounterMsg(e) in events.read() {
        fright.turn = fright.turn.wrapping_add(1);
        let turn = fright.turn;
        match e {
            Event::OmenLampsDie => {
                fright.lamps = Some(0.0);
                stings.write(Sting::Lamps);
            }
            Event::OmenSilence => fright.silence = Some(0.0),
            Event::OmenBones => {
                stings.write(Sting::Bones);
            }
            Event::OmenDrag => {
                if let Some(p) = spot(layout, eye, fwd, (4.0, 9.0), (0.1, 0.9), turn) {
                    drag.0.0 = DRAG_LIFE;
                    *drag.1 =
                        Transform::from_translation(ground(p)).with_rotation(Quat::from_rotation_y(turn as f32 * 1.3));
                    *drag.2 = Visibility::Inherited;
                    stings.write(Sting::Swell);
                }
            }
            Event::OmenHat => {
                if let Some(p) = spot(layout, eye, fwd, (10.0, 16.0), (0.0, 0.4), turn) {
                    hat.0.0 = HAT_LIFE;
                    *hat.1 = Transform::from_translation(ground(p) + Vec3::Y * 0.03)
                        .with_rotation(Quat::from_rotation_y(turn as f32 * 2.1) * Quat::from_rotation_x(0.12));
                    *hat.2 = Visibility::Inherited;
                    stings.write(Sting::Swell);
                }
            }
            Event::OmenPhantom => {
                if let Some(p) = spot(layout, eye, fwd, (9.0, 15.0), (0.3, 0.6), turn) {
                    phantom.0.0 = PHANTOM_LIFE;
                    let face = eye - p;
                    *phantom.1 = Transform::from_translation(ground(p))
                        .with_rotation(Quat::from_rotation_y((-face.x).atan2(-face.y) + std::f32::consts::PI));
                    *phantom.2 = Visibility::Inherited;
                    stings.write(Sting::Phantom);
                }
            }
            Event::OmenStolenLight => {
                if let Some(p) = spot(layout, eye, fwd, (30.0, 42.0), (0.2, 1.0), turn) {
                    let across = Vec2::new(-fwd.y, fwd.x) * if turn.is_multiple_of(2) { 0.6 } else { -0.6 };
                    torch.0.left = STOLEN_LIFE;
                    torch.0.drift = across;
                    torch.1.translation = ground(p) + Vec3::Y * 1.1;
                    *torch.2 = Visibility::Inherited;
                }
            }
            _ => {}
        }
    }

    // Timers.
    if let Some(s) = &mut fright.lamps {
        *s += dt;
        if *s >= LAMPS_DARK {
            fright.lamps = None;
        }
    }
    if let Some(s) = &mut fright.silence {
        *s += dt;
        let t = *s;
        if t >= SILENCE {
            fright.silence = None;
        } else if t >= SILENCE - 1.0 && t - dt < SILENCE - 1.0 {
            stings.write(Sting::Clack);
        }
    }
    if let Some(s) = &mut fright.lunge {
        *s += dt;
        if *s >= LUNGE {
            fright.lunge = None;
        }
    }

    // Caught: this player just went down.
    let status = net.status();
    if fright.last_status == 0 && status != 0 && net.snapshot().is_some_and(|s| s.started) {
        fright.lunge = Some(0.0);
        stings.write(Sting::Caught);
    }
    fright.last_status = status;

    // The torch catches him as he comes: a hard light just ahead of the eye.
    let (mut light, mut light_tf) = lunge_light.into_inner();
    let want = match fright.lunge {
        Some(s) => {
            let fade = (1.0 - (s / LUNGE).powi(2)).max(0.0);
            let stutter = if (s * 30.0).sin() > -0.6 { 1.0 } else { 0.35 };
            60_000.0 * fade * stutter
        }
        None => 0.0,
    };
    if light.intensity != want {
        light.intensity = want;
    }
    light_tf.translation = eye3 + f3 * 0.6 - Vec3::Y * 0.2;

    // A strike that shows him close.
    fright.reveal_cool = (fright.reveal_cool - dt).max(0.0);
    let flash = crate::storm::flash(tuning.0.seed, clock.t);
    if flash > 0.5
        && fright.last_flash <= 0.5
        && fright.reveal_cool <= 0.0
        && let Some(th) = net.snapshot().and_then(|s| s.threat)
        && Vec2::from_array(th.position).distance(eye) < 32.0
    {
        fright.reveal_cool = REVEAL_COOL;
        stings.write(Sting::Reveal);
    }
    fright.last_flash = flash;

    // What is shown fades on its own clock, or when you come close.
    let (drag_left, _, drag_vis) = &mut *drag;
    drag_left.0 -= dt;
    if drag_left.0 <= 0.0 && **drag_vis != Visibility::Hidden {
        **drag_vis = Visibility::Hidden;
    }
    let (hat_left, hat_tf, hat_vis) = &mut *hat;
    hat_left.0 -= dt;
    if (hat_left.0 <= 0.0 || Vec2::new(hat_tf.translation.x, hat_tf.translation.z).distance(eye) < 5.0)
        && **hat_vis != Visibility::Hidden
    {
        hat_left.0 = 0.0;
        **hat_vis = Visibility::Hidden;
    }
    let (ph_left, _, ph_vis) = &mut *phantom;
    ph_left.0 -= dt;
    if ph_left.0 <= 0.0 && **ph_vis != Visibility::Hidden {
        **ph_vis = Visibility::Hidden;
    }
    let (st, st_tf, st_vis) = &mut *torch;
    if **st_vis != Visibility::Hidden {
        st.left -= dt;
        let d = st.drift * dt;
        st_tf.translation += Vec3::new(d.x, 0.0, d.y);
        let at = Vec2::new(st_tf.translation.x, st_tf.translation.z);
        st_tf.translation.y = layout.surface_height(at) + 1.1 + 0.05 * (time.elapsed_secs() * 2.7).sin();
        // The beam sweeps the grass, searching.
        let sweep = (time.elapsed_secs() * 0.7).sin() * 1.2;
        st_tf.rotation = Quat::from_rotation_y(sweep);
        if st.left <= 0.0 || at.distance(eye) < 16.0 {
            **st_vis = Visibility::Hidden;
        }
    }
}
