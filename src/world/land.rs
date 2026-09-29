//! The llano: ground, dirt road, sky, moon and stars, and the moonlight.

use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::mesh::{MeshBuilder, mix_rgb, scale_rgb, srgb};
use super::{SpawnCtx, fbm2, noise2};
use crate::geometry::Layout;

/// Direction toward the moon (world space, normalized at use): high in the
/// south-south-west, behind the road, so the house front, the ceiba's trunk
/// and crown and anyone crossing the yard are moonlit from the player's side
/// against the dark northern sky.
pub const MOON_DIR: Vec3 = Vec3::new(-0.38, 0.62, 0.68);

/// Linear colour of the sky at the horizon; the fog uses the same value.
pub const HORIZON: [f32; 3] = [0.0068, 0.0092, 0.0152];
const ZENITH: [f32; 3] = [0.0011, 0.0017, 0.0042];

pub fn fog_visibility(_layout: &Layout) -> f32 {
    105.0
}

pub fn spawn(ctx: &mut SpawnCtx) {
    ground(ctx);
    road(ctx);
    sky(ctx);
    moonlight(ctx);
}

/// Height of the land under `(x, z)`.
pub fn ground_height(layout: &Layout, x: f32, z: f32) -> f32 {
    layout.terrain(Vec2::new(x, z))
}

fn trail_distance(layout: &Layout, p: Vec2) -> f32 {
    layout.district.trail_distance(p) + 0.65
}

fn ground(ctx: &mut SpawnCtx) {
    let layout = ctx.layout;
    let seed = ctx.seed as u32;
    let mut mb = MeshBuilder::new();
    let size = 320.0;
    let origin = Vec3::new(-size * 0.5, 0.0, size * 0.5 - 45.0);
    let fp = layout.house.footprint;
    let porch = layout.house.porch;
    let tree = layout.ceiba.center;
    let grass_tint = srgb(1.0, 1.0, 1.0);
    let mud = srgb(0.72, 0.63, 0.49);
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
            let y = ground_height(layout, x, z);
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
        super::weather::Moonlight,
        DirectionalLight {
            color: Color::srgb(0.66, 0.75, 1.0),
            illuminance: 1150.0,
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
