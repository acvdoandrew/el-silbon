//! Growth and mist: the tall wet grass that hides a crouching player, reeds
//! and lily pads along the flooded shores, and low mist over the water. The
//! grass rectangles come from the layout (`district.grass`), so what looks
//! like cover is cover.
//!
//! Grass and reeds stay batched into 12 m chunks; the wind is a vertex stage
//! (`shaders/grass.wgsl`) that bends each blade about its pinned root from the
//! world position and the global clock, so nothing is rebuilt or uploaded per
//! frame. Blades carry what the shader needs in ordinary attributes: UV_0 is
//! the blade's root (world x, z) and vertex alpha is its sway weight.

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;

use super::mesh::{MeshBuilder, Rgba, scale_rgb, srgb};
use super::{SpawnCtx, fbm2};
use crate::geometry::Rect2;
use crate::geometry::district::{LandmarkId, WATER_LEVEL};
use crate::rng::Rng;

/// Downwind direction on the ground (x, z); the blades' static lean uses it too.
const WIND: Vec2 = Vec2::new(0.85, 0.25);
/// Tip travel in metres per unit of sway weight at gust 1 (shader gusts peak
/// near 1.5), so a 1.5 m blade sways about 0.2 m at the height of a gust.
const SWAY: f32 = 0.09;

/// Wind for `shaders/grass.wgsl`. The uniform is (downwind x, downwind z,
/// strength, tempo).
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct GrassWind {
    // Extension bindings start at 100; 0..99 belong to StandardMaterial.
    #[uniform(100)]
    pub wind: Vec4,
}

impl MaterialExtension for GrassWind {
    fn vertex_shader() -> ShaderRef {
        "shaders/grass.wgsl".into()
    }
}

pub type GrassMaterial = ExtendedMaterial<StandardMaterial, GrassWind>;

pub struct FloraPlugin;

impl Plugin for FloraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GrassMaterial>::default())
            .add_systems(Update, drift.in_set(crate::app::GameSet::Present));
    }
}

/// Materials the animation systems retune.
#[derive(Resource)]
pub struct Mist {
    pub material: Handle<StandardMaterial>,
}

fn v(p: Vec2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

/// One blade: a tapered ribbon along `points` whose flat side faces `side`.
/// Every vertex carries the root (UV) and a sway weight (alpha) that is zero
/// at the root and grows with the square of the height up the blade, so the
/// base stays planted and the tip travels furthest.
fn blade(mb: &mut MeshBuilder, points: &[Vec3], widths: &[f32], side: Vec3, colors: &[Rgba]) {
    let n = points.len();
    let root = points[0];
    let height = (points[n - 1].y - root.y).max(0.05);
    let uv = Vec2::new(root.x, root.z);
    let mut start = 0;
    for i in 0..n {
        let t = (points[(i + 1).min(n - 1)] - points[i.saturating_sub(1)]).normalize_or(Vec3::Y);
        let normal = side.cross(t).normalize_or(Vec3::Z);
        let w = widths[i.min(widths.len() - 1)] * 0.5;
        let f = ((points[i].y - root.y) / height).clamp(0.0, 1.0);
        let mut c = colors[i.min(colors.len() - 1)];
        c[3] = height * f * f;
        let left = mb.vertex(points[i] - side * w, normal, uv, c);
        if i == 0 {
            start = left;
        }
        mb.vertex(points[i] + side * w, normal, uv, c);
    }
    for i in 0..n as u32 - 1 {
        let a = start + i * 2;
        mb.quad_idx(a, a + 1, a + 3, a + 2);
    }
}

/// A tuft of blades leaning with the wind.
fn tuft(mb: &mut MeshBuilder, base: Vec3, h: f32, lean: Vec3, lo: Rgba, hi: Rgba, rng: &mut Rng, blades: usize) {
    for _ in 0..blades {
        let a = rng.range(0.0, std::f32::consts::TAU);
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let root = base + dir * rng.range(0.0, 0.16);
        let height = h * rng.range(0.65, 1.1);
        let bend = dir * 0.16 * height * rng.range(0.4, 1.0) + lean * height * rng.range(0.25, 0.5);
        let w = rng.range(0.04, 0.075);
        blade(
            mb,
            &[
                root,
                root + Vec3::Y * height * 0.55 + bend * 0.35,
                root + Vec3::Y * height * 0.9 + bend * 0.8,
                root + Vec3::Y * height * 0.98 + bend,
            ],
            &[w, w * 0.9, w * 0.5, 0.003],
            Vec3::new(-dir.z, 0.0, dir.x),
            &[scale_rgb(lo, 0.5), lo, mix(lo, hi, 0.65), hi],
        );
    }
}

/// A cattail: a thin stalk (two crossed strips) topped by a brown head. The
/// head is rigid and rides the stalk's tip, so it takes the tip's sway weight.
fn cattail(mb: &mut MeshBuilder, root: Vec3, h: f32) {
    let top = root + Vec3::Y * h;
    let stalk = srgb(0.2, 0.26, 0.1);
    for side in [Vec3::X, Vec3::Z] {
        blade(
            mb,
            &[root, root + Vec3::Y * h * 0.5, top],
            &[0.034, 0.03, 0.026],
            side,
            &[scale_rgb(stalk, 0.6), stalk],
        );
    }
    let mut c = srgb(0.22, 0.13, 0.07);
    c[3] = h;
    let uv = Vec2::new(root.x, root.z);
    let profile = [(0.0, 0.0), (0.035, 0.02), (0.04, 0.12), (0.03, 0.2), (0.0, 0.23)];
    let base = top - Vec3::Y * 0.02;
    let sides = 7;
    let mut starts = [0u32; 5];
    for (i, &(r, y)) in profile.iter().enumerate() {
        let (r0, y0) = profile[i.saturating_sub(1)];
        let (r1, y1) = profile[(i + 1).min(profile.len() - 1)];
        let pn = Vec2::new(y1 - y0, -(r1 - r0)).normalize_or(Vec2::X);
        for s in 0..=sides {
            let (sa, ca) = (s as f32 / sides as f32 * std::f32::consts::TAU).sin_cos();
            let id = mb.vertex(
                base + Vec3::new(ca * r, y, sa * r),
                Vec3::new(ca * pn.x, pn.y, sa * pn.x),
                uv,
                c,
            );
            if s == 0 {
                starts[i] = id;
            }
        }
    }
    for i in 0..profile.len() - 1 {
        for s in 0..sides as u32 {
            mb.quad_idx(
                starts[i] + s,
                starts[i + 1] + s,
                starts[i + 1] + s + 1,
                starts[i] + s + 1,
            );
        }
    }
}

fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    super::mesh::mix_rgb(a, b, t)
}

/// Spawn one merged patch of swaying growth.
fn grow(ctx: &mut SpawnCtx, name: &'static str, mb: MeshBuilder, material: &Handle<GrassMaterial>) {
    if mb.is_empty() {
        return;
    }
    let handle = ctx.meshes.add(mb.build());
    ctx.commands.spawn((
        Name::new(name),
        Mesh3d(handle),
        MeshMaterial3d(material.clone()),
        // The shader reads the blade's root straight from the mesh, so the
        // chunk must stay untransformed.
        Transform::IDENTITY,
        NotShadowCaster,
    ));
}

/// A soft, tileable cloud of alpha for the mist layers: sparse wisps, not a sheet.
fn mist_texture(images: &mut Assets<Image>, seed: u32) -> Handle<Image> {
    let n = 128usize;
    let mut data = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let (u, w) = (x as f32 / n as f32, y as f32 / n as f32);
            // Wrapped noise: blend the four shifted samples so it tiles.
            let s = |a: f32, b: f32| fbm2(a * 4.0, b * 4.0, seed);
            let k = u;
            let l = w;
            let t = s(u, w) * (1.0 - k) * (1.0 - l)
                + s(u - 1.0, w) * k * (1.0 - l)
                + s(u, w - 1.0) * (1.0 - k) * l
                + s(u - 1.0, w - 1.0) * k * l;
            let a = ((t - 0.4) / 0.32).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (a * a * 255.0) as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    images.add(image)
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A layer of mist over `r`, hanging at `y`. It has no border: the alpha of
/// each vertex fades to nothing over `feather` metres toward the mesh's edge
/// and wherever the ground climbs to meet the layer, so the mist thins out
/// along shores instead of ending in a straight cut or a hard line through
/// the terrain.
fn mist_layer(ctx: &SpawnCtx, r: Rect2, y: f32, uv_scale: f32, feather: f32, strength: f32) -> MeshBuilder {
    let layout = ctx.layout;
    let cell = 1.6;
    let nx = ((r.max.x - r.min.x) / cell).ceil() as usize;
    let nz = ((r.max.y - r.min.y) / cell).ceil() as usize;
    let mut mb = MeshBuilder::new();
    // A fresh builder: vertex ids are grid indices.
    for j in 0..=nz {
        for i in 0..=nx {
            let p = Vec2::new(
                r.min.x + (r.max.x - r.min.x) * i as f32 / nx as f32,
                r.min.y + (r.max.y - r.min.y) * j as f32 / nz as f32,
            );
            let edge = (p.x - r.min.x).min(r.max.x - p.x).min(p.y - r.min.y).min(r.max.y - p.y);
            let clearance = y - layout.terrain(p);
            let a = smooth(edge / feather) * smooth((clearance - 0.03) / 0.3) * strength;
            mb.vertex(v(p, y), Vec3::Y, p * uv_scale, [1.0, 1.0, 1.0, a]);
        }
    }
    let row = nx as u32 + 1;
    for j in 0..nz as u32 {
        for i in 0..nx as u32 {
            let a = j * row + i;
            mb.quad_idx(a, a + row, a + row + 1, a + 1);
        }
    }
    mb
}

pub fn spawn(
    ctx: &mut SpawnCtx,
    materials: &mut Assets<StandardMaterial>,
    grass_materials: &mut Assets<GrassMaterial>,
    images: &mut Assets<Image>,
) {
    let layout = ctx.layout;
    let d = &layout.district;
    let field = d.landmark(LandmarkId::Fields).center;
    let mut rng = ctx.rng(0xF10A);
    let lean = Vec3::new(WIND.x, 0.0, WIND.y);

    // Wet blades shine a little under lantern and moon.
    let grass = grass_materials.add(GrassMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.42,
            reflectance: 0.5,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: GrassWind {
            wind: Vec4::new(WIND.normalize().x, WIND.normalize().y, SWAY, 1.0),
        },
    });

    // ---- Tall grass: every layout grass rectangle, densely planted, kept
    // clear of trails and solid things, and cropped shorter beside a trail so
    // the path reads as a corridor.
    let chunk = 12.0;
    for r in &d.grass {
        let tall = r.contains(field) || r.min.y > -30.0;
        let nx = ((r.max.x - r.min.x) / chunk).ceil() as usize;
        let nz = ((r.max.y - r.min.y) / chunk).ceil() as usize;
        for ix in 0..nx {
            for iz in 0..nz {
                let cell = Rect2::new(
                    r.min + Vec2::new(ix as f32, iz as f32) * chunk,
                    (r.min + Vec2::new(ix as f32 + 1.0, iz as f32 + 1.0) * chunk).min(r.max),
                );
                let area = (cell.max.x - cell.min.x) * (cell.max.y - cell.min.y);
                let mut mb = MeshBuilder::new();
                for _ in 0..(area * 2.4) as usize {
                    let p = Vec2::new(rng.range(cell.min.x, cell.max.x), rng.range(cell.min.y, cell.max.y));
                    let edge = d.route_distance(p);
                    if edge < 0.7 || !layout.is_free(p, 0.3) || d.water_at(p) || d.shallow_at(p) {
                        continue;
                    }
                    let n = fbm2(p.x * 0.09, p.y * 0.09, 0x6A55);
                    let corridor = 0.55 + 0.45 * smooth((edge - 0.7) / 2.8);
                    let h = if tall {
                        rng.range(1.0, 1.85)
                    } else {
                        rng.range(0.7, 1.35)
                    } * (0.75 + 0.5 * n)
                        * corridor;
                    let lo = srgb(0.05 + 0.03 * n, 0.09 + 0.05 * n, 0.035);
                    let hi = srgb(0.22 + 0.1 * n, 0.3 + 0.1 * n, 0.12);
                    tuft(&mut mb, v(p, layout.terrain(p) - 0.03), h, lean, lo, hi, &mut rng, 5);
                }
                grow(ctx, "tall grass", mb, &grass);
            }
        }
    }

    // ---- Shores: reeds and cattails in the shallow band inside each flooded
    // basin, and lily pads on the open water.
    let mut reeds = MeshBuilder::new();
    let mut pads = MeshBuilder::new();
    for r in &d.water {
        let area = (r.max.x - r.min.x) * (r.max.y - r.min.y);
        for _ in 0..(area * 0.9).min(2600.0) as usize {
            let p = Vec2::new(rng.range(r.min.x, r.max.x), rng.range(r.min.y, r.max.y));
            let depth = WATER_LEVEL - layout.terrain(p);
            let on_planks = d
                .surfaces
                .iter()
                .any(|s| Rect2::from_center(s.rect.center(), s.rect.half() + Vec2::splat(1.2)).contains(p));
            // The ford's lane across the caño stays open water.
            if on_planks || d.shallow_at(p) {
                continue;
            }
            if (-0.12..0.34).contains(&depth) {
                // The shore band: dense reeds.
                let h = rng.range(0.9, 1.9);
                let lo = srgb(0.06, 0.1, 0.05);
                let hi = srgb(0.3, 0.34, 0.15);
                tuft(
                    &mut reeds,
                    v(p, layout.terrain(p) - 0.02),
                    h,
                    lean * 0.5,
                    lo,
                    hi,
                    &mut rng,
                    4,
                );
                if rng.f32() < 0.12 {
                    cattail(&mut reeds, v(p, layout.terrain(p)), h * 0.85);
                }
            } else if depth > 0.34 && rng.f32() < 0.16 {
                let radius = rng.range(0.13, 0.32);
                let y = -0.118;
                let tint = srgb(0.1 + rng.f32() * 0.05, 0.26 + rng.f32() * 0.08, 0.1);
                let notch = rng.range(0.0, std::f32::consts::TAU);
                let hub = pads.vertex(v(p, y), Vec3::Y, Vec2::ZERO, tint);
                let sectors = 12;
                let mut ring: Vec<u32> = Vec::new();
                for s in 0..sectors {
                    let a = notch + s as f32 / sectors as f32 * std::f32::consts::TAU;
                    // A notch cut into one side, like a real pad.
                    let cut = if s == 0 { 0.35 } else { 1.0 };
                    let q = p + Vec2::new(a.cos(), a.sin()) * radius * cut;
                    ring.push(pads.vertex(v(q, y + 0.004), Vec3::Y, Vec2::X, scale_rgb(tint, 0.85)));
                }
                for s in 0..sectors {
                    pads.tri(hub, ring[(s + 1) % sectors], ring[s]);
                }
            }
        }
    }
    grow(ctx, "shore reeds", reeds, &grass);
    let pad_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.35,
        reflectance: 0.5,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    ctx.static_mesh("lily pads", pads, &pad_mat);

    // ---- Mist lying on the water: a low sparse layer and a higher thinner one
    // over each basin, feathered out at every edge.
    let mist_tex = mist_texture(images, ctx.seed as u32 ^ 0x3157);
    let mist = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.42, 0.54, 0.17),
        base_color_texture: Some(mist_tex),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    for r in &d.water {
        let outer = Rect2::new(r.min - Vec2::splat(8.0), r.max + Vec2::splat(8.0));
        for (y, scale, strength) in [(0.28, 0.05, 1.0), (0.85, 0.032, 0.7)] {
            let mb = mist_layer(ctx, outer, y, scale, 8.0, strength);
            let handle = ctx.meshes.add(mb.build());
            ctx.commands.spawn((
                Name::new("marsh mist"),
                Mesh3d(handle),
                MeshMaterial3d(mist.clone()),
                Transform::IDENTITY,
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }
    }
    ctx.commands.insert_resource(Mist { material: mist });
}

/// The mist drifts slowly across the water. It moves about a hundredth of a
/// pixel per step, so it is retuned four times a second instead of every frame.
fn drift(time: Res<Time>, mist: Res<Mist>, mut materials: ResMut<Assets<StandardMaterial>>, mut step: Local<u32>) {
    let now = (time.elapsed_secs() * 4.0) as u32;
    if now == *step {
        return;
    }
    *step = now;
    let t = now as f32 * 0.25;
    if let Some(mut m) = materials.get_mut(&mist.material) {
        m.uv_transform = bevy::math::Affine2::from_translation(Vec2::new(t * 0.006, t * 0.0035));
    }
}
