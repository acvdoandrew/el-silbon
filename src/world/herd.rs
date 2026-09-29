//! The corral's cattle: zebu-type cows (a shoulder hump, drooping ears, up-
//! swept horns), each a single merged mesh, with a slow idle breath and a
//! nervous shudder while the herd is bellowing.

use bevy::prelude::*;

use super::mesh::{MeshBuilder, Rgba, Ring, mix_rgb, scale_rgb, srgb};
use super::{SpawnCtx, noise2};
use crate::geometry::district::PropKind;
use crate::net::Network;

#[derive(Component)]
pub struct Cow {
    phase: f32,
    rest: Vec3,
    yaw: f32,
}

fn cow_mesh(seed: u32, index: usize) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let s = seed ^ (index as u32).wrapping_mul(0x9E37);
    // Coat: one of dark, red-brown, grey-white or pied.
    let style = index % 4;
    let base = match style {
        0 => srgb(0.16, 0.11, 0.09),
        1 => srgb(0.3, 0.16, 0.1),
        2 => srgb(0.55, 0.52, 0.46),
        _ => srgb(0.6, 0.57, 0.5),
    };
    let patch = srgb(0.13, 0.09, 0.07);
    let coat = move |p: Vec3| -> Rgba {
        let n = noise2(p.x * 1.6 + 3.0, p.z * 1.6 + p.y * 1.3, s);
        match style {
            3 => mix_rgb(base, patch, ((n - 0.5) * 6.0 + 0.5).clamp(0.0, 1.0)),
            _ => scale_rgb(base, 0.85 + 0.3 * n),
        }
    };
    // Body and hump.
    m.blob(
        Vec3::new(0.0, 0.98, 0.0),
        Vec3::new(1.05, 0.5, 0.44),
        10,
        16,
        1.0,
        coat(Vec3::ZERO),
        &|d| 1.0 + 0.04 * (d.x * 5.0).sin(),
    );
    m.blob(
        Vec3::new(0.5, 1.4, 0.0),
        Vec3::new(0.3, 0.22, 0.26),
        8,
        12,
        1.0,
        scale_rgb(coat(Vec3::X), 0.95),
        &|_| 1.0,
    );
    // Neck and head.
    let ring = |c: Vec3, r: f32, k: f32| Ring {
        center: c,
        radius: r,
        color: scale_rgb(base, k),
    };
    m.tube(
        &[
            ring(Vec3::new(0.7, 1.15, 0.0), 0.3, 0.95),
            ring(Vec3::new(1.05, 1.08, 0.0), 0.23, 0.95),
            ring(Vec3::new(1.4, 0.98, 0.0), 0.17, 1.0),
        ],
        10,
        1.0,
        1.0,
        false,
        &|_, _| 1.0,
    );
    m.blob(
        Vec3::new(1.55, 0.92, 0.0),
        Vec3::new(0.3, 0.2, 0.17),
        8,
        12,
        1.0,
        scale_rgb(base, 1.05),
        &|_| 1.0,
    );
    m.blob(
        Vec3::new(1.8, 0.85, 0.0),
        Vec3::new(0.14, 0.11, 0.12),
        6,
        10,
        1.0,
        srgb(0.2, 0.15, 0.13),
        &|_| 1.0,
    );
    // Dewlap.
    m.blob(
        Vec3::new(0.95, 0.85, 0.0),
        Vec3::new(0.35, 0.16, 0.14),
        6,
        10,
        1.0,
        scale_rgb(base, 0.9),
        &|_| 1.0,
    );
    // Ears and horns.
    for z in [-1.0_f32, 1.0] {
        m.blob(
            Vec3::new(1.42, 1.06, z * 0.24),
            Vec3::new(0.05, 0.09, 0.14),
            5,
            8,
            1.0,
            scale_rgb(base, 0.9),
            &|_| 1.0,
        );
        let horn = srgb(0.78, 0.74, 0.62);
        m.tube(
            &[
                Ring {
                    center: Vec3::new(1.46, 1.1, z * 0.12),
                    radius: 0.035,
                    color: horn,
                },
                Ring {
                    center: Vec3::new(1.44, 1.18, z * 0.22),
                    radius: 0.028,
                    color: horn,
                },
                Ring {
                    center: Vec3::new(1.5, 1.3, z * 0.3),
                    radius: 0.018,
                    color: horn,
                },
                Ring {
                    center: Vec3::new(1.55, 1.38, z * 0.27),
                    radius: 0.006,
                    color: horn,
                },
            ],
            6,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
    }
    // Legs with hooves.
    for x in [-0.66_f32, 0.6] {
        for z in [-0.26_f32, 0.26] {
            m.tube(
                &[
                    Ring {
                        center: Vec3::new(x, 0.75, z),
                        radius: 0.12,
                        color: scale_rgb(base, 0.9),
                    },
                    Ring {
                        center: Vec3::new(x, 0.35, z),
                        radius: 0.07,
                        color: scale_rgb(base, 0.85),
                    },
                    Ring {
                        center: Vec3::new(x, 0.09, z),
                        radius: 0.065,
                        color: srgb(0.1, 0.09, 0.08),
                    },
                    Ring {
                        center: Vec3::new(x + 0.03, 0.0, z),
                        radius: 0.075,
                        color: srgb(0.05, 0.05, 0.05),
                    },
                ],
                8,
                1.0,
                1.0,
                true,
                &|_, _| 1.0,
            );
        }
    }
    // Tail with a dark switch.
    m.tube(
        &[
            ring(Vec3::new(-1.0, 1.1, 0.0), 0.04, 0.9),
            ring(Vec3::new(-1.12, 0.85, 0.0), 0.03, 0.9),
            ring(Vec3::new(-1.14, 0.45, 0.0), 0.025, 0.85),
            Ring {
                center: Vec3::new(-1.13, 0.3, 0.0),
                radius: 0.06,
                color: srgb(0.06, 0.05, 0.04),
            },
        ],
        6,
        1.0,
        1.0,
        true,
        &|_, _| 1.0,
    );
    m
}

pub fn spawn(ctx: &mut SpawnCtx, materials: &mut Assets<StandardMaterial>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.82,
        reflectance: 0.3,
        ..default()
    });
    let herd: Vec<_> = ctx
        .layout
        .district
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Cow)
        .map(|p| p.center)
        .collect();
    let mut rng = ctx.rng(0xC0);
    for (i, c) in herd.into_iter().enumerate() {
        let mesh = ctx.meshes.add(cow_mesh(ctx.seed as u32, i).build());
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        let rest = Vec3::new(c.x, 0.0, c.y);
        ctx.commands.spawn((
            Name::new("cow"),
            Cow {
                phase: rng.range(0.0, std::f32::consts::TAU),
                rest,
                yaw,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(rest).with_rotation(Quat::from_rotation_y(yaw)),
        ));
    }
}

/// Idle breathing, and a shudder while the herd bellows.
pub fn animate(time: Res<Time>, net: Res<Network>, mut cows: Query<(&Cow, &mut Transform)>) {
    let t = time.elapsed_secs();
    let alarm = net.snapshot().is_some_and(|s| s.world.cattle > 0.0);
    for (cow, mut tf) in &mut cows {
        let breath = (t * 1.3 + cow.phase).sin() * 0.006;
        let sway = (t * 0.4 + cow.phase).sin() * 0.05;
        let shudder = if alarm {
            (t * 21.0 + cow.phase * 3.0).sin() * 0.05
        } else {
            0.0
        };
        tf.translation = cow.rest + Vec3::Y * (breath.abs() + if alarm { shudder.abs() * 0.3 } else { 0.0 });
        tf.rotation = Quat::from_rotation_y(cow.yaw + sway + shudder * 1.4);
        tf.scale = Vec3::new(1.0, 1.0 + breath, 1.0);
    }
}
