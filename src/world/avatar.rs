//! Teammates: a lean llanero in a wide straw hat and a coloured scarf, with
//! a torch in one hand and, when carrying, a bundle of bones on the back.
//! One small kit shared by every remote player; colours follow the party
//! slot so avatars, roster and map agree.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::mesh::{MeshBuilder, Ring, WHITE, scale_rgb, srgb};
use crate::ui::player_color;

/// Marks the child that lights up while the teammate's torch is on.
#[derive(Component)]
pub struct AvatarTorch;

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
        }
    }

    /// Spawn a teammate of party `slot` at `at`; the root's origin is at
    /// their feet and they face -Z at zero yaw.
    pub fn spawn(
        &self,
        commands: &mut Commands,
        slot: usize,
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
            ))
            .id();
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
            Transform::from_xyz(0.27, 0.95, -0.36),
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
}

/// Build the shared kit once the world's palette exists.
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    palette: Res<super::Palette>,
    satchel: Res<super::SatchelAsset>,
) {
    commands.insert_resource(AvatarKit::build(&mut meshes, &mut materials, &palette, &satchel));
}
