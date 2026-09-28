//! The abandoned llanero house, built board by board from the shared wall
//! specification: vertical weathered boards around real door and window
//! openings, shutters hanging open, inner girts and studs, a gable roof of
//! rusted corrugated zinc on rafters and purlins (one sheet missing), and a
//! front corredor on forked horcón posts.

use bevy::prelude::*;

use super::SpawnCtx;
use super::mesh::{MeshBuilder, Rgba, Ring, mix_rgb, scale_rgb, srgb};
use crate::geometry::{Facing, OpeningKind, Wall};
use crate::rng::Rng;

const BOARD_T: f32 = 0.026;

/// Yaw that maps local +Z onto a ground-plane direction.
fn yaw_to(n: Vec3) -> Quat {
    Quat::from_rotation_y(n.x.atan2(n.z))
}

fn board_tint(rng: &mut Rng) -> Rgba {
    let base = srgb(1.0, 0.97, 0.93);
    let r = rng.f32();
    if r < 0.1 {
        // A newer replacement board.
        srgb(0.78, 0.62, 0.48)
    } else if r < 0.24 {
        // Remnants of old blue-green paint.
        mix_rgb(base, srgb(0.62, 0.8, 0.82), rng.range(0.3, 0.6))
    } else {
        scale_rgb(base, rng.range(0.72, 1.08))
    }
}

pub fn spawn(ctx: &mut SpawnCtx) {
    let house = &ctx.layout.house;
    let mut rng = ctx.rng(71);
    let mut boards = MeshBuilder::new();
    let mut frame = MeshBuilder::new();
    let mut zinc = MeshBuilder::new();
    let mut posts = MeshBuilder::new();
    let half_t = house.wall_thickness * 0.5;

    for wall in &house.walls {
        build_wall(house, wall, half_t, &mut rng, &mut boards, &mut frame);
    }

    // Corner posts.
    let fp = house.footprint;
    let dark = srgb(0.7, 0.62, 0.55);
    for (x, z) in [
        (fp.min.x, fp.min.y),
        (fp.max.x, fp.min.y),
        (fp.min.x, fp.max.y),
        (fp.max.x, fp.max.y),
    ] {
        frame.cuboid(
            Vec3::new(x, house.eave * 0.5, z),
            Quat::IDENTITY,
            Vec3::new(0.08, house.eave * 0.5 + 0.03, 0.08),
            1.0,
            Vec2::new(rng.range(0.0, 4.0), 0.0),
            dark,
        );
    }

    build_roof(house, &mut rng, &mut frame, &mut zinc);
    build_porch(house, &mut rng, &mut frame, &mut zinc, &mut posts);

    // The inner horcón that also holds the hammock.
    let p = house.inner_post;
    let post_col = srgb(0.62, 0.54, 0.46);
    let top = house.roof_height_at(p.y) - 0.12;
    posts.tube(
        &[
            Ring {
                center: Vec3::new(p.x, -0.05, p.y),
                radius: 0.11,
                color: post_col,
            },
            Ring {
                center: Vec3::new(p.x + 0.03, top * 0.5, p.y),
                radius: 0.095,
                color: post_col,
            },
            Ring {
                center: Vec3::new(p.x, top, p.y),
                radius: 0.085,
                color: post_col,
            },
        ],
        7,
        1.0,
        1.0,
        true,
        &|_, _| 1.0,
    );

    let wood = ctx.palette.wood.clone();
    let wood_dark = ctx.palette.wood_dark.clone();
    let zinc_mat = ctx.palette.zinc.clone();
    ctx.static_mesh("house boards", boards, &wood);
    ctx.static_mesh("house frame", frame, &wood_dark);
    ctx.static_mesh("house zinc", zinc, &zinc_mat);
    ctx.static_mesh("house posts", posts, &wood);
}

fn wall_top(house: &crate::geometry::House, wall: &Wall, s: f32) -> f32 {
    match wall.facing {
        Facing::North | Facing::South => house.eave,
        Facing::East | Facing::West => house.roof_height_at(wall.at(s).y),
    }
}

fn build_wall(
    house: &crate::geometry::House,
    wall: &Wall,
    half_t: f32,
    rng: &mut Rng,
    boards: &mut MeshBuilder,
    frame: &mut MeshBuilder,
) {
    let n2 = wall.facing.normal();
    let n = Vec3::new(n2.x, 0.0, n2.y);
    let d2 = wall.dir();
    let d = Vec3::new(d2.x, 0.0, d2.y);
    let rot = yaw_to(n);
    let len = wall.length();
    let out = half_t - BOARD_T * 0.5;
    let gable = matches!(wall.facing, Facing::East | Facing::West);

    // Split the wall into solid runs and opening runs.
    let mut openings = wall.openings.clone();
    openings.sort_by(|a, b| a.from.total_cmp(&b.from));
    let mut runs: Vec<(f32, f32, Option<crate::geometry::Opening>)> = Vec::new();
    let mut s = -half_t;
    for o in &openings {
        runs.push((s, o.from, None));
        runs.push((o.from, o.to, Some(*o)));
        s = o.to;
    }
    runs.push((s, len + half_t, None));

    let mut place_board = |s0: f32, s1: f32, y0: f32, y1: f32, rng: &mut Rng| {
        if s1 - s0 < 0.02 || y1 - y0 < 0.04 {
            return;
        }
        let mid = (s0 + s1) * 0.5;
        let g = wall.at(mid);
        let jitter = rng.signed(0.004);
        let c = Vec3::new(g.x, (y0 + y1) * 0.5, g.y) + n * (out + jitter);
        let tilt = Quat::from_rotation_z(rng.signed(0.01));
        boards.cuboid(
            c,
            rot * tilt,
            Vec3::new((s1 - s0) * 0.5 - 0.004, (y1 - y0) * 0.5, BOARD_T * 0.5),
            1.0,
            Vec2::new(rng.range(0.0, 8.0), rng.range(0.0, 8.0)),
            board_tint(rng),
        );
    };

    for &(a, b, opening) in &runs {
        let mut s = a;
        while s < b - 0.01 {
            let mut w = rng.range(0.17, 0.25);
            if b - (s + w) < 0.09 {
                w = b - s;
            }
            let s1 = (s + w).min(b);
            let mid = (s + s1) * 0.5;
            let top = if gable {
                wall_top(house, wall, mid) - 0.02
            } else {
                house.eave - rng.range(0.0, 0.05)
            };
            match opening {
                None => place_board(s, s1, 0.02, top, rng),
                Some(o) => {
                    if o.bottom > 0.05 {
                        place_board(s, s1, 0.02, o.bottom, rng);
                    }
                    place_board(s, s1, o.top, top, rng);
                }
            }
            s = s1;
        }
    }

    // Inner girts (skipping openings) and a top plate.
    let inner = -(half_t - 0.035);
    for h in [0.35_f32, 1.25, 2.2] {
        for &(a, b, opening) in &runs {
            if let Some(o) = opening
                && h > o.bottom - 0.05
                && h < o.top + 0.05
            {
                continue;
            }
            let pa = wall.at(a.max(0.0));
            let pb = wall.at(b.min(len));
            frame.beam(
                Vec3::new(pa.x, h, pa.y) + n * inner,
                Vec3::new(pb.x, h, pb.y) + n * inner,
                0.05,
                0.09,
                0.0,
                1.0,
                Vec2::new(rng.range(0.0, 4.0), 0.0),
                srgb(0.8, 0.75, 0.7),
            );
        }
    }
    let pa = wall.at(0.0);
    let pb = wall.at(len);
    frame.beam(
        Vec3::new(pa.x, house.eave - 0.06, pa.y) + n * inner,
        Vec3::new(pb.x, house.eave - 0.06, pb.y) + n * inner,
        0.09,
        0.1,
        0.0,
        1.0,
        Vec2::ZERO,
        srgb(0.7, 0.64, 0.58),
    );

    // Openings: studs, frames, shutters, door leaf.
    let trim = srgb(0.72, 0.66, 0.6);
    for o in &openings {
        for (edge, sign) in [(o.from, -1.0_f32), (o.to, 1.0)] {
            let g = wall.at(edge + sign * 0.04);
            // Stud inside.
            frame.cuboid(
                Vec3::new(g.x, house.eave * 0.5, g.y) + n * inner,
                rot,
                Vec3::new(0.04, house.eave * 0.5, 0.04),
                1.0,
                Vec2::ZERO,
                trim,
            );
            // Jamb trim outside.
            let top = o.top + 0.06;
            let bottom = (o.bottom - 0.06).max(0.0);
            frame.cuboid(
                Vec3::new(g.x, (top + bottom) * 0.5, g.y) + n * (half_t + 0.012),
                rot,
                Vec3::new(0.045, (top - bottom) * 0.5, 0.012),
                1.0,
                Vec2::new(rng.range(0.0, 4.0), 0.0),
                trim,
            );
        }
        let m = wall.at((o.from + o.to) * 0.5);
        let half_w = (o.to - o.from) * 0.5 + 0.09;
        // Head.
        frame.cuboid(
            Vec3::new(m.x, o.top + 0.05, m.y) + n * (half_t + 0.014),
            rot,
            Vec3::new(half_w, 0.05, 0.014),
            1.0,
            Vec2::ZERO,
            trim,
        );
        match o.kind {
            OpeningKind::Window => {
                // Protruding sill.
                frame.cuboid(
                    Vec3::new(m.x, o.bottom - 0.02, m.y) + n * (half_t + 0.04),
                    rot * Quat::from_rotation_z(rng.signed(0.02)),
                    Vec3::new(half_w + 0.03, 0.025, 0.06),
                    1.0,
                    Vec2::ZERO,
                    trim,
                );
                // Two shutter leaves, hanging open at uneven angles.
                let width = o.to - o.from;
                for (edge, into) in [(o.from, 1.0_f32), (o.to, -1.0)] {
                    if rng.f32() < 0.18 {
                        continue; // this leaf is long gone
                    }
                    let hinge2 = wall.at(edge);
                    let hinge = Vec3::new(hinge2.x, 0.0, hinge2.y) + n * (half_t + 0.03);
                    let closed = d * into;
                    let a = rng.range(1.75, 2.75);
                    let open = (closed * a.cos() + n * a.sin()).normalize();
                    let leaf_w = width * 0.5 - 0.02;
                    let sag = rng.signed(0.08);
                    shutter(boards, hinge, open, leaf_w, (o.bottom + 0.02, o.top - 0.02), sag, rng);
                }
            }
            OpeningKind::Door => {
                // Threshold.
                frame.cuboid(
                    Vec3::new(m.x, 0.02, m.y),
                    rot,
                    Vec3::new(half_w - 0.05, 0.02, half_t + 0.02),
                    1.0,
                    Vec2::ZERO,
                    trim,
                );
                if wall.facing == Facing::South {
                    // The front door hangs open inward, flat to the wall side.
                    let hinge2 = wall.at(o.from);
                    let hinge = Vec3::new(hinge2.x, 0.0, hinge2.y) - n * (half_t - 0.02);
                    let a = 1.62_f32;
                    let open = (d * a.cos() - n * a.sin()).normalize();
                    door_leaf(boards, frame, hinge, open, o.to - o.from - 0.05, o.top - 0.03, rng);
                }
            }
        }
    }
}

/// A shutter leaf of vertical boards with two battens.
fn shutter(mb: &mut MeshBuilder, hinge: Vec3, open: Vec3, width: f32, (y0, y1): (f32, f32), sag: f32, rng: &mut Rng) {
    let rot = Quat::from_rotation_y((-open.z).atan2(open.x)) * Quat::from_rotation_z(sag);
    let count = 3;
    let bw = width / count as f32;
    let tint = board_tint(rng);
    for i in 0..count {
        let along = bw * (i as f32 + 0.5);
        let c = hinge + open * along + Vec3::Y * ((y0 + y1) * 0.5 + sag * along);
        mb.cuboid(
            c,
            rot,
            Vec3::new(bw * 0.5 - 0.004, (y1 - y0) * 0.5, 0.012),
            1.0,
            Vec2::new(rng.range(0.0, 8.0), rng.range(0.0, 8.0)),
            scale_rgb(tint, rng.range(0.9, 1.05)),
        );
    }
    for y in [y0 + 0.15, y1 - 0.15] {
        let c = hinge + open * (width * 0.5) + Vec3::Y * (y + sag * width * 0.5);
        let side = rot * Vec3::Z;
        mb.cuboid(
            c + side * 0.02,
            rot,
            Vec3::new(width * 0.5, 0.04, 0.01),
            1.0,
            Vec2::new(rng.range(0.0, 8.0), 0.0),
            scale_rgb(tint, 0.85),
        );
    }
}

fn door_leaf(
    boards: &mut MeshBuilder,
    frame: &mut MeshBuilder,
    hinge: Vec3,
    open: Vec3,
    width: f32,
    height: f32,
    rng: &mut Rng,
) {
    let rot = Quat::from_rotation_y((-open.z).atan2(open.x));
    let count = 5;
    let bw = width / count as f32;
    let tint = board_tint(rng);
    for i in 0..count {
        let along = bw * (i as f32 + 0.5);
        let c = hinge + open * along + Vec3::Y * (0.04 + height * 0.5);
        boards.cuboid(
            c,
            rot,
            Vec3::new(bw * 0.5 - 0.004, height * 0.5, 0.014),
            1.0,
            Vec2::new(rng.range(0.0, 8.0), rng.range(0.0, 8.0)),
            scale_rgb(tint, rng.range(0.9, 1.05)),
        );
    }
    // Z brace on the inner side.
    let side = rot * Vec3::Z;
    let c0 = hinge + open * 0.08 + side * 0.03;
    let c1 = hinge + open * (width - 0.08) + side * 0.03;
    let dark = srgb(0.7, 0.62, 0.55);
    frame.beam(
        c0 + Vec3::Y * 0.4,
        c1 + Vec3::Y * 0.4,
        0.1,
        0.03,
        0.0,
        1.0,
        Vec2::ZERO,
        dark,
    );
    frame.beam(
        c0 + Vec3::Y * (height - 0.3),
        c1 + Vec3::Y * (height - 0.3),
        0.1,
        0.03,
        0.0,
        1.0,
        Vec2::ZERO,
        dark,
    );
    frame.beam(
        c1 + Vec3::Y * 0.4,
        c0 + Vec3::Y * (height - 0.3),
        0.09,
        0.03,
        0.0,
        1.0,
        Vec2::ZERO,
        dark,
    );
}

/// One corrugated zinc sheet on a plane.
fn zinc_sheet(zinc: &mut MeshBuilder, origin: Vec3, across: Vec3, down: Vec3, lift: f32, tint: Rgba) {
    let mut u = across;
    let mut o = origin;
    // Keep the visible side up.
    if u.cross(down).y < 0.0 {
        o += u;
        u = -u;
    }
    let normal = u.cross(down).normalize();
    let waves = (across.length() / 0.076).round().max(1.0);
    zinc.grid(o, u, down, (waves as usize) * 4, 3, Vec2::new(1.25, 0.6), &|s, t| {
        let corr = (s * waves * std::f32::consts::TAU).sin() * 0.014;
        let bend = lift * t * t;
        (normal * (corr + bend), tint)
    });
}

fn build_roof(house: &crate::geometry::House, rng: &mut Rng, frame: &mut MeshBuilder, zinc: &mut MeshBuilder) {
    let fp = house.footprint;
    let zc = fp.center().y;
    let half_d = fp.half().y;
    let ov = house.overhang;
    let x0 = fp.min.x - ov;
    let x1 = fp.max.x + ov;
    let slope = (house.ridge - house.eave) / half_d;
    let rafter_col = srgb(0.72, 0.66, 0.6);
    // Ridge beam.
    frame.beam(
        Vec3::new(x0, house.ridge - 0.06, zc),
        Vec3::new(x1, house.ridge - 0.06, zc),
        0.1,
        0.14,
        0.0,
        1.0,
        Vec2::ZERO,
        rafter_col,
    );
    for side in [-1.0_f32, 1.0] {
        let z_eave = zc + side * (half_d + ov);
        let y_eave = house.eave - slope * ov;
        let top = Vec3::new(0.0, house.ridge, zc);
        let bottom = Vec3::new(0.0, y_eave, z_eave);
        let down = bottom - top;
        let roof_n = {
            let n = Vec3::X.cross(down).normalize();
            if n.y < 0.0 { -n } else { n }
        };
        // Rafters.
        let mut x = x0 + 0.1;
        while x <= x1 - 0.05 {
            frame.beam(
                Vec3::new(x, house.ridge - 0.1, zc),
                Vec3::new(x, y_eave - 0.06, z_eave),
                0.06,
                0.11,
                0.0,
                1.0,
                Vec2::new(rng.range(0.0, 4.0), 0.0),
                scale_rgb(rafter_col, rng.range(0.85, 1.05)),
            );
            x += 0.95;
        }
        // Purlins.
        for t in [0.12_f32, 0.45, 0.78] {
            let p = top + down * t + roof_n * 0.05;
            frame.beam(
                Vec3::new(x0, p.y, p.z),
                Vec3::new(x1, p.y, p.z),
                0.05,
                0.07,
                0.0,
                1.0,
                Vec2::new(rng.range(0.0, 4.0), 0.0),
                rafter_col,
            );
        }
        // Fascia along the eave.
        frame.beam(
            Vec3::new(x0, y_eave - 0.02, z_eave),
            Vec3::new(x1, y_eave - 0.02, z_eave),
            0.03,
            0.16,
            0.0,
            1.0,
            Vec2::ZERO,
            rafter_col,
        );
        // Sheets.
        let width: f32 = 0.82;
        let step = 0.76;
        let mut i = 0;
        let mut x = x0;
        while x < x1 - 0.1 {
            let missing = side < 0.0 && i == 8;
            let slid = side > 0.0 && i == 3;
            if !missing {
                let slide = if slid { 0.35 } else { 0.0 };
                let origin = Vec3::new(x, 0.0, 0.0) + top + roof_n * 0.1 + down.normalize() * slide;
                let tint = scale_rgb([1.0; 4], rng.range(0.78, 1.1));
                let lift = if rng.f32() < 0.25 { rng.range(0.03, 0.09) } else { 0.0 };
                zinc_sheet(zinc, origin, Vec3::X * width.min(x1 - x), down, lift, tint);
            }
            x += step;
            i += 1;
        }
        // Barge boards along the gable edges.
        for gx in [x0 - 0.02, x1 + 0.02] {
            frame.beam(
                Vec3::new(gx, house.ridge + 0.1, zc),
                Vec3::new(gx, y_eave + 0.1, z_eave),
                0.03,
                0.18,
                0.0,
                1.0,
                Vec2::ZERO,
                rafter_col,
            );
        }
    }
    // Ridge cap.
    zinc.cuboid(
        Vec3::new((x0 + x1) * 0.5, house.ridge + 0.13, zc),
        Quat::IDENTITY,
        Vec3::new((x1 - x0) * 0.5, 0.012, 0.17),
        1.0,
        Vec2::ZERO,
        scale_rgb([1.0; 4], 0.85),
    );
}

fn build_porch(
    house: &crate::geometry::House,
    rng: &mut Rng,
    frame: &mut MeshBuilder,
    zinc: &mut MeshBuilder,
    posts: &mut MeshBuilder,
) {
    let porch = house.porch;
    let fp = house.footprint;
    let high = house.eave - 0.08;
    let low = house.porch_low;
    let beam_z = house.porch_posts.first().map(|p| p.y).unwrap_or(porch.max.y - 0.2);
    let slope_at = |z: f32| high + (low - high) * ((z - fp.max.y) / (porch.max.y + 0.15 - fp.max.y));
    let beam_y = slope_at(beam_z) - 0.12;
    let post_col = srgb(0.6, 0.53, 0.46);
    // Forked horcón posts.
    for &p in &house.porch_posts {
        let lean = Vec3::new(rng.signed(0.04), 0.0, rng.signed(0.03));
        let base = Vec3::new(p.x, -0.05, p.y);
        let fork = Vec3::new(p.x, beam_y - 0.18, p.y) + lean;
        posts.tube(
            &[
                Ring {
                    center: base,
                    radius: 0.12,
                    color: post_col,
                },
                Ring {
                    center: base.lerp(fork, 0.5) + Vec3::new(rng.signed(0.04), 0.0, 0.0),
                    radius: 0.105,
                    color: post_col,
                },
                Ring {
                    center: fork,
                    radius: 0.1,
                    color: scale_rgb(post_col, 0.9),
                },
            ],
            7,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
        for dx in [-0.09_f32, 0.09] {
            posts.tube(
                &[
                    Ring {
                        center: fork,
                        radius: 0.07,
                        color: post_col,
                    },
                    Ring {
                        center: fork + Vec3::new(dx, 0.3, 0.0),
                        radius: 0.05,
                        color: post_col,
                    },
                ],
                6,
                1.0,
                1.0,
                true,
                &|_, _| 1.0,
            );
        }
    }
    let x0 = porch.min.x;
    let x1 = porch.max.x;
    let beam_col = srgb(0.7, 0.62, 0.55);
    frame.beam(
        Vec3::new(x0, beam_y, beam_z),
        Vec3::new(x1, beam_y, beam_z),
        0.12,
        0.16,
        0.02,
        1.0,
        Vec2::ZERO,
        beam_col,
    );
    let z_wall = fp.max.y + house.wall_thickness * 0.5;
    let z_edge = porch.max.y + 0.15;
    let mut x = x0 + 0.1;
    while x <= x1 {
        frame.beam(
            Vec3::new(x, slope_at(z_wall) - 0.08, z_wall),
            Vec3::new(x, slope_at(z_edge) - 0.08, z_edge),
            0.05,
            0.09,
            0.0,
            1.0,
            Vec2::new(rng.range(0.0, 3.0), 0.0),
            scale_rgb(beam_col, rng.range(0.85, 1.05)),
        );
        x += 1.05;
    }
    let top = Vec3::new(0.0, slope_at(z_wall) + 0.02, z_wall);
    let bottom = Vec3::new(0.0, slope_at(z_edge) + 0.02, z_edge);
    let down = bottom - top;
    let mut x = x0;
    let mut i = 0;
    while x < x1 - 0.1 {
        let tint = scale_rgb([1.0; 4], rng.range(0.75, 1.05));
        let lift = if i == 5 { 0.12 } else { 0.0 };
        zinc_sheet(
            zinc,
            Vec3::new(x, 0.0, 0.0) + top,
            Vec3::X * 0.82_f32.min(x1 - x),
            down,
            lift,
            tint,
        );
        x += 0.76;
        i += 1;
    }
}
