//! Presentation of the authored layout: procedural llano, fences, weathered
//! house, the ceiba, props, sky and the Silbón model. Everything is placed
//! from [`crate::geometry::Layout`], so what you see is what collides.

pub mod ceiba;
pub mod house;
pub mod land;
pub mod mesh;
pub mod props;
pub mod silbon;
pub mod texture;

use bevy::prelude::*;

use crate::app::{LayoutRes, TuningRes};
use crate::rng::Rng;

/// Every material of the world, created once.
#[derive(Resource, Clone)]
pub struct Palette {
    pub ground: Handle<StandardMaterial>,
    pub road: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>,
    pub wood_dark: Handle<StandardMaterial>,
    pub zinc: Handle<StandardMaterial>,
    pub bark: Handle<StandardMaterial>,
    pub leaves: Handle<StandardMaterial>,
    pub fronds: Handle<StandardMaterial>,
    pub grass: Handle<StandardMaterial>,
    pub burlap: Handle<StandardMaterial>,
    pub hammock: Handle<StandardMaterial>,
    pub bone: Handle<StandardMaterial>,
    pub cloth: Handle<StandardMaterial>,
    /// The Silbón's pale, moon-catching shirt.
    pub shirt: Handle<StandardMaterial>,
    pub skin: Handle<StandardMaterial>,
    pub straw: Handle<StandardMaterial>,
    pub clay: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub tin: Handle<StandardMaterial>,
    pub rust_metal: Handle<StandardMaterial>,
    pub lamp_glass: Handle<StandardMaterial>,
    pub paper_note: Handle<StandardMaterial>,
    pub calendar: Handle<StandardMaterial>,
    pub sign: Handle<StandardMaterial>,
    pub wire: Handle<StandardMaterial>,
    pub soil: Handle<StandardMaterial>,
    pub stone: Handle<StandardMaterial>,
    pub leather: Handle<StandardMaterial>,
    pub sky: Handle<StandardMaterial>,
    pub moon: Handle<StandardMaterial>,
    pub halo: Handle<StandardMaterial>,
    pub stars: Handle<StandardMaterial>,
}

/// Satchel on the table (visible until taken).
#[derive(Component)]
pub struct TableSatchel;

/// Satchel resting in the ceiba's hollow (visible once returned).
#[derive(Component)]
pub struct TreeSatchel;

/// Satchel carried in the lower left of the view.
#[derive(Component)]
pub struct CarriedSatchel;

/// Shared satchel mesh for the three satchel views.
#[derive(Resource, Clone)]
pub struct SatchelAsset {
    pub sack: Handle<Mesh>,
    pub bones: Handle<Mesh>,
}

/// Warm light that breathes gently like a kerosene flame.
#[derive(Component)]
pub struct Flicker {
    pub base: f32,
    pub phase: f32,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world).add_systems(
            Update,
            (flicker_lights, silbon::animate_silbon).in_set(crate::app::GameSet::Present),
        );
    }
}

pub fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
) {
    let layout = &layout.0;
    let seed = tuning.0.seed;
    let tex = texture::generate(&mut images, seed);
    let palette = make_palette(&mut materials, &tex);

    let mut ctx = SpawnCtx {
        commands: &mut commands,
        meshes: &mut meshes,
        palette: &palette,
        layout,
        seed,
    };
    land::spawn(&mut ctx);
    house::spawn(&mut ctx);
    ceiba::spawn(&mut ctx);
    let satchel = props::spawn(&mut ctx);
    silbon::spawn(&mut ctx);

    commands.insert_resource(satchel);
    commands.insert_resource(palette);
}

/// Everything a builder needs to put meshes into the world.
pub struct SpawnCtx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub palette: &'a Palette,
    pub layout: &'a crate::geometry::Layout,
    pub seed: u64,
}

impl SpawnCtx<'_, '_, '_> {
    /// Spawn a static merged mesh with a material.
    pub fn static_mesh(&mut self, name: &'static str, mesh: mesh::MeshBuilder, material: &Handle<StandardMaterial>) {
        if mesh.is_empty() {
            return;
        }
        let handle = self.meshes.add(mesh.build());
        self.commands.spawn((
            Name::new(name),
            Mesh3d(handle),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
        ));
    }

    pub fn rng(&self, stream: u64) -> Rng {
        Rng::fork(self.seed, stream)
    }
}

fn make_palette(materials: &mut Assets<StandardMaterial>, tex: &texture::Textures) -> Palette {
    let textured = |materials: &mut Assets<StandardMaterial>, t: &Handle<Image>, rough: f32| {
        materials.add(StandardMaterial {
            base_color_texture: Some(t.clone()),
            perceptual_roughness: rough,
            reflectance: 0.3,
            ..default()
        })
    };
    let plain = |materials: &mut Assets<StandardMaterial>, c: Color, rough: f32| {
        materials.add(StandardMaterial {
            base_color: c,
            perceptual_roughness: rough,
            reflectance: 0.3,
            ..default()
        })
    };
    let two_sided = |materials: &mut Assets<StandardMaterial>, t: Option<&Handle<Image>>, c: Color, rough: f32| {
        materials.add(StandardMaterial {
            base_color: c,
            base_color_texture: t.cloned(),
            perceptual_roughness: rough,
            reflectance: 0.25,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let unlit = |materials: &mut Assets<StandardMaterial>, c: Color, alpha: AlphaMode, t: Option<&Handle<Image>>| {
        materials.add(StandardMaterial {
            base_color: c,
            base_color_texture: t.cloned(),
            unlit: true,
            fog_enabled: false,
            alpha_mode: alpha,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };

    Palette {
        ground: textured(materials, &tex.ground, 0.96),
        road: textured(materials, &tex.road, 0.92),
        wood: textured(materials, &tex.wood, 0.9),
        wood_dark: textured(materials, &tex.wood_dark, 0.78),
        zinc: materials.add(StandardMaterial {
            base_color_texture: Some(tex.zinc.clone()),
            perceptual_roughness: 0.62,
            metallic: 0.35,
            reflectance: 0.4,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        bark: textured(materials, &tex.bark, 0.88),
        leaves: textured(materials, &tex.leaves, 0.85),
        fronds: two_sided(materials, Some(&tex.leaves), Color::srgb(0.8, 0.9, 0.75), 0.8),
        grass: two_sided(materials, None, Color::WHITE, 0.9),
        burlap: textured(materials, &tex.burlap, 0.95),
        hammock: two_sided(materials, Some(&tex.burlap), Color::WHITE, 0.95),
        bone: plain(materials, Color::srgb(0.8, 0.76, 0.64), 0.7),
        cloth: materials.add(StandardMaterial {
            base_color_texture: Some(tex.cloth.clone()),
            perceptual_roughness: 0.97,
            reflectance: 0.2,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        shirt: materials.add(StandardMaterial {
            base_color_texture: Some(tex.cloth_pale.clone()),
            perceptual_roughness: 0.9,
            reflectance: 0.35,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        skin: plain(materials, Color::srgb(0.46, 0.45, 0.43), 0.75),
        straw: two_sided(materials, Some(&tex.straw), Color::WHITE, 0.92),
        clay: textured(materials, &tex.clay, 0.85),
        glass: materials.add(StandardMaterial {
            base_color: Color::srgba(0.55, 0.62, 0.58, 0.35),
            perceptual_roughness: 0.12,
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        tin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.3, 0.27),
            perceptual_roughness: 0.55,
            metallic: 0.7,
            ..default()
        }),
        rust_metal: materials.add(StandardMaterial {
            base_color_texture: Some(tex.zinc.clone()),
            base_color: Color::srgb(0.75, 0.55, 0.45),
            perceptual_roughness: 0.75,
            metallic: 0.3,
            ..default()
        }),
        lamp_glass: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.55),
            emissive: LinearRgba::rgb(9.0, 4.2, 1.2),
            perceptual_roughness: 0.2,
            ..default()
        }),
        paper_note: textured(materials, &tex.paper_note, 0.95),
        calendar: textured(materials, &tex.calendar, 0.95),
        sign: textured(materials, &tex.sign, 0.9),
        wire: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.19, 0.18),
            perceptual_roughness: 0.5,
            metallic: 0.8,
            ..default()
        }),
        soil: plain(materials, Color::srgb(0.36, 0.27, 0.19), 0.97),
        stone: plain(materials, Color::srgb(0.42, 0.41, 0.38), 0.9),
        leather: plain(materials, Color::srgb(0.34, 0.22, 0.13), 0.7),
        sky: unlit(materials, Color::WHITE, AlphaMode::Opaque, None),
        moon: unlit(materials, Color::srgb(1.0, 0.98, 0.9), AlphaMode::Opaque, None),
        halo: unlit(
            materials,
            Color::srgba(0.7, 0.78, 1.0, 0.55),
            AlphaMode::Add,
            Some(&tex.halo),
        ),
        stars: unlit(materials, Color::WHITE, AlphaMode::Opaque, None),
    }
}

fn flicker_lights(time: Res<Time>, mut lights: Query<(&Flicker, &mut PointLight)>) {
    let t = time.elapsed_secs();
    for (f, mut light) in &mut lights {
        let w = (t * 7.3 + f.phase).sin() * 0.04
            + (t * 13.1 + f.phase * 2.0).sin() * 0.025
            + (t * 1.7 + f.phase).sin() * 0.03;
        let target = f.base * (1.0 + w);
        if (light.intensity - target).abs() > f.base * 0.002 {
            light.intensity = target;
        }
    }
}

// ----------------------------------------------------------------------------
// Shared world-space helpers for builders
// ----------------------------------------------------------------------------

fn hash(ix: i32, iz: i32, seed: u32) -> f32 {
    let mut h =
        (ix as u32).wrapping_mul(0x27D4_EB2D) ^ (iz as u32).wrapping_mul(0x1656_67B1) ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Smooth world-space value noise in 0..1.
pub fn noise2(x: f32, z: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let fx = x - xi as f32;
    let fz = z - zi as f32;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sz) = (s(fx), s(fz));
    let a = hash(xi, zi, seed);
    let b = hash(xi + 1, zi, seed);
    let c = hash(xi, zi + 1, seed);
    let d = hash(xi + 1, zi + 1, seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sz
}

/// Two-octave world noise.
pub fn fbm2(x: f32, z: f32, seed: u32) -> f32 {
    noise2(x, z, seed) * 0.65 + noise2(x * 2.1, z * 2.1, seed ^ 0x5151) * 0.35
}
