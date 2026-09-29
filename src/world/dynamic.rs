//! The moving parts of the world, driven by the latest snapshot: bone bundles
//! (on the ground, carried, laid at the altar), peppers and pepper wards,
//! pings, the lamp light pool, the pump's power lamps, the beacon's fire, the
//! truck's lamps and the first-person bundle.
//!
//! Nothing here knows where the Silbón is; it reads only what a listener may
//! legitimately see (bundles, wards, marks, the shared world state).

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::mesh::{MeshBuilder, Ring, WHITE, srgb};
use super::vehicles::{TruckAssets, TruckLamp, TruckRoot};
use super::{CarriedSatchel, Palette, SatchelAsset};
use crate::app::{LayoutRes, TuningRes};
use crate::geometry::district::{ALTAR_TABLE_HALF, ALTAR_TABLE_OUT, ALTAR_TABLE_TOP};
use crate::geometry::ground;
use crate::net::Network;
use crate::player::Player;
use crate::sim::PlayerId;
use crate::ui::player_color;

/// Lights in the pool: only the lamps nearest the camera really shine.
const LAMP_POOL: usize = 14;
const MAX_WARDS: usize = 8;
const MAX_PINGS: usize = 4;

#[derive(Component)]
pub struct BundleView(usize);
#[derive(Component)]
pub struct AjiView(usize);
#[derive(Component)]
pub struct WardView(usize);
#[derive(Component)]
pub struct WardGlow;
#[derive(Component)]
pub struct PingMarker(usize);
#[derive(Component)]
pub struct BeaconFlame;
#[derive(Component)]
pub struct BeaconLight;
/// Reused buffers for the lamp pool's selection.
#[derive(Default)]
pub struct LampScratch {
    desired: Vec<usize>,
    held: Vec<usize>,
    pending: Vec<usize>,
}

#[derive(Component)]
pub struct LampSlot {
    lamp: Option<usize>,
    level: f32,
    phase: f32,
}

/// Every practical light of the district, as the layout authored it.
#[derive(Resource)]
pub struct LampRig {
    lamps: Vec<(Vec3, bool)>,
}

/// Materials the systems retune at run time.
#[derive(Resource)]
pub struct DynAssets {
    powered_glass: Handle<StandardMaterial>,
}

fn radial_glow(images: &mut Assets<Image>) -> Handle<Image> {
    let n = 64;
    let mut data = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let r = (u * u + v * v).sqrt();
            let a = ((1.0 - r).clamp(0.0, 1.0)).powf(2.2);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

fn pepper_bunch(m: &mut MeshBuilder) {
    // Three drying pods on a cord, with green calyxes.
    let red = srgb(0.85, 0.1, 0.06);
    let deep = srgb(0.55, 0.05, 0.04);
    let green = srgb(0.2, 0.35, 0.1);
    for (k, (dx, dz, len)) in [(-0.05, 0.02, 0.15), (0.02, -0.03, 0.18), (0.07, 0.04, 0.13)]
        .into_iter()
        .enumerate()
    {
        let base = Vec3::new(dx, 0.02, dz);
        let tilt = Vec3::new(0.03 * (k as f32 - 1.0), 1.0, 0.02);
        let tip = base + tilt.normalize() * len;
        m.tube(
            &[
                Ring {
                    center: base,
                    radius: 0.028,
                    color: green,
                },
                Ring {
                    center: base + (tip - base) * 0.2,
                    radius: 0.032,
                    color: red,
                },
                Ring {
                    center: base + (tip - base) * 0.6,
                    radius: 0.024,
                    color: red,
                },
                Ring {
                    center: tip,
                    radius: 0.004,
                    color: deep,
                },
            ],
            7,
            1.0,
            1.0,
            false,
            &|_, _| 1.0,
        );
    }
    // The tie.
    m.beam(
        Vec3::new(-0.09, 0.02, 0.0),
        Vec3::new(0.1, 0.02, 0.0),
        0.012,
        0.012,
        0.0,
        1.0,
        Vec2::ZERO,
        srgb(0.6, 0.5, 0.3),
    );
}

/// A small plank on a post, so a prop at `top` does not hover; nothing is
/// drawn when it already rests within a hand of the floor.
fn perch(m: &mut MeshBuilder, top: Vec3, floor: f32, half: f32) {
    let gap = top.y - floor;
    if gap < 0.1 {
        return;
    }
    let wood = srgb(0.62, 0.52, 0.38);
    let dark = srgb(0.45, 0.37, 0.28);
    let yaw = Quat::from_rotation_y(top.x * 3.1 + top.z * 1.7);
    // Plank, just below the resting point.
    m.cuboid(
        Vec3::new(top.x, top.y - 0.02, top.z),
        yaw,
        Vec3::new(half * 1.25, 0.02, half),
        1.0,
        Vec2::ZERO,
        wood,
    );
    // Post to the floor, sunk a little into the ground.
    let post = (gap - 0.04 + 0.08).max(0.05);
    m.cuboid(
        Vec3::new(top.x, floor - 0.08 + post * 0.5, top.z),
        yaw,
        Vec3::new(half * 0.32, post * 0.5, half * 0.32),
        1.0,
        Vec2::ZERO,
        dark,
    );
}

/// Scattered flakes in a ring of `radius`, lying flat.
fn ward_ring(radius: f32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let mut rng = crate::rng::Rng::new(0x3A21);
    for _ in 0..90 {
        let a = rng.range(0.0, std::f32::consts::TAU);
        let r = radius * (0.35 + 0.65 * rng.f32().sqrt());
        let c = Vec3::new(a.cos() * r, 0.03, a.sin() * r);
        let s = rng.range(0.03, 0.07);
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        let (sn, cs) = yaw.sin_cos();
        let tint = if rng.f32() < 0.6 {
            srgb(0.9, 0.16, 0.06)
        } else {
            srgb(0.95, 0.5, 0.1)
        };
        let ax = Vec3::new(cs, 0.0, sn) * s;
        let az = Vec3::new(-sn, 0.0, cs) * s * 0.45;
        m.quad(
            [c - ax - az, c + ax - az, c + ax + az, c - ax + az],
            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
            tint,
        );
    }
    m
}

fn diamond(size: f32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let top = Vec3::Y * size * 1.5;
    let bot = -Vec3::Y * size * 1.5;
    let eq = [Vec3::X * size, Vec3::Z * size, -Vec3::X * size, -Vec3::Z * size];
    for i in 0..4 {
        let a = eq[i];
        let b = eq[(i + 1) % 4];
        for (p, q, r) in [(top, b, a), (bot, a, b)] {
            let n = (q - p).cross(r - p).normalize_or(Vec3::Y);
            let ia = m.vertex(p, n, Vec2::ZERO, WHITE);
            let ib = m.vertex(q, n, Vec2::X, WHITE);
            let ic = m.vertex(r, n, Vec2::Y, WHITE);
            m.tri(ia, ib, ic);
        }
    }
    m
}

fn lantern_parts(tin: &mut MeshBuilder, glass: &mut MeshBuilder, c: Vec3) {
    let iron = srgb(0.5, 0.5, 0.5);
    tin.lathe(
        c - Vec3::Y * 0.2,
        &[(0.0, 0.0), (0.12, 0.0), (0.12, 0.05), (0.0, 0.05)],
        10,
        1.0,
        iron,
    );
    glass.lathe(
        c - Vec3::Y * 0.15,
        &[(0.075, 0.0), (0.085, 0.14), (0.07, 0.28)],
        10,
        1.0,
        WHITE,
    );
    tin.lathe(
        c + Vec3::Y * 0.13,
        &[(0.14, 0.0), (0.1, 0.06), (0.03, 0.1), (0.0, 0.14)],
        10,
        1.0,
        iron,
    );
    for x in [-0.09, 0.09] {
        tin.beam(
            c + Vec3::new(x, -0.15, 0.0),
            c + Vec3::new(x, 0.14, 0.0),
            0.014,
            0.014,
            0.0,
            1.0,
            Vec2::ZERO,
            iron,
        );
    }
    // Bail and a short chain to whatever it hangs from.
    tin.beam(
        c + Vec3::new(-0.08, 0.2, 0.0),
        c + Vec3::new(0.08, 0.2, 0.0),
        0.01,
        0.01,
        0.0,
        1.0,
        Vec2::ZERO,
        iron,
    );
    tin.beam(
        c + Vec3::Y * 0.2,
        c + Vec3::Y * 0.5,
        0.012,
        0.012,
        0.0,
        1.0,
        Vec2::ZERO,
        iron,
    );
}

fn pole_lamp_parts(tin: &mut MeshBuilder, glass: &mut MeshBuilder, wood: &mut MeshBuilder, c: Vec3) {
    let iron = srgb(0.45, 0.45, 0.45);
    // The pole itself runs to the ground; the lamp is on a short arm.
    wood.beam(
        Vec3::new(c.x, -0.3, c.z),
        Vec3::new(c.x, c.y + 0.7, c.z),
        0.24,
        0.24,
        0.0,
        1.0,
        Vec2::ZERO,
        srgb(0.85, 0.8, 0.7),
    );
    wood.beam(
        Vec3::new(c.x - 0.9, c.y + 0.55, c.z),
        Vec3::new(c.x + 0.9, c.y + 0.55, c.z),
        0.1,
        0.1,
        0.0,
        1.0,
        Vec2::ZERO,
        srgb(0.85, 0.8, 0.7),
    );
    tin.beam(
        Vec3::new(c.x, c.y + 0.5, c.z),
        Vec3::new(c.x + 0.5, c.y + 0.35, c.z),
        0.05,
        0.05,
        0.0,
        1.0,
        Vec2::ZERO,
        iron,
    );
    let lamp = Vec3::new(c.x + 0.5, c.y, c.z);
    tin.lathe(
        lamp + Vec3::Y * 0.12,
        &[(0.06, 0.0), (0.24, -0.12), (0.26, -0.15), (0.0, 0.05)],
        12,
        1.0,
        iron,
    );
    glass.lathe(
        lamp,
        &[(0.0, -0.12), (0.06, -0.12), (0.09, -0.02), (0.0, 0.08)],
        10,
        1.0,
        WHITE,
    );
    // Insulators and the wire's sag.
    for x in [-0.8, -0.3, 0.3, 0.8] {
        tin.beam(
            Vec3::new(c.x + x, c.y + 0.6, c.z),
            Vec3::new(c.x + x, c.y + 0.75, c.z),
            0.04,
            0.04,
            0.0,
            1.0,
            Vec2::ZERO,
            srgb(0.3, 0.55, 0.5),
        );
    }
}

pub fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    palette: Res<Palette>,
    satchel: Res<SatchelAsset>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
) {
    let layout = &layout.0;
    let d = &layout.district;

    // ---- Bone bundles.
    for i in 0..d.relics.len() {
        let root = commands
            .spawn((
                Name::new("bone bundle"),
                BundleView(i),
                Transform::from_translation(d.relics[i]),
                Visibility::Hidden,
            ))
            .id();
        commands.spawn((
            Mesh3d(satchel.sack.clone()),
            MeshMaterial3d(palette.burlap.clone()),
            Transform::from_scale(Vec3::splat(1.15)),
            ChildOf(root),
        ));
        commands.spawn((
            Mesh3d(satchel.bones.clone()),
            MeshMaterial3d(palette.bone.clone()),
            Transform::from_scale(Vec3::splat(1.15)),
            NotShadowCaster,
            ChildOf(root),
        ));
        // A faint, cold glimmer: the bones remember the tree.
        commands.spawn((
            PointLight {
                color: Color::srgb(0.62, 0.78, 1.0),
                intensity: 4_500.0,
                range: 4.5,
                radius: 0.05,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.2, 0.35, 0.0),
            ChildOf(root),
        ));
    }

    // ---- Discoverable notes and the perches that hold props off the ground.
    // Site 0 (note, bone bundle) lies on the house table, which props.rs owns.
    let mut perches = MeshBuilder::new();
    let mut papers = MeshBuilder::new();
    // What a prop rests on: the deck or ground, a barrel or crate top, or the
    // altar table in the ceiba's hollow.
    let floor = |p: Vec3| {
        let g = Vec2::new(p.x, p.z);
        let ground = layout.surface_height(g);
        let mut rest = ground;
        if let Some(top) = d.prop_top(g) {
            rest = rest.max(ground + top);
        }
        let off = layout.ceiba.offering;
        let out = (Vec2::new(off.x, off.z) - layout.ceiba.center).normalize_or(Vec2::X);
        let rel = g - Vec2::new(off.x, off.z);
        let local = Vec2::new(rel.dot(Vec2::new(-out.y, out.x)), rel.dot(out) - ALTAR_TABLE_OUT);
        if local.x.abs() <= ALTAR_TABLE_HALF.x && local.y.abs() <= ALTAR_TABLE_HALF.y {
            rest = rest.max(layout.terrain(Vec2::new(off.x, off.z)) + ALTAR_TABLE_TOP);
        }
        rest
    };
    for r in d.relics.iter().skip(1) {
        perch(&mut perches, *r, floor(*r), 0.3);
    }
    for (i, p) in d.aji.iter().enumerate() {
        perch(
            &mut perches,
            *p - Vec3::Y * 0.02,
            floor(*p),
            0.13 + 0.01 * (i % 3) as f32,
        );
    }
    for n in d.notes.iter().filter(|n| n.id != 0) {
        perch(&mut perches, n.pos, floor(n.pos), 0.16);
        let rot = Quat::from_rotation_y(0.9 + n.id as f32 * 1.37);
        let (hx, hz) = (0.1, 0.13);
        papers.grid(
            n.pos + rot * Vec3::new(-hx, 0.0, hz),
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
    }
    for (name, mb, material) in [
        ("perches", perches, palette.wood.clone()),
        ("field notes", papers, palette.paper_note.clone()),
    ] {
        if !mb.is_empty() {
            commands.spawn((
                Name::new(name),
                Mesh3d(meshes.add(mb.build())),
                MeshMaterial3d(material),
                NotShadowCaster,
            ));
        }
    }

    // ---- Peppers.
    let mut mb = MeshBuilder::new();
    pepper_bunch(&mut mb);
    let pepper_mesh = meshes.add(mb.build());
    let pepper_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(0.25, 0.01, 0.0),
        perceptual_roughness: 0.45,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    for (i, p) in d.aji.iter().enumerate() {
        commands.spawn((
            Name::new("pepper bunch"),
            AjiView(i),
            Mesh3d(pepper_mesh.clone()),
            MeshMaterial3d(pepper_mat.clone()),
            Transform::from_translation(*p - Vec3::Y * 0.02).with_rotation(Quat::from_rotation_y(i as f32 * 1.7)),
            NotShadowCaster,
            Visibility::Hidden,
        ));
    }

    // ---- Pepper wards.
    let ward_mesh = meshes.add(ward_ring(tuning.0.aji_zone_radius).build());
    let ward_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(0.9, 0.12, 0.02),
        perceptual_roughness: 0.6,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let glow_tex = radial_glow(&mut images);
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.24, 0.05, 0.55),
        base_color_texture: Some(glow_tex.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let mut disc = MeshBuilder::new();
    let r = tuning.0.aji_zone_radius * 1.05;
    disc.quad(
        [
            Vec3::new(-r, 0.04, r),
            Vec3::new(r, 0.04, r),
            Vec3::new(r, 0.04, -r),
            Vec3::new(-r, 0.04, -r),
        ],
        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
        WHITE,
    );
    let disc_mesh = meshes.add(disc.build());
    for i in 0..MAX_WARDS {
        let root = commands
            .spawn((
                Name::new("pepper ward"),
                WardView(i),
                Transform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        commands.spawn((
            Mesh3d(ward_mesh.clone()),
            MeshMaterial3d(ward_mat.clone()),
            NotShadowCaster,
            NotShadowReceiver,
            ChildOf(root),
        ));
        commands.spawn((
            WardGlow,
            Mesh3d(disc_mesh.clone()),
            MeshMaterial3d(glow_mat.clone()),
            NotShadowCaster,
            NotShadowReceiver,
            ChildOf(root),
        ));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.3, 0.1),
                intensity: 6_000.0,
                range: 7.0,
                radius: 0.1,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.5, 0.0),
            ChildOf(root),
        ));
    }

    // ---- Pings.
    let diamond_mesh = meshes.add(diamond(0.18).build());
    for i in 0..MAX_PINGS {
        let c = player_color(i).to_linear();
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(c.red.min(1.0), c.green.min(1.0), c.blue.min(1.0)),
            emissive: LinearRgba::rgb(c.red * 6.0, c.green * 6.0, c.blue * 6.0),
            unlit: true,
            fog_enabled: false,
            ..default()
        });
        commands.spawn((
            Name::new("ping"),
            PingMarker(i),
            Mesh3d(diamond_mesh.clone()),
            MeshMaterial3d(mat),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::IDENTITY,
            Visibility::Hidden,
        ));
    }

    // ---- Lamps: fixtures are static, the light comes from a small pool.
    let mut tin = MeshBuilder::new();
    let mut glass = MeshBuilder::new();
    let mut pole_glass = MeshBuilder::new();
    let mut wood = MeshBuilder::new();
    for lamp in &d.lamps {
        if lamp.powered {
            pole_lamp_parts(&mut tin, &mut pole_glass, &mut wood, lamp.pos);
        } else {
            lantern_parts(&mut tin, &mut glass, lamp.pos);
        }
    }
    let powered_glass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.75),
        emissive: LinearRgba::BLACK,
        perceptual_roughness: 0.2,
        ..default()
    });
    for (name, mb, material) in [
        ("lamp ironwork", tin, palette.tin.clone()),
        ("lantern glass", glass, palette.lamp_glass.clone()),
        ("pole lamp glass", pole_glass, powered_glass.clone()),
        ("power poles", wood, palette.wood.clone()),
    ] {
        if mb.is_empty() {
            continue;
        }
        commands.spawn((
            Name::new(name),
            Mesh3d(meshes.add(mb.build())),
            MeshMaterial3d(material),
            NotShadowCaster,
        ));
    }
    let rig = LampRig {
        lamps: d.lamps.iter().map(|l| (l.pos, l.powered)).collect(),
    };
    for i in 0..LAMP_POOL {
        commands.spawn((
            Name::new("lamp light"),
            LampSlot {
                lamp: None,
                level: 0.0,
                phase: i as f32 * 1.9,
            },
            PointLight {
                color: Color::srgb(1.0, 0.65, 0.32),
                intensity: 0.0,
                range: 13.0,
                radius: 0.1,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
        ));
    }
    commands.insert_resource(rig);
    commands.insert_resource(DynAssets { powered_glass });

    // ---- The beacon's fire (dark until it is lit).
    let mut flame = MeshBuilder::new();
    flame.lathe(
        Vec3::ZERO,
        &[
            (0.0, 0.0),
            (0.26, 0.0),
            (0.24, 0.25),
            (0.13, 0.65),
            (0.05, 0.95),
            (0.0, 1.2),
        ],
        10,
        1.0,
        [1.0, 0.5, 0.15, 1.0],
    );
    let flame_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.5, 0.14, 0.9),
        emissive: LinearRgba::rgb(9.0, 3.0, 0.5),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let beacon = commands
        .spawn((
            Name::new("beacon fire"),
            BeaconFlame,
            Mesh3d(meshes.add(flame.build())),
            MeshMaterial3d(flame_mat),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::from_translation(d.beacon - Vec3::Y * 0.25),
            Visibility::Hidden,
        ))
        .id();
    commands.spawn((
        BeaconLight,
        PointLight {
            color: Color::srgb(1.0, 0.55, 0.2),
            intensity: 0.0,
            range: 60.0,
            radius: 0.3,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::Y * 0.4),
        ChildOf(beacon),
    ));
}

/// Bundles rest where they lie, sit in the altar's arc once delivered and
/// vanish from the world while somebody carries them.
pub fn bundles(
    net: Res<Network>,
    layout: Res<LayoutRes>,
    mut views: Query<(&BundleView, &mut Transform, &mut Visibility)>,
) {
    let Some(s) = net.snapshot() else {
        for (_, _, mut v) in &mut views {
            *v = Visibility::Hidden;
        }
        return;
    };
    let l = &layout.0;
    let altar = l.ceiba.offering;
    let out = (ground(altar) - l.ceiba.center).normalize_or(Vec2::X);
    let side = Vec2::new(-out.y, out.x);
    for (view, mut tf, mut vis) in &mut views {
        let Some(r) = s.relics.get(view.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let yaw = view.0 as f32 * 2.399;
        match r.state {
            0 => {
                tf.translation = Vec3::from_array(r.pos);
                tf.rotation = Quat::from_rotation_y(yaw);
                *vis = Visibility::Inherited;
            }
            2 => {
                let slot = view.0 as f32 - 2.0;
                let at = ground(altar) + out * 0.75 + side * slot * 0.42;
                let y = l.surface_height(at) + 0.14;
                tf.translation = Vec3::new(at.x, y, at.y);
                tf.rotation = Quat::from_rotation_y(yaw);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

pub fn peppers(net: Res<Network>, mut views: Query<(&AjiView, &mut Visibility)>) {
    for (view, mut vis) in &mut views {
        let taken = net.snapshot().and_then(|s| s.aji.get(view.0).copied()).unwrap_or(true);
        *vis = if taken {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

pub fn wards(
    time: Res<Time>,
    net: Res<Network>,
    layout: Res<LayoutRes>,
    mut views: Query<(&WardView, &mut Transform, &mut Visibility)>,
    mut glows: Query<&mut Transform, (With<WardGlow>, Without<WardView>)>,
) {
    let zones = net.snapshot().map_or(&[][..], |s| s.zones.as_slice());
    for (view, mut tf, mut vis) in &mut views {
        match zones.get(view.0) {
            Some(z) => {
                let fade = (z[2] / 2.5).clamp(0.0, 1.0);
                let at = Vec2::new(z[0], z[1]);
                tf.translation = Vec3::new(at.x, layout.0.surface_height(at) + 0.02, at.y);
                tf.scale = Vec3::new(fade, 1.0, fade);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    // A slow pulse in the glow.
    let pulse = 0.94 + 0.06 * (time.elapsed_secs() * 3.0).sin();
    for mut g in &mut glows {
        g.scale = Vec3::new(pulse, 1.0, pulse);
    }
}

/// Marks hover over the spot and keep a readable size at any distance.
pub fn pings(
    time: Res<Time>,
    net: Res<Network>,
    camera: Single<&Transform, (With<Player>, Without<PingMarker>)>,
    mut views: Query<(&PingMarker, &mut Transform, &mut Visibility)>,
) {
    let list = net.snapshot().map_or(&[][..], |s| s.pings.as_slice());
    let slot_of = |by: PlayerId| {
        net.snapshot()
            .and_then(|s| s.players.iter().position(|p| p.id == by))
            .unwrap_or(0)
    };
    // One marker per player slot, so colours stay stable.
    for (view, mut tf, mut vis) in &mut views {
        let ping = list.iter().find(|p| slot_of(p.by) == view.0);
        match ping {
            Some(p) => {
                let at = Vec3::from_array(p.pos);
                let dist = at.distance(camera.translation);
                let bob = (time.elapsed_secs() * 2.4 + view.0 as f32).sin() * 0.08;
                tf.translation = at + Vec3::Y * (0.9 + bob);
                tf.scale = Vec3::splat((dist * 0.03).clamp(0.6, 4.0));
                tf.rotation = Quat::from_rotation_y(time.elapsed_secs() * 1.4);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// Light the nearest lamps; fade the others out, one pool for the whole map.
pub fn lamp_lights(
    time: Res<Time>,
    net: Res<Network>,
    rig: Res<LampRig>,
    camera: Single<&Transform, (With<Player>, Without<LampSlot>)>,
    mut slots: Query<(&mut LampSlot, &mut PointLight, &mut Transform)>,
    mut scratch: Local<LampScratch>,
) {
    let power = net.snapshot().is_some_and(|s| s.world.power >= 1.0);
    let eye = camera.translation;
    let LampScratch { desired, held, pending } = &mut *scratch;
    // The immutable target: the nearest lit lamps within reach.
    desired.clear();
    desired.extend((0..rig.lamps.len()).filter(|&i| {
        let (pos, powered) = rig.lamps[i];
        (!powered || power) && pos.distance_squared(eye) < 70.0 * 70.0
    }));
    desired.sort_by(|&a, &b| {
        rig.lamps[a]
            .0
            .distance_squared(eye)
            .total_cmp(&rig.lamps[b].0.distance_squared(eye))
    });
    desired.truncate(LAMP_POOL);
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    // Slots keep their lamp while it stays desired; faded strays free up.
    held.clear();
    for (mut slot, _, _) in &mut slots {
        if let Some(lamp) = slot.lamp {
            if desired.contains(&lamp) {
                held.push(lamp);
            } else if slot.level <= 0.01 {
                slot.lamp = None;
            }
        }
    }
    pending.clear();
    pending.extend(desired.iter().copied().filter(|l| !held.contains(l)));
    for (mut slot, mut light, mut tf) in &mut slots {
        if slot.lamp.is_none()
            && let Some(next) = pending.pop()
        {
            slot.lamp = Some(next);
            slot.level = 0.0;
        }
        let Some(lamp) = slot.lamp else {
            light.intensity = 0.0;
            continue;
        };
        let (pos, powered) = rig.lamps[lamp];
        // A lamp that has left the desired set fades out.
        let target = if desired.contains(&lamp) { 1.0 } else { 0.0 };
        slot.level += (target - slot.level) * (dt * 5.0).min(1.0);
        let flick = 1.0
            + 0.05 * (t * 7.3 + slot.phase).sin()
            + 0.03 * (t * 13.1 + slot.phase * 2.0).sin()
            + 0.04 * (t * 1.7 + slot.phase).sin();
        let (base, color, range) = if powered {
            (34_000.0, Color::srgb(1.0, 0.9, 0.72), 17.0)
        } else {
            (11_000.0, Color::srgb(1.0, 0.64, 0.3), 12.0)
        };
        light.color = color;
        light.range = range;
        light.intensity = base * slot.level * if powered { 1.0 } else { flick };
        tf.translation = pos - Vec3::Y * if powered { 0.1 } else { 0.0 };
    }
}

/// The pole lamps' bulbs glow only while the windmill's power holds.
pub fn power_look(
    net: Res<Network>,
    assets: Res<DynAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut last: Local<Option<bool>>,
) {
    let on = net.snapshot().is_some_and(|s| s.world.power >= 1.0);
    if *last == Some(on) {
        return;
    }
    *last = Some(on);
    if let Some(mut m) = materials.get_mut(&assets.powered_glass) {
        m.emissive = if on {
            LinearRgba::rgb(14.0, 12.0, 8.0)
        } else {
            LinearRgba::BLACK
        };
    }
}

/// The signal fire on the tower: lit while the beacon burns.
pub fn beacon_fire(
    time: Res<Time>,
    net: Res<Network>,
    mut flame: Query<(&mut Transform, &mut Visibility), With<BeaconFlame>>,
    mut light: Query<&mut PointLight, With<BeaconLight>>,
) {
    let burning = net.snapshot().is_some_and(|s| s.world.beacon > 0.0);
    let t = time.elapsed_secs();
    let f = 0.85 + 0.15 * (t * 11.0).sin() * (t * 4.3).cos();
    for (mut tf, mut vis) in &mut flame {
        *vis = if burning {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        tf.scale = Vec3::new(f, 1.0 + 0.2 * (t * 9.0).sin(), f) * 2.2;
    }
    for mut l in &mut light {
        l.intensity = if burning { 900_000.0 * f } else { 0.0 };
    }
}

/// Headlamps and taillamps come on with the engine, and the truck shivers.
pub fn truck_engine(
    time: Res<Time>,
    net: Res<Network>,
    assets: Res<TruckAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lamps: Query<(&TruckLamp, &mut SpotLight), Without<PointLight>>,
    mut tails: Query<(&TruckLamp, &mut PointLight), Without<SpotLight>>,
    mut roots: Query<(&TruckRoot, &mut Transform)>,
    mut last: Local<Option<bool>>,
) {
    let running = net
        .snapshot()
        .is_some_and(|s| s.world.truck >= 1.0 && !s.outcome().is_over());
    let warm = net.snapshot().map_or(0.0, |s| s.world.warm);
    if *last != Some(running) {
        *last = Some(running);
        if let Some(mut m) = materials.get_mut(&assets.headlamp) {
            m.emissive = if running {
                LinearRgba::rgb(14.0, 12.0, 8.0)
            } else {
                LinearRgba::BLACK
            };
        }
        if let Some(mut m) = materials.get_mut(&assets.taillamp) {
            m.emissive = if running {
                LinearRgba::rgb(5.0, 0.2, 0.1)
            } else {
                LinearRgba::BLACK
            };
        }
    }
    for (lamp, mut spot) in &mut lamps {
        spot.intensity = if running { lamp.base } else { 0.0 };
    }
    for (lamp, mut point) in &mut tails {
        point.intensity = if running { lamp.base } else { 0.0 };
    }
    let t = time.elapsed_secs();
    for (root, mut tf) in &mut roots {
        if running {
            // A rough idle that smooths out as the engine warms.
            let k = 1.0 - 0.6 * warm;
            tf.translation = root.rest + Vec3::Y * (t * 41.0).sin() * 0.006 * k;
            tf.rotation = Quat::from_euler(
                EulerRot::XYZ,
                (t * 33.0).sin() * 0.0025 * k,
                0.0,
                (t * 27.0 + 1.0).sin() * 0.003 * k,
            );
        } else {
            tf.translation = root.rest;
            tf.rotation = Quat::IDENTITY;
        }
    }
}

/// The bundle you carry rides low at your left hip, in view.
pub fn carried_view(net: Res<Network>, mut q: Query<&mut Visibility, With<CarriedSatchel>>) {
    let show = net.carrying() > 0 && net.status() == 0;
    for mut v in &mut q {
        *v = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
