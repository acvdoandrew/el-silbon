//! In-application smoke input and scene census. Never touches desktop input.
use super::{NetControl, Network, RemotePlayer, smoke::Driver};
use crate::{
    app::{GameSet, Launch, LayoutRes, TuningRes},
    audio::AmbienceLoop,
    encounter::CurrentTarget,
    player::{CurrentIntent, Player},
    world::{CarriedSatchel, avatar::AvatarTorch, silbon::SilbonRoot},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
};

#[derive(Resource)]
struct RenderSmoke {
    driver: Driver,
    baseline: Option<[usize; 7]>,
    observed_run: u64,
    frames: u64,
    shots: std::collections::BTreeMap<(u64, &'static str), u64>,
    /// Consecutive frames the avatar or torch count disagreed with the roster.
    roster_bad: u32,
    exit_frames: Option<u32>,
    failed: bool,
}

pub struct NetworkSmokePlugin;

impl Plugin for NetworkSmokePlugin {
    fn build(&self, app: &mut App) {
        let driver = Driver::new(
            &app.world().resource::<LayoutRes>().0,
            &app.world().resource::<TuningRes>().0,
        );
        app.insert_resource(RenderSmoke {
            driver,
            baseline: None,
            observed_run: 0,
            frames: 0,
            shots: Default::default(),
            roster_bad: 0,
            exit_frames: None,
            failed: false,
        })
        .add_systems(
            Update,
            drive.in_set(GameSet::Control).after(crate::player::reset_player),
        )
        .add_systems(Update, inspect.in_set(GameSet::Present).after(super::avatars));
    }
}

fn drive(
    time: Res<Time<Real>>,
    net: Res<Network>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    player: Single<&Player>,
    target: Res<CurrentTarget>,
    mut state: ResMut<RenderSmoke>,
    mut input: ResMut<CurrentIntent>,
    mut control: MessageWriter<NetControl>,
) {
    input.0 = Default::default();
    if state.exit_frames.is_some() {
        return;
    }
    let Some(endpoint) = &net.endpoint else {
        state.failed = true;
        state.exit_frames = Some(30);
        error!("NET RENDER SMOKE FAIL: {}", net.error);
        return;
    };
    let frame = state.driver.tick(
        endpoint,
        player.pose,
        target.0,
        time.delta_secs().min(0.1),
        &layout.0,
        &tuning.0,
    );
    input.0 = frame.intent;
    if let Some(action) = frame.action {
        // The regular update system transmits this frame's orientation first.
        control.write(NetControl::Action(action));
    }
    if frame.leave {
        control.write(NetControl::Leave);
    }
    if let Some(e) = &state.driver.error {
        error!("NET RENDER SMOKE FAIL: {e}");
        state.failed = true;
        state.exit_frames = Some(30);
    } else if state.driver.done {
        state.exit_frames = Some(30);
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect(
    mut commands: Commands,
    net: Res<Network>,
    launch: Res<Launch>,
    mut state: ResMut<RenderSmoke>,
    cameras: Query<(), With<Camera3d>>,
    spots: Query<(), (With<SpotLight>, Without<AvatarTorch>)>,
    points: Query<(), With<PointLight>>,
    moon: Query<(), With<DirectionalLight>>,
    ambience: Query<(), With<AmbienceLoop>>,
    silbon: Query<(), With<SilbonRoot>>,
    carried: Query<(), With<CarriedSatchel>>,
    remote: Query<(), With<RemotePlayer>>,
    torches: Query<(), With<AvatarTorch>>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    state.frames += 1;
    let census = [
        cameras.iter().count(),
        spots.iter().count(),
        points.iter().count(),
        moon.iter().count(),
        ambience.iter().count(),
        silbon.iter().count(),
        carried.iter().count(),
    ];
    if state.frames > 120 {
        if let Some(baseline) = state.baseline {
            if census != baseline {
                error!("NET RENDER SMOKE FAIL: scene census changed {baseline:?} -> {census:?}");
                state.failed = true;
                state.exit_frames = Some(1);
            }
        } else {
            info!("NET RENDER baseline cameras/world spots/points/moon/audio/threat/carried={census:?}");
            state.baseline = Some(census);
        }
    }
    if let Some(s) = net.snapshot() {
        if state.observed_run != s.run {
            state.observed_run = s.run;
            info!(
                "NET RENDER run={} census={census:?} remote={}",
                s.run,
                remote.iter().count()
            );
        }
        // Every teammate carries one torch beam; the world census above
        // excludes them, so roster changes (a partner joining or leaving)
        // cannot read as a leak while a torch or avatar left behind still does.
        let expected = s.players.len().saturating_sub(1);
        let (avatars, beams) = (remote.iter().count(), torches.iter().count());
        if state.frames > 120 && (avatars != expected || beams != expected) {
            state.roster_bad += 1;
            if state.roster_bad > 30 {
                error!(
                    "NET RENDER SMOKE FAIL: {avatars} teammate avatars and {beams} torch beams for a roster of {expected}"
                );
                state.failed = true;
                state.exit_frames = Some(1);
            }
        } else {
            state.roster_bad = 0;
        }
        let stage = if s.outcome == 1 {
            Some("won")
        } else if s.outcome == 2 {
            Some("failed")
        } else if net.status() == 1 {
            Some("downed")
        } else if s.danger == 1 {
            Some("warning")
        } else if s.world.delivered > 0 {
            Some("delivered")
        } else if s.relics.iter().any(|r| r.state == 1) {
            Some("carrying")
        } else if s.elapsed > 5.0 {
            Some("connected")
        } else {
            None
        };
        if state.frames > 120
            && let Some(stage) = stage
        {
            // Let outcome state transitions and UI layout settle before capturing.
            let frame = state.frames;
            let due = state.shots.entry((s.run, stage)).or_insert(frame + 3);
            if frame >= *due {
                *due = u64::MAX;
                let role = if net.id() == Some(1) { "host" } else { "client" };
                let folder = launch.shots_dir.join("network");
                match std::fs::create_dir_all(&folder) {
                    Ok(()) => {
                        commands
                            .spawn(Screenshot::primary_window())
                            .observe(save_to_disk(folder.join(format!("{role}-run{}-{stage}.png", s.run))));
                    }
                    Err(e) => {
                        error!("NET RENDER screenshot failed: {e}");
                        state.failed = true;
                        state.exit_frames = Some(1);
                    }
                }
            }
        }
    }
    if let Some(frames) = state.exit_frames {
        if frames > 0 || !capturing.is_empty() {
            state.exit_frames = Some(frames.saturating_sub(1));
        } else {
            if state.failed {
                exit.write(AppExit::error());
            } else {
                info!(
                    "NET RENDER SMOKE PASS: scene census preserved; last run {}",
                    state.observed_run
                );
                exit.write(AppExit::Success);
            }
            state.exit_frames = None;
        }
    }
}
