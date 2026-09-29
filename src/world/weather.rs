//! The storm on screen: falling rain (one mesh, animated in a shader),
//! lightning (a flash across sky, fog, ambience and moonlight plus a jagged
//! bolt on the horizon) and the swell of the rain itself. Everything is a
//! function of the shared storm clock, so every player sees the same sky.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

use super::Palette;
use super::land::{HORIZON, MOON_COLOR, MOON_LUX, NIGHT_AMBIENT, NIGHT_AMBIENT_COLOR};
use super::mesh::{MeshBuilder, WHITE};
use crate::app::{LayoutRes, StormClock, TuningRes};
use crate::geometry::Layout;
use crate::player::Player;
use crate::rng::Rng;
use crate::storm;

/// The directional light that stands for the moon.
#[derive(Component)]
pub struct Moonlight;

#[derive(Component)]
pub struct RainCurtain;

#[derive(Component)]
pub struct Bolt;

/// The rain box: streak count and dimensions. A thin veil, not a curtain:
/// about 0.12 streaks per cubic metre, so a few hundred are ever in view and
/// most of those are faded by distance.
const STREAKS: usize = 1800;
const BOX: Vec3 = Vec3::new(30.0, 16.0, 30.0);
/// Streak length (m) and width (m); the shader jitters length by ±30 %.
const STREAK_LENGTH: f32 = 0.45;
const STREAK_WIDTH: f32 = 0.014;
/// Streak colour: cold blue-grey, linear.
const RAIN_COLOR: [f32; 3] = [0.30, 0.40, 0.58];
/// Streak opacity at the storm's calmest and at its heaviest.
const RAIN_ALPHA: (f32, f32) = (0.075, 0.125);
/// Roofed places the shader knows about (two `Vec4`s each).
const MAX_SHELTERS: usize = 16;
/// Roof margin: overhang not already counted in a shed's posts.
const SHED_OVERHANG: f32 = 0.3;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RainMaterial {
    #[uniform(0)]
    pub tint: LinearRgba,
    #[uniform(1)]
    pub volume: Vec4,
    #[uniform(2)]
    pub motion: Vec4,
    /// Distance drifted (xyz) and the number of roofed places (w).
    #[uniform(3)]
    pub flow: Vec4,
    #[uniform(4)]
    pub shelters: [Vec4; MAX_SHELTERS * 2],
}

impl Material for RainMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/rain.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/rain.wgsl".into()
    }

    /// Bevy's `Add` and `Premultiplied` share one blend state (`src + dst *
    /// (1 - src.a)`); the shader returns colour already multiplied by its
    /// coverage, so this is plain alpha blending of a streak over the scene.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// One mesh of camera-facing streaks; the shader does all the moving.
fn rain_mesh(seed: u64) -> Mesh {
    let mut rng = Rng::fork(seed, 0x8A17);
    let mut m = MeshBuilder::new();
    for _ in 0..STREAKS {
        let anchor = Vec3::new(rng.f32(), rng.f32(), rng.f32());
        let jitter = [rng.f32(), rng.f32(), rng.f32(), 1.0];
        let corners = [(-1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)];
        let [a, b, c, d] = corners.map(|(x, y)| m.vertex(anchor, Vec3::Y, Vec2::new(x, y), jitter));
        m.quad_idx(a, b, c, d);
    }
    m.build()
}

/// Every roofed place the rain must not fall into, straight from the layout
/// (the single geometry truth): the ranch house and its porch, each shed, and
/// the watchtower's cabin roof (`District::watch_shelter`, the same shed
/// `world/district.rs::watchtower` builds).
/// Packed as the shader reads them; returns the array and how many are used.
/// Panics if the layout has more roofs than the shader's fixed array holds, so
/// a shelter is never silently left out.
fn shelters(layout: &Layout) -> ([Vec4; MAX_SHELTERS * 2], usize) {
    let d = &layout.district;
    let needed = 2 + d.sheds.len() + 1;
    assert!(
        needed <= MAX_SHELTERS,
        "layout has {needed} roofed places but the rain shader holds {MAX_SHELTERS}; raise MAX_SHELTERS and the shader array"
    );
    let mut out = [Vec4::ZERO; MAX_SHELTERS * 2];
    let mut count = 0;
    let mut roof = |min: Vec2, max: Vec2, top: f32| {
        out[count * 2] = Vec4::new(min.x, min.y, max.x, max.y);
        out[count * 2 + 1] = Vec4::new(top, 0.0, 0.0, 0.0);
        count += 1;
    };
    let house = &layout.house;
    let over = Vec2::splat(house.overhang);
    roof(house.footprint.min - over, house.footprint.max + over, house.ridge);
    roof(house.porch.min, house.porch.max, house.porch_low);
    for shed in d.sheds.iter().copied().chain([d.watch_shelter()]) {
        let half = shed.half + Vec2::splat(SHED_OVERHANG);
        roof(shed.center - half, shed.center + half, shed.floor + shed.ridge);
    }
    debug_assert_eq!(count, needed);
    (out, count)
}

pub fn spawn_rain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RainMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    tuning: Res<TuningRes>,
    layout: Res<LayoutRes>,
) {
    let (roofs, count) = shelters(&layout.0);
    let material = materials.add(RainMaterial {
        tint: LinearRgba::new(RAIN_COLOR[0], RAIN_COLOR[1], RAIN_COLOR[2], RAIN_ALPHA.0),
        volume: Vec4::new(BOX.x, BOX.y, BOX.z, 10.0),
        motion: Vec4::new(-1.6, 0.5, STREAK_LENGTH, STREAK_WIDTH),
        flow: Vec4::new(0.0, 0.0, 0.0, count as f32),
        shelters: roofs,
    });
    commands.spawn((
        Name::new("rain"),
        RainCurtain,
        Mesh3d(meshes.add(rain_mesh(tuning.0.seed))),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        NoFrustumCulling,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    // The bolt: a jagged ribbon far out on the llano, redrawn per strike. It
    // starts as a real (hidden) bolt, so its mesh is never empty or a speck.
    let bolt = standard.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::BLACK,
        unlit: true,
        fog_enabled: false,
        alpha_mode: AlphaMode::Add,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Name::new("lightning bolt"),
        Bolt,
        Mesh3d(meshes.add(bolt_mesh(tuning.0.seed, 0.0))),
        MeshMaterial3d(bolt),
        Transform::IDENTITY,
        NoFrustumCulling,
        NotShadowCaster,
        NotShadowReceiver,
        Visibility::Hidden,
    ));
}

/// What the rain last showed, and how far it has drifted.
#[derive(Default)]
pub struct RainState {
    /// Total drift of the rain since the storm began, metres (f64: a long
    /// session must not lose the fraction of a metre that makes streaks crawl).
    drift: [f64; 3],
    /// The `(level, flash)` last written to the material.
    shown: Option<[f32; 2]>,
}

/// Rain follows the shared storm clock: it swells and eases with the storm,
/// pales under the veil of a lightning flash, and falls by the distance the
/// storm's own time has carried it (so pausing a solo run holds every drop,
/// while a shared menu does not).
pub fn rain_swell(
    clock: Res<StormClock>,
    tuning: Res<TuningRes>,
    mut materials: ResMut<Assets<RainMaterial>>,
    handles: Query<&MeshMaterial3d<RainMaterial>, With<RainCurtain>>,
    mut state: Local<RainState>,
) {
    let seed = tuning.0.seed;
    let level = storm::rain(seed, clock.t);
    let flash = storm::flash(seed, clock.t);
    // A jump of the clock (a photo, a restart) moves the sky, not the drops.
    let dt = clock.t - clock.prev;
    let step = if dt > 0.0 && dt <= 0.5 { f64::from(dt) } else { 0.0 };
    if step == 0.0 && state.shown == Some([level, flash]) {
        return;
    }
    let swell = ((level - 0.6) / 0.4).clamp(0.0, 1.0);
    let speed = 9.0 + 2.5 * level;
    let wind = Vec2::new(-1.0 - 1.4 * level, 0.5);
    state.drift[0] += f64::from(wind.x) * step;
    state.drift[1] -= f64::from(speed) * step;
    state.drift[2] += f64::from(wind.y) * step;
    state.shown = Some([level, flash]);
    // Lightning lifts the streaks' colour but thins them: they stay a cue
    // against a suddenly bright sky, never a bar over the scene.
    let lift = 1.0 + 1.5 * flash;
    let alpha = (RAIN_ALPHA.0 + (RAIN_ALPHA.1 - RAIN_ALPHA.0) * swell) * (1.0 - 0.5 * flash);
    let drift = state.drift;
    for h in &handles {
        if let Some(mut m) = materials.get_mut(&h.0) {
            m.tint = LinearRgba::new(RAIN_COLOR[0] * lift, RAIN_COLOR[1] * lift, RAIN_COLOR[2] * lift, alpha);
            m.volume.w = speed;
            m.motion.x = wind.x;
            m.motion.y = wind.y;
            m.flow.x = drift[0] as f32;
            m.flow.y = drift[1] as f32;
            m.flow.z = drift[2] as f32;
        }
    }
}

/// A jagged channel from the clouds to the far ground, with two forks. The
/// shape is a pure function of the seed and the strike's exact onset time, so
/// every player sees the same bolt.
fn bolt_mesh(seed: u64, at: f32) -> Mesh {
    let mut rng = Rng::fork(seed, u64::from(at.to_bits()));
    let azimuth = rng.range(0.0, std::f32::consts::TAU);
    let dist = rng.range(260.0, 520.0);
    let base = Vec3::new(azimuth.cos() * dist, 0.0, azimuth.sin() * dist);
    let top_h = rng.range(230.0, 330.0);
    let mut pts = Vec::new();
    let mut p = base + Vec3::new(rng.signed(40.0), top_h, rng.signed(40.0));
    let steps = 16;
    for i in 0..=steps {
        pts.push(p);
        let f = 1.0 - (i as f32 / steps as f32);
        p = Vec3::new(
            p.x + rng.signed(14.0) * (0.4 + f),
            p.y - top_h / steps as f32,
            p.z + rng.signed(14.0) * (0.4 + f),
        );
        if i == steps - 1 {
            p = Vec3::new(base.x, 0.0, base.z);
        }
    }
    let side = Vec3::new(-azimuth.sin(), 0.0, azimuth.cos());
    let mut m = MeshBuilder::new();
    let widths: Vec<f32> = (0..pts.len())
        .map(|i| 3.2 - 1.6 * (i as f32 / pts.len() as f32))
        .collect();
    m.ribbon(&pts, &widths, side, &[WHITE]);
    for _ in 0..2 {
        let from = pts[rng.below(pts.len() - 4) + 2];
        let mut q = from;
        let mut fork = vec![q];
        let dir = Vec3::new(rng.signed(1.0), -0.55, rng.signed(1.0)).normalize();
        for _ in 0..6 {
            q += dir * rng.range(9.0, 18.0) + Vec3::new(rng.signed(6.0), 0.0, rng.signed(6.0));
            fork.push(q);
        }
        let w: Vec<f32> = (0..fork.len())
            .map(|i| 1.6 - 1.2 * (i as f32 / fork.len() as f32))
            .collect();
        m.ribbon(&fork, &w, side, &[WHITE]);
    }
    m.build()
}

/// Flash the whole night: sky, fog, ambience, moonlight, bloom and the bolt.
#[allow(clippy::too_many_arguments)]
pub fn lightning(
    clock: Res<StormClock>,
    tuning: Res<TuningRes>,
    palette: Res<Palette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut moon: Single<&mut DirectionalLight, With<Moonlight>>,
    camera: Single<(&mut Bloom, &mut DistanceFog), With<Player>>,
    bolt: Single<(&Mesh3d, &MeshMaterial3d<StandardMaterial>, &mut Visibility), With<Bolt>>,
    mut drawn: Local<Option<f32>>,
    mut built: Local<Option<u32>>,
) {
    let seed = tuning.0.seed;
    let f = storm::flash(seed, clock.t);
    let (mut bloom, mut fog) = camera.into_inner();
    let [mr, mg, mb] = MOON_COLOR;
    moon.illuminance = MOON_LUX + 21_000.0 * f;
    moon.color = Color::srgb(mr + 0.3 * f, mg + 0.2 * f, mb);
    let [ar, ag, ab] = NIGHT_AMBIENT_COLOR;
    ambient.brightness = NIGHT_AMBIENT + 1300.0 * f;
    ambient.color = Color::srgb(ar + 0.3 * f, ag + 0.25 * f, ab);
    bloom.intensity = crate::player::BLOOM + 0.16 * f;
    let k = 1.0 + 55.0 * f;
    fog.color = Color::linear_rgba(HORIZON[0] * k, HORIZON[1] * k, HORIZON[2] * k * 1.1, 1.0);
    let (mesh, material, mut visibility) = bolt.into_inner();
    // The bolt is redrawn once per strike, whenever its flash is on screen
    // (so a jump in the clock never leaves the last strike's channel up).
    if let Some(strike) = storm::active_strike(seed, clock.t) {
        let key = strike.at.to_bits();
        if *built != Some(key) {
            let _ = meshes.insert(&mesh.0, bolt_mesh(seed, strike.at));
            *built = Some(key);
        }
    }
    let shown = if f > 0.05 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != shown {
        *visibility = shown;
    }
    // The sky and bolt materials only change with the flash itself.
    if *drawn != Some(f) {
        *drawn = Some(f);
        if let Some(mut sky) = materials.get_mut(&palette.sky) {
            sky.base_color = Color::linear_rgb(1.0 + 34.0 * f, 1.0 + 44.0 * f, 1.0 + 70.0 * f);
        }
        if let Some(mut m) = materials.get_mut(&material.0) {
            let e = 6.0 * f;
            m.emissive = LinearRgba::rgb(e * 10.0, e * 11.0, e * 15.0);
        }
    }
}
