//! Purposeful props: the table with its kerosene lantern, the bone satchel and
//! the note, shelf and jars, hammock, tinajas, drum, firewood, bench, a straw
//! hat on a nail, a 1998 calendar and the porch lamp. Their footprints are the
//! layout's furniture blockers.

use bevy::prelude::*;

use super::mesh::{MeshBuilder, Ring, WHITE, mix_rgb, scale_rgb, srgb};
use super::{Flicker, SatchelAsset, SpawnCtx, noise2};
use crate::geometry::{Furniture, Rect2, Shape};

/// Luminous power of the table lantern (lumens).
pub const LANTERN_LUMENS: f32 = 42_000.0;
/// Luminous power of the porch lamp (lumens).
pub const PORCH_LUMENS: f32 = 26_000.0;

fn furniture<'a>(layout: &'a crate::geometry::Layout, name: &str) -> Option<&'a Furniture> {
    layout.furniture.iter().find(|f| f.name == name)
}

fn rect_of(f: &Furniture) -> Rect2 {
    match f.shape {
        Shape::Rect(r) => r,
        Shape::Circle { center, radius } => Rect2::from_center(center, Vec2::splat(radius)),
    }
}

pub fn spawn(ctx: &mut SpawnCtx) -> SatchelAsset {
    let mut rng = ctx.rng(91);
    let mut wood = MeshBuilder::new();
    let mut dark = MeshBuilder::new();
    let mut clay = MeshBuilder::new();
    let mut glass = MeshBuilder::new();
    let mut tin = MeshBuilder::new();
    let mut rust = MeshBuilder::new();
    let mut leather = MeshBuilder::new();
    let mut cloth = MeshBuilder::new();
    let layout = ctx.layout;

    // --- Table.
    let t = layout.table;
    let h = layout.table_height;
    let top_col = srgb(0.95, 0.9, 0.85);
    let planks = 3;
    let pw = (t.max.y - t.min.y) / planks as f32;
    for i in 0..planks {
        let z = t.min.y + pw * (i as f32 + 0.5);
        dark.cuboid(
            Vec3::new(t.center().x, h - 0.025, z),
            Quat::from_rotation_x(rng.signed(0.01)),
            Vec3::new(t.half().x, 0.025, pw * 0.5 - 0.004),
            1.0,
            Vec2::new(rng.range(0.0, 5.0), rng.range(0.0, 5.0)),
            scale_rgb(top_col, rng.range(0.85, 1.05)),
        );
    }
    for (x, z) in [
        (t.min.x + 0.08, t.min.y + 0.08),
        (t.max.x - 0.08, t.min.y + 0.08),
        (t.min.x + 0.08, t.max.y - 0.08),
        (t.max.x - 0.08, t.max.y - 0.08),
    ] {
        dark.cuboid(
            Vec3::new(x, (h - 0.05) * 0.5, z),
            Quat::IDENTITY,
            Vec3::new(0.035, (h - 0.05) * 0.5, 0.035),
            1.0,
            Vec2::ZERO,
            top_col,
        );
    }
    dark.beam(
        Vec3::new(t.min.x + 0.08, 0.18, t.center().y),
        Vec3::new(t.max.x - 0.08, 0.18, t.center().y),
        0.04,
        0.06,
        0.0,
        1.0,
        Vec2::ZERO,
        top_col,
    );

    // --- Chair (rawhide seat, back toward the wall).
    if let Some(f) = furniture(layout, "chair") {
        let r = rect_of(f);
        let c = r.center();
        let hx = r.half().x;
        let hz = r.half().y;
        for (dx, dz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let top = if dx > 0.0 { 0.95 } else { 0.45 };
            dark.cuboid(
                Vec3::new(c.x + dx * (hx - 0.03), top * 0.5, c.y + dz * (hz - 0.03)),
                Quat::IDENTITY,
                Vec3::new(0.025, top * 0.5, 0.025),
                1.0,
                Vec2::ZERO,
                top_col,
            );
        }
        leather.cuboid(
            Vec3::new(c.x, 0.45, c.y),
            Quat::IDENTITY,
            Vec3::new(hx - 0.01, 0.015, hz - 0.01),
            1.0,
            Vec2::ZERO,
            WHITE,
        );
        for y in [0.68_f32, 0.85] {
            dark.cuboid(
                Vec3::new(c.x + hx - 0.03, y, c.y),
                Quat::IDENTITY,
                Vec3::new(0.02, 0.04, hz - 0.02),
                1.0,
                Vec2::ZERO,
                top_col,
            );
        }
    }

    // --- Shelf with jars, bottles and a folded cloth.
    if let Some(f) = furniture(layout, "shelf") {
        let r = rect_of(f);
        let c = r.center();
        for y in [0.5_f32, 1.05, 1.6] {
            dark.cuboid(
                Vec3::new(c.x, y, c.y),
                Quat::IDENTITY,
                Vec3::new(r.half().x, 0.02, r.half().y),
                1.0,
                Vec2::new(rng.range(0.0, 4.0), 0.0),
                top_col,
            );
            let mut z = r.min.y + 0.15;
            while z < r.max.y - 0.12 {
                let kind = rng.below(4);
                let p = Vec3::new(c.x + rng.signed(0.05), y + 0.02, z);
                match kind {
                    0 | 1 => {
                        let hh = rng.range(0.16, 0.26);
                        let rr = rng.range(0.05, 0.08);
                        glass.lathe(
                            p,
                            &[
                                (0.0, 0.0),
                                (rr, 0.0),
                                (rr, hh * 0.85),
                                (rr * 0.8, hh),
                                (rr * 0.8, hh + 0.02),
                            ],
                            12,
                            1.0,
                            WHITE,
                        );
                        // Contents: seeds, lard, dark liquid.
                        let fill = rng.range(0.3, 0.8);
                        let col = [srgb(0.45, 0.3, 0.12), srgb(0.8, 0.75, 0.6), srgb(0.12, 0.08, 0.05)][rng.below(3)];
                        clay.lathe(
                            p + Vec3::Y * 0.005,
                            &[
                                (0.0, 0.0),
                                (rr * 0.92, 0.0),
                                (rr * 0.92, hh * 0.85 * fill),
                                (0.0, hh * 0.85 * fill),
                            ],
                            10,
                            1.0,
                            col,
                        );
                        tin.lathe(
                            p + Vec3::Y * (hh + 0.02),
                            &[(0.0, 0.0), (rr * 0.85, 0.0), (rr * 0.85, 0.02), (0.0, 0.025)],
                            10,
                            1.0,
                            WHITE,
                        );
                    }
                    2 => {
                        let hh = rng.range(0.24, 0.32);
                        glass.lathe(
                            p,
                            &[
                                (0.0, 0.0),
                                (0.04, 0.0),
                                (0.042, hh * 0.6),
                                (0.015, hh * 0.8),
                                (0.013, hh),
                            ],
                            10,
                            1.0,
                            WHITE,
                        );
                    }
                    _ => {
                        cloth.cuboid(
                            p + Vec3::Y * 0.04,
                            Quat::from_rotation_y(rng.signed(0.3)),
                            Vec3::new(0.12, 0.04, 0.1),
                            1.0,
                            Vec2::ZERO,
                            srgb(0.7, 0.3, 0.22),
                        );
                    }
                }
                z += rng.range(0.16, 0.24);
            }
        }
    }

    // --- Hammock (chinchorro) between the west wall and the inner post.
    {
        let (a, b) = layout.hammock;
        let len = a.distance(b);
        let along = (b - a) / len;
        let side = Vec3::new(-along.z, 0.0, along.x);
        let inset = 0.45;
        let pa = a + along * inset;
        let pb = b - along * inset;
        let width = 1.05;
        let stripes = [
            srgb(0.72, 0.2, 0.14),
            srgb(0.85, 0.72, 0.42),
            srgb(0.2, 0.32, 0.38),
            srgb(0.85, 0.72, 0.42),
        ];
        let mut hm = MeshBuilder::new();
        let origin = pa - Vec3::Y * 0.25 - side * (width * 0.5);
        hm.grid(origin, pb - pa, side * width, 24, 8, Vec2::splat(2.0), &|u, v| {
            let sag = 0.85 * 4.0 * u * (1.0 - u);
            let w = width * (u * (1.0 - u) * 4.0).powf(0.6);
            // Narrow the cloth toward the ropes: replace the flat width by w.
            let across = (v - 0.5) * (w - width);
            let dip = 0.18 * (1.0 - (2.0 * v - 1.0).powi(2)) * (u * (1.0 - u) * 4.0);
            let band = ((u * 18.0) as usize) % stripes.len();
            (side * across - Vec3::Y * (sag + dip), stripes[band])
        });
        for (hook, end) in [(a, pa - Vec3::Y * 0.25), (b, pb - Vec3::Y * 0.25)] {
            for k in 0..5 {
                let off = side * ((k as f32 / 4.0 - 0.5) * 0.12);
                hm.ribbon(&[hook, end + off], &[0.015, 0.015], Vec3::Y, &[srgb(0.8, 0.72, 0.55)]);
            }
        }
        let mat = ctx.palette.hammock.clone();
        ctx.static_mesh("hammock", hm, &mat);
        // Hook on the wall.
        tin.cuboid(a, Quat::IDENTITY, Vec3::new(0.03, 0.03, 0.03), 1.0, Vec2::ZERO, WHITE);
    }

    // --- Tinajas (water jars).
    for f in ctx.layout.furniture.iter().filter(|f| f.name.starts_with("pot_")) {
        let Shape::Circle { center, radius } = f.shape else {
            continue;
        };
        let hh = f.height;
        let r = radius;
        let profile = [
            (0.0, 0.0),
            (r * 0.55, 0.0),
            (r * 0.9, hh * 0.2),
            (r, hh * 0.45),
            (r * 0.85, hh * 0.72),
            (r * 0.45, hh * 0.9),
            (r * 0.42, hh),
            (r * 0.5, hh * 1.03),
            (r * 0.38, hh * 1.02),
        ];
        clay.lathe(
            Vec3::new(center.x, 0.0, center.y),
            &profile,
            16,
            1.5,
            scale_rgb(WHITE, rng.range(0.85, 1.0)),
        );
    }

    // --- Rusty drum.
    if let Some(f) = furniture(layout, "barrel")
        && let Shape::Circle { center, radius } = f.shape
    {
        let base = Vec3::new(center.x, 0.0, center.y);
        let r = radius - 0.02;
        let hh = f.height;
        rust.lathe(
            base,
            &[
                (0.0, 0.0),
                (r, 0.0),
                (r, hh * 0.3),
                (r + 0.015, hh * 0.32),
                (r, hh * 0.34),
                (r, hh * 0.66),
                (r + 0.015, hh * 0.68),
                (r, hh * 0.7),
                (r, hh),
                (r * 0.95, hh),
                (0.0, hh - 0.01),
            ],
            18,
            1.0,
            WHITE,
        );
    }

    // --- Firewood stack on the porch.
    if let Some(f) = furniture(layout, "firewood") {
        let r = rect_of(f);
        let log_col = srgb(0.7, 0.6, 0.5);
        for layer in 0..3 {
            let y = 0.08 + layer as f32 * 0.14;
            let mut z = r.min.y + 0.08;
            while z < r.max.y - 0.05 {
                let x0 = r.min.x + rng.range(0.0, 0.1);
                let x1 = r.max.x - rng.range(0.0, 0.1);
                let rr = rng.range(0.05, 0.075);
                wood.tube(
                    &[
                        Ring {
                            center: Vec3::new(x0, y, z),
                            radius: rr,
                            color: log_col,
                        },
                        Ring {
                            center: Vec3::new(x1, y + rng.signed(0.02), z + rng.signed(0.03)),
                            radius: rr * 0.9,
                            color: log_col,
                        },
                    ],
                    7,
                    1.0,
                    1.0,
                    true,
                    &|_, _| 1.0,
                );
                z += rr * 2.1;
            }
        }
    }

    // --- Bench on the porch with a gourd on it.
    if let Some(f) = furniture(layout, "bench") {
        let r = rect_of(f);
        let c = r.center();
        dark.cuboid(
            Vec3::new(c.x, 0.44, c.y),
            Quat::from_rotation_z(0.015),
            Vec3::new(r.half().x, 0.03, r.half().y),
            1.0,
            Vec2::ZERO,
            top_col,
        );
        for dx in [-1.0_f32, 1.0] {
            dark.cuboid(
                Vec3::new(c.x + dx * (r.half().x - 0.15), 0.21, c.y),
                Quat::IDENTITY,
                Vec3::new(0.04, 0.21, r.half().y - 0.03),
                1.0,
                Vec2::ZERO,
                top_col,
            );
        }
        clay.blob(
            Vec3::new(c.x + 0.4, 0.55, c.y),
            Vec3::new(0.1, 0.08, 0.1),
            6,
            10,
            1.0,
            srgb(0.7, 0.55, 0.3),
            &|_| 1.0,
        );
    }

    // --- Straw hat hanging on a nail by the door (an echo of his).
    {
        let p = Vec3::new(
            1.2,
            1.72,
            layout.house.footprint.max.y - layout.house.wall_thickness * 0.5 - 0.06,
        );
        let tilt = Quat::from_rotation_x(1.25);
        let mut hat = MeshBuilder::new();
        hat.lathe(Vec3::ZERO, &[(0.1, 0.0), (0.2, -0.005), (0.28, -0.02)], 18, 1.0, WHITE);
        hat.lathe(
            Vec3::ZERO,
            &[(0.1, 0.0), (0.1, 0.09), (0.07, 0.11), (0.0, 0.115)],
            14,
            1.0,
            WHITE,
        );
        let mesh = ctx.meshes.add(hat.build());
        ctx.commands.spawn((
            Name::new("straw hat"),
            Mesh3d(mesh),
            MeshMaterial3d(ctx.palette.straw.clone()),
            Transform::from_translation(p).with_rotation(tilt),
        ));
    }

    // --- Calendar on the back wall.
    {
        let z = layout.house.footprint.min.y + layout.house.wall_thickness * 0.5 + 0.035;
        let c = Vec3::new(1.15, 1.62, z);
        let mut cal = MeshBuilder::new();
        let (hx, hy) = (0.16, 0.2);
        cal.quad(
            [
                c + Vec3::new(-hx, -hy, 0.0),
                c + Vec3::new(hx, -hy, 0.0),
                c + Vec3::new(hx, hy, 0.0),
                c + Vec3::new(-hx, hy, 0.0),
            ],
            [
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 0.0),
            ],
            WHITE,
        );
        let mat = ctx.palette.calendar.clone();
        ctx.static_mesh("calendar", cal, &mat);
    }

    // --- The note, slightly curled, beside the satchel.
    {
        let n = layout.district.notes[0].pos;
        let rot = Quat::from_rotation_y(0.35);
        let (hx, hz) = (0.1, 0.13);
        let mut note = MeshBuilder::new();
        note.grid(
            n + rot * Vec3::new(-hx, 0.0, hz),
            rot * Vec3::new(2.0 * hx, 0.0, 0.0),
            rot * Vec3::new(0.0, 0.0, -2.0 * hz),
            4,
            4,
            Vec2::new(1.0 / (2.0 * hx), 1.0 / (2.0 * hz)),
            &|u, v| {
                let curl = 0.012 * ((u - 0.5) * 2.0).powi(2) + 0.008 * v * v;
                (Vec3::Y * (0.004 + curl), WHITE)
            },
        );
        let mat = ctx.palette.paper_note.clone();
        ctx.static_mesh("note", note, &mat);
    }

    // --- Lanterns and their warm lights.
    let lamp = |tin: &mut MeshBuilder, glass_mb: &mut MeshBuilder, base: Vec3| {
        tin.lathe(
            base,
            &[(0.0, 0.0), (0.075, 0.0), (0.08, 0.03), (0.06, 0.06), (0.05, 0.07)],
            14,
            1.0,
            WHITE,
        );
        glass_mb.lathe(
            base + Vec3::Y * 0.07,
            &[(0.035, 0.0), (0.055, 0.05), (0.05, 0.12), (0.028, 0.18)],
            14,
            1.0,
            WHITE,
        );
        tin.lathe(
            base + Vec3::Y * 0.25,
            &[(0.0, 0.0), (0.05, 0.0), (0.03, 0.03), (0.0, 0.05)],
            12,
            1.0,
            WHITE,
        );
        for a in [0.0_f32, std::f32::consts::PI] {
            let o = Vec3::new(a.cos() * 0.065, 0.0, a.sin() * 0.065);
            tin.beam(
                base + o + Vec3::Y * 0.05,
                base + o + Vec3::Y * 0.25,
                0.008,
                0.008,
                0.0,
                1.0,
                Vec2::ZERO,
                WHITE,
            );
        }
        let handle: Vec<Vec3> = (0..=6)
            .map(|i| {
                let a = i as f32 / 6.0 * std::f32::consts::PI;
                base + Vec3::new(a.cos() * 0.07, 0.3 + a.sin() * 0.07, 0.0)
            })
            .collect();
        tin.ribbon(&handle, &[0.006], Vec3::Z, &[WHITE]);
    };
    let mut lamp_glass = MeshBuilder::new();
    lamp(&mut tin, &mut lamp_glass, layout.lantern);
    let porch = layout.porch_lamp - Vec3::Y * 0.32;
    lamp(&mut tin, &mut lamp_glass, porch);
    tin.beam(
        porch + Vec3::Y * 0.37,
        layout.porch_lamp + Vec3::Y * 0.12,
        0.006,
        0.006,
        0.0,
        1.0,
        Vec2::ZERO,
        WHITE,
    );
    let lamp_mat = ctx.palette.lamp_glass.clone();
    ctx.static_mesh("lamp glass", lamp_glass, &lamp_mat);

    ctx.commands.spawn((
        Name::new("table lantern light"),
        PointLight {
            color: Color::srgb(1.0, 0.64, 0.32),
            intensity: LANTERN_LUMENS,
            range: 14.0,
            radius: 0.04,
            shadow_maps_enabled: true,
            ..default()
        },
        Flicker {
            base: LANTERN_LUMENS,
            phase: 0.0,
        },
        Transform::from_translation(layout.lantern + Vec3::Y * 0.17),
    ));
    ctx.commands.spawn((
        Name::new("porch lamp light"),
        PointLight {
            color: Color::srgb(1.0, 0.6, 0.3),
            intensity: PORCH_LUMENS,
            range: 12.0,
            radius: 0.04,
            shadow_maps_enabled: true,
            ..default()
        },
        Flicker {
            base: PORCH_LUMENS,
            phase: 2.1,
        },
        Transform::from_translation(porch + Vec3::Y * 0.16),
    ));

    let mats = [
        ("props wood", wood, ctx.palette.wood.clone()),
        ("props dark wood", dark, ctx.palette.wood_dark.clone()),
        ("props clay", clay, ctx.palette.clay.clone()),
        ("props glass", glass, ctx.palette.glass.clone()),
        ("props tin", tin, ctx.palette.tin.clone()),
        ("props rust", rust, ctx.palette.rust_metal.clone()),
        ("props leather", leather, ctx.palette.leather.clone()),
        ("props cloth", cloth, ctx.palette.burlap.clone()),
    ];
    for (name, mb, mat) in mats {
        ctx.static_mesh(name, mb, &mat);
    }

    // The bone bundles are dynamic (`world::dynamic`); only the shared
    // meshes are built here.
    satchel_asset(ctx)
}

/// Burlap sack with a tied neck and bones jutting out, centred on its middle.
fn satchel_asset(ctx: &mut SpawnCtx) -> SatchelAsset {
    let seed = ctx.seed as u32;
    let mut sack = MeshBuilder::new();
    sack.blob(
        Vec3::ZERO,
        Vec3::new(0.24, 0.15, 0.19),
        10,
        16,
        1.2,
        WHITE,
        &move |d: Vec3| {
            let lump = noise2(d.x * 3.0 + 3.0, d.z * 3.0 + d.y * 2.0, seed ^ 0x5A);
            // Flattened bottom where it rests.
            let flat = if d.y < -0.5 { 0.85 } else { 1.0 };
            (0.82 + 0.35 * lump) * flat
        },
    );
    // Gathered neck and tie.
    sack.tube(
        &[
            Ring {
                center: Vec3::new(0.12, 0.08, 0.0),
                radius: 0.09,
                color: WHITE,
            },
            Ring {
                center: Vec3::new(0.2, 0.15, 0.0),
                radius: 0.045,
                color: WHITE,
            },
            Ring {
                center: Vec3::new(0.25, 0.19, 0.0),
                radius: 0.07,
                color: WHITE,
            },
        ],
        10,
        1.0,
        2.0,
        false,
        &|_, _| 1.0,
    );
    sack.tube(
        &[
            Ring {
                center: Vec3::new(0.18, 0.135, -0.05),
                radius: 0.056,
                color: scale_rgb(WHITE, 0.55),
            },
            Ring {
                center: Vec3::new(0.21, 0.16, 0.05),
                radius: 0.056,
                color: scale_rgb(WHITE, 0.55),
            },
        ],
        8,
        1.0,
        1.0,
        false,
        &|_, _| 1.0,
    );
    // Shoulder strap.
    let strap: Vec<Vec3> = (0..=8)
        .map(|i| {
            let a = i as f32 / 8.0 * std::f32::consts::PI;
            Vec3::new(-0.15 + 0.32 * (1.0 - a.cos()) * 0.5, 0.05 + a.sin() * 0.22, -0.12)
        })
        .collect();
    sack.ribbon(&strap, &[0.035], Vec3::Z, &[mix_rgb(WHITE, srgb(0.5, 0.35, 0.2), 0.6)]);

    let mut bones = MeshBuilder::new();
    let bone = |mb: &mut MeshBuilder, a: Vec3, b: Vec3, r: f32| {
        let d = (b - a).normalize();
        mb.tube(
            &[
                Ring {
                    center: a - d * 0.02,
                    radius: r * 1.7,
                    color: WHITE,
                },
                Ring {
                    center: a + d * 0.03,
                    radius: r * 1.5,
                    color: WHITE,
                },
                Ring {
                    center: a + (b - a) * 0.25,
                    radius: r,
                    color: WHITE,
                },
                Ring {
                    center: a + (b - a) * 0.75,
                    radius: r * 0.9,
                    color: WHITE,
                },
                Ring {
                    center: b - d * 0.03,
                    radius: r * 1.4,
                    color: WHITE,
                },
                Ring {
                    center: b + d * 0.02,
                    radius: r * 1.6,
                    color: WHITE,
                },
            ],
            8,
            1.0,
            1.0,
            true,
            &|i, s| if (i == 0 || i == 5) && s % 4 < 2 { 1.2 } else { 1.0 },
        );
    };
    bone(
        &mut bones,
        Vec3::new(0.2, 0.15, 0.01),
        Vec3::new(0.46, 0.33, 0.03),
        0.017,
    );
    bone(
        &mut bones,
        Vec3::new(0.22, 0.16, -0.03),
        Vec3::new(0.4, 0.37, -0.1),
        0.014,
    );
    bone(
        &mut bones,
        Vec3::new(0.23, 0.17, 0.04),
        Vec3::new(0.36, 0.3, 0.13),
        0.012,
    );
    SatchelAsset {
        sack: ctx.meshes.add(sack.build()),
        bones: ctx.meshes.add(bones.build()),
    }
}
