//! In-application smoke input and scene census. Never touches desktop input.
use super::{
    NetControl, Network, RemotePlayer,
    protocol::{Action, Satchel},
    smoke::Driver,
};
use crate::{
    app::{GameSet, Launch, LayoutRes, TuningRes},
    audio::AmbienceLoop,
    player::{CurrentIntent, Player},
    world::{CarriedSatchel, TableSatchel, TreeSatchel, silbon::SilbonRoot},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
};

#[derive(Resource)]
struct RenderSmoke {
    driver: Driver,
    baseline: Option<[usize; 9]>,
    observed_run: u64,
    frames: u64,
    shots: std::collections::BTreeMap<(u64, &'static str), u64>,
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
    let frame = state
        .driver
        .tick(endpoint, player.pose, time.delta_secs().min(0.1), &layout.0, &tuning.0);
    input.0 = frame.intent;
    if let Some(action) = frame.action {
        // The interaction goes through the regular update system, which first
        // transmits this frame's orientation; no duplicate pickup request here.
        if !matches!(action, Action::Take) {
            control.write(NetControl::Action(action));
        } else {
            input.0.interact_pressed = true;
        }
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
fn inspect(
    mut commands: Commands,
    net: Res<Network>,
    launch: Res<Launch>,
    mut state: ResMut<RenderSmoke>,
    cameras: Query<(), With<Camera3d>>,
    spots: Query<(), With<SpotLight>>,
    points: Query<(), With<PointLight>>,
    moon: Query<(), With<DirectionalLight>>,
    ambience: Query<(), With<AmbienceLoop>>,
    silbon: Query<(), With<SilbonRoot>>,
    table: Query<(), With<TableSatchel>>,
    tree: Query<(), With<TreeSatchel>>,
    carried: Query<(), With<CarriedSatchel>>,
    remote: Query<(), With<RemotePlayer>>,
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
        table.iter().count(),
        tree.iter().count(),
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
            info!("NET RENDER baseline cameras/spots/points/moon/audio/threat/table/tree/carried={census:?}");
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
        let expected = s.players.len().saturating_sub(1);
        if state.frames > 120 && remote.iter().count() != expected {
            error!(
                "NET RENDER SMOKE FAIL: remote roster count {} expected {expected}",
                remote.iter().count()
            );
            state.failed = true;
            state.exit_frames = Some(1);
        }
        let stage = if s.objective == 3 {
            Some("won")
        } else if s.objective == 4 {
            Some("failed")
        } else if net.caught() {
            Some("caught")
        } else if s.danger == 1 {
            Some("warning")
        } else if matches!(s.satchel, Satchel::Carried(_)) {
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
