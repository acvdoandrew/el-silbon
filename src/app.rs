//! App assembly: launch options, shared resources, the run flow
//! (briefing → playing ⇄ paused → outcome), cursor capture and focus safety.
//! Gameplay truth lives in `sim` and `net::session`; this module only wires
//! the presentation to it.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowFocused, WindowResolution};

use crate::geometry::Layout;
use crate::net::transport::Mode;
use crate::perception::WhistlePhrase;
use crate::sim::{Encounter, Event};
use crate::tuning::{DEFAULT_SEED, Tuning};

/// The authored layout, shared by every system.
#[derive(Resource)]
pub struct LayoutRes(pub Layout);

#[derive(Resource)]
pub struct TuningRes(pub Tuning);

/// The presentation mirror of what this player may legitimately know: the
/// visible threat, the outcome and the run clock, refilled from every
/// snapshot. Hidden AI state never lives here; it stays in `net::session`.
#[derive(Resource)]
pub struct Truth {
    pub encounter: Encounter,
}

/// Player-adjustable settings (pause menu).
#[derive(Resource, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Master volume, 0..1.
    pub volume: f32,
    /// Mouse sensitivity multiplier.
    pub sensitivity: f32,
    /// Show whistle captions (perceived impression only).
    pub captions: bool,
    /// Mouse up looks down.
    pub invert_y: bool,
    /// Vertical field of view, degrees.
    pub fov: f32,
    /// Brightness offset (−1 darker .. +1 brighter), in exposure stops/2.
    pub brightness: f32,
    /// The head rises and falls as you walk.
    pub head_bob: bool,
    /// Borderless fullscreen rather than a window.
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.8,
            sensitivity: 1.0,
            captions: true,
            invert_y: false,
            fov: 68.0,
            brightness: 0.0,
            head_bob: true,
            fullscreen: false,
        }
    }
}

/// How the app was launched.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Launch {
    /// DEBUG: run the scripted smoke route and exit.
    pub smoke: bool,
    pub seed: u64,
    /// How hard the night is.
    pub night: crate::tuning::Night,
    pub shots_dir: PathBuf,
    pub size: (u32, u32),
    /// Solo (default), host or join.
    pub network: Mode,
    pub net_smoke: bool,
    pub headless: bool,
    /// DEBUG: ground-level tour of every place, then the objective smoke.
    pub tour: bool,
    /// DEBUG: fly a camera to authored viewpoints, capture, exit. No gameplay.
    pub photos: bool,
    /// Start on the title screen and its menus (a plain launch). The debug
    /// routes, `--play` and `--host`/`--join` go straight into a run.
    pub menu: bool,
    /// DEBUG `--menu-shots`: walk the title screen and every menu page,
    /// save a screenshot of each, exit.
    pub menu_shots: bool,
    /// DEBUG `--trailer`: render the teaser's shots frame by frame, exit.
    /// Implies `photos` (a staged presentation, never gameplay).
    pub trailer: bool,
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            smoke: false,
            seed: DEFAULT_SEED,
            night: crate::tuning::Night::Normal,
            shots_dir: PathBuf::from("screenshots"),
            size: (1600, 900),
            network: Mode::Solo,
            net_smoke: false,
            headless: false,
            tour: false,
            photos: false,
            menu: false,
            menu_shots: false,
            trailer: false,
        }
    }
}

pub const USAGE: &str = "\
El Silbón — The Return

USAGE: el_silbon [--seed N] [--size WxH] [--shots DIR] [--smoke]

  --seed N      the night: bundle hiding places, the padlock code, which of him
                walks, scatter and jitter (solo play without it: a new night
                every launch; the debug routes and shared sessions: 1997)
  --night N     gentle, normal (default) or hard (every peer must agree)
  --play        skip the title screen and go straight into a solo night
  --size WxH    window size (default 1600x900)
  --shots DIR   screenshot folder for F12 and the debug routes (default ./screenshots)
  --tour        DEBUG: walk to every place, then play the scripted full run
  --smoke       DEBUG: play the deterministic scripted full run (win, restart,
                downed, restart), save screenshots, print a summary, exit
  --photos      DEBUG: fly a camera to authored viewpoints, save screenshots, exit
  --menu-shots  DEBUG: show the title screen and every menu page, save screenshots, exit
  --trailer     DEBUG: render the teaser's shots frame by frame (1920x1080, 30 fps), exit
  --host ADDR   host and play, e.g. 127.0.0.1:5000 (loopback/private LAN only)
  --join ADDR   join a host before the run starts
  --net-smoke   DEBUG: real two-process shared-run route
  --headless    with --net-smoke: run real networking without graphics
";

impl Launch {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut launch = Self::default();
        let mut seeded = false;
        let mut play = false;
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--smoke" => launch.smoke = true,
                "--tour" => {
                    launch.tour = true;
                    launch.smoke = true;
                }
                "--photos" => launch.photos = true,
                "--menu-shots" => launch.menu_shots = true,
                "--trailer" => {
                    launch.trailer = true;
                    launch.photos = true;
                }
                "--play" => play = true,
                "--host" | "--join" => {
                    if launch.network != Mode::Solo {
                        return Err("Choose either --host or --join, not both.".into());
                    }
                    let value = it.next().ok_or("host/join needs IP:PORT")?;
                    let addr = value.parse().map_err(|_| format!("Invalid socket address: {value}"))?;
                    let addr = crate::net::transport::local_address(addr)?;
                    launch.network = if arg == "--host" {
                        Mode::Host(addr)
                    } else {
                        Mode::Join(addr)
                    };
                }
                "--net-smoke" => launch.net_smoke = true,
                "--headless" => launch.headless = true,
                "--seed" => {
                    let v = it.next().ok_or("--seed needs a value")?;
                    launch.seed = v.parse().map_err(|_| format!("bad --seed value: {v}"))?;
                    seeded = true;
                }
                "--night" => {
                    let v = it.next().ok_or("--night needs gentle, normal or hard")?;
                    launch.night = crate::tuning::Night::parse(&v).ok_or_else(|| format!("bad --night value: {v}"))?;
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
        if (launch.smoke || launch.photos) && launch.network != Mode::Solo {
            return Err("--smoke and --photos are solo only; use --net-smoke for a shared session.".into());
        }
        if launch.menu_shots && (launch.smoke || launch.photos || launch.network != Mode::Solo || play) {
            return Err("--menu-shots runs alone, from the title screen.".into());
        }
        if launch.smoke && launch.photos {
            return Err("Choose --smoke/--tour or --photos, not both.".into());
        }
        if launch.net_smoke && launch.network == Mode::Solo {
            return Err("--net-smoke needs --host or --join.".into());
        }
        if launch.headless && !launch.net_smoke {
            return Err("--headless is only available with --net-smoke.".into());
        }
        // A player alone gets a new night each time; the debug routes stay
        // on the fixed night, and a shared session needs every peer to agree
        // (pass the same --seed on every machine for another night).
        if !seeded && launch.network == Mode::Solo && !launch.smoke && !launch.photos {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64);
            launch.seed = 1 + now % 1_000_000;
        }
        launch.menu = !play && launch.network == Mode::Solo && !launch.smoke && !launch.photos;
        Ok(launch)
    }
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Flow {
    /// The title screen and its menus, over the llano at night. No run yet.
    Title,
    /// Title and controls; click Begin to capture the mouse.
    #[default]
    Briefing,
    Playing,
    /// Solo: the run is frozen. Shared: only local input stops.
    Paused,
    /// Won or lost; restart available.
    Outcome,
}

/// Truth events of this frame, for audio and UI.
#[derive(Message, Clone, Copy, Debug)]
pub struct EncounterMsg(pub Event);

/// A whistle phrase to play now (perceived cue only).
#[derive(Message, Clone, Copy, Debug)]
pub struct WhistleMsg(pub WhistlePhrase);

/// What the game remembers between launches, and whether it may write it
/// (never during the automated routes).
#[derive(Resource)]
pub struct ProfileRes {
    pub profile: crate::profile::Profile,
    pub persist: bool,
}

impl ProfileRes {
    /// Save now (best effort).
    pub fn save(&self) {
        if self.persist
            && let Err(e) = crate::profile::save(&self.profile)
        {
            warn!("could not save the profile: {e}");
        }
    }
}

/// The run was reset (a new epoch began): views clear themselves.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct RunReset;

/// The run time the sky is showing (seconds): rain, lightning and thunder are
/// pure functions of `(seed, t)` so every player sees the same storm.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct StormClock {
    pub t: f32,
    /// Where the sky was last frame, to catch onsets in `(prev, t]`.
    pub prev: f32,
}

/// Frame ordering of gameplay systems in `Update`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Pause keys, devices or the debug route → intent.
    Control,
    /// Intent → look.
    Motion,
    /// Crosshair targeting.
    Target,
    /// Session step and snapshot mirroring.
    Simulate,
    /// The storm clock advances once everything else has moved.
    Weather,
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
    if launch.headless {
        return match crate::net::smoke::run_headless(launch) {
            Ok(()) => AppExit::Success,
            Err(e) => {
                eprintln!("NET SMOKE FAIL: {e}");
                AppExit::error()
            }
        };
    }
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
    let tuning = Tuning::with_seed(launch.seed).with_night(launch.night);
    let layout = Layout::with_seed(launch.seed);
    let encounter = Encounter::new(&layout);
    let automated = launch.smoke || launch.photos;
    let launch_trailer = launch.trailer;
    // A player's settings and journal come back each launch; the automated
    // routes run on the defaults and never write the profile.
    let persist = !automated && !launch.net_smoke && !launch.menu_shots;
    let profile = if persist {
        crate::profile::load()
    } else {
        crate::profile::Profile::default()
    };
    let menu = launch.menu;

    let mut app = App::new();
    let present_mode = if automated {
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
    if automated {
        // Fixed 60 Hz simulated time regardless of render speed: every smoke
        // run steps the same truth in the same order.
        let step = if launch_trailer {
            1.0 / f64::from(crate::trailer::FPS)
        } else {
            1.0 / 60.0
        };
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(step)));
    }
    let fog = crate::world::land::HORIZON;
    app.insert_resource(ClearColor(Color::linear_rgb(fog[0], fog[1], fog[2])))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(
                crate::world::land::NIGHT_AMBIENT_COLOR[0],
                crate::world::land::NIGHT_AMBIENT_COLOR[1],
                crate::world::land::NIGHT_AMBIENT_COLOR[2],
            ),
            brightness: crate::world::land::NIGHT_AMBIENT,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(LayoutRes(layout))
        .insert_resource(TuningRes(tuning))
        .insert_resource(Truth { encounter })
        .insert_resource(profile.settings.clone())
        .insert_resource(ProfileRes { profile, persist })
        .insert_resource(launch)
        .insert_state(if menu { Flow::Title } else { Flow::Briefing })
        .add_message::<EncounterMsg>()
        .add_message::<WhistleMsg>()
        .add_message::<RunReset>()
        .init_resource::<StormClock>()
        .add_systems(Update, advance_storm.in_set(GameSet::Weather))
        .configure_sets(
            Update,
            (
                GameSet::Control,
                GameSet::Motion,
                GameSet::Target,
                GameSet::Simulate,
                GameSet::Weather,
                GameSet::Present,
            )
                .chain(),
        )
        .add_plugins((
            crate::net::NetworkPlugin,
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
            (pause_keys, pause_on_focus_loss).chain().in_set(GameSet::Control),
        );
    app
}

/// The sky follows the run clock while a run is on, and simply keeps moving
/// in the lobby, the briefing and after the outcome. Only a solo pause holds
/// it (the run itself is frozen); in a shared game the storm is the same
/// for everyone and the menu cannot stop it.
pub(crate) fn advance_storm(
    time: Res<Time<Real>>,
    launch: Res<Launch>,
    truth: Res<Truth>,
    state: Res<State<Flow>>,
    mut clock: ResMut<StormClock>,
) {
    clock.prev = clock.t;
    if *state.get() == Flow::Paused && launch.network.is_solo() {
        return;
    }
    let run = truth.encounter.elapsed;
    if run > 0.0 && !truth.encounter.outcome.is_over() {
        clock.t = run;
    } else {
        clock.t += time.delta_secs();
    }
}

fn capture_cursor(launch: Res<Launch>, mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    if launch.smoke || launch.net_smoke || launch.photos || launch.menu_shots {
        return; // never grab the desktop's pointer during automation
    }
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

fn freeze_time(launch: Res<Launch>, mut time: ResMut<Time<Virtual>>) {
    if launch.network.is_solo() {
        time.pause();
    }
}

fn thaw_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

fn pause_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<Flow>>,
    launch: Res<Launch>,
    mut next: ResMut<NextState<Flow>>,
    mut control: MessageWriter<crate::net::NetControl>,
) {
    if launch.smoke || launch.photos {
        return;
    }
    match state.get() {
        // Esc opens the menu; inside it, the menu handles Esc (back, resume).
        Flow::Playing if keys.just_pressed(KeyCode::Escape) => next.set(Flow::Paused),
        Flow::Briefing if keys.just_pressed(KeyCode::Enter) && launch.network.is_solo() => next.set(Flow::Playing),
        Flow::Outcome if launch.network.is_host() && keys.just_pressed(KeyCode::KeyR) => {
            control.write(crate::net::NetControl::Action(crate::net::protocol::Action::Restart));
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
    if lost
        && !launch.smoke
        && !launch.net_smoke
        && !launch.photos
        && !launch.menu_shots
        && *state.get() == Flow::Playing
    {
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
        let l = args("--smoke --seed 42 --size 1280x720 --shots out").unwrap();
        assert!(l.smoke);
        assert_eq!(l.seed, 42);
        assert_eq!(l.size, (1280, 720));
        assert_eq!(l.shots_dir, PathBuf::from("out"));
        assert_eq!(l.network, Mode::Solo);
        assert!(args("--seed nope").is_err());
        assert!(args("--size 12").is_err());
        assert!(args("--fly").is_err());
        assert!(args("--map district").is_err(), "there is only one map now");
        assert!(args("--headless").is_err());
        assert!(args("--net-smoke").is_err());
        assert!(args("--tour").unwrap().smoke);
        assert!(args("--photos").unwrap().photos);
        assert!(args("--photos --smoke").is_err());
        assert_eq!(
            args("--smoke").unwrap().seed,
            DEFAULT_SEED,
            "the debug route's night is fixed"
        );
        assert!(args("").unwrap().menu, "a plain launch opens on the title screen");
        assert!(!args("--play").unwrap().menu && !args("--smoke").unwrap().menu);
        assert!(!args("--host 127.0.0.1:5000").unwrap().menu);
        assert_eq!(args("--seed 7").unwrap().seed, 7);
        assert_eq!(args("--night hard").unwrap().night, crate::tuning::Night::Hard);
        assert!(args("--night brutal").is_err());
        assert_eq!(
            args("--host 127.0.0.1:5000").unwrap().seed,
            DEFAULT_SEED,
            "peers agree on the night"
        );
        assert!(args("--host 0.0.0.0:5000").is_err());
        assert!(args("--host 8.8.8.8:5000").is_err());
        assert!(args("--host 127.0.0.1:5000 --join 127.0.0.1:5000").is_err());
        assert!(args("--host 127.0.0.1:5000 --smoke").is_err());
        assert!(matches!(args("--host 127.0.0.1:5000").unwrap().network, Mode::Host(_)));
    }
}
