//! The Silbón, after the concept sheet: a gaunt figure three metres tall in a
//! huge dark woven sombrero that hides his face, ragged brown clothes hanging
//! in long strips, pale bony hands with claw-like nails, and a burlap sack of
//! bones on his back with femurs jutting out over the shoulder.
//!
//! Procedural meshes on a small joint hierarchy, animated from the truth
//! layer's position, facing, speed and presence (he rises from and sinks into
//! the grass; no teleport pops). Restrained sway, long slow strides.

use bevy::prelude::*;

use super::SpawnCtx;
use super::mesh::{MeshBuilder, Rgba, Ring, WHITE, scale_rgb, srgb};
use super::noise2;
use crate::app::{Truth, TuningRes};
use crate::control::wrap_angle;
use crate::sim::ThreatState;

const HIP_Y: f32 = 1.66;
const THIGH: f32 = 0.84;
const SHIN: f32 = 0.84;
const STRIDE: f32 = 1.9;
/// How far below the ground he is when fully sunk.
const SINK_DEPTH: f32 = 3.7;
/// Torso joint to shoulder line, and arm segment lengths.
const CHEST: f32 = 0.84;
const UPPER_ARM: f32 = 0.74;
const FOREARM: f32 = 0.72;

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
    ShoulderR,
    ElbowR,
    Sack,
    /// The hanging coat strips, swaying behind the body.
    Coat,
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
    skin: Handle<StandardMaterial>,
    hat: Handle<StandardMaterial>,
    sack: Handle<StandardMaterial>,
    bone: Handle<StandardMaterial>,
    leather: Handle<StandardMaterial>,
}

// The palette of the sheet: browns nearly black in shadow, straw-dark hat,
// bone-pale hands.
fn rag() -> Rgba {
    srgb(0.36, 0.23, 0.15)
}
fn rag_dark() -> Rgba {
    srgb(0.2, 0.13, 0.09)
}

fn tube_mesh(points: &[(Vec3, f32)], sides: usize, seed: u32, rag: f32, tint: Rgba) -> MeshBuilder {
    let mut mb = MeshBuilder::new();
    let rings: Vec<Ring> = points
        .iter()
        .map(|&(c, r)| Ring {
            center: c,
            radius: r,
            color: tint,
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

/// Ragged strips of cloth hanging from a ring around a vertical axis at
/// `top`, longer behind (+Z) than in front.
fn strips(
    mb: &mut MeshBuilder,
    top: Vec3,
    radius: Vec2,
    count: usize,
    len: (f32, f32),
    width: f32,
    seed: u32,
    tint: Rgba,
) {
    for k in 0..count {
        let a = k as f32 / count as f32 * std::f32::consts::TAU + 0.11 * noise2(k as f32, 2.0, seed);
        let out = Vec3::new(a.cos(), 0.0, a.sin());
        // Behind him (+Z) the coat is longest, in front it is open and short.
        let behind = (out.z * 0.5 + 0.5).powf(0.8);
        let n = noise2(k as f32 * 1.9, 5.0, seed ^ 0x33);
        let length = (len.0 + (len.1 - len.0) * behind) * (0.55 + 0.8 * n * n.sqrt().max(0.3));
        let base = top + Vec3::new(out.x * radius.x, 0.0, out.z * radius.y);
        let side = Vec3::new(-out.z, 0.0, out.x);
        let sway = 0.05 + 0.09 * n;
        let pts = [
            base,
            base + out * (0.02 + sway * 0.4) - Vec3::Y * length * 0.35,
            base + out * (0.05 + sway) - Vec3::Y * length * 0.7 + side * 0.06 * (n - 0.5),
            base + out * (0.07 + sway * 1.5) - Vec3::Y * length + side * 0.12 * (n - 0.45),
        ];
        let w = width * (0.6 + 0.9 * noise2(k as f32 * 0.7, 9.0, seed ^ 0x44));
        let tip = 0.08 + 0.4 * noise2(k as f32 * 1.1, 13.0, seed ^ 0x45);
        mb.ribbon(
            &pts,
            &[w, w * 1.05, w * 0.8, w * tip],
            side,
            &[
                scale_rgb(tint, 0.9),
                tint,
                scale_rgb(tint, 0.75 + 0.3 * n),
                scale_rgb(tint, 0.5 + 0.2 * n),
            ],
        );
    }
}

/// A long, bony hand hanging from the wrist at `wrist` (pointing along -Y):
/// a narrow palm, four knuckled fingers with claw-like nails and a thumb.
fn hand(mb: &mut MeshBuilder, wrist: Vec3, side: f32) {
    let bone = srgb(0.72, 0.66, 0.55);
    let dark = srgb(0.42, 0.35, 0.26);
    let nail = srgb(0.5, 0.43, 0.3);
    // Palm.
    mb.blob(
        wrist + Vec3::new(0.0, -0.075, 0.0),
        Vec3::new(0.05, 0.085, 0.022),
        6,
        9,
        1.0,
        bone,
        &|_| 1.0,
    );
    // (x offset, length, sideways splay, curl gain): uneven, as if the
    // fingers were half-hooked on something unseen.
    let fingers = [
        (-0.036_f32, 0.29_f32, 0.55_f32, 1.25_f32),
        (-0.012, 0.35, 0.25, 0.85),
        (0.014, 0.33, -0.1, 1.05),
        (0.038, 0.25, -0.45, 1.5),
    ];
    for (x, length, spread, gain) in fingers {
        let base = wrist + Vec3::new(x, -0.15, 0.0);
        // Each finger: three bones, curling gently toward the front (-Z).
        let mut p = base;
        let mut rings = vec![Ring {
            center: p,
            radius: 0.011,
            color: bone,
        }];
        let dir0 = Vec3::new(spread * 0.45 * side, -1.0, 0.0).normalize();
        for (i, (frac, curl)) in [(0.42, 0.18), (0.33, 0.6), (0.25, 1.1)].into_iter().enumerate() {
            let dir = (dir0 + Vec3::new(-spread * 0.1 * side * i as f32, 0.0, -curl * gain)).normalize();
            p += dir * length * frac;
            // Knuckle thickening at each joint.
            rings.push(Ring {
                center: p - dir * 0.008,
                radius: 0.0072 + 0.0035 * (1.0 - i as f32 * 0.25),
                color: dark,
            });
            rings.push(Ring {
                center: p,
                radius: 0.0078 - 0.0012 * i as f32,
                color: bone,
            });
        }
        mb.tube(&rings, 6, 1.0, 1.0, false, &|_, _| 1.0);
        // The nail: a long pointed claw.
        let tip_dir = (dir0 + Vec3::new(0.0, 0.0, -1.6 * gain)).normalize();
        mb.tube(
            &[
                Ring {
                    center: p - tip_dir * 0.004,
                    radius: 0.0085,
                    color: nail,
                },
                Ring {
                    center: p + tip_dir * 0.03,
                    radius: 0.0055,
                    color: nail,
                },
                Ring {
                    center: p + tip_dir * 0.075,
                    radius: 0.0004,
                    color: scale_rgb(nail, 0.7),
                },
            ],
            5,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
    }
    // Thumb, hooking inward.
    let base = wrist + Vec3::new(0.05 * -side, -0.08, 0.0);
    let dir = Vec3::new(-0.35 * side, -1.0, -0.35).normalize();
    let mid = base + dir * 0.1;
    let tip = mid + (dir + Vec3::new(0.0, 0.0, -0.4)).normalize() * 0.09;
    mb.tube(
        &[
            Ring {
                center: base,
                radius: 0.013,
                color: bone,
            },
            Ring {
                center: mid,
                radius: 0.01,
                color: dark,
            },
            Ring {
                center: tip,
                radius: 0.006,
                color: bone,
            },
            Ring {
                center: tip + (tip - mid).normalize() * 0.05,
                radius: 0.0004,
                color: nail,
            },
        ],
        6,
        1.0,
        1.0,
        false,
        &|_, _| 1.0,
    );
}

/// A femur: a shaft with a knobbed end at each side.
fn femur(mb: &mut MeshBuilder, a: Vec3, b: Vec3, r: f32) {
    let bone = srgb(0.86, 0.8, 0.66);
    let d = (b - a).normalize();
    let ring = |c: Vec3, k: f32, tint: Rgba| Ring {
        center: c,
        radius: r * k,
        color: tint,
    };
    mb.tube(
        &[
            ring(a - d * 0.025, 1.5, bone),
            ring(a + d * 0.025, 1.9, bone),
            ring(a + d * 0.07, 1.0, bone),
            ring(a + (b - a) * 0.5, 0.82, scale_rgb(bone, 0.9)),
            ring(b - d * 0.07, 1.0, bone),
            ring(b - d * 0.025, 1.9, bone),
            ring(b + d * 0.025, 1.6, bone),
        ],
        8,
        1.0,
        1.0,
        true,
        &|i, s| if (i == 1 || i == 5) && s % 4 < 2 { 1.15 } else { 1.0 },
    );
    // The second condyle.
    let side = d.cross(Vec3::Y).normalize_or(Vec3::X) * r * 1.4;
    mb.blob(b + d * 0.01 + side, Vec3::splat(r * 1.4), 5, 8, 1.0, bone, &|_| 1.0);
    mb.blob(a + d * -0.01 - side, Vec3::splat(r * 1.3), 5, 8, 1.0, bone, &|_| 1.0);
}

pub fn spawn(ctx: &mut SpawnCtx) {
    let parts = Parts {
        cloth: ctx.palette.cloth.clone(),
        skin: ctx.palette.skin.clone(),
        hat: ctx.palette.straw.clone(),
        sack: ctx.palette.burlap.clone(),
        bone: ctx.palette.bone.clone(),
        leather: ctx.palette.leather.clone(),
    };
    let seed = ctx.seed as u32 ^ 0x51B;
    let start = ctx.layout.patrol.nodes[0];
    let meshes = &mut *ctx.meshes;

    // ---- Legs: thin, wrapped in rags, ending in ragged cuffs and boots.
    let pelvis = {
        let mut mb = MeshBuilder::new();
        mb.blob(
            Vec3::new(0.0, HIP_Y + 0.02, 0.0),
            Vec3::new(0.17, 0.11, 0.11),
            6,
            10,
            1.0,
            rag_dark(),
            &|_| 1.0,
        );
        mb
    };
    let thigh = tube_mesh(
        &[
            (Vec3::ZERO, 0.075),
            (Vec3::new(0.0, -THIGH * 0.5, -0.01), 0.062),
            (Vec3::new(0.0, -THIGH, 0.0), 0.05),
        ],
        8,
        seed,
        0.0,
        rag_dark(),
    );
    let shin = {
        let mut mb = tube_mesh(
            &[
                (Vec3::ZERO, 0.052),
                (Vec3::new(0.0, -SHIN * 0.5, 0.01), 0.045),
                (Vec3::new(0.0, -SHIN * 0.78, 0.0), 0.06),
            ],
            8,
            seed ^ 1,
            0.4,
            rag_dark(),
        );
        // Frayed cuff strips over the boot.
        strips(
            &mut mb,
            Vec3::new(0.0, -SHIN * 0.5, 0.0),
            Vec2::splat(0.055),
            9,
            (0.3, 0.42),
            0.045,
            seed ^ 0x77,
            rag_dark(),
        );
        mb
    };
    let foot = {
        let mut mb = tube_mesh(
            &[
                (Vec3::new(0.0, -SHIN * 0.78, 0.0), 0.04),
                (Vec3::new(0.0, -SHIN + 0.05, 0.0), 0.035),
            ],
            7,
            seed ^ 2,
            0.0,
            srgb(0.12, 0.09, 0.07),
        );
        mb.blob(
            Vec3::new(0.0, -SHIN + 0.04, -0.09),
            Vec3::new(0.06, 0.045, 0.17),
            5,
            8,
            1.0,
            srgb(0.1, 0.075, 0.06),
            &|_| 1.0,
        );
        mb
    };

    // ---- Torso: narrow shoulders, a long ragged coat.
    let torso = {
        let mut mb = tube_mesh(
            &[
                (Vec3::new(0.0, -0.03, 0.0), 0.135),
                (Vec3::new(0.0, 0.3, 0.02), 0.125),
                (Vec3::new(0.0, 0.62, 0.0), 0.16),
                (Vec3::new(0.0, CHEST - 0.06, 0.01), 0.175),
                (Vec3::new(0.0, CHEST + 0.06, 0.0), 0.075),
            ],
            10,
            seed ^ 3,
            0.0,
            rag(),
        );
        // Collar strips at the neck.
        strips(
            &mut mb,
            Vec3::new(0.0, CHEST + 0.04, 0.0),
            Vec2::splat(0.09),
            8,
            (0.22, 0.4),
            0.05,
            seed ^ 0x99,
            rag_dark(),
        );
        mb
    };
    // The long skirt of the coat: strips that hang from the waist and sway.
    let coat = {
        let mut mb = MeshBuilder::new();
        strips(
            &mut mb,
            Vec3::new(0.0, 0.06, 0.0),
            Vec2::new(0.155, 0.13),
            26,
            (0.62, 1.3),
            0.13,
            seed ^ 0x4A,
            rag(),
        );
        // A second, inner layer in darker cloth.
        strips(
            &mut mb,
            Vec3::new(0.0, 0.12, 0.0),
            Vec2::new(0.13, 0.11),
            18,
            (0.5, 1.0),
            0.11,
            seed ^ 0x4B,
            rag_dark(),
        );
        mb
    };
    let neck = tube_mesh(
        &[
            (Vec3::new(0.0, CHEST + 0.02, 0.0), 0.05),
            (Vec3::new(0.0, CHEST + 0.2, -0.02), 0.042),
        ],
        7,
        seed ^ 4,
        0.0,
        srgb(0.32, 0.27, 0.22),
    );
    // A narrow skull, all in shadow under the brim.
    let head = {
        let mut mb = MeshBuilder::new();
        mb.blob(
            Vec3::new(0.0, 0.14, -0.015),
            Vec3::new(0.085, 0.14, 0.1),
            8,
            12,
            1.0,
            srgb(0.16, 0.13, 0.11),
            &|d: Vec3| {
                let cheek = if d.y < 0.1 && d.y > -0.6 && d.x.abs() > 0.5 {
                    0.86
                } else {
                    1.0
                };
                let jaw = if d.y < -0.5 { 1.1 } else { 1.0 };
                cheek * jaw
            },
        );
        // Two pale eyes sunk in the dark: only light finds them.
        for x in [-0.032_f32, 0.032] {
            mb.blob(
                Vec3::new(x, 0.165, -0.1),
                Vec3::splat(0.012),
                4,
                6,
                1.0,
                srgb(0.92, 0.88, 0.74),
                &|_| 1.0,
            );
        }
        mb
    };

    // ---- The sombrero: a broad, slightly drooping woven brim with a frayed
    // rim, a low crown and a leather band.
    let hat = {
        let mut mb = MeshBuilder::new();
        let straw = srgb(0.5, 0.36, 0.26);
        let brim_r = 0.94;
        mb.lathe(
            Vec3::new(0.0, 0.0, 0.0),
            &[(0.17, 0.02), (0.4, 0.0), (0.68, -0.05), (brim_r, -0.13)],
            36,
            2.0,
            straw,
        );
        mb.lathe(
            Vec3::new(0.0, -0.012, 0.0),
            &[(brim_r, -0.135), (0.68, -0.062), (0.4, -0.014), (0.17, 0.006)],
            36,
            2.0,
            scale_rgb(straw, 0.55),
        );
        // Crown.
        mb.lathe(
            Vec3::new(0.0, 0.0, 0.0),
            &[(0.19, 0.0), (0.195, 0.12), (0.17, 0.19), (0.08, 0.215), (0.0, 0.205)],
            22,
            1.5,
            scale_rgb(straw, 0.95),
        );
        // Band and buckle.
        mb.lathe(
            Vec3::new(0.0, 0.0, 0.0),
            &[(0.199, 0.005), (0.2, 0.05)],
            22,
            1.0,
            srgb(0.08, 0.055, 0.04),
        );
        // Frayed rim: two uneven layers of straw and thread, in clumps of
        // very different length so the edge reads torn, not combed.
        for (layer, (count, inset, drop)) in [(52_usize, 0.02_f32, 0.0_f32), (34, 0.13, 0.02)]
            .into_iter()
            .enumerate()
        {
            for k in 0..count {
                let jitter = noise2(k as f32 * 2.3, 7.0 + layer as f32, seed ^ 0xF2) - 0.5;
                let a = (k as f32 + jitter * 0.9) / count as f32 * std::f32::consts::TAU;
                let out = Vec3::new(a.cos(), 0.0, a.sin());
                let n = noise2(k as f32 * 1.3, 4.0 + layer as f32 * 3.0, seed ^ 0xF1);
                let len = 0.05 + 0.45 * n.powf(2.4);
                let wide = 0.012 + 0.03 * noise2(k as f32 * 0.8, 11.0, seed ^ 0xF3);
                // The brim itself droops lower on one side.
                let sag = 0.05 * out.x.max(0.0);
                let base = out * (brim_r - inset) + Vec3::Y * (-0.13 + 0.06 * inset - sag - drop);
                let side = Vec3::new(-out.z, 0.0, out.x);
                let bend = (n - 0.4) * 0.09;
                mb.ribbon(
                    &[
                        base,
                        base + out * (0.01 + bend * 0.4) - Vec3::Y * len * 0.5 + side * bend * 0.5,
                        base + out * (0.02 + bend) - Vec3::Y * len + side * bend,
                    ],
                    &[wide, wide * 0.8, wide * 0.1],
                    side,
                    &[
                        scale_rgb(straw, 0.85 - 0.15 * layer as f32),
                        scale_rgb(straw, 0.62),
                        scale_rgb(straw, 0.4),
                    ],
                );
            }
        }
        mb
    };

    // ---- Arms: dark sleeves ending in strips, bare bone below the elbow.
    let upper_arm = {
        let mut mb = tube_mesh(
            &[
                (Vec3::ZERO, 0.055),
                (Vec3::new(0.0, -UPPER_ARM * 0.5, 0.0), 0.046),
                (Vec3::new(0.0, -UPPER_ARM, 0.0), 0.045),
            ],
            8,
            seed ^ 5,
            0.3,
            rag(),
        );
        strips(
            &mut mb,
            Vec3::new(0.0, -UPPER_ARM * 0.45, 0.0),
            Vec2::splat(0.05),
            8,
            (0.28, 0.52),
            0.06,
            seed ^ 0x51,
            rag_dark(),
        );
        mb
    };
    let forearm = {
        let mut mb = tube_mesh(
            &[
                (Vec3::ZERO, 0.038),
                (Vec3::new(0.0, -FOREARM * 0.5, 0.01), 0.028),
                (Vec3::new(0.0, -FOREARM, 0.0), 0.024),
            ],
            8,
            seed ^ 6,
            0.0,
            srgb(0.78, 0.72, 0.6),
        );
        strips(
            &mut mb,
            Vec3::new(0.0, -0.04, 0.0),
            Vec2::splat(0.045),
            7,
            (0.24, 0.42),
            0.05,
            seed ^ 0x52,
            rag(),
        );
        hand(&mut mb, Vec3::new(0.0, -FOREARM, 0.0), 1.0);
        mb
    };
    let forearm_l = {
        let mut mb = tube_mesh(
            &[
                (Vec3::ZERO, 0.038),
                (Vec3::new(0.0, -FOREARM * 0.5, 0.01), 0.028),
                (Vec3::new(0.0, -FOREARM, 0.0), 0.024),
            ],
            8,
            seed ^ 7,
            0.0,
            srgb(0.78, 0.72, 0.6),
        );
        strips(
            &mut mb,
            Vec3::new(0.0, -0.04, 0.0),
            Vec2::splat(0.045),
            7,
            (0.24, 0.42),
            0.05,
            seed ^ 0x53,
            rag(),
        );
        hand(&mut mb, Vec3::new(0.0, -FOREARM, 0.0), -1.0);
        mb
    };

    // ---- The burden: a burlap sack slung on his back, femurs jutting out
    // over the shoulder, a strap across the chest.
    let sack = {
        let mut mb = MeshBuilder::new();
        let s = seed ^ 9;
        mb.blob(
            Vec3::new(-0.02, -0.46, 0.19),
            Vec3::new(0.25, 0.36, 0.2),
            10,
            14,
            1.4,
            scale_rgb(WHITE, 0.62),
            &move |d: Vec3| 0.82 + 0.32 * noise2(d.x * 3.0 + 5.0, d.y * 3.0 + d.z * 2.0, s),
        );
        // Gathered neck, tied with cord.
        mb.tube(
            &[
                Ring {
                    center: Vec3::new(0.0, -0.06, 0.08),
                    radius: 0.16,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(0.0, 0.06, 0.05),
                    radius: 0.075,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(0.01, 0.16, 0.04),
                    radius: 0.09,
                    color: WHITE,
                },
                Ring {
                    center: Vec3::new(0.01, 0.2, 0.04),
                    radius: 0.1,
                    color: WHITE,
                },
            ],
            10,
            1.0,
            1.5,
            false,
            &|_, _| 1.0,
        );
        // Patches sewn on.
        for (c, half) in [
            (Vec3::new(0.16, -0.4, 0.4), Vec3::new(0.08, 0.1, 0.015)),
            (Vec3::new(-0.18, -0.62, 0.38), Vec3::new(0.09, 0.07, 0.015)),
        ] {
            mb.cuboid(
                c,
                Quat::from_rotation_y(0.3),
                half,
                1.0,
                Vec2::ZERO,
                scale_rgb(WHITE, 0.55),
            );
        }
        mb
    };
    let cord = {
        let mut mb = MeshBuilder::new();
        mb.tube(
            &[
                Ring {
                    center: Vec3::new(0.0, 0.1, 0.045),
                    radius: 0.078,
                    color: srgb(0.2, 0.14, 0.09),
                },
                Ring {
                    center: Vec3::new(0.0, 0.12, 0.045),
                    radius: 0.079,
                    color: srgb(0.2, 0.14, 0.09),
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
        femur(&mut mb, Vec3::new(0.02, 0.04, 0.06), Vec3::new(0.2, 0.62, 0.12), 0.024);
        femur(
            &mut mb,
            Vec3::new(-0.02, 0.05, 0.07),
            Vec3::new(-0.18, 0.56, 0.16),
            0.022,
        );
        femur(&mut mb, Vec3::new(0.0, 0.04, 0.09), Vec3::new(0.02, 0.5, 0.24), 0.02);
        femur(
            &mut mb,
            Vec3::new(0.06, -0.15, 0.24),
            Vec3::new(0.3, -0.05, 0.42),
            0.018,
        );
        mb
    };
    // The strap is thin leather: a few beams across the chest.
    let strap = {
        let mut mb = MeshBuilder::new();
        let pts = [
            Vec3::new(0.09, 0.02, 0.1),
            Vec3::new(0.19, 0.78, 0.02),
            Vec3::new(0.14, 0.7, -0.12),
            Vec3::new(0.02, 0.36, -0.15),
            Vec3::new(-0.12, 0.05, -0.1),
        ];
        for w in pts.windows(2) {
            mb.beam(w[0], w[1], 0.05, 0.014, 0.0, 1.0, Vec2::ZERO, srgb(0.14, 0.1, 0.07));
        }
        mb
    };

    let mesh = |mb: MeshBuilder, meshes: &mut Assets<Mesh>| meshes.add(mb.build());
    let pelvis = mesh(pelvis, meshes);
    let thigh = mesh(thigh, meshes);
    let shin = mesh(shin, meshes);
    let foot = mesh(foot, meshes);
    let torso = mesh(torso, meshes);
    let coat = mesh(coat, meshes);
    let neck = mesh(neck, meshes);
    let head = mesh(head, meshes);
    let hat = mesh(hat, meshes);
    let upper_arm = mesh(upper_arm, meshes);
    let forearm = mesh(forearm, meshes);
    let forearm_l = mesh(forearm_l, meshes);
    let sack = mesh(sack, meshes);
    let cord = mesh(cord, meshes);
    let sack_bones = mesh(sack_bones, meshes);
    let strap = mesh(strap, meshes);

    let m = |h: &Handle<Mesh>, mat: &Handle<StandardMaterial>| {
        (Mesh3d(h.clone()), MeshMaterial3d(mat.clone()), Transform::IDENTITY)
    };
    let joint = |kind: JointKind, rest: Vec3| {
        (
            Joint { kind, rest },
            Transform::from_translation(rest),
            Visibility::Inherited,
        )
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
                        (JointKind::HipL, JointKind::KneeL, -0.1),
                        (JointKind::HipR, JointKind::KneeR, 0.1),
                    ] {
                        body.spawn(joint(hip, Vec3::new(x, HIP_Y, 0.0))).with_children(|leg| {
                            leg.spawn(m(&thigh, &parts.cloth));
                            leg.spawn(joint(knee, Vec3::new(0.0, -THIGH, 0.0))).with_children(|k| {
                                k.spawn(m(&shin, &parts.cloth));
                                k.spawn(m(&foot, &parts.leather));
                            });
                        });
                    }
                    body.spawn(joint(JointKind::Torso, Vec3::new(0.0, HIP_Y, 0.0)))
                        .with_children(|t| {
                            t.spawn(m(&torso, &parts.cloth));
                            t.spawn(m(&neck, &parts.skin));
                            t.spawn(m(&strap, &parts.leather));
                            t.spawn(joint(JointKind::Coat, Vec3::new(0.0, 0.0, 0.0)))
                                .with_children(|c| {
                                    c.spawn(m(&coat, &parts.cloth));
                                });
                            t.spawn(joint(JointKind::Head, Vec3::new(0.0, CHEST + 0.16, -0.01)))
                                .with_children(|h| {
                                    h.spawn(m(&head, &parts.skin));
                                    // The hat sits low and a little forward, hiding the face.
                                    h.spawn((
                                        Mesh3d(hat.clone()),
                                        MeshMaterial3d(parts.hat.clone()),
                                        Transform::from_xyz(0.0, 0.2, -0.02)
                                            .with_rotation(Quat::from_rotation_x(-0.11)),
                                    ));
                                });
                            for (sh, el, x, fore) in [
                                (JointKind::ShoulderL, JointKind::ElbowL, -0.21, &forearm_l),
                                (JointKind::ShoulderR, JointKind::ElbowR, 0.21, &forearm),
                            ] {
                                t.spawn(joint(sh, Vec3::new(x, CHEST - 0.03, 0.0))).with_children(|s| {
                                    s.spawn(m(&upper_arm, &parts.cloth));
                                    s.spawn(joint(el, Vec3::new(0.0, -UPPER_ARM, 0.0))).with_children(|e| {
                                        e.spawn(m(fore, &parts.bone));
                                    });
                                });
                            }
                            t.spawn(joint(JointKind::Sack, Vec3::new(0.05, CHEST - 0.02, 0.12)))
                                .with_children(|s| {
                                    s.spawn(m(&sack, &parts.sack));
                                    s.spawn(m(&cord, &parts.leather));
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
    layout: Res<crate::app::LayoutRes>,
    mut roots: Query<(&mut Transform, &mut Visibility, &mut SilbonAnim), With<SilbonRoot>>,
    mut bodies: Query<&mut Transform, (With<SilbonBody>, Without<SilbonRoot>, Without<Joint>)>,
    mut joints: Query<(&Joint, &mut Transform), (Without<SilbonRoot>, Without<SilbonBody>)>,
    fright: Res<super::omen::Fright>,
    camera: Single<
        &Transform,
        (
            With<crate::player::Player>,
            Without<SilbonRoot>,
            Without<SilbonBody>,
            Without<Joint>,
        ),
    >,
) {
    let dt = time.delta_secs();
    let th = &truth.encounter.threat;
    let Ok((mut root, mut vis, mut anim)) = roots.single_mut() else {
        return;
    };
    // Caught: he is in your face, arms up, whatever the snapshot says.
    if let Some(s) = fright.lunge {
        let k = (s / super::omen::LUNGE_IN).min(1.0);
        let ease = k * k * (3.0 - 2.0 * k);
        let f3 = camera.rotation * Vec3::NEG_Z;
        let fwd = Vec3::new(f3.x, 0.0, f3.z).normalize_or(Vec3::NEG_Z);
        // He looms at arm's length, his face a little above the eye; from the
        // ground he rises out of the earth to meet it.
        let (near, lift, torso, head) = (1.7, HIP_Y + CHEST + 0.25, -0.15, 0.6);
        let dist = 3.6 + (near - 3.6) * ease;
        let shake = 0.025 * (s * 61.0).sin() * (1.0 - (s / super::omen::LUNGE).min(1.0));
        root.translation = camera.translation + fwd * dist - Vec3::Y * lift + Vec3::X * shake;
        root.rotation = Quat::from_rotation_y(fwd.x.atan2(fwd.z));
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
        anim.placed = false;
        for (joint, mut tf) in &mut joints {
            tf.translation = joint.rest;
            tf.rotation = match joint.kind {
                // The head snaps back: the brim lifts off the face. Arms thrown
                // wide, the hooked hands at the edges of sight.
                JointKind::Torso => Quat::from_rotation_x(torso),
                JointKind::Head => Quat::from_rotation_x(head) * Quat::from_rotation_z(0.2 * (s * 9.0).sin()),
                JointKind::ShoulderL => Quat::from_rotation_x(-0.7) * Quat::from_rotation_z(-1.1),
                JointKind::ShoulderR => Quat::from_rotation_x(-0.7) * Quat::from_rotation_z(1.1),
                JointKind::ElbowL | JointKind::ElbowR => Quat::from_rotation_x(0.7),
                _ => Quat::IDENTITY,
            };
        }
        return;
    }
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
    root.translation = Vec3::new(
        th.pos.x,
        layout.0.surface_height(th.pos) - (1.0 - ease) * SINK_DEPTH,
        th.pos.y,
    );
    root.rotation = Quat::from_rotation_y(anim.yaw);

    let p = anim.phase;
    let w = anim.walk;
    let t = anim.t;
    let warning = th.state == ThreatState::Warning;
    let hunting = th.state == ThreatState::Hunting;
    let counting = th.state == ThreatState::Counting;
    let breathe = (t * 0.9).sin();

    if let Ok(mut body) = bodies.single_mut() {
        // Counting his bones, he stoops right down.
        let stoop = if counting { -0.32 } else { 0.0 };
        body.translation = Vec3::new(0.0, 0.03 * (2.0 * p).cos() * w - 0.03 * w + stoop, 0.0);
        body.rotation = Quat::from_rotation_z(0.04 * p.sin() * w + 0.015 * (t * 0.4).sin());
    }

    for (joint, mut tf) in &mut joints {
        tf.translation = joint.rest;
        tf.rotation = match joint.kind {
            JointKind::HipL => Quat::from_rotation_x(0.4 * p.sin() * w),
            JointKind::HipR => Quat::from_rotation_x(-0.4 * p.sin() * w),
            JointKind::KneeL => Quat::from_rotation_x(-0.72 * (p + 1.1).sin().max(0.0) * w - 0.05),
            JointKind::KneeR => {
                Quat::from_rotation_x(-0.72 * (p + 1.1 + std::f32::consts::PI).sin().max(0.0) * w - 0.05)
            }
            JointKind::Torso => {
                let lean = if counting {
                    -0.55
                } else if hunting {
                    -0.24
                } else {
                    -0.13
                };
                Quat::from_rotation_x(lean + 0.02 * breathe) * Quat::from_rotation_y(-0.06 * p.sin() * w)
            }
            JointKind::Head => {
                let tilt = if warning { 0.3 } else { 0.07 * (t * 0.3).sin() };
                let nod = if hunting {
                    -0.14
                } else if counting {
                    0.4
                } else {
                    0.05 * (t * 0.5).sin()
                };
                Quat::from_rotation_z(tilt) * Quat::from_rotation_x(nod)
            }
            // The long arms hang and swing against the legs, hands reaching
            // for the bones when he counts; a hunt lifts them, fingers spread.
            JointKind::ShoulderL => {
                let reach = if hunting {
                    -0.55
                } else if counting {
                    -0.3
                } else {
                    0.0
                };
                Quat::from_rotation_x(-0.35 * p.sin() * w + 0.03 * breathe + reach) * Quat::from_rotation_z(-0.09)
            }
            JointKind::ShoulderR => {
                let reach = if hunting {
                    -0.55
                } else if counting {
                    -0.3
                } else {
                    0.0
                };
                Quat::from_rotation_x(0.35 * p.sin() * w + 0.03 * breathe + reach) * Quat::from_rotation_z(0.09)
            }
            JointKind::ElbowL => Quat::from_rotation_x(0.14 + 0.1 * p.sin() * w),
            JointKind::ElbowR => Quat::from_rotation_x(0.14 - 0.1 * p.sin() * w),
            JointKind::Sack => {
                Quat::from_rotation_x(0.08 * (p - 0.7).sin() * w + 0.03 * (t * 0.7).sin())
                    * Quat::from_rotation_z(0.05 * p.cos() * w)
            }
            // The coat trails behind him as he walks and stirs in the wind.
            JointKind::Coat => Quat::from_rotation_x(0.1 * w + 0.045 * (2.0 * p).sin() * w + 0.02 * (t * 0.8).sin()),
        };
    }
}
