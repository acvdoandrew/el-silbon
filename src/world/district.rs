//! Stylized district kit: original procedural geometry, shared palette, and
//! merged meshes. Solid footprints and traversal surfaces come from Layout.
use super::{
    SpawnCtx,
    mesh::{MeshBuilder, Rgba, Ring, scale_rgb, srgb},
    noise2,
};
use crate::geometry::{
    Layout, Rect2,
    district::{
        ALTAR_TABLE_HALF, ALTAR_TABLE_OUT, ALTAR_TABLE_TOP, District, LandmarkId, MARSH_WATER, Prop, PropKind, Rail,
        RailStyle, Shed, ShedStyle, Surface, SurfaceKind,
    },
};
use crate::rng::Rng;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

#[derive(Default)]
struct Kit {
    wood: MeshBuilder,
    dark: MeshBuilder,
    roof: MeshBuilder,
    metal: MeshBuilder,
    stone: MeshBuilder,
    cloth: MeshBuilder,
    glow: MeshBuilder,
    straw: MeshBuilder,
}
fn v(p: Vec2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}
fn box_at(m: &mut MeshBuilder, c: Vec3, half: Vec3, tint: Rgba) {
    m.cuboid(c, Quat::IDENTITY, half, 1.0, Vec2::ZERO, tint);
}
fn beam(m: &mut MeshBuilder, a: Vec3, b: Vec3, w: f32, h: f32, tint: Rgba) {
    m.beam(a, b, w, h, 0., 1., Vec2::ZERO, tint);
}
fn cylinder(m: &mut MeshBuilder, c: Vec3, r: f32, h: f32, tint: Rgba) {
    m.lathe(c, &[(0., 0.), (r, 0.), (r, h), (0., h)], 12, 1., tint);
}
fn wood() -> Rgba {
    srgb(0.92, 0.85, 0.73)
}
fn iron() -> Rgba {
    srgb(0.85, 0.91, 0.94)
}
/// Radius of the ceiba trunk at height `y` (as built in world/ceiba.rs: a
/// flared foot and a slow taper), with clearance for the bark's fluting.
fn trunk_r(tree: &crate::geometry::Ceiba, y: f32) -> f32 {
    let top = tree.height * 0.6;
    let t = ((y + 0.3) / (top + 0.3)).clamp(0., 1.);
    let flare = (1. - (y / 2.2).clamp(0., 1.)).powi(2) * 0.9;
    (tree.trunk_radius * (1.02 - 0.3 * t) + flare) * 1.05 + 0.03
}
fn rail_count(r: &Rail) -> usize {
    (r.a.distance(r.b) / 2.6).ceil().max(1.) as usize
}
/// Height of post `i` above its base.
fn rail_top(r: &Rail, i: usize) -> f32 {
    match r.style {
        RailStyle::Broken if i % 3 == 2 => r.height * 0.45,
        RailStyle::Wire => r.height + 0.1,
        _ => r.height + 0.2,
    }
}
/// Height of a shed's gable roof over `p`, if `p` lies under it.
fn roof_height(s: &Shed, p: Vec2) -> Option<f32> {
    let dz = (p.y - s.center.y).abs();
    let hz = s.half.y + 0.4;
    if dz > hz || (p.x - s.center.x).abs() > s.half.x + 0.35 {
        return None;
    }
    let (e, top) = (s.floor + s.eave, s.floor + s.ridge);
    Some(top + (e - top) * dz / hz)
}
/// Vertical boards along a wall line from `a` to `b`, `y0` to `y1`.
fn boards(m: &mut MeshBuilder, a: Vec2, b: Vec2, y0: f32, y1: f32) {
    let len = a.distance(b);
    let n = (len / 0.3).ceil().max(1.) as usize;
    let along = (b - a).normalize_or_zero();
    let rot = Quat::from_rotation_y(along.x.atan2(along.y));
    for i in 0..n {
        let p = a.lerp(b, (i as f32 + 0.5) / n as f32);
        m.cuboid(
            v(p, (y0 + y1) * 0.5),
            rot,
            Vec3::new(0.05, (y1 - y0) * 0.5, len / n as f32 * 0.48),
            1.,
            Vec2::new(i as f32 * 0.17, 0.),
            scale_rgb(wood(), 0.65 + 0.1 * ((i * 7) % 5) as f32),
        );
    }
}
/// Three turns of rope hung on a post; `normal` is the way the coil faces.
fn rope_coil(m: &mut MeshBuilder, at: Vec3, normal: Vec3, r: f32, tint: Rgba) {
    let n = normal.normalize_or_zero();
    let u = n.cross(Vec3::Y).normalize_or_zero();
    let w = n.cross(u);
    for turn in 0..3 {
        let ring = |t: f32| at + n * turn as f32 * 0.04 + (u * t.cos() + w * t.sin()) * r;
        for s in 0..10 {
            let a = s as f32 * std::f32::consts::TAU / 10.;
            let b = (s + 1) as f32 * std::f32::consts::TAU / 10.;
            beam(m, ring(a), ring(b), 0.05, 0.05, tint);
        }
    }
}
/// Evenly spaced points from `a` to `b`, about `step` apart.
fn post_run(a: Vec2, b: Vec2, step: f32) -> Vec<Vec2> {
    let n = (a.distance(b) / step).round().max(1.) as usize;
    (0..=n).map(|i| a.lerp(b, i as f32 / n as f32)).collect()
}

/// Post spacing along the decorative ruined wire lines.
const RUIN_POST_STEP: f32 = 3.;

impl Kit {
    fn finish(self, ctx: &mut SpawnCtx) {
        ctx.static_mesh("district timber", self.wood, &ctx.palette.wood.clone());
        ctx.static_mesh("district bracing", self.dark, &ctx.palette.wood_dark.clone());
        ctx.static_mesh("district zinc roofs", self.roof, &ctx.palette.zinc.clone());
        ctx.static_mesh("district ironwork", self.metal, &ctx.palette.tin.clone());
        ctx.static_mesh("district stonework", self.stone, &ctx.palette.stone.clone());
        ctx.static_mesh("district cloth offerings", self.cloth, &ctx.palette.shirt.clone());
        ctx.static_mesh("district practical flames", self.glow, &ctx.palette.lamp_glass.clone());
        ctx.static_mesh("district hay", self.straw, &ctx.palette.straw.clone());
    }
    fn roof(&mut self, s: &Shed) {
        let c = s.center;
        let e = s.floor + s.eave;
        let top = s.floor + s.ridge;
        let x0 = c.x - s.half.x - 0.35;
        let x1 = c.x + s.half.x + 0.35;
        let z0 = c.y - s.half.y - 0.4;
        let z1 = c.y + s.half.y + 0.4;
        let panels = ((x1 - x0) / 0.65).ceil() as usize;
        let spans = (((x1 - x0) / 2.4).ceil() as usize).max(2);
        let rafters: Vec<f32> = (0..=spans)
            .map(|i| x0 + 0.2 + (x1 - x0 - 0.4) * i as f32 / spans as f32)
            .collect();
        for side in [-1., 1.] {
            let edge = if side < 0. { z0 } else { z1 };
            for n in 0..panels {
                let a = x0 + (x1 - x0) * n as f32 / panels as f32;
                let b = x0 + (x1 - x0) * (n + 1) as f32 / panels as f32;
                let tint = scale_rgb(iron(), 0.8 + 0.12 * (n % 3) as f32);
                self.roof.quad(
                    [
                        Vec3::new(a, top, c.y),
                        Vec3::new(b, top, c.y),
                        Vec3::new(b, e, edge),
                        Vec3::new(a, e, edge),
                    ],
                    [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                    tint,
                );
                for f in [0.15, 0.5, 0.85] {
                    let x = a + (b - a) * f;
                    beam(
                        &mut self.roof,
                        Vec3::new(x, top + 0.015, c.y),
                        Vec3::new(x, e + 0.015, edge),
                        0.025,
                        0.035,
                        tint,
                    );
                }
            }
            for &x in &rafters {
                beam(
                    &mut self.dark,
                    Vec3::new(x, top - 0.08, c.y),
                    Vec3::new(x, e - 0.1, edge),
                    0.12,
                    0.16,
                    wood(),
                );
            }
            beam(
                &mut self.dark,
                Vec3::new(x0, e - 0.08, edge),
                Vec3::new(x1, e - 0.08, edge),
                0.15,
                0.18,
                wood(),
            );
        }
        // Tall open roofs (the corral shelter, the lookout) tie their rafters across.
        if s.eave >= 2.5 {
            for &x in &rafters {
                beam(
                    &mut self.dark,
                    Vec3::new(x, e - 0.12, z0),
                    Vec3::new(x, e - 0.12, z1),
                    0.12,
                    0.14,
                    wood(),
                );
            }
        }
        beam(
            &mut self.roof,
            Vec3::new(x0, top + 0.03, c.y),
            Vec3::new(x1, top + 0.03, c.y),
            0.16,
            0.08,
            iron(),
        );
    }
    fn shed(&mut self, s: &Shed, rng: &mut Rng) {
        let r = Rect2::from_center(s.center, s.half);
        for x in [r.min.x, r.max.x] {
            for z in [r.min.y, r.max.y] {
                beam(
                    &mut self.dark,
                    Vec3::new(x, -0.2, z),
                    Vec3::new(x, s.floor + s.eave, z),
                    0.24,
                    0.24,
                    wood(),
                );
            }
        }
        self.roof(s);
        if s.enclosed {
            for (a, b) in [
                (r.min, Vec2::new(r.max.x, r.min.y)),
                (r.min, Vec2::new(r.min.x, r.max.y)),
                (Vec2::new(r.max.x, r.min.y), r.max),
            ] {
                let n = (a.distance(b) / 0.32).ceil() as usize;
                let along = (b - a).normalize();
                for i in 0..n {
                    let p = a.lerp(b, (i as f32 + 0.5) / n as f32);
                    let tint = if i % 9 == 0 {
                        srgb(0.53, 0.73, 0.72)
                    } else {
                        scale_rgb(wood(), rng.range(0.65, 1.05))
                    };
                    self.wood.cuboid(
                        v(p, s.floor + s.eave * 0.48),
                        Quat::from_rotation_y(along.x.atan2(along.y)),
                        Vec3::new(0.07, s.eave * 0.48, a.distance(b) / n as f32 * 0.48),
                        1.,
                        Vec2::new(i as f32 * 0.17, 0.),
                        tint,
                    );
                }
                for y in [0.5, s.eave - 0.35] {
                    beam(&mut self.dark, v(a, s.floor + y), v(b, s.floor + y), 0.15, 0.12, wood());
                }
            }
        }
        // Front is genuinely open, with knee braces well above head height.
        for x in [r.min.x, r.max.x] {
            let sign = if x < s.center.x { 1. } else { -1. };
            beam(
                &mut self.dark,
                Vec3::new(x, s.floor + s.eave - 0.7, r.max.y),
                Vec3::new(x + sign * 0.8, s.floor + s.eave - 0.1, r.max.y),
                0.12,
                0.12,
                wood(),
            );
        }
    }
    /// Framed window set in a plank wall: warm pane, frame and swung-open
    /// shutters. `c` lies on the wall face; `out` is the way it looks.
    fn window(&mut self, c: Vec3, out: Vec3, w: f32, h: f32) {
        let yaw = Quat::from_rotation_y(out.x.atan2(out.z));
        let side = Vec3::new(out.z, 0., -out.x);
        self.glow.cuboid(
            c,
            yaw,
            Vec3::new(w * 0.5 - 0.04, h * 0.5 - 0.04, 0.01),
            1.,
            Vec2::ZERO,
            [0.32; 4],
        );
        for sx in [-1., 1.] {
            self.dark.cuboid(
                c + side * sx * w * 0.5,
                yaw,
                Vec3::new(0.035, h * 0.5 + 0.035, 0.045),
                1.,
                Vec2::ZERO,
                wood(),
            );
            self.dark.cuboid(
                c + Vec3::Y * sx * h * 0.5,
                yaw,
                Vec3::new(w * 0.5 + 0.035, 0.035, 0.045),
                1.,
                Vec2::ZERO,
                wood(),
            );
            let hinge = c + side * sx * (w * 0.5 + 0.04);
            let d = (side * sx * 0.6 + out * 0.8).normalize();
            let len = w * 0.5;
            self.dark.cuboid(
                hinge + d * len * 0.5,
                Quat::from_rotation_y(d.x.atan2(d.z)),
                Vec3::new(0.02, h * 0.5, len * 0.5),
                1.,
                Vec2::ZERO,
                scale_rgb(wood(), 0.8),
            );
        }
        self.dark
            .cuboid(c, yaw, Vec3::new(0.012, h * 0.5, 0.02), 1., Vec2::ZERO, wood());
        self.dark
            .cuboid(c, yaw, Vec3::new(w * 0.5, 0.012, 0.02), 1., Vec2::ZERO, wood());
    }
    fn sack(&mut self, p: Vec3, k: f32) {
        self.cloth.blob(
            p + Vec3::Y * 0.24 * k,
            Vec3::new(0.3 * k, 0.26 * k, 0.22 * k),
            4,
            6,
            1.,
            srgb(0.66, 0.58, 0.42),
            &|_| 1.,
        );
    }
    /// A clay planter with leaves.
    fn pot(&mut self, at: Vec3, s: f32) {
        self.stone.lathe(
            at,
            &[
                (0., 0.),
                (0.12 * s, 0.),
                (0.19 * s, 0.14 * s),
                (0.18 * s, 0.3 * s),
                (0.13 * s, 0.36 * s),
                (0.14 * s, 0.4 * s),
            ],
            10,
            1.,
            srgb(0.6, 0.32, 0.18),
        );
        for n in 0..6 {
            let a = n as f32 * 1.05;
            let dir = Vec3::new(a.cos(), 0., a.sin());
            let base = at + Vec3::Y * 0.36 * s;
            let tip = base + dir * 0.32 * s + Vec3::Y * (0.4 + 0.08 * (n % 3) as f32) * s;
            self.cloth.ribbon(
                &[base, base.lerp(tip, 0.5) + Vec3::Y * 0.06, tip],
                &[0.05, 0.06, 0.005],
                Vec3::new(-dir.z, 0., dir.x),
                &[srgb(0.2, 0.36, 0.14)],
            );
        }
    }
    /// A spoked cartwheel (or its rim only) standing in the plane of `u`, `w`.
    fn wheel_ring(&mut self, hub: Vec3, u: Vec3, w: Vec3, r: f32) {
        for n in 0..12 {
            let a = n as f32 * std::f32::consts::TAU / 12.;
            let b = (n + 1) as f32 * std::f32::consts::TAU / 12.;
            let rim = hub + (u * a.cos() + w * a.sin()) * r;
            let next = hub + (u * b.cos() + w * b.sin()) * r;
            beam(&mut self.dark, rim, next, 0.08, 0.08, wood());
            if n % 2 == 0 {
                beam(&mut self.wood, hub, rim, 0.045, 0.045, wood());
            }
        }
    }
    /// A shovel or hoe leaning on a post: shaft, and the iron at the foot.
    fn leaning_tool(&mut self, foot: Vec3, head: Vec3) {
        beam(&mut self.dark, foot, head, 0.04, 0.04, wood());
        let dir = (head - foot).normalize();
        self.metal.cuboid(
            foot + dir * 0.1,
            Quat::from_rotation_arc(Vec3::Y, dir),
            Vec3::new(0.09, 0.13, 0.012),
            1.,
            Vec2::ZERO,
            iron(),
        );
    }
    /// A fishing net drying on a wall: a top pole and hanging cords along z.
    fn net(&mut self, x: f32, z0: f32, z1: f32, top: f32) {
        beam(
            &mut self.dark,
            Vec3::new(x, top, z0),
            Vec3::new(x, top, z1),
            0.06,
            0.06,
            wood(),
        );
        let cord = scale_rgb(wood(), 0.6);
        let n = ((z1 - z0) / 0.28).ceil().max(1.) as usize;
        for i in 0..=n {
            let z = z0 + (z1 - z0) * i as f32 / n as f32;
            beam(
                &mut self.metal,
                Vec3::new(x, top, z),
                Vec3::new(x, top - 1.1 + 0.12 * (i % 3) as f32, z),
                0.014,
                0.014,
                cord,
            );
        }
        for row in 1..4 {
            let y = top - row as f32 * 0.28;
            beam(
                &mut self.metal,
                Vec3::new(x, y, z0),
                Vec3::new(x, y - 0.05, z1),
                0.014,
                0.014,
                cord,
            );
        }
    }
    /// What makes each outbuilding read as itself, in the manner of the
    /// reference plates: lit windows, stoves, tools, sacks, nets.
    fn dress(&mut self, s: &Shed) {
        let r = Rect2::from_center(s.center, s.half);
        let (f, c) = (s.floor, s.center);
        let (back, left, front) = (r.min.y + 0.07, r.min.x + 0.07, r.max.y);
        let pale = srgb(0.85, 0.78, 0.62);
        match s.style {
            ShedStyle::Casita => {
                for dx in [-1.9, 1.9] {
                    self.window(Vec3::new(c.x + dx, f + 1.3, back), Vec3::Z, 0.9, 0.85);
                }
                // A cross between the windows.
                self.wood.cuboid(
                    Vec3::new(c.x, f + 1.55, back + 0.02),
                    Quat::IDENTITY,
                    Vec3::new(0.035, 0.32, 0.02),
                    1.,
                    Vec2::ZERO,
                    pale,
                );
                self.wood.cuboid(
                    Vec3::new(c.x, f + 1.7, back + 0.02),
                    Quat::IDENTITY,
                    Vec3::new(0.2, 0.035, 0.02),
                    1.,
                    Vec2::ZERO,
                    pale,
                );
                // The side windows glow toward the yard and the road.
                self.window(Vec3::new(r.min.x - 0.07, f + 1.3, c.y - 0.5), -Vec3::X, 0.8, 0.8);
                self.window(Vec3::new(r.max.x + 0.07, f + 1.3, c.y - 0.5), Vec3::X, 0.8, 0.8);
                // A bench along the west wall inside.
                self.wood.cuboid(
                    Vec3::new(left + 0.27, f + 0.46, c.y + 0.6),
                    Quat::IDENTITY,
                    Vec3::new(0.25, 0.03, 0.9),
                    1.,
                    Vec2::ZERO,
                    scale_rgb(wood(), 0.85),
                );
                for z in [c.y - 0.2, c.y + 1.4] {
                    box_at(
                        &mut self.dark,
                        Vec3::new(left + 0.27, f + 0.22, z),
                        Vec3::new(0.2, 0.22, 0.04),
                        wood(),
                    );
                }
                // Planters beside the front posts.
                for x in [r.min.x + 0.65, r.max.x - 0.65] {
                    self.pot(Vec3::new(x, f, front + 0.4), 1.);
                }
            }
            ShedStyle::Cocina => {
                // Clay stove against the back wall, its chimney through the roof.
                let (sx, sz) = (c.x + 1.3, back + 0.42);
                box_at(
                    &mut self.stone,
                    Vec3::new(sx, f + 0.36, sz),
                    Vec3::new(0.75, 0.36, 0.4),
                    scale_rgb(iron(), 0.72),
                );
                box_at(
                    &mut self.glow,
                    Vec3::new(sx, f + 0.725, sz),
                    Vec3::new(0.32, 0.015, 0.2),
                    [0.55; 4],
                );
                for dx in [-0.35, 0.35] {
                    cylinder(
                        &mut self.metal,
                        Vec3::new(sx + dx, f + 0.73, sz + 0.05),
                        0.17,
                        0.2,
                        scale_rgb(iron(), 0.5),
                    );
                }
                let roof = roof_height(s, Vec2::new(sx, sz)).unwrap_or(f + s.ridge);
                let (y0, y1) = (f + 0.72, roof + 0.55);
                box_at(
                    &mut self.stone,
                    Vec3::new(sx, (y0 + y1) * 0.5, sz),
                    Vec3::new(0.24, (y1 - y0) * 0.5, 0.24),
                    scale_rgb(iron(), 0.7),
                );
                box_at(
                    &mut self.stone,
                    Vec3::new(sx, y1 + 0.03, sz),
                    Vec3::new(0.3, 0.04, 0.3),
                    scale_rgb(iron(), 0.6),
                );
                // Firewood stacked outside the west wall.
                for layer in 0..3 {
                    for j in 0..(4 - layer) {
                        let x = r.min.x - 0.2 - j as f32 * 0.17 - layer as f32 * 0.085;
                        let y = f + 0.09 + layer as f32 * 0.15;
                        beam(
                            &mut self.dark,
                            Vec3::new(x, y, c.y - 0.8),
                            Vec3::new(x, y, c.y + 0.8),
                            0.15,
                            0.15,
                            scale_rgb(wood(), 0.8 + 0.05 * ((j + layer) % 3) as f32),
                        );
                    }
                }
                self.window(Vec3::new(r.max.x + 0.07, f + 1.3, c.y), Vec3::X, 0.7, 0.7);
            }
            ShedStyle::Deposito => {
                // Open tool shed: rope on the rear post, tools, sacks and a spare wheel.
                rope_coil(
                    &mut self.dark,
                    Vec3::new(r.max.x - 0.14, f + 1.25, r.min.y),
                    -Vec3::X,
                    0.24,
                    scale_rgb(wood(), 0.85),
                );
                for dx in [0.22, 0.36] {
                    self.leaning_tool(
                        Vec3::new(r.min.x + dx, f + 0.03, r.min.y + 0.4),
                        Vec3::new(r.min.x + dx, f + 1.75, r.min.y + 0.12),
                    );
                }
                self.sack(Vec3::new(r.min.x + 0.55, f, r.min.y + 0.5), 1.);
                self.sack(Vec3::new(r.min.x + 1.1, f, r.min.y + 0.45), 0.9);
                self.wheel_ring(
                    Vec3::new(r.max.x - 0.7, f + 0.44, r.min.y + 0.16),
                    Vec3::X,
                    Vec3::Y,
                    0.43,
                );
            }
            ShedStyle::PumpHouse => {
                self.window(Vec3::new(r.max.x + 0.07, f + 1.4, c.y), Vec3::X, 0.8, 0.8);
                self.window(Vec3::new(c.x, f + 1.4, r.min.y - 0.07), -Vec3::Z, 0.8, 0.8);
            }
            // Its posts, bracing and rope are added by corral().
            ShedStyle::Shelter => {}
            ShedStyle::LeanTo => {
                // Field shelter: sacks, tools and a rope tucked into the rear corners.
                self.sack(Vec3::new(r.min.x + 0.7, f, r.min.y + 0.6), 1.);
                self.sack(Vec3::new(r.min.x + 1.3, f, r.min.y + 0.5), 0.85);
                self.sack(Vec3::new(r.min.x + 0.65, f + 0.36, r.min.y + 0.7), 0.7);
                for dx in [-0.22, -0.36] {
                    self.leaning_tool(
                        Vec3::new(r.max.x + dx, f + 0.03, r.min.y + 0.4),
                        Vec3::new(r.max.x + dx, f + 1.7, r.min.y + 0.12),
                    );
                }
                rope_coil(
                    &mut self.dark,
                    Vec3::new(r.min.x + 0.14, f + 1.2, r.min.y),
                    Vec3::X,
                    0.24,
                    scale_rgb(wood(), 0.85),
                );
            }
            ShedStyle::StiltHut => {
                // Lit windows on the side walls and a net drying on the west wall.
                self.window(Vec3::new(r.min.x - 0.07, f + 1.25, c.y - 1.2), -Vec3::X, 0.75, 0.75);
                self.window(Vec3::new(r.max.x + 0.07, f + 1.25, c.y - 0.6), Vec3::X, 0.75, 0.75);
                self.net(r.min.x - 0.12, c.y + 0.3, c.y + 2.3, f + 2.15);
            }
            ShedStyle::BaseShed => {
                // Shelves of tins and bottles on the back wall, a net on the west wall.
                for (n, y) in [0.9_f32, 1.5].into_iter().enumerate() {
                    self.wood.cuboid(
                        Vec3::new(c.x, f + y, back + 0.16),
                        Quat::IDENTITY,
                        Vec3::new(1.7, 0.025, 0.16),
                        1.,
                        Vec2::ZERO,
                        scale_rgb(wood(), 0.85),
                    );
                    for x in [-1.45, 1.45] {
                        box_at(
                            &mut self.dark,
                            Vec3::new(c.x + x, f + y - 0.1, back + 0.1),
                            Vec3::new(0.025, 0.1, 0.1),
                            wood(),
                        );
                    }
                    for i in 0..6 {
                        let x = c.x - 1.4 + i as f32 * 0.56 + 0.1 * n as f32;
                        let tint = if (i + n) % 2 == 0 {
                            srgb(0.16, 0.34, 0.2)
                        } else {
                            iron()
                        };
                        cylinder(
                            &mut self.metal,
                            Vec3::new(x, f + y + 0.025, back + 0.16),
                            0.06,
                            0.13 + 0.03 * (i % 3) as f32,
                            tint,
                        );
                    }
                }
                self.net(r.min.x - 0.12, c.y - 1.5, c.y + 1.2, f + 2.1);
                rope_coil(
                    &mut self.dark,
                    Vec3::new(r.max.x - 0.14, f + 1.2, r.max.y),
                    -Vec3::X,
                    0.24,
                    scale_rgb(wood(), 0.85),
                );
            }
        }
    }
    fn rail(&mut self, r: &Rail, ground: &dyn Fn(Vec2) -> f32) {
        let n = rail_count(r);
        let dir = (r.b - r.a).normalize_or_zero();
        let side = Vec3::new(-dir.y, 0., dir.x);
        let post = |i: usize| r.a.lerp(r.b, i as f32 / n as f32);
        // Deck and bridge rails carry their own base; the rest stand on the terrain.
        let base = |p: Vec2| if r.base != 0. { r.base } else { ground(p) };
        // Broken fences: posts lean by a repeatable amount, some are stumps.
        let lean = |i: usize| match r.style {
            RailStyle::Broken => side * (((i * 7 + 3) % 5) as f32 - 2.) * 0.14,
            _ => Vec3::ZERO,
        };
        let (pw, mesh_dark) = match r.style {
            RailStyle::Timber => (0.23, wood()),
            RailStyle::Wire => (0.14, scale_rgb(wood(), 0.85)),
            RailStyle::Broken => (0.17, scale_rgb(wood(), 0.7)),
        };
        for i in 0..=n {
            let p = post(i);
            let b = base(p);
            beam(
                &mut self.dark,
                v(p, b - 0.3),
                v(p, b + rail_top(r, i)) + lean(i),
                pw,
                pw,
                mesh_dark,
            );
        }
        let levels: &[f32] = match r.style {
            RailStyle::Timber => &[0.35, 0.75, 1.05],
            RailStyle::Wire => &[0.45, 0.78, 1.1],
            RailStyle::Broken => &[0.4, 0.85],
        };
        for &y in levels {
            if y > r.height {
                continue;
            }
            for i in 0..n {
                let broken = r.style == RailStyle::Broken;
                // Broken runs lose alternating segments, the rest droop.
                if broken && (i + (y * 10.) as usize) % 3 == 1 {
                    continue;
                }
                let sag = if broken { -0.12 * ((i % 2) as f32 + 0.5) } else { 0. };
                let a = v(post(i), base(post(i)) + y) + lean(i);
                let b = v(post(i + 1), base(post(i + 1)) + y + sag) + lean(i + 1);
                match r.style {
                    RailStyle::Wire => beam(&mut self.metal, a, b, 0.02, 0.02, iron()),
                    _ => beam(&mut self.wood, a, b, 0.1, 0.16, wood()),
                }
            }
        }
    }
    /// Posts and a backing board for a painted sign; returns the corners of
    /// its lettered face (bottom-left, bottom-right, top-right, top-left).
    fn signboard(&mut self, pos: Vec2, yaw: f32, size: Vec2) -> [Vec3; 4] {
        let rot = Quat::from_rotation_y(yaw);
        let base = v(pos, 0.);
        let sill = 1.05;
        for sx in [-1., 1.] {
            let p = base + rot * Vec3::new(sx * (size.x * 0.5 - 0.09), 0., 0.);
            beam(
                &mut self.dark,
                p - Vec3::Y * 0.15,
                p + Vec3::Y * (sill + size.y + 0.12),
                0.17,
                0.17,
                wood(),
            );
        }
        let center = base + Vec3::Y * (sill + size.y * 0.5);
        self.wood.cuboid(
            center - rot * Vec3::Z * 0.03,
            rot,
            Vec3::new(size.x * 0.5, size.y * 0.5, 0.04),
            1.,
            Vec2::ZERO,
            scale_rgb(wood(), 0.75),
        );
        // A crown board keeps it looking hand-built.
        beam(
            &mut self.dark,
            center + rot * Vec3::new(-size.x * 0.5, size.y * 0.5 + 0.08, -0.07),
            center + rot * Vec3::new(size.x * 0.5, size.y * 0.5 + 0.12, -0.07),
            0.12,
            0.1,
            wood(),
        );
        let front = rot * Vec3::Z * 0.045;
        let hx = rot * Vec3::X * size.x * 0.5;
        let hy = Vec3::Y * size.y * 0.5;
        [
            center - hx - hy + front,
            center + hx - hy + front,
            center + hx + hy + front,
            center - hx + hy + front,
        ]
    }
    fn gate_frame(&mut self, a: Vec2, b: Vec2, height: f32) {
        for p in [a, b] {
            beam(&mut self.dark, v(p, -0.05), v(p, height), 0.4, 0.4, wood());
        }
        beam(&mut self.wood, v(a, height), v(b, height), 0.4, 0.36, wood());
        for (a, b) in [(a, a.lerp(b, 0.22)), (b, b.lerp(a, 0.22))] {
            beam(&mut self.dark, v(a, height - 0.8), v(b, height), 0.18, 0.18, wood());
        }
    }
    fn prop(&mut self, p: &Prop, ground: f32) {
        let c = v(p.center, ground);
        let h = p.height;
        let a = p.half;
        match p.kind {
            PropKind::Barrel => {
                self.metal.lathe(
                    c,
                    &[
                        (0., 0.),
                        (a.x * 0.85, 0.),
                        (a.x, h * 0.2),
                        (a.x, h * 0.8),
                        (a.x * 0.85, h),
                        (0., h),
                    ],
                    12,
                    1.,
                    iron(),
                );
                for y in [0.14, h * 0.82] {
                    self.dark
                        .lathe(c, &[(a.x * 1.02, y), (a.x * 1.02, y + 0.045)], 12, 1., wood());
                }
            }
            PropKind::Crates => {
                box_at(
                    &mut self.wood,
                    c + Vec3::Y * h * 0.5,
                    Vec3::new(a.x, h * 0.5, a.y),
                    scale_rgb(wood(), 0.8),
                );
                for y in [0.08, h - 0.08] {
                    for z in [-a.y - 0.02, a.y + 0.02] {
                        beam(
                            &mut self.dark,
                            c + Vec3::new(-a.x, y, z),
                            c + Vec3::new(a.x, y, z),
                            0.1,
                            0.14,
                            wood(),
                        );
                    }
                }
                beam(
                    &mut self.dark,
                    c + Vec3::new(-a.x, 0.1, a.y + 0.03),
                    c + Vec3::new(a.x, h - 0.1, a.y + 0.03),
                    0.13,
                    0.1,
                    wood(),
                );
            }
            PropKind::Hay => {
                for x in [-0.45, 0.45] {
                    for z in [-0.45, 0.45] {
                        box_at(
                            &mut self.straw,
                            c + Vec3::new(a.x * x, h * 0.5, a.y * z),
                            Vec3::new(a.x * 0.43, h * 0.48, a.y * 0.43),
                            srgb(0.84, 0.74, 0.48),
                        );
                    }
                }
                for x in [-0.6, 0.6] {
                    box_at(
                        &mut self.dark,
                        c + Vec3::new(a.x * x, h * 0.5, 0.),
                        Vec3::new(0.025, h * 0.5, a.y),
                        wood(),
                    );
                }
            }
            PropKind::Trough => {
                for z in [-a.y, a.y] {
                    box_at(
                        &mut self.wood,
                        c + Vec3::new(0., h * 0.65, z),
                        Vec3::new(a.x, h * 0.35, 0.09),
                        wood(),
                    );
                }
                for x in [-a.x, a.x] {
                    box_at(
                        &mut self.wood,
                        c + Vec3::new(x, h * 0.65, 0.),
                        Vec3::new(0.09, h * 0.35, a.y),
                        wood(),
                    );
                }
                box_at(
                    &mut self.dark,
                    c + Vec3::Y * h * 0.42,
                    Vec3::new(a.x, 0.08, a.y),
                    wood(),
                );
                for x in [-a.x * 0.8, a.x * 0.8] {
                    for z in [-a.y * 0.7, a.y * 0.7] {
                        box_at(
                            &mut self.dark,
                            c + Vec3::new(x, 0.22, z),
                            Vec3::new(0.12, 0.25, 0.12),
                            wood(),
                        );
                    }
                }
            }
            PropKind::Well => {
                for row in 0..4 {
                    for n in 0..12 {
                        let angle = (n as f32 + row as f32 * 0.5) * std::f32::consts::TAU / 12.;
                        let q = Quat::from_rotation_y(-angle);
                        self.stone.cuboid(
                            c + Vec3::new(
                                angle.cos() * a.x * 0.84,
                                0.13 + row as f32 * 0.25,
                                angle.sin() * a.x * 0.84,
                            ),
                            q,
                            Vec3::new(0.24, 0.12, 0.28),
                            1.,
                            Vec2::ZERO,
                            scale_rgb(iron(), 0.7 + 0.08 * (n % 3) as f32),
                        );
                    }
                }
                for x in [-a.x, a.x] {
                    beam(
                        &mut self.dark,
                        c + Vec3::new(x, 0., 0.),
                        c + Vec3::new(x, 2.5, 0.),
                        0.15,
                        0.15,
                        wood(),
                    );
                }
                beam(
                    &mut self.dark,
                    c + Vec3::new(-a.x, 2.4, 0.),
                    c + Vec3::new(a.x, 2.4, 0.),
                    0.2,
                    0.2,
                    wood(),
                );
                // Windlass roller with a crank, and the bucket hanging over the water.
                beam(
                    &mut self.dark,
                    c + Vec3::new(-a.x, 2.2, 0.),
                    c + Vec3::new(a.x, 2.2, 0.),
                    0.14,
                    0.14,
                    wood(),
                );
                beam(
                    &mut self.metal,
                    c + Vec3::new(a.x * 0.85, 2.2, 0.),
                    c + Vec3::new(a.x * 0.85, 2.2, 0.4),
                    0.03,
                    0.03,
                    iron(),
                );
                beam(
                    &mut self.metal,
                    c + Vec3::new(a.x * 0.85, 2.2, 0.4),
                    c + Vec3::new(a.x * 0.85, 1.95, 0.4),
                    0.05,
                    0.05,
                    iron(),
                );
                beam(
                    &mut self.metal,
                    c + Vec3::Y * 2.2,
                    c + Vec3::Y * 1.8,
                    0.025,
                    0.025,
                    iron(),
                );
                cylinder(&mut self.metal, c + Vec3::Y * 1.45, 0.2, 0.33, iron());
                cylinder(&mut self.dark, c + Vec3::Y * 0.55, 0.66, 0.03, scale_rgb(wood(), 0.3));
            }
            PropKind::Cart => {
                box_at(
                    &mut self.wood,
                    c + Vec3::Y * 0.55,
                    Vec3::new(a.x, 0.09, a.y * 0.9),
                    wood(),
                );
                for z in [-a.y, a.y] {
                    box_at(
                        &mut self.wood,
                        c + Vec3::new(0., 0.88, z),
                        Vec3::new(a.x, 0.25, 0.07),
                        wood(),
                    );
                    let hub = c + Vec3::new(0., 0.46, z * 1.1);
                    for n in 0..12 {
                        let a = n as f32 * std::f32::consts::TAU / 12.;
                        let b = (n + 1) as f32 * std::f32::consts::TAU / 12.;
                        let rim = hub + Vec3::new(a.cos() * 0.43, a.sin() * 0.43, 0.);
                        beam(
                            &mut self.dark,
                            rim,
                            hub + Vec3::new(b.cos() * 0.43, b.sin() * 0.43, 0.),
                            0.08,
                            0.08,
                            wood(),
                        );
                        if n % 2 == 0 {
                            beam(&mut self.wood, hub, rim, 0.045, 0.045, wood());
                        }
                    }
                }
                beam(
                    &mut self.dark,
                    c + Vec3::new(-a.x, 0.5, 0.),
                    c + Vec3::new(-a.x - 1.4, 0.25, 0.),
                    0.12,
                    0.12,
                    wood(),
                );
            }
            PropKind::TankTower => {
                // Splayed timber legs, X bracing, a plank deck and a riveted tank.
                let top = 4.3;
                let rust = srgb(0.62, 0.4, 0.3);
                for sx in [-1.0_f32, 1.0] {
                    for sz in [-1.0_f32, 1.0] {
                        beam(
                            &mut self.dark,
                            c + Vec3::new(sx * a.x, -0.1, sz * a.y),
                            c + Vec3::new(sx * a.x * 0.72, top, sz * a.y * 0.72),
                            0.24,
                            0.24,
                            wood(),
                        );
                    }
                }
                for (y, k) in [(1.4_f32, 0.9_f32), (2.8, 0.78)] {
                    let e = |sx: f32, sz: f32| c + Vec3::new(sx * a.x * k, y, sz * a.y * k);
                    for (p0, p1) in [((-1., -1.), (1., 1.)), ((1., -1.), (-1., 1.))] {
                        beam(
                            &mut self.metal,
                            e(p0.0, p0.1) - Vec3::Y * 0.7,
                            e(p1.0, p1.1) + Vec3::Y * 0.7,
                            0.05,
                            0.05,
                            iron(),
                        );
                    }
                    for (p0, p1) in [
                        ((-1., -1.), (1., -1.)),
                        ((1., -1.), (1., 1.)),
                        ((1., 1.), (-1., 1.)),
                        ((-1., 1.), (-1., -1.)),
                    ] {
                        beam(&mut self.dark, e(p0.0, p0.1), e(p1.0, p1.1), 0.12, 0.1, wood());
                    }
                }
                box_at(
                    &mut self.wood,
                    c + Vec3::Y * top,
                    Vec3::new(a.x * 0.85, 0.08, a.y * 0.85),
                    wood(),
                );
                self.metal.lathe(
                    c + Vec3::Y * (top + 0.08),
                    &[
                        (0., 0.),
                        (0.98, 0.),
                        (1.06, 0.3),
                        (1.08, 0.8),
                        (1.04, 1.4),
                        (0.95, 1.6),
                        (0., 1.6),
                    ],
                    16,
                    1.,
                    scale_rgb(iron(), 0.75),
                );
                for y in [0.35, 0.85, 1.35] {
                    self.dark.lathe(
                        c + Vec3::Y * (top + 0.08 + y),
                        &[(1.09, 0.), (1.09, 0.06)],
                        16,
                        1.,
                        scale_rgb(rust, 0.7),
                    );
                }
                self.roof.lathe(
                    c + Vec3::Y * (top + 1.68),
                    &[(1.22, 0.), (0.9, 0.22), (0.4, 0.5), (0.05, 0.62), (0., 0.62)],
                    16,
                    1.,
                    iron(),
                );
                // Ladder up one side and a down-pipe.
                for z in [-0.2, 0.2] {
                    beam(
                        &mut self.dark,
                        c + Vec3::new(a.x * 0.98, 0., a.y * 0.5 + z),
                        c + Vec3::new(a.x * 0.72, top, a.y * 0.5 + z),
                        0.06,
                        0.06,
                        wood(),
                    );
                }
                for k in 0..11 {
                    let f = (k as f32 + 0.5) / 11.0;
                    beam(
                        &mut self.dark,
                        c + Vec3::new(a.x * (0.98 - 0.26 * f), top * f, a.y * 0.5 - 0.2),
                        c + Vec3::new(a.x * (0.98 - 0.26 * f), top * f, a.y * 0.5 + 0.2),
                        0.04,
                        0.04,
                        wood(),
                    );
                }
                beam(
                    &mut self.metal,
                    c + Vec3::new(-0.6, top, 0.2),
                    c + Vec3::new(-0.6, 0.0, 0.2),
                    0.09,
                    0.09,
                    iron(),
                );
            }
            PropKind::Coop => {
                for x in [-0.6_f32, 0.6] {
                    for z in [-0.4_f32, 0.4] {
                        beam(
                            &mut self.dark,
                            c + Vec3::new(x, 0., z),
                            c + Vec3::new(x, 0.45, z),
                            0.09,
                            0.09,
                            wood(),
                        );
                    }
                }
                box_at(
                    &mut self.wood,
                    c + Vec3::new(0., 0.85, 0.),
                    Vec3::new(a.x * 0.85, 0.4, a.y * 0.85),
                    scale_rgb(wood(), 0.85),
                );
                // Lean-back tin roof and a ramp.
                self.roof.quad(
                    [
                        c + Vec3::new(-a.x, 1.35, -a.y),
                        c + Vec3::new(-a.x, 1.35, a.y),
                        c + Vec3::new(a.x, 1.08, a.y),
                        c + Vec3::new(a.x, 1.08, -a.y),
                    ],
                    [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                    iron(),
                );
                beam(
                    &mut self.dark,
                    c + Vec3::new(a.x * 0.9, 0.5, 0.),
                    c + Vec3::new(a.x * 1.7, 0.02, 0.),
                    0.3,
                    0.05,
                    wood(),
                );
                box_at(
                    &mut self.glow,
                    c + Vec3::new(0., 0.7, a.y * 0.86),
                    Vec3::new(0.1, 0.12, 0.01),
                    [0.3; 4],
                );
            }
            _ => {}
        }
    }
}

pub fn spawn(ctx: &mut SpawnCtx, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) {
    let layout = ctx.layout;
    let d = &layout.district;
    let ground = |p: Vec2| layout.terrain(p);
    let mut kit = Kit::default();
    let mut water = MeshBuilder::new();
    let mut rng = ctx.rng(0xD157);
    for shed in &d.sheds {
        kit.shed(shed, &mut rng);
        kit.dress(shed);
    }
    for rail in &d.rails {
        kit.rail(rail, &ground);
    }
    for prop in &d.props {
        kit.prop(prop, layout.surface_height(prop.center));
    }
    entry(&mut kit, d);
    supports(&mut kit, layout);
    water_tower(&mut kit, d);
    pump(&mut kit, layout);
    watchtower(&mut kit, layout);
    shrine(&mut kit, ctx);
    altar(&mut kit, ctx);
    crossings(&mut kit, layout);
    boat(&mut kit, layout);
    ranch(&mut kit, layout);
    corral(&mut kit, layout, &mut water);
    fields(&mut kit, layout);
    marsh(&mut kit, layout);
    for (a, b) in corral_gates(d) {
        kit.gate_frame(a, b, 2.6);
    }
    // The painted boards: posts and backing in the kit, the lettered face as
    // its own textured quad (each text is its own small texture).
    for (i, sign) in d.signs.iter().enumerate() {
        let corners = kit.signboard(sign.pos, sign.yaw, sign.size);
        let image = super::texture::sign_lines(images, crate::lore::sign(sign.text), ctx.seed ^ (i as u64 * 977));
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(image),
            perceptual_roughness: 0.7,
            reflectance: 0.35,
            ..default()
        });
        let mut face = MeshBuilder::new();
        face.quad(corners, [Vec2::Y, Vec2::ONE, Vec2::X, Vec2::ZERO], [1.; 4]);
        ctx.static_mesh("painted sign", face, &material);
    }
    kit.finish(ctx);
    ctx.static_mesh("corral trough water", water, &ctx.palette.puddle.clone());
    vegetation(ctx);
    trees(ctx);
}

/// The pen-aisle gate frames stand on the fence-line posts at both ends of
/// the six-metre aisle, so each post is on a solid footprint.
fn corral_gates(d: &District) -> [(Vec2, Vec2); 2] {
    let c = d.landmark(LandmarkId::Corral).center;
    [
        (c + Vec2::new(-3., 5.), c + Vec2::new(3., 5.)),
        (c + Vec2::new(-3., -7.), c + Vec2::new(3., -7.)),
    ]
}

fn bridge_rails(d: &District) -> [Rail; 2] {
    let bridge = d.surfaces[0];
    [bridge.rect.min.x, bridge.rect.max.x].map(|x| Rail {
        a: Vec2::new(x, bridge.rect.min.y),
        b: Vec2::new(x, bridge.rect.max.y),
        height: 1.1,
        base: bridge.north,
        style: RailStyle::Timber,
    })
}

fn watch_rails(d: &District) -> [Rail; 5] {
    let deck = d.watch_deck;
    let r = d.watch_ramp;
    [
        (deck.min, Vec2::new(deck.max.x, deck.min.y)),
        (deck.min, Vec2::new(deck.min.x, deck.max.y)),
        (Vec2::new(deck.max.x, deck.min.y), deck.max),
        (Vec2::new(deck.min.x, deck.max.y), Vec2::new(r.min.x, deck.max.y)),
        (Vec2::new(r.max.x, deck.max.y), deck.max),
    ]
    .map(|(a, b)| Rail {
        a,
        b,
        height: 1.15,
        base: d.watch_height,
        style: RailStyle::Timber,
    })
}

/// A tall post a lantern can hook onto instead of standing on a pole of its own.
struct Post {
    at: Vec2,
    /// Absolute height of its top.
    top: f32,
}

fn push_rail_posts(out: &mut Vec<Post>, layout: &Layout, r: &Rail) {
    let n = rail_count(r);
    for i in 0..=n {
        let p = r.a.lerp(r.b, i as f32 / n as f32);
        let base = if r.base != 0. { r.base } else { layout.terrain(p) };
        out.push(Post {
            at: p,
            top: base + rail_top(r, i),
        });
    }
}

/// Every fence, gate, sign, bridge and ramp post the kit stands, as built above.
fn hook_posts(layout: &Layout) -> Vec<Post> {
    let d = &layout.district;
    let mut out = Vec::new();
    for r in &d.rails {
        push_rail_posts(&mut out, layout, r);
    }
    for r in bridge_rails(d).iter().chain(watch_rails(d).iter()) {
        push_rail_posts(&mut out, layout, r);
    }
    for (a, b) in corral_gates(d) {
        out.push(Post { at: a, top: 2.6 });
        out.push(Post { at: b, top: 2.6 });
    }
    for s in &d.signs {
        let along = Vec2::new(s.yaw.cos(), -s.yaw.sin());
        for sx in [-1., 1.] {
            out.push(Post {
                at: s.pos + along * sx * (s.size.x * 0.5 - 0.09),
                top: 1.05 + s.size.y + 0.12,
            });
        }
    }
    let ramp = d.watch_ramp;
    let len = ramp.max.y - ramp.min.y;
    let count = (len / 2.5).ceil() as usize;
    for x in [ramp.min.x, ramp.max.x] {
        for i in 0..=count {
            let p = Vec2::new(x, ramp.min.y + len * i as f32 / count as f32);
            let t = i as f32 / count as f32;
            out.push(Post {
                at: p,
                top: d.watch_height * (1. - t) + 1.2,
            });
        }
    }
    out
}

/// Visual supports for practical lamps that would otherwise hang in the air
/// (notes and relics are perched by dynamic.rs). Heights come from the layout.
/// Lamps under a shed roof hang on a short chain from it; the shrine, pump and
/// lookout cabin carry their own; a lamp within reach of a fence, gate, sign or
/// bridge post hooks onto that post rather than adding a second pole beside it.
fn supports(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    let shrine = layout.ceiba.center;
    let tower = d.landmark(LandmarkId::WaterTower).center;
    let pump = Vec2::new(d.pump.x, d.pump.z);
    // The entry gate and extraction bridge were reviewed with their own poles.
    let own_pole = [
        (d.landmark(LandmarkId::Entry).center, 9.),
        (d.landmark(LandmarkId::Extraction).center, 12.),
    ];
    let posts = hook_posts(layout);
    for lamp in d.lamps.iter().filter(|l| !l.powered) {
        let p = Vec2::new(lamp.pos.x, lamp.pos.z);
        if p.distance(shrine) < 6. || p.distance(pump) < 0.9 || d.watch_deck.contains(p) {
            continue;
        }
        let hang = lamp.pos.y + 0.16;
        let pad = Vec2::splat(0.6);
        if let Some(s) = d
            .sheds
            .iter()
            .find(|s| Rect2::new(s.center - s.half - pad, s.center + s.half + pad).contains(p) && lamp.pos.y > s.floor)
        {
            if let Some(y) = roof_height(s, p)
                && y - hang > 0.08
            {
                beam(
                    &mut k.metal,
                    Vec3::new(p.x, y - 0.02, p.y),
                    Vec3::new(p.x, hang, p.y),
                    0.02,
                    0.02,
                    iron(),
                );
            }
            continue;
        }
        if d.props.iter().any(|q| {
            Rect2::new(q.center - q.half - pad, q.center + q.half + pad).contains(p)
                && q.height >= lamp.pos.y - layout.surface_height(q.center) - 0.35
        }) {
            continue;
        }
        let g = layout.surface_height(p);
        let gap = lamp.pos.y - g;
        // Lanterns on the tower frame hang from its legs.
        if gap > 5. && p.distance(tower) < 6. {
            let h = d.tower_height;
            let t = ((lamp.pos.y - d.tower_ground) / h).clamp(0., 1.);
            let leg = |sx: f32, sz: f32| {
                let base = v(tower, d.tower_ground) + Vec3::new(d.tower_half.x * sx, 0., d.tower_half.y * sz);
                let top =
                    v(tower, d.tower_ground) + Vec3::new(d.tower_half.x * sx * 0.62, h, d.tower_half.y * sz * 0.62);
                base.lerp(top, t)
            };
            let best = [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)]
                .into_iter()
                .map(|(sx, sz)| leg(sx, sz))
                .min_by(|a, b| a.distance_squared(lamp.pos).total_cmp(&b.distance_squared(lamp.pos)))
                .unwrap_or(lamp.pos);
            beam(&mut k.metal, best, lamp.pos + Vec3::Y * 0.2, 0.05, 0.05, iron());
            continue;
        }
        if gap < 0.5 || gap > 5. {
            continue;
        }
        let hook = if own_pole.iter().all(|(c, r)| p.distance(*c) > *r) {
            posts
                .iter()
                .filter(|q| q.at.distance(p) <= 1.7)
                .min_by(|a, b| a.at.distance(p).total_cmp(&b.at.distance(p)))
        } else {
            None
        };
        let (foot, top) = hook.map_or((p, f32::MIN), |q| (q.at, q.top));
        let base = layout.surface_height(foot);
        // A fresh pole where nothing tall stands already; a taller post is carried up.
        if top < hang - 0.02 {
            let w = if hook.is_some() { 0.12 } else { 0.09 };
            beam(&mut k.dark, v(foot, base - 0.2), v(foot, hang), w, w, wood());
        }
        let reach = foot.distance(p);
        if reach > 0.08 {
            beam(
                &mut k.metal,
                v(foot, hang),
                lamp.pos + Vec3::Y * 0.12,
                0.03,
                0.03,
                iron(),
            );
            if reach > 0.8 {
                beam(
                    &mut k.dark,
                    v(foot, hang - 0.55),
                    v(foot.lerp(p, 0.55), hang - 0.02),
                    0.05,
                    0.05,
                    wood(),
                );
            }
        } else {
            // Hook arm over the lantern.
            beam(&mut k.metal, v(p, hang), lamp.pos + Vec3::Y * 0.12, 0.03, 0.03, iron());
        }
    }
}

fn entry(k: &mut Kit, d: &District) {
    let c = d.landmark(LandmarkId::Entry).center;
    let half = (d.rails[1].a.x - d.rails[0].b.x) * 0.5;
    k.gate_frame(c - Vec2::X * half, c + Vec2::X * half, 3.9);
    beam(
        &mut k.wood,
        v(c - Vec2::X * (half + 0.5), 4.2),
        v(c + Vec2::X * (half + 0.5), 4.05),
        0.5,
        0.42,
        wood(),
    );
    // Open gate leaves sit along the roadside fence, never across the opening.
    for side in [-1., 1.] {
        let a = c + Vec2::new(side * (half + 0.4), 0.35);
        let b = a + Vec2::new(side * 2.8, 0.);
        for y in [0.3, 0.65, 1., 1.35] {
            beam(&mut k.wood, v(a, y), v(b, y), 0.12, 0.17, wood());
        }
        beam(&mut k.dark, v(a, 0.3), v(b, 1.35), 0.16, 0.12, wood());
        beam(&mut k.dark, v(a, 1.35), v(b, 0.3), 0.14, 0.1, wood());
    }
    // A weathered cattle skull nailed to the crossbeam: long-faced, hollow
    // eyed, with up-swept horns.
    let bone = srgb(0.86, 0.82, 0.7);
    let socket = srgb(0.05, 0.04, 0.035);
    let sk = v(c, 3.62) + Vec3::Z * 0.3;
    k.stone
        .blob(sk + Vec3::Y * 0.1, Vec3::new(0.2, 0.17, 0.15), 6, 9, 1., bone, &|_| 1.);
    k.stone.tube(
        &[
            Ring {
                center: sk + Vec3::new(0., 0.05, 0.02),
                radius: 0.15,
                color: bone,
            },
            Ring {
                center: sk + Vec3::new(0., -0.2, 0.06),
                radius: 0.115,
                color: bone,
            },
            Ring {
                center: sk + Vec3::new(0., -0.46, 0.09),
                radius: 0.085,
                color: scale_rgb(bone, 0.9),
            },
            Ring {
                center: sk + Vec3::new(0., -0.56, 0.1),
                radius: 0.09,
                color: scale_rgb(bone, 0.8),
            },
        ],
        8,
        1.,
        1.,
        true,
        &|_, _| 1.,
    );
    for side in [-1., 1.] {
        // Sockets and the brow above them.
        k.dark.blob(
            sk + Vec3::new(side * 0.1, 0.03, 0.12),
            Vec3::new(0.055, 0.06, 0.04),
            5,
            7,
            1.,
            socket,
            &|_| 1.,
        );
        k.stone.blob(
            sk + Vec3::new(side * 0.12, 0.11, 0.1),
            Vec3::new(0.08, 0.03, 0.05),
            4,
            6,
            1.,
            bone,
            &|_| 1.,
        );
        // Horn: out, then up.
        k.stone.tube(
            &[
                Ring {
                    center: sk + Vec3::new(side * 0.16, 0.14, 0.0),
                    radius: 0.075,
                    color: bone,
                },
                Ring {
                    center: sk + Vec3::new(side * 0.44, 0.2, -0.02),
                    radius: 0.055,
                    color: bone,
                },
                Ring {
                    center: sk + Vec3::new(side * 0.72, 0.36, -0.03),
                    radius: 0.035,
                    color: scale_rgb(bone, 0.9),
                },
                Ring {
                    center: sk + Vec3::new(side * 0.86, 0.62, -0.03),
                    radius: 0.008,
                    color: scale_rgb(bone, 0.7),
                },
            ],
            7,
            1.,
            1.,
            false,
            &|_, _| 1.,
        );
    }
    // A chained bell to warn of visitors.
    let bell = v(c + Vec2::X * half * 0.55, 3.35);
    for dx in [-0.12, 0.12] {
        beam(
            &mut k.metal,
            bell + Vec3::new(dx, 0.55, 0.),
            bell + Vec3::new(dx * 0.4, 0.2, 0.),
            0.02,
            0.02,
            iron(),
        );
    }
    k.metal.lathe(
        bell - Vec3::Y * 0.22,
        &[(0.19, 0.0), (0.17, 0.1), (0.12, 0.28), (0.08, 0.36), (0.0, 0.4)],
        12,
        1.,
        scale_rgb(iron(), 0.8),
    );
}

fn water_tower(k: &mut Kit, d: &District) {
    let center = d.landmark(LandmarkId::WaterTower).center;
    let c = v(center, d.tower_ground);
    let h = d.tower_height;
    let a = d.tower_half;
    for x in [-1., 1.] {
        for z in [-1., 1.] {
            let b = c + Vec3::new(a.x * x, 0., a.y * z);
            let top = c + Vec3::new(a.x * x * 0.62, h, a.y * z * 0.62);
            beam(&mut k.metal, b, top, 0.18, 0.18, iron());
            box_at(&mut k.stone, b + Vec3::Y * 0.15, Vec3::new(0.35, 0.15, 0.35), iron());
        }
    }
    for level in 0..3 {
        let y = level as f32 * h / 3.;
        let y1 = (level + 1) as f32 * h / 3.;
        let lo = a * (1. - 0.38 * y / h);
        let hi = a * (1. - 0.38 * y1 / h);
        for side in [-1., 1.] {
            for swap in [false, true] {
                let point = |x, y, z| c + if swap { Vec3::new(z, y, x) } else { Vec3::new(x, y, z) };
                beam(
                    &mut k.metal,
                    point(-lo.x, y, lo.y * side),
                    point(hi.x, y1, hi.y * side),
                    0.08,
                    0.08,
                    iron(),
                );
                beam(
                    &mut k.metal,
                    point(lo.x, y, lo.y * side),
                    point(-hi.x, y1, hi.y * side),
                    0.08,
                    0.08,
                    iron(),
                );
                beam(
                    &mut k.metal,
                    point(-lo.x, y, lo.y * side),
                    point(lo.x, y, lo.y * side),
                    0.12,
                    0.12,
                    iron(),
                );
            }
        }
    }
    let r = d.tank_radius;
    k.roof.lathe(
        c,
        &[
            (0., h - 0.1),
            (r, h - 0.1),
            (r, h + 2.1),
            (0.15, h + 2.55),
            (0., h + 2.55),
        ],
        20,
        1.,
        iron(),
    );
    for y in [h + 0.1, h + 1.0, h + 1.9] {
        k.metal.lathe(c, &[(r + 0.04, y), (r + 0.04, y + 0.08)], 20, 1., iron());
    }
    let hub = c + Vec3::new(0., h + 4.1, 0.);
    beam(&mut k.metal, c + Vec3::Y * (h + 1.8), hub, 0.16, 0.16, iron());
    for n in 0..12 {
        let a = n as f32 * std::f32::consts::TAU / 12.;
        let rot = Quat::from_rotation_z(a);
        beam(&mut k.metal, hub, hub + rot * Vec3::Y * 2.1, 0.055, 0.055, iron());
        k.roof.cuboid(
            hub + rot * Vec3::Y * 1.6,
            rot * Quat::from_rotation_y(0.2),
            Vec3::new(0.26, 0.62, 0.05),
            1.,
            Vec2::ZERO,
            iron(),
        );
    }
    beam(&mut k.metal, hub, hub + Vec3::new(3., 0., 0.), 0.08, 0.08, iron());
    k.roof.quad(
        [
            hub + Vec3::new(2.1, -0.4, 0.),
            hub + Vec3::new(3.5, -0.65, 0.),
            hub + Vec3::new(3.5, 0.65, 0.),
            hub + Vec3::new(2.1, 0.4, 0.),
        ],
        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
        iron(),
    );
    // Service pipe; the tower is an orientation landmark, not a ladder promise.
    beam(
        &mut k.metal,
        c + Vec3::new(-r, h, 0.),
        c + Vec3::new(-r, 0.4, 0.),
        0.11,
        0.11,
        iron(),
    );
}

fn crossings(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    for s in &d.surfaces {
        let width = s.rect.max.x - s.rect.min.x;
        let len = s.rect.max.y - s.rect.min.y;
        let n = (len / 0.32).ceil() as usize;
        let slope = (s.north - s.south) / len;
        let ramp = (s.north - s.south).abs() > 0.1;
        for i in 0..n {
            let z = s.rect.min.y + (i as f32 + 0.5) * len / n as f32;
            let p = Vec2::new(s.rect.center().x, z);
            k.wood.cuboid(
                v(p, s.height(p) - 0.055),
                Quat::from_rotation_x(slope.atan()),
                Vec3::new(width * 0.5, 0.065, len / n as f32 * 0.48),
                1.,
                Vec2::new(i as f32 * 0.15, 0.),
                scale_rgb(wood(), 0.8 + 0.06 * (i % 4) as f32),
            );
        }
        for x in [s.rect.min.x, s.rect.max.x] {
            beam(
                &mut k.dark,
                Vec3::new(x, s.north - 0.17, s.rect.min.y),
                Vec3::new(x, s.south - 0.17, s.rect.max.y),
                0.2,
                0.2,
                wood(),
            );
            let n = (len / 3.).ceil() as usize;
            for i in 0..=n {
                let z = s.rect.min.y + len * i as f32 / n as f32;
                let p = Vec2::new(x, z);
                // Piles reach the bed under water, not a guessed depth.
                let low = (layout.terrain(p) - 0.3).min(-0.5);
                beam(&mut k.dark, v(p, low), v(p, s.height(p)), 0.22, 0.22, wood());
                // A raised ramp is tied across under each pile row.
                let tie = s.height(p) - 0.42;
                if ramp && x == s.rect.min.x && tie > 0.9 {
                    beam(
                        &mut k.dark,
                        Vec3::new(s.rect.min.x, tie, z),
                        Vec3::new(s.rect.max.x, tie, z),
                        0.16,
                        0.14,
                        wood(),
                    );
                }
            }
            // ...and X-braced between rows where it stands well clear of the ground.
            if ramp {
                for i in 0..n {
                    let p0 = Vec2::new(x, s.rect.min.y + len * i as f32 / n as f32);
                    let p1 = Vec2::new(x, s.rect.min.y + len * (i + 1) as f32 / n as f32);
                    let lo = |p: Vec2| layout.terrain(p) + 0.3;
                    let hi = |p: Vec2| (s.height(p) - 0.42).max(lo(p));
                    if hi(p0).max(hi(p1)) - lo(p0).min(lo(p1)) < 0.9 {
                        continue;
                    }
                    beam(&mut k.dark, v(p0, lo(p0)), v(p1, hi(p1)), 0.11, 0.09, wood());
                    beam(&mut k.dark, v(p1, lo(p1)), v(p0, hi(p0)), 0.11, 0.09, wood());
                }
            }
        }
    }
    // Bridge handrails stop at the banks. Water blockers are authoritative.
    for rail in bridge_rails(d) {
        k.rail(&rail, &|_: Vec2| 0.);
    }
}

fn watchtower(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    let deck = d.watch_deck;
    let r = d.watch_ramp;
    let h = d.watch_height;
    let c = deck.center();
    let shed = d.watch_shelter();
    k.roof(&shed);
    for x in [deck.min.x, deck.max.x] {
        for z in [deck.min.y, deck.max.y] {
            beam(
                &mut k.dark,
                Vec3::new(x, 0., z),
                Vec3::new(x, h + shed.eave, z),
                0.32,
                0.32,
                wood(),
            );
        }
    }
    for x in [deck.min.x, deck.max.x] {
        beam(
            &mut k.dark,
            Vec3::new(x, 0.3, deck.min.y),
            Vec3::new(x, h - 0.2, deck.max.y),
            0.2,
            0.22,
            wood(),
        );
        beam(
            &mut k.dark,
            Vec3::new(x, 0.3, deck.max.y),
            Vec3::new(x, h - 0.2, deck.min.y),
            0.2,
            0.22,
            wood(),
        );
    }
    // The north face is X-braced too, and girts ring the frame at two heights.
    let (x0, x1, z0, z1) = (deck.min.x, deck.max.x, deck.min.y, deck.max.y);
    beam(
        &mut k.dark,
        Vec3::new(x0, 0.3, z0),
        Vec3::new(x1, h - 0.3, z0),
        0.2,
        0.22,
        wood(),
    );
    beam(
        &mut k.dark,
        Vec3::new(x1, 0.3, z0),
        Vec3::new(x0, h - 0.3, z0),
        0.2,
        0.22,
        wood(),
    );
    for (a, b) in [
        (Vec2::new(x0, z0), Vec2::new(x1, z0)),
        (Vec2::new(x0, z0), Vec2::new(x0, z1)),
        (Vec2::new(x1, z0), Vec2::new(x1, z1)),
        (Vec2::new(x0, z1), Vec2::new(r.min.x, z1)),
        (Vec2::new(r.max.x, z1), Vec2::new(x1, z1)),
    ] {
        for y in [h * 0.36, h * 0.68] {
            beam(&mut k.dark, v(a, y), v(b, y), 0.2, 0.16, wood());
        }
    }
    for rail in watch_rails(d) {
        k.rail(&rail, &|_: Vec2| 0.);
    }
    let surface = Surface {
        rect: r,
        north: h,
        south: 0.,
        kind: SurfaceKind::Timber,
    };
    for x in [r.min.x, r.max.x] {
        let len = r.max.y - r.min.y;
        let count = (len / 2.5).ceil() as usize;
        for i in 0..=count {
            let p = Vec2::new(x, r.min.y + len * i as f32 / count as f32);
            beam(
                &mut k.dark,
                v(p, surface.height(p) - 0.1),
                v(p, surface.height(p) + 1.2),
                0.17,
                0.17,
                wood(),
            );
        }
        for height in [0.5, 1.1] {
            beam(
                &mut k.wood,
                Vec3::new(x, h + height, r.min.y),
                Vec3::new(x, height, r.max.y),
                0.12,
                0.13,
                wood(),
            );
        }
    }
    // The cabin: a plank back wall with a window opening and short returns,
    // open toward the ramp and the view.
    let (cx, zb) = (c.x, z0 + 0.1);
    let wall = |k: &mut Kit, a: Vec2, b: Vec2, y0: f32, y1: f32| boards(&mut k.wood, a, b, h + y0, h + y1);
    wall(k, Vec2::new(x0 + 0.1, zb), Vec2::new(cx - 0.65, zb), 0., 2.55);
    wall(k, Vec2::new(cx + 0.65, zb), Vec2::new(x1 - 0.1, zb), 0., 2.55);
    wall(k, Vec2::new(cx - 0.65, zb), Vec2::new(cx + 0.65, zb), 0., 1.3);
    wall(k, Vec2::new(cx - 0.65, zb), Vec2::new(cx + 0.65, zb), 2.15, 2.55);
    for x in [x0 + 0.1, x1 - 0.1] {
        wall(k, Vec2::new(x, z0 + 0.1), Vec2::new(x, z0 + 2.5), 0., 2.55);
    }
    for y in [1.3, 2.15] {
        beam(
            &mut k.dark,
            Vec3::new(cx - 0.72, h + y, zb + 0.04),
            Vec3::new(cx + 0.72, h + y, zb + 0.04),
            0.14,
            0.06,
            wood(),
        );
    }
    for x in [cx - 0.65, cx + 0.65] {
        beam(
            &mut k.dark,
            Vec3::new(x, h + 1.3, zb + 0.04),
            Vec3::new(x, h + 2.15, zb + 0.04),
            0.06,
            0.06,
            wood(),
        );
    }
    // The cabin lantern hangs from the ridge on a chain.
    for lamp in d
        .lamps
        .iter()
        .filter(|l| !l.powered && deck.contains(Vec2::new(l.pos.x, l.pos.z)))
    {
        let p = Vec2::new(lamp.pos.x, lamp.pos.z);
        if let Some(y) = roof_height(&shed, p)
            && y - lamp.pos.y > 0.3
        {
            beam(
                &mut k.metal,
                Vec3::new(p.x, y - 0.02, p.y),
                lamp.pos + Vec3::Y * 0.16,
                0.025,
                0.025,
                iron(),
            );
        }
    }
    // A weather banner with a chalk cross hangs over the south rail beside the
    // ramp foot, where anyone climbing sees it.
    let (bx, bz) = ((x0 + r.min.x) * 0.5, z1 + 0.09);
    box_at(
        &mut k.cloth,
        Vec3::new(bx, h + 0.32, bz),
        Vec3::new(0.7, 0.85, 0.018),
        srgb(0.45, 0.56, 0.59),
    );
    for y in [h + 1.17, h - 0.53] {
        beam(
            &mut k.dark,
            Vec3::new(bx - 0.75, y, bz),
            Vec3::new(bx + 0.75, y, bz),
            0.05,
            0.05,
            wood(),
        );
    }
    box_at(
        &mut k.dark,
        Vec3::new(bx, h + 0.4, bz + 0.024),
        Vec3::new(0.035, 0.4, 0.01),
        scale_rgb(wood(), 0.5),
    );
    box_at(
        &mut k.dark,
        Vec3::new(bx, h + 0.62, bz + 0.024),
        Vec3::new(0.24, 0.035, 0.01),
        scale_rgb(wood(), 0.5),
    );
}

/// The pump the windmill lever works: a squat iron standpipe with its handle,
/// a feed pipe under the tower, and the base lantern on a post beside it.
fn pump(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    let at = Vec2::new(d.pump.x, d.pump.z);
    let g = layout.terrain(at);
    let c = v(at, g);
    box_at(&mut k.stone, c + Vec3::Y * 0.06, Vec3::new(0.42, 0.06, 0.42), iron());
    k.metal.lathe(
        c + Vec3::Y * 0.12,
        &[
            (0., 0.),
            (0.2, 0.),
            (0.16, 0.12),
            (0.12, 0.3),
            (0.12, 0.85),
            (0.17, 0.92),
            (0.17, 1.02),
            (0., 1.02),
        ],
        12,
        1.,
        scale_rgb(iron(), 0.8),
    );
    beam(
        &mut k.metal,
        c + Vec3::new(0.1, 0.75, 0.),
        c + Vec3::new(0.5, 0.68, 0.),
        0.07,
        0.07,
        iron(),
    );
    beam(
        &mut k.dark,
        c + Vec3::new(-0.7, 1.05, 0.),
        c + Vec3::new(0.45, 1.3, 0.),
        0.06,
        0.06,
        wood(),
    );
    let tower = d.landmark(LandmarkId::WaterTower).center;
    beam(
        &mut k.metal,
        c + Vec3::Y * 0.22,
        v(Vec2::new(tower.x, tower.y + 0.5), g + 0.22),
        0.1,
        0.1,
        iron(),
    );
    if let Some(l) = d
        .lamps
        .iter()
        .find(|l| !l.powered && Vec2::new(l.pos.x, l.pos.z).distance(at) < 0.9)
    {
        let foot = at + Vec2::new(0.5, 0.);
        let top = l.pos.y + 0.18;
        beam(&mut k.dark, v(foot, g - 0.1), v(foot, top), 0.09, 0.09, wood());
        beam(&mut k.metal, v(foot, top), l.pos + Vec3::Y * 0.12, 0.03, 0.03, iron());
    }
}

fn shrine(k: &mut Kit, ctx: &SpawnCtx) {
    let layout = ctx.layout;
    let tree = &layout.ceiba;
    let c = tree.center;
    for n in 0..10 {
        let a = n as f32 * std::f32::consts::TAU / 10.;
        let p = c + Vec2::new(a.cos(), a.sin()) * 4.8;
        let g = layout.terrain(p);
        k.stone
            .blob(v(p, g + 0.13), Vec3::new(0.35, 0.23, 0.25), 4, 6, 1., iron(), &|_| 1.);
        if n % 2 == 0 {
            cylinder(&mut k.cloth, v(p, g + 0.3), 0.065, 0.25, srgb(0.9, 0.83, 0.61));
            cylinder(&mut k.glow, v(p, g + 0.55), 0.025, 0.065, [1.; 4]);
        }
    }
    // The shrine's practical lights. No limb reaches down to lantern height,
    // so the lantern hangs from a leaning crook post; the low ones are candle
    // clusters on stone stands (skipped where the flared trunk already holds them).
    for lamp in layout.district.lamps.iter().filter(|l| !l.powered) {
        let p = Vec2::new(lamp.pos.x, lamp.pos.z);
        if p.distance(c) >= 6. {
            continue;
        }
        let g = layout.terrain(p);
        let outward = (p - c).normalize_or(Vec2::X);
        if lamp.pos.y > 1.5 {
            let foot = p + outward * 0.5;
            let top = lamp.pos.y + 0.55;
            beam(&mut k.dark, v(foot, g - 0.2), v(foot, top), 0.14, 0.14, wood());
            beam(&mut k.dark, v(foot, top), v(p, top), 0.09, 0.09, wood());
            beam(
                &mut k.dark,
                v(foot, top - 0.55),
                v(foot.lerp(p, 0.45), top - 0.03),
                0.07,
                0.07,
                wood(),
            );
            beam(&mut k.metal, v(p, top), lamp.pos + Vec3::Y * 0.12, 0.03, 0.03, iron());
        } else if p.distance(c) > trunk_r(tree, lamp.pos.y) + 0.25 {
            let top = (lamp.pos.y - g - 0.36).max(0.2);
            k.stone.lathe(
                v(p, g),
                &[
                    (0., 0.),
                    (0.22, 0.),
                    (0.18, 0.1),
                    (0.16, top - 0.03),
                    (0.2, top),
                    (0., top),
                ],
                10,
                1.,
                iron(),
            );
            for (dx, dz, h) in [(-0.07, 0.03, 0.34), (0.06, -0.05, 0.24), (0.02, 0.09, 0.3)] {
                let b = v(p + Vec2::new(dx, dz), g + top);
                cylinder(&mut k.cloth, b, 0.03, h, srgb(0.92, 0.86, 0.68));
                cylinder(&mut k.glow, b + Vec3::Y * h, 0.012, 0.045, [1.; 4]);
            }
        }
    }
}

/// The votive altar in the ceiba's root hollow: a plank table tucked against
/// the flared trunk with candles in jars, bottles, flowers and a clay pot; a
/// cross, framed saints and rosaries fixed to the bark itself (radially,
/// following the trunk's real radius); and cloth offerings tied to a rope
/// ring round the trunk. The bundles are laid in front, beyond the table.
fn altar(k: &mut Kit, ctx: &SpawnCtx) {
    let layout = ctx.layout;
    let ceiba = &layout.ceiba;
    let off = ceiba.offering;
    let out2 = (crate::geometry::ground(off) - ceiba.center).normalize_or(Vec2::X);
    let out = Vec3::new(out2.x, 0., out2.y);
    let side = Vec3::new(-out2.y, 0., out2.x);
    let rot = Quat::from_rotation_y(out.x.atan2(out.z));
    let yaw = |n: Vec3| Quat::from_rotation_y(n.x.atan2(n.z));
    // The offering point marks the hollow; the ground under the altar is the terrain.
    let floor = layout.terrain(crate::geometry::ground(off));
    let at = |s: f32, o: f32, y: f32| Vec3::new(off.x, floor, off.z) + side * s + out * o + Vec3::Y * y;
    let cen = ceiba.center;
    let ang0 = out2.y.atan2(out2.x);
    // A point on the bark `s` metres round from the altar axis and `y` up,
    // `lift` proud of it, with the outward normal there.
    let bark = |s: f32, y: f32, lift: f32| {
        let r = trunk_r(ceiba, y);
        let a = ang0 + s / r;
        let n = Vec3::new(a.cos(), 0., a.sin());
        (Vec3::new(cen.x, floor + y, cen.y) + n * (r + lift), n)
    };
    let mut rng = ctx.rng(0xA17A2);

    // Table on two crates, its back edge in the trunk's flare.
    for s in [-0.62, 0.62] {
        k.wood.cuboid(
            at(s, 0.1, 0.2),
            rot,
            Vec3::new(0.2, 0.2, 0.24),
            1.,
            Vec2::ZERO,
            scale_rgb(wood(), 0.75),
        );
    }
    k.wood.cuboid(
        at(0., ALTAR_TABLE_OUT, ALTAR_TABLE_TOP - 0.035),
        rot,
        Vec3::new(ALTAR_TABLE_HALF.x, 0.035, ALTAR_TABLE_HALF.y),
        1.,
        Vec2::ZERO,
        scale_rgb(wood(), 0.85),
    );
    let top = ALTAR_TABLE_TOP;
    // Cross fixed to the trunk above the table.
    let pale = srgb(0.85, 0.78, 0.62);
    let (p, n) = bark(0., 1.55, 0.07);
    k.wood
        .cuboid(p, yaw(n), Vec3::new(0.035, 0.7, 0.03), 1., Vec2::ZERO, pale);
    let (p, n) = bark(0., 1.95, 0.07);
    k.wood
        .cuboid(p, yaw(n), Vec3::new(0.3, 0.035, 0.03), 1., Vec2::ZERO, pale);
    // Framed saints on the bark around it.
    let frames = [
        (-0.75, 1.1, 0.34, 0.44, srgb(0.1, 0.32, 0.36)),
        (0.75, 1.2, 0.3, 0.4, srgb(0.42, 0.12, 0.1)),
        (-0.3, 1.85, 0.26, 0.34, srgb(0.16, 0.22, 0.38)),
        (1.25, 0.98, 0.26, 0.34, srgb(0.3, 0.2, 0.12)),
    ];
    for (s, y, w, h, tint) in frames {
        let (c, n) = bark(s, y, 0.06);
        let tan = Vec3::new(-n.z, 0., n.x);
        k.dark.cuboid(
            c,
            yaw(n),
            Vec3::new(w * 0.5, h * 0.5, 0.02),
            1.,
            Vec2::ZERO,
            srgb(0.3, 0.2, 0.12),
        );
        // The painted saint: a robe, a pale face and a gold halo.
        let quad = |dx: f32, dy: f32, hw: f32, hh: f32| {
            let (a, b) = (tan * hw, Vec3::Y * hh);
            let m = c + n * 0.026 + tan * dx + Vec3::Y * dy;
            [m - a - b, m + a - b, m + a + b, m - a + b]
        };
        for (corners, t) in [
            (quad(0., -h * 0.05, w * 0.43, h * 0.43), tint),
            (quad(0., h * 0.22, w * 0.11, h * 0.09), srgb(0.7, 0.55, 0.42)),
            (quad(0., h * 0.3, w * 0.2, h * 0.17), srgb(0.85, 0.65, 0.2)),
        ] {
            k.cloth.quad(corners, [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y], t);
        }
    }
    // Candles: wax stubs with flames, some in jars, across the table and on the
    // ground beside and before it.
    let candle = |k: &mut Kit, p: Vec3, h: f32, jar: bool| {
        if jar {
            k.metal.lathe(
                p,
                &[(0.05, 0.0), (0.052, 0.09), (0.047, 0.12)],
                8,
                1.,
                srgb(0.5, 0.55, 0.5),
            );
        }
        cylinder(&mut k.cloth, p + Vec3::Y * 0.01, 0.026, h, srgb(0.92, 0.86, 0.68));
        cylinder(&mut k.glow, p + Vec3::Y * (h + 0.01), 0.011, 0.045, [1.; 4]);
    };
    // dynamic.rs stands a small perch under any note, relic or pepper lying
    // near the altar; keep the candles off those posts.
    let d = &layout.district;
    let mut avoid: Vec<Vec2> = Vec::new();
    for q in d
        .notes
        .iter()
        .map(|n| n.pos)
        .chain(d.relics.iter().copied())
        .chain(d.aji.iter().copied())
    {
        let rel = Vec2::new(q.x - off.x, q.z - off.z);
        if rel.length() < 3. {
            avoid.push(Vec2::new(rel.dot(Vec2::new(side.x, side.z)), rel.dot(out2)));
        }
    }
    let clear = |s: f32, o: f32| avoid.iter().all(|a| a.distance(Vec2::new(s, o)) > 0.24);
    for n in 0..14 {
        let s = -0.85 + n as f32 * 0.131;
        let o = 0.12 + rng.signed(0.2);
        let h = 0.06 + rng.f32() * 0.22;
        if clear(s, o) {
            candle(k, at(s, o, top), h, n % 3 != 0);
        }
    }
    for n in 0..9 {
        let s = rng.signed(1.6);
        let o = if s.abs() < 1.15 {
            0.5 + rng.f32() * 0.1
        } else {
            -0.05 + rng.f32() * 0.5
        };
        let h = 0.08 + rng.f32() * 0.2;
        // The bundles rest in front of the table.
        if (o > 0.45 && s > -0.35 && s < 1.15) || !clear(s, o) {
            continue;
        }
        candle(k, at(s, o, 0.0), h, n % 2 == 0);
    }
    // Bottles, flowers in a jar, a clay pot and a crate.
    for (s, h, tint) in [
        (-0.9, 0.3, srgb(0.16, 0.34, 0.2)),
        (-0.78, 0.26, srgb(0.5, 0.28, 0.08)),
        (0.88, 0.32, srgb(0.16, 0.34, 0.2)),
        (0.76, 0.24, srgb(0.42, 0.24, 0.1)),
    ] {
        k.metal.lathe(
            at(s, -0.02, top),
            &[
                (0.0, 0.0),
                (0.045, 0.0),
                (0.048, 0.1),
                (0.04, h * 0.6),
                (0.02, h * 0.75),
                (0.018, h),
            ],
            8,
            1.,
            tint,
        );
    }
    k.stone.lathe(
        at(-1.35, 0.25, 0.0),
        &[
            (0.0, 0.0),
            (0.1, 0.0),
            (0.17, 0.12),
            (0.16, 0.26),
            (0.1, 0.34),
            (0.11, 0.38),
        ],
        10,
        1.,
        srgb(0.6, 0.32, 0.18),
    );
    for k2 in 0..7 {
        let a = k2 as f32 * 0.9;
        let base = at(0.32, 0.0, top + 0.1);
        let tip = base + Vec3::new(a.cos() * 0.13, 0.32 + 0.05 * (k2 % 3) as f32, a.sin() * 0.13);
        k.cloth.ribbon(
            &[base, (base + tip) * 0.5, tip],
            &[0.012, 0.012, 0.01],
            Vec3::X,
            &[srgb(0.2, 0.36, 0.14)],
        );
        let petal = if k2 % 2 == 0 {
            srgb(0.75, 0.12, 0.1)
        } else {
            srgb(0.9, 0.86, 0.75)
        };
        k.cloth.blob(tip, Vec3::splat(0.035), 4, 6, 1., petal, &|_| 1.);
    }
    k.metal.lathe(
        at(0.32, 0.0, top),
        &[(0.045, 0.0), (0.05, 0.1), (0.04, 0.14)],
        8,
        1.,
        srgb(0.5, 0.55, 0.5),
    );
    k.wood.cuboid(
        at(1.5, 0.2, 0.17),
        rot,
        Vec3::new(0.24, 0.17, 0.2),
        1.,
        Vec2::ZERO,
        scale_rgb(wood(), 0.8),
    );
    candle(k, at(1.5, 0.2, 0.34), 0.2, true);
    // Cloth offerings tied to a rope ring round the trunk, hanging down the
    // bark (they follow its flare), with rosaries between them.
    let ring_y = 3.5;
    let rr = trunk_r(ceiba, ring_y) + 0.02;
    let ring: Vec<Vec3> = (0..=24)
        .map(|i| {
            let a = i as f32 / 24. * std::f32::consts::TAU;
            Vec3::new(cen.x + a.cos() * rr, floor + ring_y, cen.y + a.sin() * rr)
        })
        .collect();
    for w in ring.windows(2) {
        beam(&mut k.dark, w[0], w[1], 0.06, 0.06, scale_rgb(wood(), 0.75));
    }
    let hang = |a: f32, len: f32| -> [Vec3; 3] {
        let p = |t: f32| {
            let y = ring_y - len * t;
            let r = trunk_r(ceiba, y) + 0.05;
            Vec3::new(cen.x + a.cos() * r, floor + y, cen.y + a.sin() * r)
        };
        [p(0.), p(0.5), p(1.)]
    };
    for n in 0..20 {
        let a = n as f32 / 20. * std::f32::consts::TAU + rng.signed(0.08);
        // Over the altar they stay short, above the framed saints.
        let near = (a - ang0).sin().abs() < 0.6 && (a - ang0).cos() > 0.;
        let len = if near {
            0.7 + rng.range(0.0, 0.55)
        } else {
            0.9 + rng.range(0.0, 1.5)
        };
        let tint = match n % 4 {
            0 | 1 => srgb(0.6, 0.14, 0.11),
            2 => srgb(0.78, 0.75, 0.62),
            _ => srgb(0.5, 0.42, 0.3),
        };
        k.cloth.ribbon(
            &hang(a, len),
            &[0.2, 0.17, 0.1],
            Vec3::new(-a.sin(), 0., a.cos()),
            &[tint],
        );
    }
    for off_a in [-0.55, 0.5, 2.0, 3.4, 4.6] {
        let a = ang0 + off_a;
        let pts = hang(a, 0.75);
        k.dark.ribbon(
            &pts,
            &[0.018, 0.018, 0.014],
            Vec3::new(-a.sin(), 0., a.cos()),
            &[srgb(0.25, 0.18, 0.1)],
        );
        let n = Vec3::new(a.cos(), 0., a.sin());
        k.wood.cuboid(
            pts[2] - Vec3::Y * 0.05,
            yaw(n),
            Vec3::new(0.012, 0.05, 0.008),
            1.,
            Vec2::ZERO,
            pale,
        );
        k.wood.cuboid(
            pts[2] - Vec3::Y * 0.03,
            yaw(n),
            Vec3::new(0.035, 0.012, 0.008),
            1.,
            Vec2::ZERO,
            pale,
        );
    }
}

fn boat(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    // Afloat: the hull sits at the flood's surface, not above it.
    let c = v(d.landmark(LandmarkId::Cano).center + Vec2::new(5., -2.), -0.26);
    let stations = [(-2.3, 0.06), (-1.5, 0.6), (0., 0.72), (1.5, 0.6), (2.3, 0.06)];
    for pair in stations.windows(2) {
        for side in [-1., 1.] {
            let (x0, r0) = pair[0];
            let (x1, r1) = pair[1];
            k.wood.quad(
                [
                    c + Vec3::new(x0, 0.34, r0 * side),
                    c + Vec3::new(x1, 0.34, r1 * side),
                    c + Vec3::new(x1, -0.06, r1 * 0.4 * side),
                    c + Vec3::new(x0, -0.06, r0 * 0.4 * side),
                ],
                [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                wood(),
            );
            beam(
                &mut k.dark,
                c + Vec3::new(x0, 0.34, r0 * side),
                c + Vec3::new(x1, 0.34, r1 * side),
                0.07,
                0.08,
                wood(),
            );
        }
    }
    for x in [-1.1, 0., 1.1] {
        box_at(
            &mut k.wood,
            c + Vec3::new(x, 0.15, 0.),
            Vec3::new(0.2, 0.06, 0.6),
            wood(),
        );
    }
    beam(
        &mut k.wood,
        c + Vec3::new(-1.5, 0.45, -0.65),
        c + Vec3::new(1.6, 0.48, 0.75),
        0.06,
        0.06,
        wood(),
    );
    // Made fast to the pier's corner pile with a sagging bow line, and two
    // mooring stumps standing in the water off the pier's head.
    let pier = d.surfaces[1];
    let pile = v(Vec2::new(pier.rect.min.x, pier.rect.max.y), pier.north + 0.05);
    let tie = c + Vec3::new(-1.5, 0.34, -0.6);
    let mid = pile.lerp(tie, 0.5) - Vec3::Y * 0.18;
    let rope = scale_rgb(wood(), 0.8);
    beam(&mut k.dark, pile, mid, 0.035, 0.035, rope);
    beam(&mut k.dark, mid, tie, 0.035, 0.035, rope);
    for (p, top) in [(Vec2::new(24.6, -68.9), 0.4), (Vec2::new(28.2, -69.3), 0.25)] {
        beam(
            &mut k.dark,
            v(p, layout.terrain(p) - 0.3),
            v(p, top),
            0.2,
            0.2,
            scale_rgb(wood(), 0.75),
        );
    }
}

/// The lookout and yard fences keep their posts, so an old wire line can be
/// carried on: posts of uneven height leaning a little, wire sagging between
/// them. `top` is each post's absolute height; nothing here is solid.
fn ruin_fence(k: &mut Kit, layout: &Layout, pts: &[Vec2], top: &dyn Fn(usize, Vec2) -> f32) {
    let t = |i: usize| top(i, pts[i]).max(layout.terrain(pts[i]) + 0.3);
    for (i, &p) in pts.iter().enumerate() {
        let lean = Vec3::new(((i * 5 + 2) % 5) as f32 - 2., 0., ((i * 3 + 1) % 5) as f32 - 2.) * 0.05;
        beam(
            &mut k.dark,
            v(p, layout.terrain(p) - 0.3),
            v(p, t(i)) + lean,
            0.17,
            0.17,
            scale_rgb(wood(), 0.7),
        );
    }
    for i in 1..pts.len() {
        if i % 4 == 0 {
            continue;
        }
        for drop in [0.18, 0.5] {
            let a = v(pts[i - 1], t(i - 1) - drop);
            let b = v(pts[i], t(i) - drop);
            let mid = a.lerp(b, 0.5) - Vec3::Y * (0.08 + 0.06 * (i % 2) as f32);
            beam(&mut k.metal, a, mid, 0.02, 0.02, iron());
            beam(&mut k.metal, mid, b, 0.02, 0.02, iron());
        }
    }
}

fn hen(k: &mut Kit, at: Vec3, yaw: f32, tint: Rgba) {
    let fwd = Quat::from_rotation_y(yaw) * Vec3::Z;
    let tan = Vec3::new(-fwd.z, 0., fwd.x);
    k.cloth
        .blob(at + Vec3::Y * 0.2, Vec3::new(0.12, 0.1, 0.16), 4, 6, 1., tint, &|_| 1.);
    k.cloth.blob(
        at + Vec3::Y * 0.34 + fwd * 0.14,
        Vec3::splat(0.05),
        4,
        6,
        1.,
        scale_rgb(tint, 1.1),
        &|_| 1.,
    );
    k.cloth.blob(
        at + Vec3::Y * 0.4 + fwd * 0.14,
        Vec3::new(0.015, 0.03, 0.03),
        4,
        6,
        1.,
        srgb(0.75, 0.15, 0.1),
        &|_| 1.,
    );
    k.cloth.ribbon(
        &[
            at + Vec3::Y * 0.24 - fwd * 0.12,
            at + Vec3::Y * 0.34 - fwd * 0.2,
            at + Vec3::Y * 0.42 - fwd * 0.22,
        ],
        &[0.06, 0.07, 0.02],
        tan,
        &[scale_rgb(tint, 0.7)],
    );
    for s in [-1., 1.] {
        let o = tan * 0.04 * s;
        beam(
            &mut k.metal,
            at + o,
            at + o + Vec3::Y * 0.12,
            0.012,
            0.012,
            srgb(0.7, 0.6, 0.35),
        );
    }
}

/// The yard: a coiled rope on the fence's end post beside the opening and hens
/// about the coop's ramp, as in the ranch plate.
fn ranch(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    let post = Vec2::new(9., 5.);
    rope_coil(
        &mut k.dark,
        v(post, layout.terrain(post) + 1.0) + Vec3::new(-0.14, 0., 0.),
        -Vec3::X,
        0.22,
        scale_rgb(wood(), 0.85),
    );
    if let Some(coop) = d.props.iter().find(|p| p.kind == PropKind::Coop) {
        for (o, yaw, tint) in [
            (Vec2::new(2.2, 0.5), 0.9, srgb(0.5, 0.3, 0.18)),
            (Vec2::new(2.9, -0.1), 3.6, srgb(0.3, 0.22, 0.16)),
            (Vec2::new(1.9, 1.3), 5.2, srgb(0.62, 0.55, 0.42)),
        ] {
            let p = coop.center + o;
            hen(k, v(p, layout.terrain(p)), yaw, tint);
        }
    }
}

/// The pens: hay heaped in the pen troughs, rain water standing in the yard
/// trough, the shelter's front posts on the pen fence line under its long
/// roof, and rope coiled on the aisle gate posts.
fn corral(k: &mut Kit, layout: &Layout, water: &mut MeshBuilder) {
    let d = &layout.district;
    let c = d.landmark(LandmarkId::Corral).center;
    for p in d
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Trough && p.center.distance(c) < 14.)
    {
        let at = v(p.center, layout.surface_height(p.center));
        if p.center.y < c.y + 2. {
            for n in 0..5 {
                let f = n as f32 / 4. - 0.5;
                k.straw.blob(
                    at + Vec3::new(
                        f * p.half.x * 1.7,
                        p.height * 0.5 + 0.03 * (n % 2) as f32,
                        0.04 * (n % 3) as f32 - 0.04,
                    ),
                    Vec3::new(0.55, 0.14, p.half.y * 0.72),
                    4,
                    6,
                    1.,
                    srgb(0.84, 0.74, 0.48),
                    &|_| 1.,
                );
            }
        } else {
            let (x, z, y) = (p.half.x - 0.1, p.half.y - 0.1, at.y + p.height * 0.7);
            water.quad(
                [
                    Vec3::new(at.x - x, y, at.z - z),
                    Vec3::new(at.x - x, y, at.z + z),
                    Vec3::new(at.x + x, y, at.z + z),
                    Vec3::new(at.x + x, y, at.z - z),
                ],
                [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                [1.; 4],
            );
        }
    }
    if let Some(s) = d.sheds.iter().find(|s| s.style == ShedStyle::Shelter) {
        let z = s.center.y + s.half.y;
        let eave = s.floor + s.eave;
        for x in [c.x - 7., c.x + 7.] {
            let p = Vec2::new(x, z);
            beam(&mut k.dark, v(p, -0.2), v(p, eave), 0.26, 0.26, wood());
            for sign in [-1., 1.] {
                beam(
                    &mut k.dark,
                    v(p, eave - 0.75) + Vec3::X * sign * 0.05,
                    v(p, eave - 0.08) + Vec3::X * sign * 0.85,
                    0.12,
                    0.12,
                    wood(),
                );
            }
        }
    }
    for (a, b) in corral_gates(d) {
        // On the pen side of the posts, clear of the aisle and the note perched over the barrel.
        rope_coil(
            &mut k.dark,
            Vec3::new(a.x - 0.23, 1.5, a.y),
            -Vec3::X,
            0.2,
            scale_rgb(wood(), 0.9),
        );
        rope_coil(
            &mut k.dark,
            Vec3::new(b.x + 0.23, 1.5, b.y),
            Vec3::X,
            0.2,
            scale_rgb(wood(), 0.9),
        );
    }
}

/// The pajonal: a chalk-marked cloth on the wire fence and two broken wire
/// lines standing off the trails through the grass, as in the plate.
fn fields(k: &mut Kit, layout: &Layout) {
    let d = &layout.district;
    if let Some(r) = d.rails.iter().find(|r| r.a.x == 48. && r.b.x == 48.) {
        let p = r.a.lerp(r.b, 2. / rail_count(r) as f32);
        let g = layout.terrain(p);
        let hang = v(p, g + 1.1) + Vec3::X * 0.13;
        k.cloth.ribbon(
            &[
                hang,
                hang + Vec3::new(0.02, -0.35, 0.03),
                hang + Vec3::new(-0.01, -0.75, -0.02),
            ],
            &[0.34, 0.32, 0.22],
            Vec3::Z,
            &[srgb(0.6, 0.55, 0.42)],
        );
        for ang in [0., 1.05, 2.1] {
            k.cloth.cuboid(
                hang + Vec3::new(0.03, -0.42, 0.),
                Quat::from_rotation_x(ang),
                Vec3::new(0.006, 0.11, 0.006),
                1.,
                Vec2::ZERO,
                srgb(0.95, 0.93, 0.85),
            );
        }
    }
    for r in d.ruins.iter().filter(|r| r.base == 0.) {
        ruin_fence(k, layout, &post_run(r.a, r.b, RUIN_POST_STEP), &|i, p| {
            layout.terrain(p) + r.height - 0.3 * (i % 3) as f32
        });
    }
}

/// Marsh edge: the broken bank fence carries on into the flood as posts that
/// stand out of the water at falling heights, with wire sagging between them
/// and a fallen rail lying at the waterline (the trail itself stays on the
/// firm bank; these stand in unwalkable deep water).
fn marsh(k: &mut Kit, layout: &Layout) {
    let water = MARSH_WATER;
    for (n, r) in layout.district.ruins.iter().filter(|r| r.base != 0.).enumerate() {
        let pts = post_run(r.a, r.b, RUIN_POST_STEP);
        ruin_fence(k, layout, &pts, &|i, _| {
            r.base + r.height - 0.16 * i as f32 - 0.1 * ((i * 3 + n) % 3) as f32
        });
        if pts.len() > 2 {
            let g = layout.terrain(pts[1]);
            beam(
                &mut k.wood,
                v(pts[1], water + 0.6 - 0.16 - 0.1 * ((3 + n) % 3) as f32 - 0.3),
                v(pts[2], (g + 0.1).min(water - 0.08)),
                0.1,
                0.16,
                scale_rgb(wood(), 0.7),
            );
        }
    }
}

fn vegetation(ctx: &mut SpawnCtx) {
    let d = &ctx.layout.district;
    let b = ctx.layout.bounds;
    let seed = ctx.seed as u32;
    let mut rng = ctx.rng(0x611A55);
    let field = d.landmark(LandmarkId::Fields).center;
    // Spatially merged patches retain culling without one entity per blade.
    for ix in 0..11 {
        for iz in 0..10 {
            let origin = b.min + Vec2::new(ix as f32 * 16., iz as f32 * 16.);
            let mut mb = MeshBuilder::new();
            for _ in 0..210 {
                let p = origin + Vec2::new(rng.range(0., 16.), rng.range(0., 16.));
                if !b.contains(p)
                    || p.y > 21.
                    || d.route_distance(p) < 0.55
                    || d.grass.iter().any(|g| g.contains(p))
                    || !ctx.layout.is_free(p, 0.45)
                    || ctx.layout.surface_height(p) > 0.01
                {
                    continue;
                }
                if d.landmarks.iter().any(|l| p.distance(l.center) < l.clearing)
                    || d.sheds
                        .iter()
                        .any(|s| Rect2::from_center(s.center, s.half + Vec2::splat(1.)).contains(p))
                {
                    continue;
                }
                let near_field = p.distance(field) < 29.;
                let wet = p.y < -58.;
                let density = noise2(p.x * 0.07, p.y * 0.07, seed);
                if !near_field && rng.f32() > 0.55 {
                    continue;
                }
                let h = if near_field {
                    rng.range(0.9, 1.9) * (0.6 + density * 0.55)
                } else if wet {
                    rng.range(0.7, 1.45)
                } else {
                    rng.range(0.25, 0.7)
                };
                for _ in 0..7 {
                    let a = rng.range(0., std::f32::consts::TAU);
                    let dir = Vec3::new(a.cos(), 0., a.sin());
                    let base = v(p, 0.) + dir * rng.range(0., 0.3);
                    let height = h * rng.range(0.7, 1.1);
                    let tint = if wet {
                        srgb(0.28, 0.37, 0.22)
                    } else {
                        srgb(0.42, 0.43, 0.22)
                    };
                    mb.ribbon(
                        &[
                            base,
                            base + Vec3::Y * height * 0.65 + dir * 0.12,
                            base + Vec3::Y * height + dir * 0.45,
                        ],
                        &[0.045, 0.075, 0.002],
                        Vec3::new(-dir.z, 0., dir.x),
                        &[scale_rgb(tint, 0.45), tint, scale_rgb(tint, 1.3)],
                    );
                    if wet && height > 1.0 {
                        cylinder(
                            &mut mb,
                            base + Vec3::Y * height * 0.87 + dir * 0.35,
                            0.035,
                            0.22,
                            srgb(0.32, 0.23, 0.14),
                        );
                    }
                }
            }
            if !mb.is_empty() {
                let handle = ctx.meshes.add(mb.build());
                ctx.commands.spawn((
                    Name::new("district grass patch"),
                    Mesh3d(handle),
                    MeshMaterial3d(ctx.palette.grass.clone()),
                    Transform::IDENTITY,
                    NotShadowCaster,
                ));
            }
        }
    }
    // Reed islands are nonblocking decoration inside already bounded water.
    let mut reeds = MeshBuilder::new();
    for water in &d.water {
        for _ in 0..45 {
            let p = Vec2::new(rng.range(water.min.x, water.max.x), rng.range(water.min.y, water.max.y));
            // The shore bands belong to flora.rs; these are clumps out in open water.
            let bed = ctx.layout.terrain(p);
            if bed > -0.5 {
                continue;
            }
            if d.surfaces
                .iter()
                .any(|s| Rect2::from_center(s.rect.center(), s.rect.half() + Vec2::splat(1.0)).contains(p))
            {
                continue;
            }
            for n in 0..5 {
                let h = rng.range(1.0, 1.9);
                let base = v(p + Vec2::new(n as f32 * 0.09, 0.), bed - 0.05);
                reeds.ribbon(
                    &[base, base + Vec3::Y * h * 0.7, base + Vec3::new(0.25, h, 0.)],
                    &[0.03, 0.055, 0.],
                    Vec3::Z,
                    &[srgb(0.13, 0.2, 0.13), srgb(0.32, 0.41, 0.27)],
                );
            }
        }
    }
    ctx.static_mesh("marsh reed islands", reeds, &ctx.palette.grass.clone());
}

fn trees(ctx: &mut SpawnCtx) {
    let d = &ctx.layout.district;
    let mut rng = ctx.rng(0xA1A5);
    let mut bark = MeshBuilder::new();
    let mut leaves = MeshBuilder::new();
    for &p in &d.palms {
        let h = rng.range(9., 14.);
        let base = v(p, 0.);
        let top = base + Vec3::new(0.45, h, -0.25);
        bark.tube(
            &[
                Ring {
                    center: base,
                    radius: 0.35,
                    color: wood(),
                },
                Ring {
                    center: base.lerp(top, 0.5),
                    radius: 0.26,
                    color: wood(),
                },
                Ring {
                    center: top,
                    radius: 0.18,
                    color: wood(),
                },
            ],
            8,
            1.,
            1.,
            false,
            &|_, _| 1.,
        );
        for n in 0..11 {
            let a = n as f32 * std::f32::consts::TAU / 11.;
            let dir = Vec3::new(a.cos(), 0., a.sin());
            let side = Vec3::new(-dir.z, 0., dir.x);
            leaves.ribbon(
                &[top, top + dir * 1.6 + Vec3::Y * 0.5, top + dir * 3.7 - Vec3::Y * 1.5],
                &[0.05, 0.7, 0.02],
                side,
                &[srgb(0.24, 0.34, 0.2), srgb(0.45, 0.55, 0.29), srgb(0.31, 0.4, 0.19)],
            );
        }
    }
    for &p in &d.groves {
        let base = v(p, 0.);
        cylinder(&mut bark, base, 0.35, 4.7, wood());
        for n in 0..5 {
            let a = n as f32 * std::f32::consts::TAU / 5.;
            let end = base + Vec3::new(a.cos() * 2.2, 4.5 + rng.range(0., 1.), a.sin() * 2.2);
            beam(&mut bark, base + Vec3::Y * 2.6, end, 0.22, 0.25, wood());
            leaves.blob(
                end + Vec3::Y,
                Vec3::new(2.8, 1.8, 2.7),
                5,
                9,
                0.4,
                srgb(0.6, 0.68, 0.46),
                &|_| 1.,
            );
        }
    }
    ctx.static_mesh("district palm and grove trunks", bark, &ctx.palette.bark.clone());
    ctx.static_mesh(
        "district broadleaf and palm crowns",
        leaves,
        &ctx.palette.fronds.clone(),
    );
}
