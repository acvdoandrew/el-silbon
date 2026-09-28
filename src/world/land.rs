//! The llano: ground, dirt road, imperfect fences, the ranch sign, grass,
//! termite mounds, far moriche palms and tree islands, sky, moon and stars,
//! and the moonlight.

use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::mesh::{MeshBuilder, Rgba, Ring, mix_rgb, scale_rgb, srgb};
use super::{SpawnCtx, fbm2, noise2};
use crate::geometry::{Layout, Shape, segment_point_distance};

/// Direction toward the moon (world space, normalized at use): high in the
/// south-south-west, behind the road, so the house front, the ceiba's trunk
/// and crown and anyone crossing the yard are moonlit from the player's side
/// against the dark northern sky.
pub const MOON_DIR: Vec3 = Vec3::new(-0.38, 0.62, 0.68);

/// Linear colour of the sky at the horizon; the fog uses the same value.
pub const HORIZON: [f32; 3] = [0.0068, 0.0092, 0.0152];
const ZENITH: [f32; 3] = [0.0011, 0.0017, 0.0042];

pub fn spawn(ctx: &mut SpawnCtx) {
    ground(ctx);
    road(ctx);
    fences(ctx);
    sign(ctx);
    grass(ctx);
    mounds(ctx);
    horizon(ctx);
    sky(ctx);
    moonlight(ctx);
}

/// Height of the land. Flat wherever anyone can walk (and along the road),
/// gently rolling beyond the fences so the horizon is not a ruler line.
pub fn ground_height(layout: &Layout, x: f32, z: f32, seed: u32) -> f32 {
    let b = layout.bounds;
    let dx = (b.min.x - 4.0 - x).max(x - b.max.x - 4.0).max(0.0);
    let dz = (b.min.y - 4.0 - z).max(0.0);
    let south = (z - 40.0).max(0.0);
    let outside = (dx * dx + dz * dz).sqrt().max(south);
    if outside <= 0.0 || (z > 24.0 && z < 38.0) {
        return 0.0;
    }
    let ramp = (outside / 30.0).clamp(0.0, 1.0);
    let ramp = ramp * ramp * (3.0 - 2.0 * ramp);
    ramp * (fbm2(x * 0.03, z * 0.03, seed) * 3.2 - 0.6)
}

fn trail_distance(layout: &Layout, p: Vec2) -> f32 {
    let mut best = f32::MAX;
    for trail in &layout.trails {
        for w in trail.windows(2) {
            best = best.min(segment_point_distance(w[0], w[1], p));
        }
    }
    best
}

fn ground(ctx: &mut SpawnCtx) {
    let layout = ctx.layout;
    let seed = ctx.seed as u32;
    let mut mb = MeshBuilder::new();
    let size = 250.0;
    let origin = Vec3::new(-size * 0.5, 0.0, size * 0.5 - 10.0);
    let fp = layout.house.footprint;
    let porch = layout.house.porch;
    let tree = layout.ceiba.center;
    let grass_tint = srgb(1.0, 1.0, 1.0);
    let mud = srgb(0.55, 0.45, 0.36);
    let packed = srgb(0.48, 0.4, 0.33);
    let litter = srgb(0.5, 0.44, 0.34);
    mb.grid(
        origin,
        Vec3::X * size,
        Vec3::NEG_Z * size,
        250,
        250,
        Vec2::splat(0.25),
        &|u, v| {
            let x = origin.x + u * size;
            let z = origin.z - v * size;
            let y = ground_height(layout, x, z, seed);
            let p = Vec2::new(x, z);
            let n = noise2(x * 0.08, z * 0.08, seed ^ 0xA0);
            let mut c = scale_rgb(grass_tint, 0.82 + n * 0.3);
            // Worn mud trails.
            let t = trail_distance(layout, p);
            let trail = (1.0 - (t - 0.35) / 0.9).clamp(0.0, 1.0);
            c = mix_rgb(c, mud, trail * 0.85);
            // Packed earth inside and around the house.
            let near_house = {
                let d = (p - p.clamp(fp.min.min(porch.min), fp.max.max(porch.max))).length();
                (1.0 - d / 2.5).clamp(0.0, 1.0)
            };
            c = mix_rgb(c, packed, near_house * 0.9);
            // Leaf litter under the ceiba.
            let dt = p.distance(tree);
            c = mix_rgb(c, litter, (1.0 - (dt - 3.0) / 9.0).clamp(0.0, 1.0) * 0.7);
            // Road shoulders.
            let shoulder = (1.0 - ((z - 27.5).abs().min((z - 35.0).abs()) / 1.4)).clamp(0.0, 1.0);
            c = mix_rgb(c, mud, shoulder * 0.6);
            (Vec3::new(0.0, y, 0.0), c)
        },
    );
    ctx.static_mesh("ground", mb, &ctx.palette.ground.clone());
}

fn road(ctx: &mut SpawnCtx) {
    let r = ctx.layout.road;
    let mut mb = MeshBuilder::new();
    let width = r.max.y - r.min.y;
    let center = (r.min.y + r.max.y) * 0.5;
    let seed = ctx.seed as u32;
    let origin = Vec3::new(r.min.x, 0.0, r.max.y);
    let len = r.max.x - r.min.x;
    let base = srgb(1.0, 1.0, 1.0);
    let rut = srgb(0.62, 0.55, 0.5);
    let median = srgb(0.62, 0.72, 0.42);
    mb.grid(
        origin,
        Vec3::X * len,
        Vec3::NEG_Z * width,
        320,
        14,
        Vec2::splat(0.35),
        &|u, v| {
            let x = r.min.x + u * len;
            let z = r.max.y - v * width;
            let off = z - center;
            let crown = 0.05 * (1.0 - (2.0 * off / width).powi(2));
            let mut y = 0.015 + crown;
            let mut c = scale_rgb(base, 0.85 + noise2(x * 0.3, z * 0.3, seed ^ 0xB0) * 0.3);
            for rz in [-1.4_f32, 1.4] {
                let d = (off - rz) / 0.45;
                let k = (-d * d).exp();
                y -= 0.035 * k;
                c = mix_rgb(c, rut, k * 0.8);
            }
            let m = (-(off / 0.35).powi(2)).exp() * (0.5 + 0.5 * noise2(x * 0.5, 3.0, seed ^ 0xB1));
            c = mix_rgb(c, median, m * 0.55);
            (Vec3::new(0.0, y, 0.0), c)
        },
    );
    ctx.static_mesh("road", mb, &ctx.palette.road.clone());
}

fn fences(ctx: &mut SpawnCtx) {
    let layout = ctx.layout;
    let mut rng = ctx.rng(21);
    let mut posts = MeshBuilder::new();
    let mut wires = MeshBuilder::new();
    let wood = srgb(0.85, 0.8, 0.74);
    let heights = [0.42_f32, 0.8, 1.16];
    for run in &layout.fences {
        let dir = (run.b - run.a).normalize();
        let mut stations: Vec<f32> = Vec::new();
        for (from, to) in run.spans() {
            let mut s = from;
            stations.push(s);
            while s + 3.3 < to {
                s += rng.range(2.3, 2.9);
                stations.push(s);
            }
            stations.push(to);
        }
        stations.sort_by(|a, b| a.total_cmp(b));
        stations.dedup_by(|a, b| (*a - *b).abs() < 0.3);
        for &s in &stations {
            let g = run.at(s);
            let gate_post = run
                .gaps
                .iter()
                .any(|&(f, t)| (s - f).abs() < 0.05 || (s - t).abs() < 0.05);
            let h = if gate_post {
                layout.fence_height + 0.35
            } else {
                layout.fence_height + rng.range(-0.12, 0.18)
            };
            let r = if gate_post { 0.085 } else { rng.range(0.045, 0.065) };
            // Imperfect: every post leans a little, some a lot.
            let lean_amount = if rng.f32() < 0.12 { 0.16 } else { 0.05 };
            let lean = Vec3::new(rng.signed(lean_amount), 0.0, rng.signed(lean_amount));
            let base = Vec3::new(g.x, -0.05, g.y);
            let top = base + Vec3::Y * h + lean * h;
            let mid = base.lerp(top, 0.5) + Vec3::new(rng.signed(0.03), 0.0, rng.signed(0.03));
            let tint = scale_rgb(wood, rng.range(0.7, 1.1));
            let rings = [
                Ring {
                    center: base,
                    radius: r * 1.1,
                    color: tint,
                },
                Ring {
                    center: mid,
                    radius: r,
                    color: tint,
                },
                Ring {
                    center: top,
                    radius: r * 0.85,
                    color: scale_rgb(tint, 0.8),
                },
            ];
            let lumps = rng.next_u64();
            posts.tube(&rings, 6, 1.0, 1.2, true, &|i, sdx| {
                0.85 + 0.3 * ((lumps >> ((i * 6 + sdx) % 60)) & 1) as f32
            });
        }
        // Barbed wire between consecutive posts, sagging, skipping gaps.
        for w in stations.windows(2) {
            let (s0, s1) = (w[0], w[1]);
            if run.in_gap((s0 + s1) * 0.5) {
                continue;
            }
            let p0 = run.at(s0);
            let p1 = run.at(s1);
            for (k, &hgt) in heights.iter().enumerate() {
                let sag = rng.range(0.025, 0.08) + k as f32 * 0.01;
                let segs = 4;
                let mut prev = Vec3::new(p0.x, hgt, p0.y);
                for i in 1..=segs {
                    let t = i as f32 / segs as f32;
                    let g = p0.lerp(p1, t);
                    let y = hgt - sag * 4.0 * t * (1.0 - t);
                    let cur = Vec3::new(g.x, y, g.y);
                    wires.beam(prev, cur, 0.011, 0.011, 0.0, 1.0, Vec2::ZERO, srgb(1.0, 1.0, 1.0));
                    prev = cur;
                }
                // Barbs near the gate, where you look closely.
                let near_gate = run.gaps.iter().any(|&(f, t)| s0 < t + 8.0 && s1 > f - 8.0);
                if near_gate {
                    let mut t = 0.12;
                    while t < 0.95 {
                        let g = p0.lerp(p1, t);
                        let y = hgt - sag * 4.0 * t * (1.0 - t);
                        let c = Vec3::new(g.x, y, g.y);
                        let a = Vec3::new(dir.y, 0.7, -dir.x).normalize() * 0.03;
                        let b = Vec3::new(-dir.y, 0.7, dir.x).normalize() * 0.03;
                        wires.beam(c - a, c + a, 0.006, 0.006, 0.0, 1.0, Vec2::ZERO, srgb(1.0, 1.0, 1.0));
                        wires.beam(c - b, c + b, 0.006, 0.006, 0.0, 1.0, Vec2::ZERO, srgb(1.0, 1.0, 1.0));
                        t += 0.22;
                    }
                }
            }
        }
    }
    // The gate itself: swung wide open, resting against the fence outside.
    let gate = &layout.fences[0];
    if let Some(&(_, to)) = gate.gaps.first() {
        let hinge = gate.at(to);
        let mut g = MeshBuilder::new();
        let wood_gate = srgb(0.75, 0.7, 0.62);
        let z = hinge.y + 0.24;
        let x0 = hinge.x + 0.1;
        let x1 = x0 + 2.9;
        for (i, y) in [0.25_f32, 0.62, 1.0].iter().enumerate() {
            let droop = i as f32 * 0.02;
            g.beam(
                Vec3::new(x0, *y, z),
                Vec3::new(x1, *y - 0.08 - droop, z),
                0.05,
                0.11,
                0.0,
                1.0,
                Vec2::new(i as f32 * 0.3, 0.0),
                wood_gate,
            );
        }
        for x in [x0 + 0.05, x0 + 1.45, x1 - 0.05] {
            g.beam(
                Vec3::new(x, 0.12, z + 0.03),
                Vec3::new(x, 1.1, z + 0.03),
                0.09,
                0.05,
                0.0,
                1.0,
                Vec2::ZERO,
                wood_gate,
            );
        }
        g.beam(
            Vec3::new(x0 + 0.1, 0.25, z - 0.03),
            Vec3::new(x1 - 0.1, 0.95, z - 0.03),
            0.08,
            0.05,
            0.0,
            1.0,
            Vec2::new(0.5, 0.0),
            scale_rgb(wood_gate, 0.85),
        );
        ctx.static_mesh("gate", g, &ctx.palette.wood.clone());
    }
    ctx.static_mesh("fence posts", posts, &ctx.palette.wood.clone());
    ctx.static_mesh("fence wire", wires, &ctx.palette.wire.clone());
}

fn sign(ctx: &mut SpawnCtx) {
    let p = ctx.layout.sign;
    let mut wood = MeshBuilder::new();
    let tint = srgb(0.8, 0.76, 0.7);
    for dx in [-1.0_f32, 1.0] {
        let base = Vec3::new(p.x + dx, -0.05, p.y);
        wood.tube(
            &[
                Ring {
                    center: base,
                    radius: 0.07,
                    color: tint,
                },
                Ring {
                    center: base + Vec3::new(0.0, 1.95, 0.0),
                    radius: 0.06,
                    color: tint,
                },
            ],
            6,
            1.0,
            1.0,
            true,
            &|_, _| 1.0,
        );
    }
    let board_c = Vec3::new(p.x, 1.55, p.y + 0.06);
    let rot = Quat::from_rotation_z(0.035);
    wood.cuboid(board_c, rot, Vec3::new(1.25, 0.3, 0.025), 1.0, Vec2::ZERO, tint);
    ctx.static_mesh("sign posts", wood, &ctx.palette.wood.clone());

    // Painted face (0..1 UVs onto the lettering texture), facing the road.
    let mut face = MeshBuilder::new();
    let hx = rot * Vec3::X * 1.22;
    let hy = rot * Vec3::Y * 0.28;
    let c = board_c + Vec3::Z * 0.027;
    face.quad(
        [c - hx - hy, c + hx - hy, c + hx + hy, c - hx + hy],
        [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ],
        [1.0; 4],
    );
    ctx.static_mesh("sign face", face, &ctx.palette.sign.clone());

    // A kilometre marker on the far shoulder: concrete with a painted band.
    let mut km = MeshBuilder::new();
    let k = Vec3::new(-7.5, 0.35, 36.0);
    km.cuboid(
        k,
        Quat::from_rotation_y(0.1),
        Vec3::new(0.14, 0.4, 0.09),
        1.0,
        Vec2::ZERO,
        srgb(0.72, 0.72, 0.68),
    );
    km.cuboid(
        k + Vec3::Y * 0.22,
        Quat::from_rotation_y(0.1),
        Vec3::new(0.145, 0.08, 0.095),
        1.0,
        Vec2::ZERO,
        srgb(0.5, 0.12, 0.08),
    );
    ctx.static_mesh("km marker", km, &ctx.palette.stone.clone());
}

/// One grass tuft variant: `blades` bent ribbons, dark at the root.
fn tuft(rng: &mut crate::rng::Rng, blades: usize, height: (f32, f32), spread: f32, dry: f32) -> MeshBuilder {
    let mut mb = MeshBuilder::new();
    let root = srgb(0.11, 0.12, 0.06);
    let green = srgb(0.3, 0.38, 0.16);
    let straw = srgb(0.62, 0.55, 0.32);
    for _ in 0..blades {
        let h = rng.range(height.0, height.1);
        let a = rng.range(0.0, std::f32::consts::TAU);
        let out = Vec3::new(a.cos(), 0.0, a.sin());
        let base = out * rng.range(0.0, spread);
        let lean = rng.range(0.15, 0.55);
        let tip_col = mix_rgb(green, straw, (dry + rng.signed(0.3)).clamp(0.0, 1.0));
        let pts: Vec<Vec3> = (0..4)
            .map(|i| {
                let t = i as f32 / 3.0;
                base + out * (lean * h * t * t) + Vec3::Y * (h * t * (1.0 - 0.15 * t * t))
            })
            .collect();
        let side = Vec3::new(-out.z, 0.0, out.x);
        let w = rng.range(0.012, 0.022);
        let cols: [Rgba; 4] = [root, mix_rgb(root, tip_col, 0.5), tip_col, scale_rgb(tip_col, 1.1)];
        mb.ribbon(&pts, &[w, w * 0.9, w * 0.6, 0.002], side, &cols);
    }
    mb
}

fn grass(ctx: &mut SpawnCtx) {
    let layout = ctx.layout;
    let mut rng = ctx.rng(31);
    let variants: [Handle<Mesh>; 5] = [
        ctx.meshes.add(tuft(&mut rng, 16, (0.35, 0.6), 0.12, 0.6).build()),
        ctx.meshes.add(tuft(&mut rng, 22, (0.5, 0.9), 0.16, 0.75).build()),
        ctx.meshes.add(tuft(&mut rng, 12, (0.25, 0.45), 0.1, 0.35).build()),
        ctx.meshes.add(tuft(&mut rng, 26, (0.8, 1.35), 0.2, 0.85).build()),
        ctx.meshes.add(tuft(&mut rng, 18, (0.6, 1.0), 0.14, 0.95).build()),
    ];
    let fp = layout.house.footprint;
    let porch = layout.house.porch;
    let tree = layout.ceiba.center;
    let seed = ctx.seed as u32;
    let mut placed = 0;
    let mut attempts = 0;
    while placed < 3400 && attempts < 40000 {
        attempts += 1;
        let x = rng.range(-70.0, 70.0);
        let z = rng.range(-85.0, 48.0);
        let p = Vec2::new(x, z);
        let pad = 0.9;
        let in_house = p.x > fp.min.x.min(porch.min.x) - pad
            && p.x < fp.max.x.max(porch.max.x) + pad
            && p.y > fp.min.y - pad
            && p.y < porch.max.y + pad;
        if in_house || (z > 27.6 && z < 34.9) || p.distance(tree) < 3.9 {
            continue;
        }
        let trail = trail_distance(layout, p);
        if trail < 1.0 {
            continue;
        }
        // Trampled yard, lush field edges and fence lines.
        let yard = p.x.abs() < 14.0 && p.y > 2.0 && p.y < 21.0;
        let density = fbm2(x * 0.07, z * 0.07, seed ^ 0xC0);
        let keep = if yard { 0.15 } else { 0.35 + density * 0.9 };
        if rng.f32() > keep {
            continue;
        }
        let near_fence = layout.fences.iter().any(|f| segment_point_distance(f.a, f.b, p) < 2.0);
        let v = if near_fence || (density > 0.62 && !yard) {
            3
        } else {
            rng.below(variants.len())
        };
        let y = ground_height(layout, x, z, seed);
        let scale = rng.range(0.75, 1.35) * if yard { 0.7 } else { 1.0 };
        ctx.commands.spawn((
            Mesh3d(variants[v].clone()),
            MeshMaterial3d(ctx.palette.grass.clone()),
            Transform::from_xyz(x, y, z)
                .with_rotation(Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU)))
                .with_scale(Vec3::new(scale, scale * rng.range(0.85, 1.2), scale)),
            NotShadowCaster,
        ));
        placed += 1;
    }
}

/// Termite mounds (the layout gives them collision).
fn mounds(ctx: &mut SpawnCtx) {
    let mut mb = MeshBuilder::new();
    let mut rng = ctx.rng(41);
    for f in ctx.layout.furniture.iter().filter(|f| f.name == "mound") {
        let Shape::Circle { center, radius } = f.shape else {
            continue;
        };
        let h = f.height;
        let tint = scale_rgb(srgb(1.0, 0.95, 0.9), rng.range(0.8, 1.05));
        let profile: Vec<(f32, f32)> = (0..=8)
            .map(|i| {
                let t = i as f32 / 8.0;
                let r = radius * 1.15 * (1.0 - t).powf(0.7) + 0.03;
                (r, -0.05 + h * t)
            })
            .collect();
        mb.lathe(Vec3::new(center.x, 0.0, center.y), &profile, 12, 1.0, tint);
        // Secondary spire.
        let off = Vec3::new(rng.signed(radius * 0.4), 0.0, rng.signed(radius * 0.4));
        let profile2: Vec<(f32, f32)> = (0..=5)
            .map(|i| {
                let t = i as f32 / 5.0;
                (radius * 0.45 * (1.0 - t) + 0.02, h * 0.3 + h * 0.55 * t)
            })
            .collect();
        mb.lathe(Vec3::new(center.x, 0.0, center.y) + off, &profile2, 8, 1.0, tint);
    }
    ctx.static_mesh("termite mounds", mb, &ctx.palette.soil.clone());
}

/// Moriche palms and tree islands (matas) on the horizon.
fn horizon(ctx: &mut SpawnCtx) {
    let mut rng = ctx.rng(51);
    let seed = ctx.seed as u32;
    let layout = ctx.layout;
    let mut trunks = MeshBuilder::new();
    let mut fronds = MeshBuilder::new();
    let mut crowns = MeshBuilder::new();
    let palms = [
        (-30.0, -86.0),
        (-14.0, -95.0),
        (6.0, -82.0),
        (24.0, -90.0),
        (47.0, -74.0),
        (58.0, -58.0),
        (-56.0, -64.0),
        (-72.0, -22.0),
        (-66.0, 12.0),
        (70.0, -26.0),
        (64.0, 8.0),
        (-44.0, 62.0),
        (18.0, 58.0),
        (58.0, 52.0),
        (-8.0, -104.0),
        (-24.0, -99.0),
    ];
    let trunk_col = srgb(0.62, 0.58, 0.52);
    let frond_col = srgb(0.55, 0.62, 0.45);
    for &(x, z) in &palms {
        let h = rng.range(13.0, 19.0);
        let y0 = ground_height(layout, x, z, seed) - 0.2;
        let bend = Vec3::new(rng.signed(1.2), 0.0, rng.signed(1.2));
        let rings: Vec<Ring> = (0..=6)
            .map(|i| {
                let t = i as f32 / 6.0;
                Ring {
                    center: Vec3::new(x, y0 + h * t, z) + bend * t * t,
                    radius: 0.26 - 0.06 * t,
                    color: scale_rgb(trunk_col, 0.8 + 0.2 * t),
                }
            })
            .collect();
        trunks.tube(&rings, 7, 1.0, 0.4, false, &|_, _| 1.0);
        let top = Vec3::new(x, y0 + h, z) + bend;
        // Costapalmate fans: long petiole, then a pleated fan of segments.
        let leaves = rng.range(11.0, 16.0) as usize;
        for l in 0..leaves {
            let a = l as f32 / leaves as f32 * std::f32::consts::TAU + rng.signed(0.2);
            let up = rng.range(-0.35, 0.55);
            let dir = Vec3::new(a.cos(), up, a.sin()).normalize();
            let petiole_end = top + dir * rng.range(1.6, 2.4);
            fronds.ribbon(
                &[top, petiole_end],
                &[0.06, 0.04],
                Vec3::new(-dir.z, 0.0, dir.x).normalize_or(Vec3::X),
                &[frond_col],
            );
            let side = Vec3::new(-dir.z, 0.0, dir.x).normalize_or(Vec3::X);
            for s in 0..9 {
                let fa = (s as f32 / 8.0 - 0.5) * 1.9;
                let seg_dir = (dir * fa.cos() + side * fa.sin() + Vec3::Y * 0.1).normalize();
                let len = rng.range(1.4, 2.0);
                let droop = Vec3::NEG_Y * len * 0.45;
                let p1 = petiole_end + seg_dir * len * 0.5 + droop * 0.2;
                let p2 = petiole_end + seg_dir * len + droop;
                let flat = seg_dir.cross(Vec3::Y).normalize_or(side);
                fronds.ribbon(
                    &[petiole_end, p1, p2],
                    &[0.05, 0.22, 0.02],
                    flat,
                    &[scale_rgb(frond_col, 0.8), frond_col, scale_rgb(frond_col, 1.1)],
                );
            }
        }
        crowns.blob(
            top,
            Vec3::new(0.45, 0.6, 0.45),
            5,
            8,
            1.0,
            srgb(0.4, 0.35, 0.28),
            &|_| 1.0,
        );
    }
    // Tree islands (matas): dark masses of rounded crowns.
    let islands = [
        (-48.0, -92.0, 6),
        (36.0, -98.0, 7),
        (-86.0, -40.0, 5),
        (88.0, -50.0, 6),
        (-90.0, 30.0, 5),
        (84.0, 30.0, 4),
        (-10.0, 78.0, 6),
    ];
    let mut canopy = MeshBuilder::new();
    let crown_col = srgb(0.55, 0.62, 0.5);
    for &(x, z, n) in &islands {
        for _ in 0..n {
            let cx = x + rng.signed(9.0);
            let cz = z + rng.signed(6.0);
            let h = rng.range(6.0, 11.0);
            let y0 = ground_height(layout, cx, cz, seed);
            trunks.tube(
                &[
                    Ring {
                        center: Vec3::new(cx, y0 - 0.2, cz),
                        radius: 0.3,
                        color: trunk_col,
                    },
                    Ring {
                        center: Vec3::new(cx, y0 + h * 0.7, cz),
                        radius: 0.2,
                        color: trunk_col,
                    },
                ],
                6,
                1.0,
                0.4,
                false,
                &|_, _| 1.0,
            );
            let r = rng.range(3.0, 5.0);
            let s = rng.next_u64() as u32;
            canopy.blob(
                Vec3::new(cx, y0 + h, cz),
                Vec3::new(r, r * 0.7, r),
                8,
                12,
                0.5,
                scale_rgb(crown_col, rng.range(0.7, 1.0)),
                &move |d: Vec3| 0.85 + 0.3 * noise2(d.x * 2.0 + 7.0, d.z * 2.0 + d.y * 3.0, s),
            );
        }
    }
    ctx.static_mesh("far trunks", trunks, &ctx.palette.bark.clone());
    ctx.static_mesh("moriche fronds", fronds, &ctx.palette.fronds.clone());
    ctx.static_mesh("moriche crowns", crowns, &ctx.palette.bark.clone());
    ctx.static_mesh("tree islands", canopy, &ctx.palette.leaves.clone());
}

/// Cloud cover in a sky direction (0 = clear, 1 = overcast), as if the
/// clouds were a flat layer seen in perspective.
fn cloud_cover(dir: Vec3, seed: u32) -> f32 {
    let k = 1.0 / (dir.y.max(0.0) + 0.12);
    let p = Vec2::new(dir.x, dir.z) * k * 1.4;
    let n = fbm2(p.x + 13.0, p.y - 7.0, seed ^ 0xC10D) * 0.75 + noise2(p.x * 3.1, p.y * 3.1, seed ^ 0xC10E) * 0.25;
    let t = ((n - 0.36) / 0.24).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn sky(ctx: &mut SpawnCtx) {
    let moon = MOON_DIR.normalize();
    let moon_flat = Vec2::new(moon.x, moon.z).normalize();
    let seed = ctx.seed as u32;
    let mut dome = MeshBuilder::new();
    let horizon = [HORIZON[0], HORIZON[1], HORIZON[2], 1.0];
    let zenith = [ZENITH[0], ZENITH[1], ZENITH[2], 1.0];
    let stacks = 40;
    let sectors = 96;
    let radius = 900.0;
    // Build rows by hand so colours follow elevation, moon and clouds.
    let mut rows: Vec<Vec<u32>> = Vec::with_capacity(stacks + 1);
    for i in 0..=stacks {
        let phi = i as f32 / stacks as f32 * std::f32::consts::PI;
        let mut row = Vec::with_capacity(sectors + 1);
        for j in 0..=sectors {
            let theta = j as f32 / sectors as f32 * std::f32::consts::TAU;
            let dir = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            let elev = dir.y.max(0.0);
            let mut c = mix_rgb(horizon, zenith, elev.powf(0.55));
            // Glow on the moon's side of the horizon.
            let az = Vec2::new(dir.x, dir.z).normalize_or(Vec2::X).dot(moon_flat).max(0.0);
            let glow = az.powf(3.0) * (1.0 - elev).powf(3.0) * 0.9;
            c = mix_rgb(c, scale_rgb(horizon, 2.4), glow);
            // Mostly overcast: dark cloud masses, their edges silvered near the moon.
            let cover = cloud_cover(dir, seed) * (elev * 8.0).min(1.0);
            let near_moon = dir.dot(moon).max(0.0).powf(6.0);
            let cloud = mix_rgb(scale_rgb(horizon, 0.75), scale_rgb(horizon, 3.2), near_moon);
            c = mix_rgb(c, cloud, cover * 0.85);
            row.push(dome.vertex(dir * radius, -dir, Vec2::ZERO, c));
        }
        rows.push(row);
    }
    for i in 0..stacks {
        for j in 0..sectors {
            // Wound to face inward.
            dome.quad_idx(rows[i][j], rows[i + 1][j], rows[i + 1][j + 1], rows[i][j + 1]);
        }
    }
    let sky = ctx.meshes.add(dome.build());
    ctx.commands.spawn((
        Name::new("sky"),
        Mesh3d(sky),
        MeshMaterial3d(ctx.palette.sky.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // A few restrained stars, only in the gaps between clouds: pinpoints of
    // about a pixel, mostly faint.
    let mut stars = MeshBuilder::new();
    let mut rng = ctx.rng(61);
    for _ in 0..2400 {
        let d = Vec3::new(rng.signed(1.0), rng.range(0.15, 1.0), rng.signed(1.0));
        if !(0.05..=1.0).contains(&d.length_squared()) {
            continue;
        }
        let d = d.normalize();
        if d.dot(moon) > 0.9 || cloud_cover(d, seed) > 0.2 {
            continue;
        }
        let size = rng.range(0.4, 0.8);
        let b = rng.range(0.1, 0.6).powf(1.6) * if rng.f32() < 0.03 { 2.5 } else { 1.0 };
        let tint = [b * rng.range(0.85, 1.0), b * rng.range(0.9, 1.0), b, 1.0];
        let c = d * 860.0;
        let right = d.cross(Vec3::Y).normalize_or(Vec3::X) * size;
        let up = right.cross(d).normalize() * size;
        stars.quad(
            [c - right - up, c - right + up, c + right + up, c + right - up],
            [Vec2::ZERO, Vec2::Y, Vec2::ONE, Vec2::X],
            tint,
        );
    }
    let stars_mesh = ctx.meshes.add(stars.build());
    ctx.commands.spawn((
        Name::new("stars"),
        Mesh3d(stars_mesh),
        MeshMaterial3d(ctx.palette.stars.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // The moon and its halo.
    let mut disc = MeshBuilder::new();
    disc.blob(
        moon * 850.0,
        Vec3::splat(16.0),
        12,
        18,
        1.0,
        [1.6, 1.6, 1.55, 1.0],
        &|_| 1.0,
    );
    let moon_mesh = ctx.meshes.add(disc.build());
    ctx.commands.spawn((
        Name::new("moon"),
        Mesh3d(moon_mesh),
        MeshMaterial3d(ctx.palette.moon.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    let mut halo = MeshBuilder::new();
    let c = moon * 840.0;
    let right = moon.cross(Vec3::Y).normalize() * 150.0;
    let up = right.cross(moon).normalize() * 150.0;
    halo.quad(
        [c - right - up, c - right + up, c + right + up, c + right - up],
        [
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
        ],
        [1.0; 4],
    );
    let halo_mesh = ctx.meshes.add(halo.build());
    ctx.commands.spawn((
        Name::new("moon halo"),
        Mesh3d(halo_mesh),
        MeshMaterial3d(ctx.palette.halo.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

fn moonlight(ctx: &mut SpawnCtx) {
    let dir = MOON_DIR.normalize();
    ctx.commands.spawn((
        Name::new("moonlight"),
        DirectionalLight {
            color: Color::srgb(0.66, 0.75, 1.0),
            illuminance: 650.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(dir * 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.1,
            first_cascade_far_bound: 14.0,
            maximum_distance: 90.0,
            ..default()
        }
        .build(),
    ));
}
