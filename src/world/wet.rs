//! Standing water: the flooded caño and marsh as translucent sheets that
//! follow the dished terrain (so the shoreline is the land's own, not a
//! rectangle), the ford, and puddles in ruts and yards. The raindrop rings
//! are drawn by `shaders/wet.wgsl` from world position and the shared clock,
//! so the meshes are built once and nothing on the CPU moves per frame.

use bevy::light::NotShadowCaster;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use super::mesh::{MeshBuilder, mix_rgb, srgb};
use super::{SpawnCtx, noise2};
use crate::geometry::{Layout, Rect2};

/// Rain rings on a lit surface. The uniform is
/// (cell size m, drop tempo, ring strength, fade-out distance m).
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RainRings {
    // Extension bindings start at 100; 0..99 belong to StandardMaterial.
    #[uniform(100)]
    pub rings: Vec4,
}

impl MaterialExtension for RainRings {
    fn fragment_shader() -> ShaderRef {
        "shaders/wet.wgsl".into()
    }
}

pub type WetMaterial = ExtendedMaterial<StandardMaterial, RainRings>;

pub struct WetPlugin;

impl Plugin for WetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WetMaterial>::default());
    }
}

/// Water surface over `rect` at `level`, drawn only where the terrain lies
/// below it; shallower water is more transparent and greener.
fn sheet(ctx: &SpawnCtx, r: Rect2, level: f32, cell: f32) -> MeshBuilder {
    let layout = ctx.layout;
    let pad = 1.2;
    let (min, max) = (r.min - Vec2::splat(pad), r.max + Vec2::splat(pad));
    let nx = ((max.x - min.x) / cell).ceil() as usize;
    let nz = ((max.y - min.y) / cell).ceil() as usize;
    let seed = ctx.seed as u32;
    let deep = srgb(0.028, 0.06, 0.075);
    let shallow = srgb(0.11, 0.17, 0.13);
    let mut m = MeshBuilder::new();
    let mut ids = vec![u32::MAX; (nx + 1) * (nz + 1)];
    let depth_at = |p: Vec2| (level - layout.terrain(p)).max(0.0);
    let mut depths = vec![0.0f32; (nx + 1) * (nz + 1)];
    for j in 0..=nz {
        for i in 0..=nx {
            depths[j * (nx + 1) + i] = depth_at(min + Vec2::new(i as f32, j as f32) * cell);
        }
    }
    for j in 0..nz {
        for i in 0..nx {
            let k = |a: usize, b: usize| depths[(j + b) * (nx + 1) + i + a];
            if k(0, 0).max(k(1, 0)).max(k(0, 1)).max(k(1, 1)) < 0.004 {
                continue;
            }
            for (a, b) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let idx = (j + b) * (nx + 1) + i + a;
                if ids[idx] != u32::MAX {
                    continue;
                }
                let p = min + Vec2::new((i + a) as f32, (j + b) as f32) * cell;
                let depth = depths[idx];
                let t = (depth / 0.55).clamp(0.0, 1.0);
                let mut c = mix_rgb(shallow, deep, t.sqrt());
                let mottling = 0.85 + 0.3 * noise2(p.x * 0.2, p.y * 0.2, seed ^ 0x77);
                c = [c[0] * mottling, c[1] * mottling, c[2] * mottling, 0.0];
                c[3] = (depth / 0.26).clamp(0.0, 1.0).powf(0.8) * 0.96;
                ids[idx] = m.vertex(Vec3::new(p.x, level, p.y), Vec3::Y, p * 0.16, c);
            }
            let (a, b, c, d) = (
                ids[j * (nx + 1) + i],
                ids[j * (nx + 1) + i + 1],
                ids[(j + 1) * (nx + 1) + i + 1],
                ids[(j + 1) * (nx + 1) + i],
            );
            m.quad_idx(a, d, c, b);
        }
    }
    m
}

/// Height a puddle rests at above `p`. The ground mesh is a 1.3 m grid, so
/// its triangles can rise a little above the exact terrain between vertices;
/// resting on the highest terrain around the point keeps them from cutting
/// through the puddle.
fn rest_height(layout: &Layout, p: Vec2) -> f32 {
    const E: f32 = 0.5;
    [Vec2::ZERO, Vec2::X * E, Vec2::NEG_X * E, Vec2::Y * E, Vec2::NEG_Y * E]
        .into_iter()
        .map(|o| layout.terrain(p + o))
        .fold(f32::MIN, f32::max)
        + 0.028
}

/// A puddle lying on the ground: an oval with a wobbling rim, `radii` along
/// and across `yaw`, fading to nothing at the edge so it has no hard outline.
fn puddle(m: &mut MeshBuilder, layout: &Layout, center: Vec2, radii: Vec2, yaw: f32, seed: u32) {
    let sectors = 16;
    let rings = 3;
    let (sin_y, cos_y) = yaw.sin_cos();
    let hub_y = rest_height(layout, center);
    let hub = m.vertex(Vec3::new(center.x, hub_y, center.y), Vec3::Y, Vec2::ZERO, [1.0; 4]);
    let mut prev: Vec<u32> = vec![hub; sectors];
    for ring in 1..=rings {
        let f = ring as f32 / rings as f32;
        let mut row = Vec::with_capacity(sectors);
        for s in 0..sectors {
            let a = s as f32 / sectors as f32 * std::f32::consts::TAU;
            let wobble = 0.72 + 0.5 * noise2(a.cos() * 1.7 + center.x, a.sin() * 1.7 + center.y, seed);
            let local = Vec2::new(a.cos() * radii.x, a.sin() * radii.y) * f * wobble;
            let p = center + Vec2::new(local.x * cos_y - local.y * sin_y, local.x * sin_y + local.y * cos_y);
            // Water finds its level: never below the puddle's own rim of ground.
            let y = rest_height(layout, p).max(hub_y - 0.02);
            let alpha = if ring == rings { 0.0 } else { 1.0 - f * 0.25 };
            row.push(m.vertex(Vec3::new(p.x, y, p.y), Vec3::Y, p * 0.5, [1.0, 1.0, 1.0, alpha]));
        }
        for s in 0..sectors {
            let n = (s + 1) % sectors;
            if ring == 1 {
                m.tri(hub, row[n], row[s]);
            } else {
                m.quad_idx(prev[s], prev[n], row[n], row[s]);
            }
        }
        prev = row;
    }
}

struct Spot {
    center: Vec2,
    radii: Vec2,
    yaw: f32,
}

fn wet_mesh(ctx: &mut SpawnCtx, name: &'static str, mesh: MeshBuilder, material: &Handle<WetMaterial>) {
    if mesh.is_empty() {
        return;
    }
    let handle = ctx.meshes.add(mesh.build());
    ctx.commands.spawn((
        Name::new(name),
        Mesh3d(handle),
        MeshMaterial3d(material.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
    ));
}

pub fn spawn(ctx: &mut SpawnCtx, standard: &Assets<StandardMaterial>, wet: &mut Assets<WetMaterial>) {
    let layout = ctx.layout;
    let d = &layout.district;

    // Open water keeps its lazy swell normal map (drifted in the shader) and
    // gains soft rings; puddles are still, dark mirrors whose only texture is
    // the rain.
    let water_base = standard.get(&ctx.palette.water).cloned().unwrap_or_default();
    let puddle_base = StandardMaterial {
        normal_map_texture: None,
        ..standard.get(&ctx.palette.puddle).cloned().unwrap_or_default()
    };
    let water = wet.add(WetMaterial {
        base: water_base,
        extension: RainRings {
            rings: Vec4::new(1.3, 1.5, 0.55, 46.0),
        },
    });
    let puddles = wet.add(WetMaterial {
        base: puddle_base,
        extension: RainRings {
            rings: Vec4::new(0.85, 1.7, 1.0, 34.0),
        },
    });

    // Floodwater and the ford.
    let mut sheets = MeshBuilder::new();
    for r in &d.water {
        let s = sheet(ctx, *r, -0.13, 0.5);
        sheets.append(s);
    }
    for r in &d.shallows {
        let s = sheet(ctx, *r, -0.06, 0.4);
        sheets.append(s);
    }
    wet_mesh(ctx, "floodwater", sheets, &water);

    // Puddles: long ruts down the worn trails and the yards where feet,
    // hooves and wheels churn the mud, then a scatter of low spots.
    let mut rng = ctx.rng(0x9DD1E);
    let mut spots: Vec<Spot> = Vec::new();
    for route in d.routes.iter().filter(|r| r.trail) {
        for w in route.points.windows(2) {
            let len = w[0].distance(w[1]);
            let n = (len / 4.5).floor() as usize;
            let dir = (w[1] - w[0]).normalize_or_zero();
            let yaw = dir.y.atan2(dir.x);
            for k in 0..n {
                let t = (k as f32 + rng.range(0.2, 0.8)) / n.max(1) as f32;
                let side = Vec2::new(-dir.y, dir.x) * rng.signed(route.width * 0.3);
                spots.push(Spot {
                    center: w[0].lerp(w[1], t) + side,
                    radii: Vec2::new(rng.range(0.8, 1.9), rng.range(0.32, 0.7)),
                    yaw: yaw + rng.signed(0.15),
                });
            }
        }
    }
    for m in &d.landmarks {
        for _ in 0..9 {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let r = m.clearing * rng.range(0.15, 0.9);
            let radius = rng.range(0.6, 2.3);
            spots.push(Spot {
                center: m.center + Vec2::new(a.cos(), a.sin()) * r,
                radii: Vec2::new(radius, radius * rng.range(0.6, 1.0)),
                yaw: rng.range(0.0, std::f32::consts::PI),
            });
        }
    }
    let b = layout.bounds;
    for _ in 0..90 {
        let p = Vec2::new(rng.range(b.min.x, b.max.x), rng.range(b.min.y, 21.0));
        let radius = rng.range(0.5, 1.6);
        spots.push(Spot {
            center: p,
            radii: Vec2::new(radius, radius * rng.range(0.6, 1.0)),
            yaw: rng.range(0.0, std::f32::consts::PI),
        });
    }
    let mut mb = MeshBuilder::new();
    let mut count = 0;
    for s in spots {
        let reach = s.radii.max_element() * 1.3;
        // No puddle on, under or poking out of a deck, ramp or bridge, in the
        // water, on cover grass, or against something solid.
        let on_raised = (0..8).any(|i| {
            let a = i as f32 / 8.0 * std::f32::consts::TAU;
            layout.surface_height(s.center + Vec2::new(a.cos(), a.sin()) * reach) > 0.01
        });
        if !b.contains(s.center)
            || !layout.is_free(s.center, s.radii.max_element() * 0.9)
            || d.water_at(s.center)
            || d.shallow_at(s.center)
            || layout.surface_height(s.center) > 0.01
            || on_raised
            || d.grass.iter().any(|g| g.contains(s.center))
        {
            continue;
        }
        puddle(&mut mb, layout, s.center, s.radii, s.yaw, ctx.seed as u32 ^ count);
        count += 1;
    }
    wet_mesh(ctx, "puddles", mb, &puddles);
}
