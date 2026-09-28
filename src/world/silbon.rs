//! The Silbón: a gaunt figure over three metres tall under a broad llanero
//! hat, carrying his sack of bones over the right shoulder. Procedural
//! meshes on a small joint hierarchy, animated from the truth layer's
//! position, facing, speed and presence (he rises from and sinks into the
//! grass; no teleport pops). Restrained sway, long slow strides.

use bevy::prelude::*;

use super::SpawnCtx;
use super::mesh::{MeshBuilder, Ring, WHITE, scale_rgb};
use super::noise2;
use crate::app::{Truth, TuningRes};
use crate::control::wrap_angle;
use crate::sim::ThreatState;

const HIP_Y: f32 = 1.92;
const THIGH: f32 = 0.93;
const SHIN: f32 = 0.95;
const STRIDE: f32 = 1.9;
/// How far below the ground he is when fully sunk.
const SINK_DEPTH: f32 = 3.7;

#[derive(Component)]
pub struct SilbonRoot;

#[derive(Component)]
pub struct SilbonBody;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointKind {
    HipL,
    HipR,
    KneeL,
    KneeR,
    Torso,
    Head,
    ShoulderL,
    ElbowL,
    Sack,
}

#[derive(Component)]
pub struct Joint {
    pub kind: JointKind,
    pub rest: Vec3,
}

#[derive(Component, Default)]
pub struct SilbonAnim {
    pub phase: f32,
    pub yaw: f32,
    pub walk: f32,
    pub t: f32,
    pub placed: bool,
}

struct Parts {
    cloth: Handle<StandardMaterial>,
    shirt: Handle<StandardMaterial>,
    skin: Handle<StandardMaterial>,
    hat: Handle<StandardMaterial>,
    sack: Handle<StandardMaterial>,
    bone: Handle<StandardMaterial>,
}

fn tube_mesh(points: &[(Vec3, f32)], sides: usize, seed: u32, rag: f32) -> MeshBuilder {
    let mut mb = MeshBuilder::new();
    let rings: Vec<Ring> = points
        .iter()
        .map(|&(c, r)| Ring {
            center: c,
            radius: r,
            color: WHITE,
        })
        .collect();
    let n = rings.len();
    mb.tube(&rings, sides, 1.0, 1.5, true, &move |i, s| {
        let ragged = if i + 1 == n {
            rag * noise2(s as f32 * 1.7, 3.0, seed)
        } else {
            0.0
        };
        0.92 + 0.12 * noise2(s as f32 * 0.9, i as f32 * 1.3, seed) + ragged
    });
    mb
}

pub fn spawn(ctx: &mut SpawnCtx) {
    let parts = Parts {
        cloth: ctx.palette.cloth.clone(),
        shirt: ctx.palette.shirt.clone(),
        skin: ctx.palette.skin.clone(),
        hat: ctx.palette.straw.clone(),
        sack: ctx.palette.burlap.clone(),
        bone: ctx.palette.bone.clone(),
    };
    let seed = ctx.seed as u32 ^ 0x51B;
    let start = ctx.layout.ring[0];
    let meshes = &mut *ctx.meshes;

    // Pre-build every mesh so the spawn closure only places them.
    let pelvis = {
        let mut mb = MeshBuilder::new();
        mb.blob(
            Vec3::new(0.0, HIP_Y + 0.02, 0.0),
            Vec3::new(0.19, 0.12, 0.12),
            6,
            10,
            1.0,
            WHITE,
            &|_| 1.0,
        );
        mb
    };
    let thigh = tube_mesh(
        &[
            (Vec3::ZERO, 0.085),
            (Vec3::new(0.0, -THIGH * 0.5, -0.01), 0.07),
            (Vec3::new(0.0, -THIGH, 0.0), 0.06),
        ],
        8,
        seed,
        0.0,
    );
    let shin = tube_mesh(
        &[
            (Vec3::ZERO, 0.058),
            (Vec3::new(0.0, -SHIN * 0.5, 0.01), 0.05),
            (Vec3::new(0.0, -SHIN * 0.8, 0.0), 0.06),
        ],
        8,
        seed ^ 1,
        0.35,
    );
    let foot = {
        let mut mb = tube_mesh(
            &[
                (Vec3::new(0.0, -SHIN * 0.8, 0.0), 0.04),
                (Vec3::new(0.0, -SHIN + 0.04, 0.0), 0.035),
            ],
            7,
            seed ^ 2,
            0.0,
        );
        mb.blob(
            Vec3::new(0.0, -SHIN + 0.03, -0.07),
            Vec3::new(0.05, 0.035, 0.14),
            5,
            8,
            1.0,
            WHITE,
            &|_| 1.0,
        );
        mb
    };
    let torso = tube_mesh(
        &[
            (Vec3::new(0.0, -0.02, 0.0), 0.15),
            (Vec3::new(0.0, 0.3, 0.02), 0.13),
            (Vec3::new(0.0, 0.62, 0.0), 0.17),
            (Vec3::new(0.0, 0.8, 0.01), 0.16),
            (Vec3::new(0.0, 0.9, 0.0), 0.08),
        ],
        10,
        seed ^ 3,
        0.0,
    );
    // Ragged shirt tails hanging below the belt.
    let tails = {
        let mut mb = MeshBuilder::new();
        for k in 0..9 {
            let a = k as f32 / 9.0 * std::f32::consts::TAU;
            let out = Vec3::new(a.cos(), 0.0, a.sin() * 0.75);
            let top = out * 0.15 + Vec3::Y * 0.12;
            let len = 0.25 + 0.2 * noise2(k as f32, 1.0, seed);
            let bottom = out * 0.19 - Vec3::Y * len;
            let side = Vec3::new(-out.z, 0.0, out.x).normalize_or(Vec3::X);
            mb.ribbon(
                &[top, (top + bottom) * 0.5 + out * 0.02, bottom],
                &[0.11, 0.09, 0.03],
                side,
                &[WHITE],
            );
        }
        mb
    };
    let neck = tube_mesh(
        &[(Vec3::new(0.0, 0.86, 0.0), 0.05), (Vec3::new(0.0, 1.0, -0.01), 0.045)],
        7,
        seed ^ 4,
        0.0,
    );
    let head = {
        let mut mb = MeshBuilder::new();
        mb.blob(
            Vec3::new(0.0, 0.13, -0.01),
            Vec3::new(0.1, 0.15, 0.115),
            8,
            12,
            1.0,
            WHITE,
            &|d: Vec3| {
                // Hollow cheeks, long jaw.
                let cheek = if d.y < 0.1 && d.y > -0.6 && d.x.abs() > 0.5 {
                    0.9
                } else {
                    1.0
                };
                let jaw = if d.y < -0.5 { 1.08 } else { 1.0 };
                cheek * jaw
            },
        );
        mb
    };
    let hat = {
        let mut mb = MeshBuilder::new();
        // Weathered straw, pale enough to catch the moon: the broad brim is
        // the silhouette people remember.
        let straw = scale_rgb(WHITE, 1.05);
        // Broad brim, drooping at the edge.
        mb.lathe(
            Vec3::new(0.0, 0.24, 0.0),
            &[(0.1, 0.02), (0.32, 0.0), (0.56, -0.035), (0.72, -0.1)],
            28,
            1.0,
            straw,
        );
        mb.lathe(
            Vec3::new(0.0, 0.235, 0.0),
            &[(0.72, -0.105), (0.56, -0.045), (0.32, -0.01), (0.1, 0.01)],
            28,
            1.0,
            scale_rgb(straw, 0.7),
        );
        // Low, dented crown with a band.
        mb.lathe(
            Vec3::new(0.0, 0.24, 0.0),
            &[(0.14, 0.0), (0.145, 0.1), (0.13, 0.15), (0.06, 0.17), (0.0, 0.155)],
            18,
            1.0,
            straw,
        );
        mb.lathe(
            Vec3::new(0.0, 0.24, 0.0),
            &[(0.148, 0.0), (0.15, 0.035)],
            18,
            1.0,
            scale_rgb(WHITE, 0.18),
        );
        mb
    };
    let upper_arm = tube_mesh(
        &[(Vec3::ZERO, 0.055), (Vec3::new(0.0, -0.68, 0.0), 0.045)],
        7,
        seed ^ 5,
        0.25,
    );
    let forearm = {
        let mut mb = tube_mesh(
            &[(Vec3::ZERO, 0.04), (Vec3::new(0.0, -0.6, 0.01), 0.03)],
            7,
            seed ^ 6,
            0.0,
        );
        // Long fingers.
        for f in 0..4 {
            let x = -0.03 + f as f32 * 0.02;
            let base = Vec3::new(x, -0.62, 0.0);
            let tip = base + Vec3::new(x * 0.4, -0.2 - 0.02 * (f % 2) as f32, -0.03);
            let mid = base.lerp(tip, 0.5) + Vec3::new(0.0, 0.0, -0.02);
            mb.tube(
                &[
                    Ring {
                        center: base,
                        radius: 0.012,
                        color: WHITE,
                    },
                    Ring {
                        center: mid,
                        radius: 0.01,
                        color: WHITE,
                    },
                    Ring {
                        center: tip,
                        radius: 0.006,
                        color: WHITE,
                    },
                ],
                5,
                1.0,
                1.0,
                true,
                &|_, _| 1.0,
            );
        }
        mb
    };
    // Right arm, fixed in torso space: elbow in front, hand gripping the sack
    // neck at the shoulder.
    let shoulder_r = Vec3::new(0.21, 0.8, 0.0);
    let elbow_r = Vec3::new(0.3, 0.42, -0.16);
    let hand_r = Vec3::new(0.2, 0.9, 0.04);
    let right_sleeve = tube_mesh(&[(shoulder_r, 0.055), (elbow_r, 0.045)], 7, seed ^ 7, 0.2);
    let right_forearm = tube_mesh(&[(elbow_r, 0.04), (hand_r, 0.032)], 7, seed ^ 8, 0.0);
    let right_hand = {
        let mut mb = MeshBuilder::new();
        mb.blob(
            hand_r + Vec3::new(0.0, 0.02, 0.0),
            Vec3::new(0.045, 0.06, 0.04),
            5,
            8,
            1.0,
            WHITE,
            &|_| 1.0,
        );
        mb
    };
    // The sack hangs from the hand down his back (+Z is behind him).
    let sack = {
        let mut mb = MeshBuilder::new();
        let s = seed ^ 9;
        mb.blob(
            Vec3::new(-0.04, -0.44, 0.2),
            Vec3::new(0.26, 0.4, 0.2),
            10,
            14,
            1.2,
            WHITE,
            &move |d: Vec3| 0.8 + 0.35 * noise2(d.x * 3.0 + 5.0, d.y * 3.0 + d.z * 2.0, s),
        );
        mb.tube(
            &[
                Ring {
                    center: Vec3::new(0.0, 0.02, 0.02),
                    radius: 0.035,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(-0.02, -0.08, 0.1),
                    radius: 0.06,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(-0.03, -0.14, 0.16),
                    radius: 0.12,
                    color: WHITE,
                },
            ],
            10,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
        mb
    };
    let sack_bones = {
        let mut mb = MeshBuilder::new();
        for (a, b, r) in [
            (Vec3::new(0.0, -0.05, 0.08), Vec3::new(0.12, 0.2, 0.18), 0.018),
            (Vec3::new(-0.03, -0.06, 0.1), Vec3::new(-0.14, 0.16, 0.2), 0.015),
        ] {
            let d = (b - a).normalize();
            mb.tube(
                &[
                    Ring {
                        center: a,
                        radius: r,
                        color: WHITE,
                    },
                    Ring {
                        center: b - d * 0.03,
                        radius: r * 0.9,
                        color: WHITE,
                    },
                    Ring {
                        center: b + d * 0.02,
                        radius: r * 1.6,
                        color: WHITE,
                    },
                ],
                7,
                1.0,
                1.0,
                true,
                &|_, _| 1.0,
            );
        }
        mb
    };

    let mesh = |mb: MeshBuilder, meshes: &mut Assets<Mesh>| meshes.add(mb.build());
    let pelvis = mesh(pelvis, meshes);
    let thigh = mesh(thigh, meshes);
    let shin = mesh(shin, meshes);
    let foot = mesh(foot, meshes);
    let torso = mesh(torso, meshes);
    let tails = mesh(tails, meshes);
    let neck = mesh(neck, meshes);
    let head = mesh(head, meshes);
    let hat = mesh(hat, meshes);
    let upper_arm = mesh(upper_arm, meshes);
    let forearm = mesh(forearm, meshes);
    let right_sleeve = mesh(right_sleeve, meshes);
    let right_forearm = mesh(right_forearm, meshes);
    let right_hand = mesh(right_hand, meshes);
    let sack = mesh(sack, meshes);
    let sack_bones = mesh(sack_bones, meshes);

    let m = |h: &Handle<Mesh>, mat: &Handle<StandardMaterial>| {
        (Mesh3d(h.clone()), MeshMaterial3d(mat.clone()), Transform::IDENTITY)
    };

    ctx.commands
        .spawn((
            Name::new("El Silbón"),
            SilbonRoot,
            SilbonAnim::default(),
            Transform::from_xyz(start.x, -SINK_DEPTH, start.y),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((SilbonBody, Transform::IDENTITY, Visibility::Inherited))
                .with_children(|body| {
                    body.spawn(m(&pelvis, &parts.cloth));
                    for (hip, knee, x) in [
                        (JointKind::HipL, JointKind::KneeL, -0.11),
                        (JointKind::HipR, JointKind::KneeR, 0.11),
                    ] {
                        let rest = Vec3::new(x, HIP_Y, 0.0);
                        body.spawn((
                            Joint { kind: hip, rest },
                            Transform::from_translation(rest),
                            Visibility::Inherited,
                        ))
                        .with_children(|leg| {
                            leg.spawn(m(&thigh, &parts.cloth));
                            let knee_rest = Vec3::new(0.0, -THIGH, 0.0);
                            leg.spawn((
                                Joint {
                                    kind: knee,
                                    rest: knee_rest,
                                },
                                Transform::from_translation(knee_rest),
                                Visibility::Inherited,
                            ))
                            .with_children(|k| {
                                k.spawn(m(&shin, &parts.cloth));
                                k.spawn(m(&foot, &parts.skin));
                            });
                        });
                    }
                    let torso_rest = Vec3::new(0.0, HIP_Y, 0.0);
                    body.spawn((
                        Joint {
                            kind: JointKind::Torso,
                            rest: torso_rest,
                        },
                        Transform::from_translation(torso_rest),
                        Visibility::Inherited,
                    ))
                    .with_children(|t| {
                        t.spawn(m(&torso, &parts.shirt));
                        t.spawn(m(&tails, &parts.shirt));
                        t.spawn(m(&neck, &parts.skin));
                        t.spawn(m(&right_sleeve, &parts.shirt));
                        t.spawn(m(&right_forearm, &parts.skin));
                        t.spawn(m(&right_hand, &parts.skin));
                        let head_rest = Vec3::new(0.0, 0.98, -0.01);
                        t.spawn((
                            Joint {
                                kind: JointKind::Head,
                                rest: head_rest,
                            },
                            Transform::from_translation(head_rest),
                            Visibility::Inherited,
                        ))
                        .with_children(|h| {
                            h.spawn(m(&head, &parts.skin));
                            h.spawn(m(&hat, &parts.hat));
                        });
                        let shoulder_rest = Vec3::new(-0.21, 0.8, 0.0);
                        t.spawn((
                            Joint {
                                kind: JointKind::ShoulderL,
                                rest: shoulder_rest,
                            },
                            Transform::from_translation(shoulder_rest),
                            Visibility::Inherited,
                        ))
                        .with_children(|s| {
                            s.spawn(m(&upper_arm, &parts.shirt));
                            let elbow_rest = Vec3::new(0.0, -0.68, 0.0);
                            s.spawn((
                                Joint {
                                    kind: JointKind::ElbowL,
                                    rest: elbow_rest,
                                },
                                Transform::from_translation(elbow_rest),
                                Visibility::Inherited,
                            ))
                            .with_children(|e| {
                                e.spawn(m(&forearm, &parts.skin));
                            });
                        });
                        t.spawn((
                            Joint {
                                kind: JointKind::Sack,
                                rest: hand_r,
                            },
                            Transform::from_translation(hand_r),
                            Visibility::Inherited,
                        ))
                        .with_children(|s| {
                            s.spawn(m(&sack, &parts.sack));
                            s.spawn(m(&sack_bones, &parts.bone));
                        });
                    });
                });
        });
}

/// Pose the model from the truth layer every frame.
pub fn animate_silbon(
    time: Res<Time>,
    truth: Res<Truth>,
    tuning: Res<TuningRes>,
    mut roots: Query<(&mut Transform, &mut Visibility, &mut SilbonAnim), With<SilbonRoot>>,
    mut bodies: Query<&mut Transform, (With<SilbonBody>, Without<SilbonRoot>, Without<Joint>)>,
    mut joints: Query<(&Joint, &mut Transform), (Without<SilbonRoot>, Without<SilbonBody>)>,
) {
    let dt = time.delta_secs();
    let th = &truth.encounter.threat;
    let Ok((mut root, mut vis, mut anim)) = roots.single_mut() else {
        return;
    };
    let v = th.visibility(&tuning.0);
    let shown = v > 0.001;
    let want = if shown { Visibility::Visible } else { Visibility::Hidden };
    if *vis != want {
        *vis = want;
    }
    let target_yaw = (-th.facing.x).atan2(-th.facing.y);
    if !shown || !anim.placed {
        anim.yaw = target_yaw;
        anim.placed = shown;
        anim.phase = 0.0;
        anim.walk = 0.0;
    }
    if !shown {
        root.translation = Vec3::new(th.pos.x, -SINK_DEPTH, th.pos.y);
        return;
    }
    anim.t += dt;
    let turn = wrap_angle(target_yaw - anim.yaw);
    anim.yaw = wrap_angle(anim.yaw + turn * (dt * 4.0).min(1.0));
    let walk_target = (th.speed / 1.4).clamp(0.0, 1.0);
    anim.walk += (walk_target - anim.walk) * (dt * 5.0).min(1.0);
    anim.phase = (anim.phase + th.speed * dt / STRIDE * std::f32::consts::TAU) % std::f32::consts::TAU;

    // Rise from / sink into the grass along an ease curve.
    let ease = v * v * (3.0 - 2.0 * v);
    root.translation = Vec3::new(th.pos.x, -(1.0 - ease) * SINK_DEPTH, th.pos.y);
    root.rotation = Quat::from_rotation_y(anim.yaw);

    let p = anim.phase;
    let w = anim.walk;
    let t = anim.t;
    let warning = th.state == ThreatState::Warning;
    let hunting = th.state == ThreatState::Hunting;
    let breathe = (t * 0.9).sin();

    if let Ok(mut body) = bodies.single_mut() {
        body.translation = Vec3::new(0.0, 0.035 * (2.0 * p).cos() * w - 0.03 * w, 0.0);
        body.rotation = Quat::from_rotation_z(0.045 * p.sin() * w + 0.015 * (t * 0.4).sin());
    }

    for (joint, mut tf) in &mut joints {
        tf.translation = joint.rest;
        tf.rotation = match joint.kind {
            JointKind::HipL => Quat::from_rotation_x(0.42 * p.sin() * w),
            JointKind::HipR => Quat::from_rotation_x(-0.42 * p.sin() * w),
            JointKind::KneeL => Quat::from_rotation_x(-0.75 * (p + 1.1).sin().max(0.0) * w - 0.05),
            JointKind::KneeR => {
                Quat::from_rotation_x(-0.75 * (p + 1.1 + std::f32::consts::PI).sin().max(0.0) * w - 0.05)
            }
            JointKind::Torso => {
                let lean = if hunting { -0.22 } else { -0.12 };
                Quat::from_rotation_x(lean + 0.02 * breathe) * Quat::from_rotation_y(-0.06 * p.sin() * w)
            }
            JointKind::Head => {
                let tilt = if warning { 0.32 } else { 0.08 * (t * 0.3).sin() };
                let nod = if hunting { -0.12 } else { 0.05 * (t * 0.5).sin() };
                Quat::from_rotation_z(tilt) * Quat::from_rotation_x(nod)
            }
            JointKind::ShoulderL => {
                Quat::from_rotation_x(-0.45 * p.sin() * w + 0.03 * breathe) * Quat::from_rotation_z(-0.08)
            }
            JointKind::ElbowL => Quat::from_rotation_x(0.25 + 0.12 * p.sin() * w),
            JointKind::Sack => {
                Quat::from_rotation_x(0.1 * (p - 0.7).sin() * w + 0.03 * (t * 0.7).sin())
                    * Quat::from_rotation_z(0.06 * p.cos() * w)
            }
        };
    }
}
