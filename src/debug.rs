//! Debug support, clearly separated from play:
//! - F12 saves a game-window screenshot (any mode).
//! - `--smoke` drives the real game with the deterministic route script
//!   (`script.rs`) through the same intent → motion → targeting → truth path
//!   as a player, saves screenshots at fixed beats, checks that restarts do
//!   not accumulate entities, logs a summary and exits (code 0 = route passed).
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

use crate::app::{Flow, GameSet, Launch, LayoutRes, RestartRequest, Truth, TuningRes};
use crate::audio::AmbienceLoop;
use crate::encounter::CurrentTarget;
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
        let smoke = app.world().get_resource::<Launch>().is_some_and(|l| l.smoke);
        if smoke {
            let script = {
                let world = app.world();
                let layout = &world.resource::<LayoutRes>().0;
                let tuning = &world.resource::<TuningRes>().0;
                RouteScript::full(layout, tuning)
            };
            // The render world reports how many pipelines are still compiling.
            let probe = PipelineProbe(Arc::new(AtomicUsize::new(usize::MAX)));
            if let Some(render) = app.get_sub_app_mut(RenderApp) {
                render
                    .insert_resource(probe.clone())
                    .add_systems(Render, probe_pipelines.in_set(RenderSystems::Cleanup));
            }
            app.insert_resource(probe)
                .insert_resource(Smoke {
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
                })
                .add_systems(
                    Update,
                    (
                        smoke_drive.in_set(GameSet::Control).after(crate::app::apply_restart),
                        smoke_census.in_set(GameSet::Present),
                        smoke_exit.in_set(GameSet::Present).after(smoke_census),
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
struct PipelineProbe(Arc<AtomicUsize>);

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
    probe: Res<PipelineProbe>,
    fonts: Res<Assets<Font>>,
    text_fonts: Query<&TextFont>,
    screenshots: Query<(), With<Screenshot>>,
    mut intent: ResMut<CurrentIntent>,
    mut restart: MessageWriter<RestartRequest>,
) {
    // Plain `&mut Smoke` so the pending shot and counters borrow disjointly.
    let smoke = &mut *smoke;
    smoke.frames += 1;
    let fonts_loaded = text_fonts.iter().all(|t| match &t.font {
        FontSource::Handle(h) => fonts.contains(h.id()),
        _ => true,
    });
    if probe.0.load(Ordering::Relaxed) == 0 && fonts_loaded {
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
        pose: player.pose,
        encounter: &truth.encounter,
        target: target.0,
        dt: time.delta_secs(),
    };
    let frame = smoke.script.tick(&obs);
    intent.0 = frame.intent;
    if let Some(msg) = frame.log {
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
    if frame.restart {
        restart.write(RestartRequest);
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
