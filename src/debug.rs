//! Debug support, clearly separated from play:
//! - F12 saves a game-window screenshot (any mode).
//! - `--smoke` drives the real game with the deterministic route script
//!   (`script.rs`) through the same intent → motion → targeting → truth path
//!   as a player, saves screenshots at fixed beats, checks that restarts do
//!   not accumulate entities, logs a summary and exits (code 0 = route passed).
//! - `--photos` places the camera at searched, unobstructed viewpoints (see
//!   `photos.rs`), captures the world, the real map/note/downed UI and
//!   time-separated pairs, labels every frame with what the driver altered
//!   (presentation review, never gameplay proof) and writes a manifest. It
//!   exits non-zero, listing what is missing, if a capture never completes.
//!
//! Screenshots are honest: the route only starts once the renderer reports no
//! pipelines left to compile (plus a real-time warm-up), and every capture
//! waits, with simulated time frozen and the script held, until the pipelines
//! for what is on screen are ready and the image has been written.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::render_resource::PipelineCache;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::text::FontSource;
use bevy::window::PrimaryWindow;

use crate::app::{Flow, GameSet, Launch, LayoutRes, Truth, TuningRes};
use crate::audio::AmbienceLoop;
use crate::encounter::CurrentTarget;
use crate::net::{NetControl, Network};
use crate::player::{CurrentIntent, Player};
use crate::script::{Observation, RouteScript};
use crate::world::silbon::SilbonRoot;

/// Consecutive frames with no pipeline waiting before the route may start.
const READY_IDLE_FRAMES: u32 = 30;
/// Real-time warm-up before the route may start.
const READY_MIN_WALL: Duration = Duration::from_secs(2);
/// Idle frames, counted after the view has frozen for a capture, before the
/// screenshot is taken (covers the render world running a frame behind and
/// pipelines first queued for the newly framed view).
const CAPTURE_IDLE_FRAMES: u32 = 6;
/// Give up waiting for the renderer after this long (real time).
const READY_TIMEOUT: Duration = Duration::from_secs(180);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShotCounter>().add_systems(Update, f12_screenshot);
        let launch = app.world().get_resource::<Launch>().cloned();
        let smoke = launch.as_ref().is_some_and(|l| l.smoke);
        let photos = launch.as_ref().is_some_and(|l| l.photos);
        let menu_shots = launch.as_ref().is_some_and(|l| l.menu_shots);
        let trailer = launch.as_ref().is_some_and(|l| l.trailer);
        if smoke || photos || menu_shots {
            // The render world reports how many pipelines are still compiling.
            let probe = PipelineProbe(Arc::new(AtomicUsize::new(usize::MAX)));
            if let Some(render) = app.get_sub_app_mut(RenderApp) {
                render
                    .insert_resource(probe.clone())
                    .add_systems(Render, probe_pipelines.in_set(RenderSystems::Cleanup));
            }
            app.insert_resource(probe);
        }
        if smoke {
            let script = {
                let world = app.world();
                let layout = &world.resource::<LayoutRes>().0;
                let tuning = &world.resource::<TuningRes>().0;
                if world.resource::<Launch>().tour {
                    RouteScript::tour(layout, tuning)
                } else {
                    RouteScript::full(layout, tuning)
                }
            };
            app.insert_resource(Smoke {
                script,
                frames: 0,
                result: None,
                exit_in: None,
                baseline: None,
                census_in: None,
                restarts_checked: 0,
                shots: Vec::new(),
                started: Instant::now(),
                idle_frames: 0,
                ready: false,
                pending: None,
                frame_at: Instant::now(),
                frame_ms: Vec::new(),
            })
            .add_systems(
                Update,
                (
                    smoke_drive.in_set(GameSet::Control).after(crate::player::reset_player),
                    smoke_census.in_set(GameSet::Present),
                    smoke_exit.in_set(GameSet::Present).after(smoke_census),
                    smoke_overview.in_set(GameSet::Present),
                ),
            );
        }
        if menu_shots {
            app.insert_resource(MenuShots {
                step: 0,
                frames: 0,
                idle: 0,
                started: Instant::now(),
                taken: Vec::new(),
                exit_in: None,
            })
            .add_systems(Update, menu_shots_drive.in_set(GameSet::Control));
        }
        if trailer {
            app.add_systems(PostStartup, crate::trailer::setup).add_systems(
                Update,
                crate::trailer::drive
                    .in_set(GameSet::Weather)
                    .after(crate::app::advance_storm),
            );
        }
        if photos && !trailer {
            let (mut shots, calm, requested) = {
                let world = app.world();
                let seed = world.resource::<TuningRes>().0.seed;
                (
                    crate::photos::shots(&world.resource::<LayoutRes>().0, &world.resource::<TuningRes>().0),
                    calm_time(seed),
                    world.resource::<Launch>().size,
                )
            };
            // DEBUG: `PHOTOS_ONLY=lunge` keeps only the frames whose names
            // contain it (quick iteration on a few views).
            if let Ok(only) = std::env::var("PHOTOS_ONLY") {
                shots.retain(|s| s.name.contains(&only));
            }
            let play_fog = crate::world::land::fog_visibility(&app.world().resource::<LayoutRes>().0);
            for shot in shots.iter_mut().filter(|s| widens_fog(&s.name)) {
                // Written on the image and in the manifest's `altered` column.
                shot.label.push_str(&format!(
                    "; FOG RANGE WIDENED to {OVERVIEW_FOG_VISIBILITY:.0} m visibility (play: {play_fog:.0} m) so the \
                     150 m-high map view is not all fog; topology only, not ground readability"
                ));
            }
            app.insert_resource(Photos {
                shots,
                calm,
                index: 0,
                take: 0,
                settle: 0,
                idle: 0,
                frames: 0,
                started: Instant::now(),
                view_since: Instant::now(),
                saving: None,
                last_spawn_sim: 0.0,
                taken: Vec::new(),
                exit_in: None,
                exited: false,
                failure: None,
                requested,
            })
            .add_systems(Startup, spawn_photo_caption)
            .add_systems(
                Update,
                (
                    photo_drive.in_set(GameSet::Weather).after(crate::app::advance_storm),
                    photo_caption.in_set(GameSet::Present),
                ),
            );
        }
    }
}

#[derive(Resource, Default)]
struct ShotCounter(u32);

/// Pipelines still waiting to compile in the latest rendered frame
/// (`usize::MAX` until the renderer has reported once).
#[derive(Resource, Clone)]
pub(crate) struct PipelineProbe(pub(crate) Arc<AtomicUsize>);

fn probe_pipelines(cache: Res<PipelineCache>, probe: Res<PipelineProbe>) {
    probe.0.store(cache.waiting_pipelines().count(), Ordering::Relaxed);
}

fn capture(commands: &mut Commands, path: PathBuf) {
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        warn!("cannot create screenshot folder {}: {e}", dir.display());
        return;
    }
    info!("screenshot → {}", path.display());
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn f12_screenshot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    launch: Res<Launch>,
    mut counter: ResMut<ShotCounter>,
) {
    if keys.just_pressed(KeyCode::F12) {
        counter.0 += 1;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = launch.shots_dir.join(format!("el_silbon_{stamp}_{:03}.png", counter.0));
        capture(&mut commands, path);
    }
}

/// Entity counts that must not change across restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Census {
    cameras: usize,
    directional: usize,
    spots: usize,
    points: usize,
    ambience: usize,
    silbon: usize,
    meshes: usize,
    ui_nodes: usize,
}

/// A screenshot the route asked for, waiting for the renderer.
struct PendingShot {
    path: PathBuf,
    spawned: bool,
    since: Instant,
    /// Idle frames seen since this capture was requested.
    idle: u32,
}

#[derive(Resource)]
struct Smoke {
    script: RouteScript,
    frames: u64,
    result: Option<Result<(), String>>,
    exit_in: Option<u32>,
    baseline: Option<Census>,
    /// Frames until the next census comparison.
    census_in: Option<u32>,
    restarts_checked: u32,
    shots: Vec<PathBuf>,
    started: Instant,
    /// Consecutive frames with no pipeline compiling and all fonts loaded.
    idle_frames: u32,
    /// The renderer finished warming up; the route is running.
    ready: bool,
    pending: Option<PendingShot>,
    frame_at: Instant,
    frame_ms: Vec<f64>,
}

fn smoke_drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    state: Res<State<Flow>>,
    mut next: ResMut<NextState<Flow>>,
    mut smoke: ResMut<Smoke>,
    truth: Res<Truth>,
    target: Res<CurrentTarget>,
    player: Single<&Player>,
    launch: Res<Launch>,
    (layout, tuning, net, light): (
        Res<LayoutRes>,
        Res<TuningRes>,
        Res<Network>,
        Res<crate::player::LightOn>,
    ),
    (probe, models): (Res<PipelineProbe>, Res<crate::world::models::ModelsPending>),
    (fonts, text_fonts): (Res<Assets<Font>>, Query<&TextFont>),
    screenshots: Query<(), With<Screenshot>>,
    mut intent: ResMut<CurrentIntent>,
    mut control: MessageWriter<NetControl>,
) {
    // Plain `&mut Smoke` so the pending shot and counters borrow disjointly.
    let smoke = &mut *smoke;
    let now = Instant::now();
    let wall_ms = now.duration_since(smoke.frame_at).as_secs_f64() * 1000.0;
    smoke.frame_at = now;
    if smoke.ready && smoke.pending.is_none() && smoke.result.is_none() {
        smoke.frame_ms.push(wall_ms);
    }
    smoke.frames += 1;
    let fonts_loaded = text_fonts.iter().all(|t| match &t.font {
        FontSource::Handle(h) => fonts.contains(h.id()),
        _ => true,
    });
    if probe.0.load(Ordering::Relaxed) == 0 && fonts_loaded && models.settled() {
        smoke.idle_frames += 1;
    } else {
        smoke.idle_frames = 0;
    }
    if smoke.result.is_some() {
        intent.0 = Default::default();
        return;
    }

    // Warm-up: enter play with simulated time frozen until the renderer has
    // compiled everything it needs for the opening view.
    if !smoke.ready {
        intent.0 = Default::default();
        if *state.get() == Flow::Briefing {
            info!("smoke: skipping the briefing; waiting for the renderer");
            next.set(Flow::Playing);
        }
        if !virtual_time.is_paused() {
            virtual_time.pause();
        }
        let waited = smoke.started.elapsed();
        if *state.get() == Flow::Playing
            && smoke.idle_frames >= READY_IDLE_FRAMES
            && waited >= READY_MIN_WALL
            && smoke.frames >= 60
        {
            smoke.ready = true;
            virtual_time.unpause();
            info!(
                "smoke: renderer ready after {} frames, {:.1}s; route starts",
                smoke.frames,
                waited.as_secs_f32()
            );
        } else if waited > READY_TIMEOUT {
            smoke.result = Some(Err(format!(
                "renderer never became ready ({} pipelines waiting, fonts loaded: {fonts_loaded})",
                probe.0.load(Ordering::Relaxed)
            )));
            smoke.exit_in = Some(45);
        }
        return;
    }

    // A requested screenshot holds the route and simulated time until the
    // frame is fully rendered and saved.
    if let Some(shot) = smoke.pending.as_mut() {
        intent.0 = Default::default();
        if !shot.spawned {
            // Count idle frames only after the request, so pipelines queued
            // for the new view are waited for, not the old view's idleness.
            shot.idle = if smoke.idle_frames > 0 { shot.idle + 1 } else { 0 };
            let timed_out = shot.since.elapsed() > READY_TIMEOUT;
            if shot.idle >= CAPTURE_IDLE_FRAMES || timed_out {
                if timed_out {
                    warn!(
                        "smoke: capturing {} while pipelines are still compiling",
                        shot.path.display()
                    );
                }
                capture(&mut commands, shot.path.clone());
                shot.spawned = true;
            }
        } else if screenshots.is_empty() {
            smoke.pending = None;
            virtual_time.unpause();
        }
        return;
    }

    let obs = Observation {
        layout: &layout.0,
        tuning: &tuning.0,
        me: net.id().unwrap_or(crate::net::protocol::HOST),
        pose: player.pose,
        encounter: &truth.encounter,
        snapshot: net.snapshot(),
        target: target.0,
        dt: time.delta_secs(),
    };
    let frame = smoke.script.tick(&obs);
    intent.0 = frame.intent;
    // The route wants the torch off while it hides: the same key a player presses.
    intent.0.toggle_flashlight = frame.dark == light.0;
    if let Some(msg) = &frame.log {
        info!("smoke: {msg} (t={:.1}s)", smoke.script.elapsed());
    }
    if let Some(name) = frame.capture {
        let path = launch.shots_dir.join("smoke").join(format!("{name}.png"));
        smoke.shots.push(path.clone());
        smoke.pending = Some(PendingShot {
            path,
            spawned: false,
            since: Instant::now(),
            idle: 0,
        });
        virtual_time.pause();
    }
    // The route's explicit session command (restart, start, mark) reaches
    // the endpoint after this frame's input, as a key press does.
    if let Some(action) = frame.command() {
        control.write(NetControl::Action(action));
    }
    if frame.restart {
        smoke.census_in = Some(20);
    }
    if smoke.script.elapsed() > 20.0 * 60.0 {
        smoke.result = Some(Err(format!(
            "route exceeded 20 minutes at {}",
            smoke.script.step_name()
        )));
    }
    if let Some(done) = frame.finished {
        let done = done.map_err(|e| format!("{e} (step {})", smoke.script.step_name()));
        smoke.result = Some(done);
    }
    if smoke.result.is_some() {
        smoke.exit_in = Some(45);
    }
}

fn smoke_census(
    mut smoke: ResMut<Smoke>,
    state: Res<State<Flow>>,
    cameras: Query<(), With<Camera3d>>,
    directional: Query<(), With<DirectionalLight>>,
    spots: Query<(), With<SpotLight>>,
    points: Query<(), With<PointLight>>,
    ambience: Query<(), With<AmbienceLoop>>,
    silbon: Query<(), With<SilbonRoot>>,
    meshes: Query<(), With<Mesh3d>>,
    nodes: Query<(), With<Node>>,
) {
    let census = Census {
        cameras: cameras.iter().count(),
        directional: directional.iter().count(),
        spots: spots.iter().count(),
        points: points.iter().count(),
        ambience: ambience.iter().count(),
        silbon: silbon.iter().count(),
        meshes: meshes.iter().count(),
        ui_nodes: nodes.iter().count(),
    };
    if smoke.baseline.is_none() && smoke.ready && *state.get() == Flow::Playing {
        info!("smoke: baseline census {census:?}");
        smoke.baseline = Some(census);
    }
    if let Some(n) = smoke.census_in {
        if n > 0 {
            smoke.census_in = Some(n - 1);
            return;
        }
        smoke.census_in = None;
        smoke.restarts_checked += 1;
        match smoke.baseline {
            Some(base) if base == census => {
                info!(
                    "smoke: census after restart #{} unchanged {census:?}",
                    smoke.restarts_checked
                );
            }
            Some(base) => {
                let msg = format!("entities accumulated across restart: before {base:?}, after {census:?}");
                error!("smoke: {msg}");
                if smoke.result.is_none() || smoke.result == Some(Ok(())) {
                    smoke.result = Some(Err(msg));
                    smoke.exit_in = Some(45);
                }
            }
            None => {}
        }
    }
}

fn smoke_exit(
    mut smoke: ResMut<Smoke>,
    screenshots: Query<(), With<Screenshot>>,
    truth: Res<Truth>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(n) = smoke.exit_in else {
        return;
    };
    // Let the last restart census and pending screenshots finish first.
    if n > 0 || !screenshots.is_empty() || smoke.census_in.is_some() || smoke.pending.is_some() {
        smoke.exit_in = Some(n.saturating_sub(1));
        return;
    }
    let s = truth.encounter.stats;
    if !smoke.frame_ms.is_empty() {
        smoke.frame_ms.sort_by(f64::total_cmp);
        let n = smoke.frame_ms.len();
        let mean = smoke.frame_ms.iter().sum::<f64>() / n as f64;
        info!(
            "SMOKE FRAME TIMING (wall update intervals, no-vsync, excludes warmup/capture): n={} mean={:.3}ms p50={:.3}ms p95={:.3}ms",
            n,
            mean,
            smoke.frame_ms[n / 2],
            smoke.frame_ms[(n * 95 / 100).min(n - 1)]
        );
    }
    match &smoke.result {
        Some(Ok(())) => {
            info!(
                "SMOKE PASS: route completed in {:.1}s simulated ({} frames); restarts checked {}; screenshots:",
                smoke.script.elapsed(),
                smoke.frames,
                smoke.restarts_checked
            );
            for p in &smoke.shots {
                info!("  {}", p.display());
            }
            exit.write(AppExit::Success);
        }
        Some(Err(e)) => {
            error!("SMOKE FAIL: {e}; last stats {s:?}");
            exit.write(AppExit::error());
        }
        None => {}
    }
    smoke.exit_in = None;
}

/// One explicitly labelled topology capture. Player pose is never teleported;
/// all landmark views and the tour use the real movement path at eye height.
fn smoke_overview(
    smoke: Res<Smoke>,
    layout: Res<LayoutRes>,
    camera: Single<(&mut Transform, &mut DistanceFog, &mut Projection), With<Player>>,
) {
    let (mut transform, mut fog, mut projection) = camera.into_inner();
    let overview = smoke
        .pending
        .as_ref()
        .is_some_and(|shot| shot.path.file_stem().is_some_and(|s| s == "00_overview"));
    if overview {
        *transform = Transform::from_xyz(25.0, 120.0, 55.0).looking_at(Vec3::new(0.0, 0.0, -35.0), Vec3::Y);
        fog.falloff = FogFalloff::from_visibility_squared(OVERVIEW_FOG_VISIBILITY);
    } else {
        fog.falloff = FogFalloff::from_visibility_squared(crate::world::land::fog_visibility(&layout.0));
    }
    if let Projection::Perspective(p) = &mut *projection {
        p.fov = if overview { 48.0_f32 } else { 68.0_f32 }.to_radians();
    }
}

/// The whole `--photos` run is abandoned after this long (real time).
const PHOTOS_TIMEOUT: Duration = Duration::from_secs(20 * 60);
/// The two `00_overview*` photos stand 150 m above the ground, farther from
/// the map's corners (~260 m) than play's fog lets anything be seen. They and
/// the `--smoke` topology capture alone widen the fog to this visibility
/// (metres); the captions and manifest say so. Fog colour (lightning) is
/// untouched, and no light or material is changed.
const OVERVIEW_FOG_VISIBILITY: f32 = 800.0;

/// This photo is a topology overview that takes the widened fog range.
fn widens_fog(name: &str) -> bool {
    name.starts_with("00_overview")
}

/// A view whose pipelines never settle is captured anyway, flagged, after this.
const SHOT_TIMEOUT: Duration = Duration::from_secs(45);
/// A screenshot that is never written fails the run after this.
const SAVE_TIMEOUT: Duration = Duration::from_secs(30);
const PHOTO_IDLE_FRAMES: u32 = 6;
const PHOTO_SETTLE_FRAMES: u32 = 24;
/// UI frames wait longer: the downed wash and the fear vignette ease in.
const PHOTO_UI_SETTLE_FRAMES: u32 = 72;

/// One saved frame, for the manifest.
struct Taken {
    path: PathBuf,
    shot: usize,
    take: u8,
    sim_t: f32,
    storm_t: f32,
    window: (u32, u32),
    fov_deg: f32,
    /// Pipelines were idle and any pair gap had elapsed when it was taken.
    settled: bool,
}

/// `--photos`: the whole tour of viewpoints, no gameplay. Everything it
/// alters is presentation (camera, pinned storm clock, the snapshot mirror,
/// UI open flags) and is written on each image and in `MANIFEST.tsv`.
#[derive(Resource)]
struct Photos {
    shots: Vec<crate::photos::Shot>,
    /// A storm moment with heavy rain and no lightning through a pair gap.
    calm: f32,
    index: usize,
    /// 0 the shot's first frame, 1 its time-separated twin (`_t1`).
    take: u8,
    /// Frames the current view has been held.
    settle: u32,
    /// Consecutive frames with no pipeline compiling and all fonts loaded.
    idle: u32,
    frames: u64,
    started: Instant,
    /// When the current view was first held.
    view_since: Instant,
    /// A screenshot is in flight since this instant.
    saving: Option<Instant>,
    /// Simulated seconds when the last screenshot was requested.
    last_spawn_sim: f32,
    taken: Vec<Taken>,
    exit_in: Option<u32>,
    exited: bool,
    failure: Option<String>,
    requested: (u32, u32),
}

/// Small label on every photo: what it is and what was altered.
#[derive(Component)]
struct PhotoCaption;

/// A moment in the storm with heavy rain and no lightning, through the gap
/// between a pair's frames (photos are calm unless a frame asks otherwise).
pub(crate) fn calm_time(seed: u64) -> f32 {
    let span = crate::photos::PAIR_GAP_SECS + 1.5;
    (60..2000)
        .map(|s| s as f32 * 0.5)
        .find(|&t| {
            crate::storm::rain(seed, t) > 0.82
                && crate::storm::rain(seed, t + crate::photos::PAIR_GAP_SECS) > 0.82
                && (0..=(span / 0.25) as u32).all(|k| crate::storm::flash(seed, t + k as f32 * 0.25) == 0.0)
        })
        .unwrap_or(100.0)
}

/// The vertical field of view that keeps a shot's horizontal frame on a
/// window narrower than the 16:9 the shots are composed for (tiling window
/// managers hand out portrait windows). Capped so it never turns to fisheye.
fn view_fov(fov_deg: f32, aspect: f32) -> f32 {
    use crate::photos::REF_ASPECT;
    if aspect >= REF_ASPECT {
        return fov_deg;
    }
    let half_width = (fov_deg.to_radians() * 0.5).tan() * REF_ASPECT;
    (2.0 * (half_width / aspect).atan())
        .to_degrees()
        .min(fov_deg * 1.5)
        .min(95.0)
}

fn spawn_photo_caption(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        PhotoCaption,
        Text::new(""),
        TextFont {
            font: assets.load::<Font>("fonts/NotoSans-Regular.ttf").into(),
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.9, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(4),
            left: px(4),
            max_width: percent(96),
            padding: UiRect::axes(px(6), px(2)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        GlobalZIndex(200),
    ));
}

/// Keeps the caption on the frame being held. Built once per frame change.
fn photo_caption(
    photos: Res<Photos>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut caption: Single<&mut Text, With<PhotoCaption>>,
    mut shown: Local<Option<(usize, u8)>>,
) {
    let now = (photos.index, photos.take);
    if *shown == Some(now) {
        return;
    }
    *shown = Some(now);
    let text = match photos.shots.get(photos.index) {
        Some(shot) => format!(
            "{}{}  |  PHOTO REVIEW, NOT GAMEPLAY PROOF  |  {}  |  window {}x{} (asked {}x{})",
            shot.name,
            if photos.take == 1 { "_t1" } else { "" },
            shot.label,
            window.physical_width(),
            window.physical_height(),
            photos.requested.0,
            photos.requested.1,
        ),
        None => String::new(),
    };
    set_caption(&mut caption, &text);
}

fn set_caption(text: &mut Text, s: &str) {
    if text.0 != s {
        text.0 = s.to_string();
    }
}

fn file_written(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.len() > 0)
}

/// Write `MANIFEST.tsv`, log the outcome and choose the exit code.
fn finish_photos(photos: &mut Photos, launch: &Launch, window: &Window) -> AppExit {
    let dir = launch.shots_dir.join("photos");
    for t in &photos.taken {
        if photos.failure.is_none() && !file_written(&t.path) {
            photos.failure = Some(format!("screenshot was not written: {}", t.path.display()));
        }
    }
    let missing: Vec<String> = photos
        .shots
        .iter()
        .enumerate()
        .flat_map(|(i, s)| (0..1 + s.pair as u8).map(move |k| (i, k, s.name.as_str())))
        .filter(|&(i, k, _)| !photos.taken.iter().any(|t| t.shot == i && t.take == k))
        .map(|(_, k, name)| format!("{name}{}", if k == 1 { "_t1" } else { "" }))
        .collect();
    if photos.failure.is_none() && !missing.is_empty() {
        photos.failure = Some(format!("{} frames never captured", missing.len()));
    }

    let mut manifest = String::new();
    manifest.push_str("# El Silbon --photos manifest. PRESENTATION REVIEW ONLY: no frame here is gameplay proof.\n");
    manifest.push_str(&format!(
        "# window asked {}x{}, actual {}x{} at exit; shots are composed for 16:9 and the vertical fov is widened on narrower windows.\n",
        photos.requested.0,
        photos.requested.1,
        window.physical_width(),
        window.physical_height()
    ));
    manifest.push_str(&format!(
        "# pairs: `_t1` is the same view with the storm clock and simulated time {:.1}s later (rain and grass motion); storm base t={:.1}s.\n",
        crate::photos::PAIR_GAP_SECS,
        photos.calm
    ));
    manifest.push_str("file\tsurface\twindow\tfov_v\tstorm_t\tsim_t\tsettled\tcam\ttarget\taltered\n");
    for t in &photos.taken {
        let s = &photos.shots[t.shot];
        manifest.push_str(&format!(
            "{}\t{:?}\t{}x{}\t{:.1}\t{:.1}\t{:.2}\t{}\t({:.1},{:.1},{:.1})\t({:.1},{:.1},{:.1})\t{}\n",
            t.path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            s.surface,
            t.window.0,
            t.window.1,
            t.fov_deg,
            t.storm_t,
            t.sim_t,
            t.settled,
            s.pos.x,
            s.pos.y,
            s.pos.z,
            s.target.x,
            s.target.y,
            s.target.z,
            s.label
        ));
    }
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join("MANIFEST.tsv"), manifest)) {
        warn!("cannot write photo manifest in {}: {e}", dir.display());
    }
    let unsettled = photos.taken.iter().filter(|t| !t.settled).count();
    let unclear = photos
        .shots
        .iter()
        .filter(|s| !s.clear)
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>();
    if !unclear.is_empty() {
        warn!("PHOTOS: no clear camera found for {}", unclear.join(", "));
    }
    match &photos.failure {
        None => {
            info!(
                "PHOTOS DONE: {} frames in {} ({unsettled} captured before pipelines settled); manifest {}",
                photos.taken.len(),
                dir.display(),
                dir.join("MANIFEST.tsv").display()
            );
            AppExit::Success
        }
        Some(reason) => {
            error!(
                "PHOTOS FAILED: {reason}; {} frames saved to {}; missing: [{}]",
                photos.taken.len(),
                dir.display(),
                missing.join(", ")
            );
            AppExit::error()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn photo_drive(
    mut commands: Commands,
    mut photos: ResMut<Photos>,
    launch: Res<Launch>,
    (state, mut next): (Res<State<Flow>>, ResMut<NextState<Flow>>),
    probe: Res<PipelineProbe>,
    screenshots: Query<(), With<Screenshot>>,
    (fonts, text_fonts): (Res<Assets<Font>>, Query<&TextFont>),
    mut hud: Query<&mut Visibility, With<crate::ui::HudRoot>>,
    mut camera: Single<(&mut Transform, &mut Projection, &mut DistanceFog), With<Player>>,
    window: Single<&Window, With<PrimaryWindow>>,
    (mut clock, mut net, mut truth): (ResMut<crate::app::StormClock>, ResMut<Network>, ResMut<Truth>),
    (mut map, mut note, mut fright): (
        ResMut<crate::ui::MapOpen>,
        ResMut<crate::encounter::NoteOpen>,
        ResMut<crate::world::omen::Fright>,
    ),
    virtual_time: Res<Time<Virtual>>,
    (layout, models): (Res<LayoutRes>, Res<crate::world::models::ModelsPending>),
    mut exit: MessageWriter<AppExit>,
) {
    use crate::photos::{PAIR_GAP_SECS, PHOTO_PLAYER_BASE, Surface};
    let photos = &mut *photos;
    if photos.exited {
        return;
    }
    photos.frames += 1;
    if *state.get() == Flow::Briefing {
        next.set(Flow::Playing);
    }

    // Leaving: wait for the last screenshot, then report and exit.
    if let Some(n) = photos.exit_in {
        let stuck = photos.saving.is_some_and(|t| t.elapsed() > SAVE_TIMEOUT);
        if n == 0 && (screenshots.is_empty() || stuck) {
            let code = finish_photos(photos, &launch, &window);
            exit.write(code);
            photos.exited = true;
        } else {
            photos.exit_in = Some(n.saturating_sub(1));
        }
        return;
    }
    if photos.started.elapsed() > PHOTOS_TIMEOUT {
        photos.failure = Some(format!(
            "gave up after {}s (frame {} of {})",
            PHOTOS_TIMEOUT.as_secs(),
            photos.index,
            photos.shots.len()
        ));
        photos.exit_in = Some(5);
        return;
    }

    let fonts_loaded = text_fonts.iter().all(|t| match &t.font {
        FontSource::Handle(h) => fonts.contains(h.id()),
        _ => true,
    });
    if probe.0.load(Ordering::Relaxed) == 0 && fonts_loaded && models.settled() {
        photos.idle += 1;
    } else {
        photos.idle = 0;
    }
    let Some(shot) = photos.shots.get(photos.index) else {
        photos.exit_in = Some(30);
        return;
    };

    // Frame it: camera, sky, UI flags and the mirrored world state. The
    // snapshot is rebuilt from the session every frame, so none of this
    // reaches the authority or survives the frame.
    let want_hud = if shot.surface.is_ui() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut hud {
        if *v != want_hud {
            *v = want_hud;
        }
    }
    // The catch, held at one instant of its clock, caught where the shot
    // stands and looking where it looks.
    fright.lunge = None;
    fright.catch = None;
    if let Surface::Lunge(at) = shot.surface {
        let eye = Vec2::new(shot.pos.x, shot.pos.z);
        let ahead = Vec2::new(shot.target.x - shot.pos.x, shot.target.z - shot.pos.z).normalize_or(Vec2::NEG_Y);
        fright.lunge = Some(at);
        fright.catch = Some(crate::world::omen::Catch {
            eye: shot.pos,
            ground: layout.0.surface_height(eye),
            dir: ahead,
            forward: ahead,
            variation: 0,
        });
    }
    let want_map = shot.surface == Surface::Map;
    if map.0 != want_map {
        map.0 = want_map;
    }
    let want_note = match shot.surface {
        Surface::Note(id) => Some(id),
        _ => None,
    };
    if note.0 != want_note {
        note.0 = want_note;
    }
    let aspect = window.physical_width() as f32 / window.physical_height().max(1) as f32;
    let fov_deg = view_fov(shot.fov_deg, aspect);
    let (tf, projection, fog) = &mut *camera;
    **tf = Transform::from_translation(shot.pos).looking_at(shot.target, Vec3::Y);
    // Only the labelled topology overviews get the widened range; every other
    // frame restores play's, so nothing carries into gameplay-like views.
    fog.falloff = FogFalloff::from_visibility_squared(if widens_fog(&shot.name) {
        OVERVIEW_FOG_VISIBILITY
    } else {
        crate::world::land::fog_visibility(&layout.0)
    });
    if let Projection::Perspective(p) = &mut **projection {
        p.fov = fov_deg.to_radians();
    }
    // The storm stands still at the calm moment (plus the pair gap for the
    // twin frame); `prev` follows so no onset falls in `(prev, t]`.
    let storm_t = photos.calm + photos.take as f32 * PAIR_GAP_SECS;
    clock.t = storm_t;
    clock.prev = storm_t;
    if let Some(endpoint) = net.endpoint.as_mut() {
        let me = endpoint.id;
        if let Some(s) = endpoint.snapshot.as_mut() {
            if shot.powered {
                s.world.power = 1.0;
                s.world.truck = 1.0;
                s.world.warm = 0.35;
                s.world.beacon = 20.0;
            }
            s.danger = 0;
            // Photo-only teammates: they exist in this mirror alone.
            s.players.retain(|p| p.id < PHOTO_PLAYER_BASE);
            for (i, c) in shot.party.iter().enumerate() {
                s.players.push(crate::net::protocol::PlayerView {
                    id: PHOTO_PLAYER_BASE + i as u64,
                    position: c.pos.to_array(),
                    yaw: c.yaw,
                    pitch: 0.0,
                    status: 0,
                    crouch: c.crouch,
                    sprint: false,
                    light: true,
                    carrying: c.carrying as u8,
                    revive: 0.0,
                    bleed: 0.0,
                    hauled: false,
                    // Each staged teammate someone else.
                    survivor: ((i + 1) % 4) as u8,
                });
            }
            if shot.surface == Surface::Downed
                && let Some(p) = s.players.iter_mut().find(|p| Some(p.id) == me)
            {
                p.status = 1;
                p.bleed = 38.0;
            }
        }
    }
    if let Some((at, facing)) = shot.silbon {
        let th = &mut truth.encounter.threat;
        th.pos = at;
        th.facing = facing;
        th.presence = crate::sim::Presence::Present;
        th.state = crate::sim::ThreatState::Stalking;
    }

    // Hold each view until the renderer has settled on it.
    if photos.frames <= 90 || photos.started.elapsed() < READY_MIN_WALL {
        return;
    }
    match photos.saving {
        None => {
            photos.settle += 1;
            let need = if shot.surface.is_ui() {
                PHOTO_UI_SETTLE_FRAMES
            } else {
                PHOTO_SETTLE_FRAMES
            };
            let sim = virtual_time.elapsed_secs();
            let gap_ok = photos.take == 0 || sim - photos.last_spawn_sim >= PAIR_GAP_SECS;
            let settled = gap_ok && photos.idle >= PHOTO_IDLE_FRAMES;
            let timed_out = photos.view_since.elapsed() > SHOT_TIMEOUT;
            if photos.settle >= need && (settled || timed_out) {
                if !settled {
                    warn!(
                        "photos: capturing {} after {}s without settling (idle frames {}, pair gap ok {gap_ok})",
                        shot.name,
                        SHOT_TIMEOUT.as_secs(),
                        photos.idle
                    );
                }
                let file = if photos.take == 0 {
                    format!("{}.png", shot.name)
                } else {
                    format!("{}_t1.png", shot.name)
                };
                let path = launch.shots_dir.join("photos").join(file);
                let taken = Taken {
                    path: path.clone(),
                    shot: photos.index,
                    take: photos.take,
                    sim_t: sim,
                    storm_t,
                    window: (window.physical_width(), window.physical_height()),
                    fov_deg,
                    settled,
                };
                photos.taken.push(taken);
                capture(&mut commands, path);
                photos.saving = Some(Instant::now());
                photos.last_spawn_sim = sim;
            }
        }
        Some(since) => {
            if screenshots.is_empty() {
                let written = photos.taken.last().is_some_and(|t| file_written(&t.path));
                if !written {
                    let path = photos
                        .taken
                        .last()
                        .map(|t| t.path.display().to_string())
                        .unwrap_or_default();
                    photos.failure = Some(format!("screenshot was not written: {path}"));
                    photos.exit_in = Some(5);
                    return;
                }
                photos.saving = None;
                photos.settle = 0;
                photos.view_since = Instant::now();
                if shot.pair && photos.take == 0 {
                    photos.take = 1;
                } else {
                    photos.index += 1;
                    photos.take = 0;
                }
            } else if since.elapsed() > SAVE_TIMEOUT {
                photos.failure = Some(format!(
                    "screenshot request for {} was never completed within {}s",
                    shot.name,
                    SAVE_TIMEOUT.as_secs()
                ));
                photos.exit_in = Some(5);
            }
        }
    }
}

/// `--menu-shots`: the title screen and every menu page, one screenshot each
/// (presentation review only). The driver opens pages directly and starts
/// and leaves a solo night through the same messages as the menu.
#[derive(Resource)]
struct MenuShots {
    step: usize,
    frames: u32,
    idle: u32,
    started: Instant,
    taken: Vec<PathBuf>,
    exit_in: Option<u32>,
}

/// What each step shows before its screenshot.
#[derive(Clone, Copy)]
enum MenuStep {
    /// The title screen at this landmark stop, on this page.
    Title(usize, crate::ui::menu::Page),
    /// Start a solo night; captured at its briefing.
    Begin,
    /// The night under way (the HUD returns, no party roster alone).
    Playing,
    Pause,
    /// Leave the night, back on the title screen.
    Leave,
}

const MENU_SHOTS: &[(&str, MenuStep)] = {
    use crate::ui::menu::Page::*;
    &[
        ("01_title_main", MenuStep::Title(0, Main)),
        ("02_title_main_ceiba", MenuStep::Title(1, Main)),
        ("03_title_main_corral", MenuStep::Title(2, Main)),
        ("04_title_main_lookout", MenuStep::Title(5, Main)),
        ("05_play_solo", MenuStep::Title(3, Solo)),
        ("06_multiplayer", MenuStep::Title(4, Multiplayer)),
        ("07_host", MenuStep::Title(4, Host)),
        ("08_join", MenuStep::Title(4, Join)),
        ("09_journal", MenuStep::Title(6, Journal)),
        ("10_reading", MenuStep::Title(6, Reading(0))),
        ("11_settings", MenuStep::Title(0, Settings)),
        ("12_how_to_play", MenuStep::Title(1, HowTo)),
        ("13_credits", MenuStep::Title(2, Credits)),
        ("14_confirm_quit", MenuStep::Title(2, ConfirmQuit)),
        ("15_briefing", MenuStep::Begin),
        ("16_playing", MenuStep::Playing),
        ("17_pause", MenuStep::Pause),
        ("18_pause_settings", MenuStep::Pause),
        ("19_back_on_title", MenuStep::Leave),
    ]
};

#[allow(clippy::too_many_arguments)]
fn menu_shots_drive(
    mut commands: Commands,
    mut shots: ResMut<MenuShots>,
    launch: Res<Launch>,
    probe: Res<PipelineProbe>,
    screenshots: Query<(), With<Screenshot>>,
    (state, mut next): (Res<State<Flow>>, ResMut<NextState<Flow>>),
    (mut menu, mut night): (ResMut<crate::ui::menu::Menu>, ResMut<crate::ui::title::TitleNight>),
    (mut starts, mut leaves): (MessageWriter<crate::net::StartRun>, MessageWriter<crate::net::LeaveRun>),
    (mut exit, models): (MessageWriter<AppExit>, Res<crate::world::models::ModelsPending>),
) {
    use crate::ui::menu::Page;
    let shots = &mut *shots;
    if let Some(n) = shots.exit_in {
        if n == 0 && screenshots.is_empty() {
            let missing: Vec<_> = shots.taken.iter().filter(|p| !file_written(p)).collect();
            if missing.is_empty() {
                info!(
                    "MENU SHOTS OK: {} screenshots in {}",
                    shots.taken.len(),
                    launch.shots_dir.join("menu").display()
                );
                exit.write(AppExit::Success);
            } else {
                error!("MENU SHOTS FAIL: not written: {missing:?}");
                exit.write(AppExit::error());
            }
            shots.exit_in = Some(u32::MAX);
        } else if n != u32::MAX {
            shots.exit_in = Some(n.saturating_sub(1));
        }
        return;
    }
    if shots.started.elapsed() > Duration::from_secs(240) {
        error!("MENU SHOTS FAIL: gave up at step {}", shots.step);
        exit.write(AppExit::error());
        shots.exit_in = Some(u32::MAX);
        return;
    }
    let Some(&(name, step)) = MENU_SHOTS.get(shots.step) else {
        shots.exit_in = Some(30);
        return;
    };
    let flow = *state.get();
    if shots.frames % 300 == 299 {
        info!(
            "menu shots: waiting on {name} (flow {flow:?}, page {:?}, idle {})",
            menu.page, shots.idle
        );
    }
    // Arrange the step (the first frame of a step also sends its message).
    let first = shots.frames == 0;
    let ready = match step {
        MenuStep::Title(stop, page) => {
            night.t = stop as f32 * crate::ui::title::STOP + 8.0;
            if menu.page != page {
                menu.jump(page);
            }
            flow == Flow::Title
        }
        MenuStep::Begin => {
            if first {
                starts.write(crate::net::StartRun {
                    mode: crate::net::transport::Mode::Solo,
                    seed: launch.seed,
                    night: launch.night,
                });
            }
            flow == Flow::Briefing
        }
        MenuStep::Playing => {
            if flow == Flow::Briefing {
                next.set(Flow::Playing);
            }
            flow == Flow::Playing
        }
        MenuStep::Pause => {
            if flow == Flow::Playing {
                next.set(Flow::Paused);
            }
            let page = if name.ends_with("settings") {
                Page::Settings
            } else {
                Page::Pause
            };
            if flow == Flow::Paused && menu.page != page {
                menu.jump(page);
            }
            flow == Flow::Paused && menu.page == page
        }
        MenuStep::Leave => {
            if first {
                leaves.write(crate::net::LeaveRun);
            }
            if flow == Flow::Title {
                night.t = 8.0;
            }
            flow == Flow::Title
        }
    };
    shots.frames += 1;
    if probe.0.load(Ordering::Relaxed) == 0 && ready && models.settled() {
        shots.idle += 1;
    } else {
        shots.idle = 0;
    }
    // The first view waits for the whole world's pipelines; later ones for
    // their own, and a moment for the fade and the menu to redraw.
    let warm = shots.step > 0 || shots.started.elapsed() > Duration::from_secs(6);
    if warm && shots.idle >= 40 && screenshots.is_empty() {
        let path = launch.shots_dir.join("menu").join(format!("{name}.png"));
        capture(&mut commands, path.clone());
        shots.taken.push(path);
        shots.step += 1;
        shots.frames = 0;
        shots.idle = 0;
    }
}
