//! App assembly: launch options, shared resources, the run flow
//! (briefing → playing ⇄ paused → outcome), cursor capture, focus safety and
//! restart. Gameplay truth lives in `sim`; this module only wires it in.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowFocused, WindowResolution};

use crate::geometry::Layout;
use crate::perception::{CueDirector, WhistlePhrase};
use crate::sim::{Encounter, Event};
use crate::tuning::{DEFAULT_SEED, Tuning};

/// The authored layout, shared by every system.
#[derive(Resource)]
pub struct LayoutRes(pub Layout);

#[derive(Resource)]
pub struct TuningRes(pub Tuning);

/// Hidden truth plus this listener's perception state.
#[derive(Resource)]
pub struct Truth {
    pub encounter: Encounter,
    pub cue: CueDirector,
}

/// Player-adjustable settings (pause menu).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Settings {
    /// Master volume, 0..1.
    pub volume: f32,
    /// Mouse sensitivity multiplier.
    pub sensitivity: f32,
    /// Show whistle captions (perceived impression only).
    pub captions: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.8,
            sensitivity: 1.0,
            captions: true,
        }
    }
}

/// How the app was launched.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Launch {
    /// DEBUG: run the scripted smoke route and exit.
    pub smoke: bool,
    pub seed: u64,
    pub shots_dir: PathBuf,
    pub size: (u32, u32),
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            smoke: false,
            seed: DEFAULT_SEED,
            shots_dir: PathBuf::from("screenshots"),
            size: (1600, 900),
        }
    }
}

pub const USAGE: &str = "\
El Silbón — The Return (first local encounter)

USAGE: el_silbon [--seed N] [--size WxH] [--shots DIR] [--smoke]

  --seed N      world scatter / whistle jitter seed (default 1997)
  --size WxH    window size (default 1600x900)
  --shots DIR   screenshot folder for F12 and the smoke route (default ./screenshots)
  --smoke       DEBUG: play the deterministic scripted route (win, restart,
                caught, restart), save screenshots, print a summary, exit
";

impl Launch {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut launch = Self::default();
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--smoke" => launch.smoke = true,
                "--seed" => {
                    let v = it.next().ok_or("--seed needs a value")?;
                    launch.seed = v.parse().map_err(|_| format!("bad --seed value: {v}"))?;
                }
                "--shots" => {
                    launch.shots_dir = PathBuf::from(it.next().ok_or("--shots needs a folder")?);
                }
                "--size" => {
                    let v = it.next().ok_or("--size needs WxH")?;
                    let (w, h) = v.split_once('x').ok_or_else(|| format!("bad --size value: {v}"))?;
                    let w: u32 = w.parse().map_err(|_| format!("bad --size width: {v}"))?;
                    let h: u32 = h.parse().map_err(|_| format!("bad --size height: {v}"))?;
                    launch.size = (w.clamp(640, 7680), h.clamp(360, 4320));
                }
                "-h" | "--help" => return Err(USAGE.to_string()),
                other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
            }
        }
        Ok(launch)
    }
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Flow {
    /// Title and controls; click Begin to capture the mouse.
    #[default]
    Briefing,
    Playing,
    /// Encounter frozen, cursor free, settings available.
    Paused,
    /// Won or caught; restart available.
    Outcome,
}

/// Truth events of this frame, for audio and UI.
#[derive(Message, Clone, Copy, Debug)]
pub struct EncounterMsg(pub Event);

/// A whistle phrase to play now (perceived cue only).
#[derive(Message, Clone, Copy, Debug)]
pub struct WhistleMsg(pub WhistlePhrase);

/// Restart the encounter from the road (menu button, R key, debug route).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct RestartRequest;

/// Frame ordering of gameplay systems in `Update`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Restart, pause keys, devices or the debug route → intent.
    Control,
    /// Intent → look and collision movement.
    Motion,
    /// Crosshair targeting.
    Target,
    /// Truth step, perception, outcome.
    Simulate,
    /// Views, audio, HUD.
    Present,
}

pub fn run() -> AppExit {
    let launch = match Launch::from_args(std::env::args().skip(1)) {
        Ok(l) => l,
        Err(msg) => {
            eprintln!("{msg}");
            return if msg.starts_with("El Silbón") {
                AppExit::Success
            } else {
                AppExit::from_code(2)
            };
        }
    };
    build_app(launch).run()
}

/// Asset folder: `BEVY_ASSET_ROOT` if set, else this crate's `assets/`
/// (so the binary also runs when launched from `target/`), else `assets`.
fn asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let crate_assets = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
    if std::path::Path::new(crate_assets).is_dir() {
        crate_assets.to_string()
    } else {
        "assets".to_string()
    }
}

pub fn build_app(launch: Launch) -> App {
    let tuning = Tuning::with_seed(launch.seed);
    let layout = Layout::authored();
    let encounter = Encounter::new(&layout);
    let cue = CueDirector::new(tuning.seed);

    let mut app = App::new();
    let present_mode = if launch.smoke {
        PresentMode::AutoNoVsync
    } else {
        PresentMode::AutoVsync
    };
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "El Silbón — The Return".to_string(),
                    resolution: WindowResolution::new(launch.size.0, launch.size.1),
                    present_mode,
                    ..default()
                }),
                primary_cursor_options: Some(CursorOptions::default()),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_root(),
                ..default()
            }),
    );
    if launch.smoke {
        // Fixed 60 Hz simulated time regardless of render speed: every smoke
        // run steps the same truth in the same order.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    }
    let fog = crate::world::land::HORIZON;
    app.insert_resource(ClearColor(Color::linear_rgb(fog[0], fog[1], fog[2])))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.55, 0.64, 0.9),
            brightness: 55.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(LayoutRes(layout))
        .insert_resource(TuningRes(tuning))
        .insert_resource(Truth { encounter, cue })
        .insert_resource(Settings::default())
        .insert_resource(launch)
        .init_state::<Flow>()
        .add_message::<EncounterMsg>()
        .add_message::<WhistleMsg>()
        .add_message::<RestartRequest>()
        .configure_sets(
            Update,
            (
                GameSet::Control,
                GameSet::Motion,
                GameSet::Target,
                GameSet::Simulate,
                GameSet::Present,
            )
                .chain(),
        )
        .add_plugins((
            crate::world::WorldPlugin,
            crate::player::PlayerPlugin,
            crate::encounter::EncounterPlugin,
            crate::audio::SoundPlugin,
            crate::ui::HudPlugin,
            crate::debug::DebugPlugin,
        ))
        .add_systems(OnEnter(Flow::Playing), capture_cursor)
        .add_systems(OnExit(Flow::Playing), release_cursor)
        .add_systems(OnEnter(Flow::Paused), freeze_time)
        .add_systems(OnExit(Flow::Paused), thaw_time)
        .add_systems(
            Update,
            (apply_restart, pause_keys, pause_on_focus_loss)
                .chain()
                .in_set(GameSet::Control),
        );
    app
}

fn capture_cursor(launch: Res<Launch>, mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    if launch.smoke {
        return; // never grab the desktop's pointer during automation
    }
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

fn freeze_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn thaw_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Restart: every truth value, timer and perception state back to the start.
/// Other plugins reset their own views on the same message.
pub fn apply_restart(
    mut requests: MessageReader<RestartRequest>,
    mut truth: ResMut<Truth>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    mut next: ResMut<NextState<Flow>>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let Truth { encounter, cue } = &mut *truth;
    encounter.reset(&layout.0);
    cue.reset(tuning.0.seed);
    next.set(Flow::Playing);
    info!("encounter restarted");
}

fn pause_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<Flow>>,
    launch: Res<Launch>,
    mut next: ResMut<NextState<Flow>>,
    mut restart: MessageWriter<RestartRequest>,
) {
    if launch.smoke {
        return;
    }
    match state.get() {
        Flow::Playing if keys.just_pressed(KeyCode::Escape) => next.set(Flow::Paused),
        Flow::Paused if keys.just_pressed(KeyCode::Escape) => next.set(Flow::Playing),
        Flow::Briefing if keys.just_pressed(KeyCode::Enter) => next.set(Flow::Playing),
        Flow::Outcome if keys.just_pressed(KeyCode::KeyR) => {
            restart.write(RestartRequest);
        }
        _ => {}
    }
}

/// Losing window focus while playing pauses (and so releases the cursor).
fn pause_on_focus_loss(
    mut focus: MessageReader<WindowFocused>,
    state: Res<State<Flow>>,
    launch: Res<Launch>,
    mut next: ResMut<NextState<Flow>>,
) {
    let lost = focus.read().any(|f| !f.focused);
    if lost && !launch.smoke && *state.get() == Flow::Playing {
        next.set(Flow::Paused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Result<Launch, String> {
        Launch::from_args(s.split_whitespace().map(str::to_string))
    }

    #[test]
    fn launch_arguments() {
        assert_eq!(args("").unwrap(), Launch::default());
        let l = args("--smoke --seed 42 --size 1280x720 --shots out").unwrap();
        assert!(l.smoke);
        assert_eq!(l.seed, 42);
        assert_eq!(l.size, (1280, 720));
        assert_eq!(l.shots_dir, PathBuf::from("out"));
        assert!(args("--seed nope").is_err());
        assert!(args("--size 12").is_err());
        assert!(args("--fly").is_err());
    }
}
