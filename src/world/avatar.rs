//! Teammates: each is one of the four survivors (`assets/models/survivor_*`,
//! see `crate::survivor`), with a torch in the right hand and, when
//! carrying, a bundle of bones on the back. They walk, swing their arms and
//! aim the torch where they look. Until a model loads (or if it cannot),
//! a lean procedural llanero in a straw hat and the party slot's colours
//! stands in.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use super::mesh::{MeshBuilder, Ring, WHITE, scale_rgb, srgb};
use super::models::{Model, ModelsPending};
use crate::survivor::Survivor;
use crate::ui::player_color;

/// Marks the child that lights up while the teammate's torch is on.
#[derive(Component)]
pub struct AvatarTorch;

/// Where the torch's beam leaves the lens (root space, at rest).
const BEAM: Vec3 = Vec3::new(0.27, 0.95, -0.36);

/// A downed teammate's torch, lying lit in the grass beside them: its own
/// entity, since the body's roll must not tip it. Placed by `net`.
#[derive(Component)]
pub struct GroundTorch(pub u64);

/// A light of a `GroundTorch`: like the teammates' beams, it comes and goes
/// with the party, so the world's light census leaves it out.
#[derive(Component)]
pub struct GroundTorchLight;

/// Where the torch lies from a downed teammate's position (their feet, as
/// the avatar root; facing -Z at zero yaw): just past the reaching right hand.
pub const GROUND_TORCH: Vec3 = Vec3::new(0.4, 0.0, -1.8);

/// A joint of a survivor's rig (`tools/models/survivors.py` JOINTS): its
/// rest position in the model (root space, feet at the origin, facing -Z)
/// and its parent. Rest rotations are identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bone {
    Pelvis,
    Torso,
    Chest,
    Neck,
    Head,
    HipL,
    KneeL,
    FootL,
    HipR,
    KneeR,
    FootR,
    ShoulderL,
    ElbowL,
    HandL,
    ShoulderR,
    ElbowR,
    HandR,
}

impl Bone {
    /// Node name, bone, rest position, parent (None: the model's root).
    pub const ALL: [(&'static str, Bone, Vec3, Option<Bone>); 17] = [
        ("Pelvis", Bone::Pelvis, Vec3::new(0.0, 0.97, 0.0), None),
        ("Torso", Bone::Torso, Vec3::new(0.0, 1.03, 0.0), Some(Bone::Pelvis)),
        ("Chest", Bone::Chest, Vec3::new(0.0, 1.25, 0.0), Some(Bone::Torso)),
        ("Neck", Bone::Neck, Vec3::new(0.0, 1.48, 0.005), Some(Bone::Chest)),
        ("Head", Bone::Head, Vec3::new(0.0, 1.56, 0.0), Some(Bone::Neck)),
        ("HipL", Bone::HipL, Vec3::new(-0.095, 0.93, 0.0), Some(Bone::Pelvis)),
        ("KneeL", Bone::KneeL, Vec3::new(-0.1, 0.5, 0.0), Some(Bone::HipL)),
        ("FootL", Bone::FootL, Vec3::new(-0.1, 0.085, 0.015), Some(Bone::KneeL)),
        ("HipR", Bone::HipR, Vec3::new(0.095, 0.93, 0.0), Some(Bone::Pelvis)),
        ("KneeR", Bone::KneeR, Vec3::new(0.1, 0.5, 0.0), Some(Bone::HipR)),
        ("FootR", Bone::FootR, Vec3::new(0.1, 0.085, 0.015), Some(Bone::KneeR)),
        (
            "ShoulderL",
            Bone::ShoulderL,
            Vec3::new(-0.18, 1.42, 0.0),
            Some(Bone::Chest),
        ),
        (
            "ElbowL",
            Bone::ElbowL,
            Vec3::new(-0.21, 1.14, 0.0),
            Some(Bone::ShoulderL),
        ),
        ("HandL", Bone::HandL, Vec3::new(-0.235, 0.87, -0.01), Some(Bone::ElbowL)),
        (
            "ShoulderR",
            Bone::ShoulderR,
            Vec3::new(0.18, 1.42, 0.0),
            Some(Bone::Chest),
        ),
        (
            "ElbowR",
            Bone::ElbowR,
            Vec3::new(0.215, 1.14, 0.0),
            Some(Bone::ShoulderR),
        ),
        (
            "HandR",
            Bone::HandR,
            Vec3::new(0.258, 0.955, -0.115),
            Some(Bone::ElbowR),
        ),
    ];

    pub fn rest(self) -> Vec3 {
        Bone::ALL.iter().find(|b| b.1 == self).map_or(Vec3::ZERO, |b| b.2)
    }

    pub fn parent(self) -> Option<Bone> {
        Bone::ALL.iter().find(|b| b.1 == self).and_then(|b| b.3)
    }
}

/// Put on a survivor model's joint nodes, with the node's rest translation
/// (relative to its parent) so a pose can offset it.
#[derive(Component)]
pub struct AvatarJoint {
    pub bone: Bone,
    pub rest: Vec3,
}

/// A survivor model is in and posed by its joints: the placer stops
/// squashing the root to crouch.
#[derive(Component)]
pub struct Rigged;

/// How a teammate is moving, for their animation: set by whoever places them.
#[derive(Component, Default)]
pub struct AvatarMotion {
    /// Where they look up (+) or down (-), radians.
    pub pitch: f32,
    pub sprint: bool,
    pub crouch: bool,
    /// Down on the ground (crawling, or waiting for a revive).
    pub down: bool,
    pub carrying: bool,
    /// Who they are, to know when to change model.
    pub survivor: Survivor,
    last: Option<Vec3>,
    speed: f32,
    phase: f32,
    clock: f32,
    gait: Gait,
}

/// Everything a pose is made from, smoothed over time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gait {
    /// Where in the stride (radians): the left foot leads at pi/2.
    pub phase: f32,
    /// Stride size: 0 standing, 1 a walk, about 1.45 a sprint.
    pub amp: f32,
    pub sprint: f32,
    pub crouch: f32,
    pub down: f32,
    pub carry: f32,
    pub pitch: f32,
    /// Seconds, for breathing and idle glances.
    pub clock: f32,
}

/// The walk, the run, the crouch, the crawl and the idle, as each joint's
/// rotation and an offset from its rest position. Positive X turns swing a
/// limb forward (toward -Z) and tip the spine back.
pub fn pose(bone: Bone, g: &Gait) -> (Quat, Vec3) {
    let (s, c) = g.phase.sin_cos();
    let a = g.amp;
    let cr = g.crouch;
    let idle = (1.0 - a).clamp(0.0, 1.0);
    let breathe = (g.clock * 1.6).sin() * 0.02;
    let x = Quat::from_rotation_x;
    let y = Quat::from_rotation_y;
    let z = Quat::from_rotation_z;
    // The body leans into speed, a sprint, a crouch and a load.
    let lean = -(0.05 * a + 0.16 * g.sprint + 0.34 * cr + 0.08 * g.carry);
    let hip_sw = (0.42 + 0.12 * g.sprint) * a * (1.0 - 0.3 * cr);
    let knee_sw = (0.95 + 0.25 * g.sprint) * a.min(1.0);
    let arm_sw = (0.45 + 0.35 * g.sprint) * a;
    let hip_base = 1.05 * cr;
    let knee_base = 1.75 * cr + 0.1 * a;
    let leg = |side: f32| {
        let hip = hip_base + side * hip_sw * s;
        // the knee folds as the leg swings through, and gives at landing
        let knee = -(knee_base + knee_sw * (side * c).max(0.0).powf(1.3) + 0.12 * a * (side * s).max(0.0));
        (hip, knee)
    };
    let (hl, kl) = leg(1.0);
    let (hr, kr) = leg(-1.0);
    let look = idle * 0.35 * (g.clock * 0.23).sin() * (g.clock * 0.11).sin();
    let pitch = g.pitch.clamp(-0.9, 0.9);
    let torch_elbow = 0.25 * g.sprint * a.min(1.0);
    let stand = match bone {
        Bone::Pelvis => (
            y(-0.12 * a * s) * z(0.035 * a * c + idle * 0.02 * (g.clock * 0.4).sin()),
            Vec3::new(0.0, -0.3 * cr - 0.028 * a * s * s + 0.012 * a, 0.0),
        ),
        Bone::Torso => (y(0.08 * a * s) * x(lean * 0.6), Vec3::ZERO),
        Bone::Chest => (y(0.06 * a * s) * x(lean * 0.4 + breathe), Vec3::ZERO),
        Bone::Neck => (x(pitch * 0.25 - lean * 0.4), Vec3::ZERO),
        Bone::Head => (
            y(-0.14 * a * s + look) * x(pitch * 0.45 - lean * 0.5 - breathe),
            Vec3::ZERO,
        ),
        Bone::HipL => (x(hl), Vec3::ZERO),
        Bone::HipR => (x(hr), Vec3::ZERO),
        Bone::KneeL => (x(kl), Vec3::ZERO),
        Bone::KneeR => (x(kr), Vec3::ZERO),
        // the feet stay near level: they undo most of the leg's turn
        Bone::FootL => (x(-(hl + kl) * 0.85), Vec3::ZERO),
        Bone::FootR => (x(-(hr + kr) * 0.85), Vec3::ZERO),
        Bone::ShoulderL => (z(-0.06) * x(-arm_sw * s + 0.3 * cr), Vec3::ZERO),
        Bone::ElbowL => (
            x(0.15 + 0.4 * a * (-s).max(0.0) + 0.9 * g.sprint * a.min(1.0)),
            Vec3::ZERO,
        ),
        Bone::HandL => (x(0.1 * a * s), Vec3::ZERO),
        // the torch arm holds the beam where they look, whatever the spine does
        Bone::ShoulderR => (x(pitch * 0.85 - lean - torch_elbow + 0.1 * a * s), Vec3::ZERO),
        Bone::ElbowR => (x(torch_elbow), Vec3::ZERO),
        Bone::HandR => (Quat::IDENTITY, Vec3::ZERO),
    };
    if g.down <= 0.0 {
        return stand;
    }
    // Down: face to the ground (the placer lays the root flat), arms reaching
    // ahead and pulling in turn, legs trailing, head up to see.
    let crawl = s * a.min(1.0);
    let prone = match bone {
        Bone::Pelvis => (Quat::IDENTITY, Vec3::ZERO),
        Bone::Torso | Bone::Chest => (x(0.12), Vec3::ZERO),
        Bone::Neck => (x(0.35), Vec3::ZERO),
        Bone::Head => (x(0.55), Vec3::ZERO),
        Bone::HipL => (x(0.15 + 0.2 * crawl), Vec3::ZERO),
        Bone::HipR => (x(0.15 - 0.2 * crawl), Vec3::ZERO),
        Bone::KneeL => (x(-0.5 - 0.3 * crawl.max(0.0)), Vec3::ZERO),
        Bone::KneeR => (x(-0.5 - 0.3 * (-crawl).max(0.0)), Vec3::ZERO),
        Bone::FootL | Bone::FootR => (x(-0.6), Vec3::ZERO),
        Bone::ShoulderL => (z(-0.25) * x(2.5 + 0.35 * crawl), Vec3::ZERO),
        Bone::ShoulderR => (z(0.2) * x(2.2 - 0.35 * crawl), Vec3::ZERO),
        Bone::ElbowL => (x(0.5 + 0.4 * (-crawl).max(0.0)), Vec3::ZERO),
        Bone::ElbowR => (x(0.3 + 0.4 * crawl.max(0.0)), Vec3::ZERO),
        Bone::HandL | Bone::HandR => (x(0.3), Vec3::ZERO),
    };
    let d = g.down.clamp(0.0, 1.0);
    (stand.0.slerp(prone.0, d), stand.1.lerp(prone.1, d))
}

/// Marks the bundle carried on a teammate's back.
#[derive(Component)]
pub struct AvatarBag;

#[derive(Resource, Clone)]
pub struct AvatarKit {
    cloth: Handle<Mesh>,
    dark: Handle<Mesh>,
    skin: Handle<Mesh>,
    hat: Handle<Mesh>,
    scarf: Handle<Mesh>,
    torch: Handle<Mesh>,
    dark_mat: Handle<StandardMaterial>,
    skin_mat: Handle<StandardMaterial>,
    hat_mat: Handle<StandardMaterial>,
    torch_mat: Handle<StandardMaterial>,
    jackets: [Handle<StandardMaterial>; 4],
    scarves: [Handle<StandardMaterial>; 4],
    sack: Handle<Mesh>,
    bones: Handle<Mesh>,
    sack_mat: Handle<StandardMaterial>,
    bone_mat: Handle<StandardMaterial>,
    /// The four survivors' scenes, loading from startup.
    models: [Handle<WorldAsset>; 4],
}

fn limb(mb: &mut MeshBuilder, a: Vec3, b: Vec3, ra: f32, rb: f32, tint: [f32; 4]) {
    let mid = (a + b) * 0.5;
    mb.tube(
        &[
            Ring {
                center: a,
                radius: ra,
                color: tint,
            },
            Ring {
                center: mid + Vec3::new(0.0, 0.0, -0.015),
                radius: (ra + rb) * 0.5,
                color: tint,
            },
            Ring {
                center: b,
                radius: rb,
                color: tint,
            },
        ],
        8,
        1.0,
        1.0,
        true,
        &|_, _| 1.0,
    );
}

impl AvatarKit {
    pub fn build(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        palette: &super::Palette,
        satchel: &super::SatchelAsset,
        assets: &AssetServer,
    ) -> Self {
        // Jacket and arms (tinted by the slot's material), legs and boots,
        // hands and head.
        let mut cloth = MeshBuilder::new();
        limb(
            &mut cloth,
            Vec3::new(0.0, 0.88, 0.0),
            Vec3::new(0.0, 1.52, 0.0),
            0.16,
            0.17,
            WHITE,
        );
        cloth.blob(
            Vec3::new(0.0, 1.5, 0.0),
            Vec3::new(0.24, 0.1, 0.13),
            6,
            10,
            1.0,
            WHITE,
            &|_| 1.0,
        );
        // A ruana draped over the shoulders and a short jacket skirt.
        cloth.blob(
            Vec3::new(0.0, 1.4, 0.03),
            Vec3::new(0.3, 0.1, 0.17),
            6,
            10,
            1.0,
            scale_rgb(WHITE, 0.8),
            &|_| 1.0,
        );
        cloth.blob(
            Vec3::new(0.0, 0.92, 0.0),
            Vec3::new(0.2, 0.1, 0.14),
            6,
            10,
            1.0,
            scale_rgb(WHITE, 0.9),
            &|_| 1.0,
        );
        for s in [-1.0_f32, 1.0] {
            limb(
                &mut cloth,
                Vec3::new(s * 0.24, 1.45, 0.0),
                Vec3::new(s * 0.27, 0.98, -0.12),
                0.055,
                0.045,
                scale_rgb(WHITE, 0.85),
            );
        }
        let mut dark = MeshBuilder::new();
        for s in [-1.0_f32, 1.0] {
            limb(
                &mut dark,
                Vec3::new(s * 0.1, 0.92, 0.0),
                Vec3::new(s * 0.11, 0.08, 0.0),
                0.085,
                0.055,
                srgb(0.17, 0.2, 0.26),
            );
            dark.blob(
                Vec3::new(s * 0.11, 0.05, -0.05),
                Vec3::new(0.06, 0.05, 0.13),
                5,
                8,
                1.0,
                srgb(0.09, 0.07, 0.06),
                &|_| 1.0,
            );
        }
        // Wide belt with a brass buckle, and a canteen on the hip.
        dark.tube(
            &[
                Ring {
                    center: Vec3::new(0.0, 0.93, 0.0),
                    radius: 0.17,
                    color: srgb(0.14, 0.09, 0.05),
                },
                Ring {
                    center: Vec3::new(0.0, 1.0, 0.0),
                    radius: 0.17,
                    color: srgb(0.14, 0.09, 0.05),
                },
            ],
            12,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
        dark.blob(
            Vec3::new(0.0, 0.965, -0.175),
            Vec3::new(0.035, 0.03, 0.01),
            4,
            6,
            1.0,
            srgb(0.65, 0.5, 0.2),
            &|_| 1.0,
        );
        dark.blob(
            Vec3::new(-0.2, 0.85, 0.06),
            Vec3::new(0.05, 0.07, 0.04),
            5,
            8,
            1.0,
            srgb(0.3, 0.32, 0.22),
            &|_| 1.0,
        );
        // Boot shafts over the trousers.
        for s in [-1.0_f32, 1.0] {
            limb(
                &mut dark,
                Vec3::new(s * 0.11, 0.3, 0.0),
                Vec3::new(s * 0.11, 0.06, 0.0),
                0.07,
                0.065,
                srgb(0.11, 0.08, 0.06),
            );
        }
        let mut skin = MeshBuilder::new();
        skin.blob(
            Vec3::new(0.0, 1.68, -0.01),
            Vec3::new(0.095, 0.12, 0.105),
            8,
            12,
            1.0,
            srgb(0.7, 0.5, 0.38),
            &|_| 1.0,
        );
        for s in [-1.0_f32, 1.0] {
            skin.blob(
                Vec3::new(s * 0.27, 0.92, -0.14),
                Vec3::splat(0.045),
                5,
                8,
                1.0,
                srgb(0.7, 0.5, 0.38),
                &|_| 1.0,
            );
        }
        skin.blob(
            Vec3::new(0.0, 1.6, 0.0),
            Vec3::new(0.05, 0.08, 0.05),
            5,
            8,
            1.0,
            srgb(0.7, 0.5, 0.38),
            &|_| 1.0,
        );
        // The hat: a wide straw brim and a low crown.
        let mut hat = MeshBuilder::new();
        let straw = srgb(0.62, 0.52, 0.34);
        hat.lathe(
            Vec3::new(0.0, 1.76, 0.0),
            &[(0.1, 0.0), (0.3, -0.012), (0.5, -0.075)],
            20,
            1.0,
            straw,
        );
        hat.lathe(
            Vec3::new(0.0, 1.76, 0.0),
            &[(0.5, -0.08), (0.3, -0.02), (0.1, -0.005)],
            20,
            1.0,
            scale_rgb(straw, 0.6),
        );
        // Crown with a pinched dent, and a dark band.
        hat.lathe(
            Vec3::new(0.0, 1.76, 0.0),
            &[(0.11, 0.0), (0.12, 0.09), (0.09, 0.14), (0.035, 0.13), (0.0, 0.108)],
            14,
            1.0,
            straw,
        );
        hat.lathe(
            Vec3::new(0.0, 1.76, 0.0),
            &[(0.117, 0.01), (0.122, 0.05)],
            14,
            1.0,
            srgb(0.1, 0.07, 0.05),
        );
        let mut scarf = MeshBuilder::new();
        scarf.tube(
            &[
                Ring {
                    center: Vec3::new(0.0, 1.52, 0.0),
                    radius: 0.125,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(0.0, 1.58, 0.0),
                    radius: 0.115,
                    color: WHITE,
                },
            ],
            12,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
        scarf.ribbon(
            &[
                Vec3::new(0.06, 1.52, 0.12),
                Vec3::new(0.08, 1.3, 0.16),
                Vec3::new(0.05, 1.12, 0.18),
            ],
            &[0.09, 0.08, 0.05],
            Vec3::X,
            &[WHITE],
        );
        // The torch: a short club of a flashlight held out in the right hand.
        let mut torch = MeshBuilder::new();
        torch.tube(
            &[
                Ring {
                    center: Vec3::new(0.27, 0.93, -0.12),
                    radius: 0.028,
                    color: srgb(0.1, 0.1, 0.1),
                },
                Ring {
                    center: Vec3::new(0.27, 0.95, -0.3),
                    radius: 0.038,
                    color: srgb(0.14, 0.14, 0.14),
                },
                Ring {
                    center: Vec3::new(0.27, 0.95, -0.34),
                    radius: 0.05,
                    color: WHITE,
                },
            ],
            8,
            1.0,
            1.0,
            true,
            &|_, _| 1.0,
        );
        let jackets = std::array::from_fn(|i| {
            let c = player_color(i).to_srgba();
            materials.add(StandardMaterial {
                base_color: Color::srgb(c.red * 0.55, c.green * 0.55, c.blue * 0.55),
                perceptual_roughness: 0.9,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        });
        let scarves = std::array::from_fn(|i| {
            let c = player_color(i).to_srgba();
            materials.add(StandardMaterial {
                base_color: Color::srgb(c.red, c.green, c.blue),
                emissive: LinearRgba::rgb(c.red * 0.12, c.green * 0.12, c.blue * 0.12),
                perceptual_roughness: 0.85,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        });
        Self {
            cloth: meshes.add(cloth.build()),
            dark: meshes.add(dark.build()),
            skin: meshes.add(skin.build()),
            hat: meshes.add(hat.build()),
            scarf: meshes.add(scarf.build()),
            torch: meshes.add(torch.build()),
            dark_mat: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.85,
                ..default()
            }),
            skin_mat: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.7,
                ..default()
            }),
            hat_mat: palette.straw.clone(),
            torch_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.62),
                emissive: LinearRgba::rgb(2.0, 1.8, 1.2),
                perceptual_roughness: 0.4,
                metallic: 0.5,
                ..default()
            }),
            jackets,
            scarves,
            sack: satchel.sack.clone(),
            bones: satchel.bones.clone(),
            sack_mat: palette.burlap.clone(),
            bone_mat: palette.bone.clone(),
            models: Survivor::ALL
                .map(|s| assets.load(GltfAssetLabel::Scene(0).from_asset(Model::Survivor.path(s.code().into())))),
        }
    }

    /// Spawn a teammate of party `slot`, who is `survivor`, at `at`; the
    /// root's origin is at their feet and they face -Z at zero yaw.
    pub fn spawn(
        &self,
        commands: &mut Commands,
        pending: &mut ModelsPending,
        slot: usize,
        survivor: Survivor,
        name: String,
        marker: impl Component,
        at: Vec3,
    ) -> Entity {
        let slot = slot % 4;
        let root = commands
            .spawn((
                marker,
                Name::new(name),
                Transform::from_translation(at),
                Visibility::Inherited,
                AvatarMotion { survivor, ..default() },
            ))
            .id();
        // The survivor, replacing the stand-in below once loaded.
        let handle = self.models[usize::from(survivor.code())].clone();
        let model = commands
            .spawn((
                Name::new(format!("model {}", survivor.name())),
                Model::Survivor,
                WorldAssetRoot(handle.clone()),
                Transform::IDENTITY,
                Visibility::Inherited,
                ChildOf(root),
            ))
            .id();
        pending.0.push((model, handle));
        let mut part = |mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, casts: bool| {
            let mut e = commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                ChildOf(root),
            ));
            if !casts {
                e.insert(NotShadowCaster);
            }
        };
        part(&self.cloth, &self.jackets[slot], true);
        part(&self.dark, &self.dark_mat, true);
        part(&self.skin, &self.skin_mat, true);
        part(&self.hat, &self.hat_mat, true);
        part(&self.scarf, &self.scarves[slot], false);
        part(&self.torch, &self.torch_mat, false);
        // The torch's beam.
        commands.spawn((
            AvatarTorch,
            SpotLight {
                color: Color::srgb(1.0, 0.93, 0.82),
                intensity: 120_000.0,
                range: 24.0,
                radius: 0.03,
                inner_angle: 0.16,
                outer_angle: 0.36,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(BEAM),
            Visibility::Inherited,
            ChildOf(root),
        ));
        // The bundle on the back.
        let bag = commands
            .spawn((
                AvatarBag,
                Transform::from_xyz(0.02, 1.22, 0.3).with_rotation(Quat::from_euler(EulerRot::YXZ, 1.4, -0.2, 0.9)),
                Visibility::Hidden,
                ChildOf(root),
            ))
            .id();
        commands.spawn((
            Mesh3d(self.sack.clone()),
            MeshMaterial3d(self.sack_mat.clone()),
            Transform::from_scale(Vec3::splat(1.1)),
            ChildOf(bag),
        ));
        commands.spawn((
            Mesh3d(self.bones.clone()),
            MeshMaterial3d(self.bone_mat.clone()),
            Transform::from_scale(Vec3::splat(1.1)),
            NotShadowCaster,
            ChildOf(bag),
        ));
        root
    }

    /// Teammate `id`'s torch as it lies in the grass when they are down: the
    /// club on its side with its lens at the origin, a beam low along the
    /// ground (where the origin faces, -Z) and a small glow round it. Hidden
    /// until it is placed.
    pub fn spawn_ground_torch(&self, commands: &mut Commands, id: u64) -> Entity {
        let root = commands
            .spawn((
                GroundTorch(id),
                Name::new(format!("ground torch {id}")),
                Transform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        commands.spawn((
            Mesh3d(self.torch.clone()),
            MeshMaterial3d(self.torch_mat.clone()),
            Transform::from_translation(Vec3::new(-0.27, -0.94, 0.23)),
            NotShadowCaster,
            ChildOf(root),
        ));
        commands.spawn((
            GroundTorchLight,
            SpotLight {
                color: Color::srgb(1.0, 0.93, 0.82),
                intensity: 60_000.0,
                range: 14.0,
                radius: 0.03,
                inner_angle: 0.2,
                outer_angle: 0.5,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.02, -0.12).with_rotation(Quat::from_rotation_x(-0.06)),
            ChildOf(root),
        ));
        commands.spawn((
            GroundTorchLight,
            PointLight {
                color: Color::srgb(1.0, 0.9, 0.75),
                intensity: 3_000.0,
                range: 3.5,
                radius: 0.05,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.2, -0.3),
            ChildOf(root),
        ));
        root
    }
}

/// Build the shared kit once the world's palette exists.
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    palette: Res<super::Palette>,
    satchel: Res<super::SatchelAsset>,
    assets: Res<AssetServer>,
) {
    commands.insert_resource(AvatarKit::build(
        &mut meshes,
        &mut materials,
        &palette,
        &satchel,
        &assets,
    ));
}

/// Teammates move like people: a stride sized to their speed over the
/// ground (a walk, a run, a crouched creep), a crawl when down, breathing
/// and glances at rest; the head and the torch arm follow where they look.
pub fn animate(
    time: Res<Time>,
    mut roots: Query<(Entity, &mut AvatarMotion, &Transform)>,
    tree: Query<&Children>,
    mut joints: Query<(&AvatarJoint, &mut Transform), Without<AvatarMotion>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut m, root) in &mut roots {
        let here = root.translation;
        let moved = m.last.map_or(0.0, |last| (here - last).xz().length());
        m.last = Some(here);
        // Teleports (a spawn, a restart) are not steps.
        let speed = if moved > 1.0 { 0.0 } else { moved / dt };
        m.speed += (speed - m.speed) * (dt * 8.0).min(1.0);
        // Longer strides the faster they go: a walk takes about 2.3 m a
        // stride (both feet), a sprint 2.6.
        let stride = 1.6 + 0.2 * m.speed;
        m.phase = (m.phase + m.speed * dt * std::f32::consts::TAU / stride) % std::f32::consts::TAU;
        m.clock += dt;
        let ease = |from: f32, to: f32, rate: f32| from + (to - from) * (dt * rate).min(1.0);
        let on = |b: bool| if b { 1.0 } else { 0.0 };
        let (sprint, crouch, down, carrying, pitch) = (m.sprint, m.crouch, m.down, m.carrying, m.pitch);
        let (speed, phase, clock) = (m.speed, m.phase, m.clock);
        let g = &mut m.gait;
        g.amp = ease(g.amp, (speed / 3.6).min(1.45), 10.0);
        g.sprint = ease(g.sprint, on(sprint), 6.0);
        g.crouch = ease(g.crouch, on(crouch && !down), 8.0);
        g.down = ease(g.down, on(down), 5.0);
        g.carry = ease(g.carry, on(carrying), 4.0);
        g.pitch = ease(g.pitch, pitch, 12.0);
        g.phase = phase;
        g.clock = clock;
        let gait = m.gait;
        for e in tree.iter_descendants(entity) {
            if let Ok((joint, mut tf)) = joints.get_mut(e) {
                let (rot, offset) = pose(joint.bone, &gait);
                let at = joint.rest + offset;
                if tf.rotation != rot {
                    tf.rotation = rot;
                }
                if tf.translation != at {
                    tf.translation = at;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where a joint is, in the model's space, under a pose.
    fn world(bone: Bone, g: &Gait) -> (Vec3, Quat) {
        let (rot, offset) = pose(bone, g);
        match bone.parent() {
            None => (bone.rest() + offset, rot),
            Some(parent) => {
                let (p, pr) = world(parent, g);
                (p + pr * (bone.rest() - parent.rest() + offset), pr * rot)
            }
        }
    }

    #[test]
    fn feet_stay_on_the_ground_and_lead_the_stride() {
        let rest = [Bone::FootL, Bone::FootR].map(|f| f.rest());
        // Standing and crouching, both feet stay near their rest height.
        for crouch in [0.0, 1.0] {
            let g = Gait {
                crouch,
                ..Default::default()
            };
            for (foot, r) in [Bone::FootL, Bone::FootR].into_iter().zip(rest) {
                let (at, _) = world(foot, &g);
                assert!((at.y - r.y).abs() < 0.06, "crouch {crouch}: {foot:?} at {at}");
            }
            let (head, _) = world(Bone::Head, &g);
            assert!(head.y < 1.6 - 0.25 * crouch, "a crouch lowers the head: {head}");
        }
        // Walking and sprinting, the ankles never sink and never fly; the
        // leading foot is ahead (toward -Z) and its arm swings back.
        for amp in [1.0, 1.45] {
            let mut lowest = f32::MAX;
            let mut highest = f32::MIN;
            for i in 0..32 {
                let g = Gait {
                    amp,
                    sprint: (amp - 1.0) / 0.45,
                    phase: i as f32 / 32.0 * std::f32::consts::TAU,
                    ..Default::default()
                };
                for foot in [Bone::FootL, Bone::FootR] {
                    let (at, _) = world(foot, &g);
                    lowest = lowest.min(at.y);
                    highest = highest.max(at.y);
                }
            }
            assert!(lowest > 0.0, "amp {amp}: a foot sinks to {lowest}");
            // a sprint kicks the heel up high; a walk barely lifts it
            let cap = if amp > 1.0 { 0.6 } else { 0.4 };
            assert!(highest < cap, "amp {amp}: a foot flies to {highest}");
            let lead = Gait {
                amp,
                phase: std::f32::consts::FRAC_PI_2,
                ..Default::default()
            };
            let (left, _) = world(Bone::FootL, &lead);
            let (right, _) = world(Bone::FootR, &lead);
            assert!(left.z < right.z - 0.3, "the left foot leads: {left} vs {right}");
            let (hand, _) = world(Bone::HandL, &lead);
            assert!(hand.z > Bone::HandL.rest().z, "the left hand swings back: {hand}");
        }
    }

    #[test]
    fn the_torch_follows_the_look_and_a_downed_player_reaches_ahead() {
        let beam = |g: &Gait| {
            let (p, r) = world(Bone::HandR, g);
            (p, r * Vec3::NEG_Z)
        };
        for pitch in [-0.6, 0.0, 0.6] {
            let g = Gait {
                pitch,
                amp: 1.0,
                sprint: 1.0,
                ..Default::default()
            };
            let (_, dir) = beam(&g);
            // Leaning into a sprint, the torch still points where they look.
            assert!((dir.y.asin() - pitch * 0.85).abs() < 0.2, "pitch {pitch}: beam {dir}");
        }
        let down = Gait {
            down: 1.0,
            ..Default::default()
        };
        // Laid face down by the placer (the root turned -1.5 about X), the
        // hands reach past the head.
        let lay = Quat::from_rotation_x(-1.5);
        let (head, _) = world(Bone::Head, &down);
        let (hand, _) = world(Bone::HandL, &down);
        assert!((lay * hand).z < (lay * head).z, "hand {hand} head {head}");
    }
}
