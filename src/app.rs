//! App assembly: launch options, shared resources, the run flow
//! (briefing → playing ⇄ paused → outcome), the window's mode, cursor capture
//! and focus safety.
//! Gameplay truth lives in `sim` and `net::session`; this module only wires
//! the presentation to it.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{
    CursorGrabMode, CursorOptions, Monitor, MonitorSelection, OnMonitor, PresentMode, PrimaryMonitor, PrimaryWindow,
    WindowFocused, WindowMode, WindowPosition, WindowResolution,
};

use crate::geometry::Layout;
use crate::mix::Bus;
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
///
/// The volume sliders are positions (0..1) on the curve in `mix`. The
/// master replaced an older linear `volume`, which is ignored on load, so
/// a stale saved level starts again from the default. `display_mode`
/// likewise replaced a `fullscreen` flag that every earlier build saved as
/// off without the player ever choosing it: that key is ignored too, so an
/// old profile opens fullscreen once and keeps whatever is chosen after.
#[derive(Resource, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Master volume: every sound, and the only one his whistle follows.
    pub master: f32,
    /// The title theme, the dread and the omens' stingers.
    pub music: f32,
    /// The night itself: insects, rain, thunder, frogs, the windmill.
    pub ambience: f32,
    /// Footsteps, machines, the dog and everything handled.
    pub effects: f32,
    /// Mouse sensitivity multiplier.
    pub sensitivity: f32,
    /// Show whistle captions (perceived impression only).
    pub captions: bool,
    /// Mouse up looks down.
    pub invert_y: bool,
    /// Vertical field of view, degrees.
    pub fov: f32,
    /// A shadow gamma (see `display`): −1 darker .. +1 lifts the darks by
    /// some 2.5 stops; white holds.
    pub brightness: f32,
    /// Log-contrast about the night's fog level (see `display`): 1 is the
    /// night as graded.
    pub contrast: f32,
    /// The head rises and falls as you walk.
    pub head_bob: bool,
    /// Over the whole screen or in a window (see [`display_mode`] for what
    /// it means on a given launch).
    pub display_mode: DisplayMode,
    /// The words on screen, English or Spanish. This player's alone: never
    /// sent to a friend, never part of the gameplay fingerprint.
    #[serde(default)]
    pub lang: crate::lang::Lang,
}

/// How the player wants the game shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    /// Borderless over the whole screen (a new player's choice).
    #[default]
    Fullscreen,
    /// A window of the launch size.
    Window,
}

impl DisplayMode {
    /// The other one.
    pub fn toggled(self) -> Self {
        match self {
            DisplayMode::Fullscreen => DisplayMode::Window,
            DisplayMode::Window => DisplayMode::Fullscreen,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master: crate::mix::DEFAULT_MASTER,
            music: 1.0,
            ambience: 1.0,
            effects: 1.0,
            sensitivity: 1.0,
            captions: true,
            invert_y: false,
            fov: 68.0,
            brightness: 0.0,
            contrast: 1.0,
            head_bob: true,
            display_mode: DisplayMode::Fullscreen,
            lang: crate::lang::Lang::En,
        }
    }
}

impl Settings {
    /// The linear gain of a sound on `bus`: headroom, the master, then the
    /// bus's own slider (the master bus has none).
    pub fn gain(&self, bus: Bus) -> f32 {
        let own = match bus {
            Bus::Master => 1.0,
            Bus::Music => self.music,
            Bus::Ambience => self.ambience,
            Bus::Effects => self.effects,
        };
        crate::mix::gain(self.master, own)
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
    /// `--survivor`: who to ask to be; otherwise the profile's choice.
    pub survivor: Option<crate::survivor::Survivor>,
    /// `--windowed`: stay in a window this launch, whatever the settings
    /// say (hands-on testing). The saved choice is left alone: the menu
    /// shows the row but will not change it.
    pub windowed: bool,
}

impl Launch {
    /// One of the debug drivers (`--smoke`, `--tour`, `--photos`,
    /// `--trailer`, `--menu-shots`, `--net-smoke`) runs this launch: a
    /// window of the given size, the desktop's pointer left alone, the
    /// profile never written.
    pub fn driven(&self) -> bool {
        self.smoke || self.net_smoke || self.photos || self.menu_shots
    }

    /// This launch keeps a window whatever the settings say.
    pub fn forces_window(&self) -> bool {
        self.windowed || self.driven()
    }

    /// The cover shows while the game starts: only a plain launch onto the
    /// title screen. `--play`, `--host`/`--join` and the debug drivers go
    /// straight in (and `--headless` builds no window at all).
    pub fn splash(&self) -> bool {
        self.menu && !self.driven() && !self.headless
    }
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
            survivor: None,
            windowed: false,
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
  --windowed    play in a window this time, keeping the saved choice (the game
                follows Settings > Video > Display mode, fullscreen at first;
                the debug routes always use a window)
  --shots DIR   screenshot folder for F12 and the debug routes (default ./screenshots)
  --tour        DEBUG: walk to every place, then play the scripted full run
  --smoke       DEBUG: play the deterministic scripted full run (win, restart,
                downed, restart), save screenshots, print a summary, exit
  --photos      DEBUG: fly a camera to authored viewpoints, save screenshots, exit
  --menu-shots  DEBUG: show the title screen and every menu page, save screenshots, exit
  --trailer     DEBUG: render the teaser's shots frame by frame (1920x1080, 30 fps), exit
  --host ADDR   host and play, e.g. 192.168.1.20:5000 (loopback, private LAN or a
                private VPN such as Tailscale, 100.64.x.x)
  --join ADDR   join a host before the run starts
  --survivor S  who you are to the others: llanero, coplera, encargado or muchacho
                (default: the choice saved from the menu)
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
                "--windowed" => launch.windowed = true,
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
                "--survivor" => {
                    let v = it
                        .next()
                        .ok_or("--survivor needs llanero, coplera, encargado or muchacho")?;
                    launch.survivor =
                        Some(crate::survivor::Survivor::parse(&v).ok_or_else(|| format!("bad --survivor value: {v}"))?);
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

/// A volume slider moved: let the player hear that bus now (never a
/// whistle; a menu must not cue him).
#[derive(Message, Clone, Copy, Debug)]
pub struct VolumePreview(pub Bus);

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
    let persist = !launch.driven();
    let profile = if persist {
        crate::profile::load()
    } else {
        crate::profile::Profile::default()
    };
    let menu = launch.menu;
    // Made in the saved mode, so a fullscreen player never sees a window
    // first (`Primary`: a window being made has no current monitor yet).
    let mode = display_mode(&profile.settings, &launch, MonitorSelection::Primary);

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
                    mode,
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
        .add_message::<VolumePreview>()
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
        )
        .add_systems(Update, apply_display.in_set(GameSet::Present))
        .add_systems(Update, crate::icon::apply);
    app
}

/// How the window shows: borderless over the whole screen when the player
/// wants it, except for the debug drivers and `--windowed`, which keep a
/// window of the launch size. `Settings.display_mode` is the only choice;
/// this decides what it means for this launch. `monitor` is `Primary` at
/// creation (a window being made has no current monitor yet).
pub fn display_mode(settings: &Settings, launch: &Launch, monitor: MonitorSelection) -> WindowMode {
    match settings.display_mode {
        DisplayMode::Fullscreen if !launch.forces_window() => WindowMode::BorderlessFullscreen(monitor),
        _ => WindowMode::Windowed,
    }
}

/// The share of the screen a restored window may take, leaving room for
/// its title bar and the taskbar.
const WINDOW_ROOM: f32 = 0.85;

/// The logical size of a window of the launch `size` on a screen of
/// `screen` logical pixels: never larger than asked, shrunk with its shape
/// kept to fit well inside the screen (1600×900 is larger than a 1080p
/// laptop at 125 % scaling). An unknown screen leaves the size alone.
pub fn fit_window(size: (u32, u32), screen: Option<Vec2>) -> Vec2 {
    let size = Vec2::new(size.0 as f32, size.1 as f32);
    let Some(screen) = screen.filter(|s| s.is_finite() && s.min_element() > 0.0) else {
        return size;
    };
    let scale = (WINDOW_ROOM * screen / size).min_element().min(1.0);
    (size * scale).round()
}

/// The window follows the settings (it was made in their mode already).
/// Leaving fullscreen brings back a centred window of the launch size,
/// fitted to the screen it was on: one made fullscreen has no size or
/// place of its own to return to.
fn apply_display(
    settings: Res<Settings>,
    launch: Res<Launch>,
    window: Single<(&mut Window, Option<&OnMonitor>), With<PrimaryWindow>>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
) {
    if !settings.is_changed() {
        return;
    }
    let (mut window, on) = window.into_inner();
    let want = display_mode(&settings, &launch, MonitorSelection::Current);
    // Only fullscreen or not counts: it was made on the primary monitor.
    let windowed = matches!(want, WindowMode::Windowed);
    if matches!(window.mode, WindowMode::Windowed) == windowed {
        return;
    }
    window.mode = want;
    if windowed {
        // The screen it is leaving, else the main one (a window learns its
        // monitor a moment after it is made).
        let screen = on
            .and_then(|m| monitors.get(m.0).ok())
            .or_else(|| monitors.iter().find(|&(_, primary)| primary))
            .map(|(m, _)| m.physical_size().as_vec2() / m.scale_factor as f32);
        let size = fit_window(launch.size, screen);
        window.resolution.set(size.x, size.y);
        window.position = WindowPosition::Centered(MonitorSelection::Current);
    }
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
    if launch.driven() {
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
    if lost && !launch.driven() && *state.get() == Flow::Playing {
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
        assert!(args("--windowed").unwrap().windowed);
    }

    #[test]
    fn a_new_player_starts_fullscreen_and_the_debug_drivers_never_do() {
        let fresh = Settings::default();
        let full = |settings: &Settings, line: &str| {
            let launch = args(line).unwrap();
            !matches!(
                display_mode(settings, &launch, MonitorSelection::Primary),
                WindowMode::Windowed
            )
        };
        for line in ["", "--play", "--seed 7 --night hard", "--host 127.0.0.1:5000"] {
            assert!(full(&fresh, line), "a new profile fills the screen: {line:?}");
        }
        for line in [
            "--windowed",
            "--play --windowed",
            "--smoke",
            "--tour",
            "--photos",
            "--trailer",
            "--menu-shots",
            "--host 127.0.0.1:5000 --net-smoke",
        ] {
            assert!(!full(&fresh, line), "{line:?} keeps its window");
        }
        let chosen = Settings {
            display_mode: DisplayMode::Window,
            ..Settings::default()
        };
        assert!(!full(&chosen, ""), "a player who chose a window keeps it");
        assert_eq!(DisplayMode::Window.toggled().toggled(), DisplayMode::Window);
    }

    #[test]
    fn only_a_plain_launch_opens_on_the_cover() {
        for line in ["", "--windowed", "--seed 7 --night hard"] {
            assert!(args(line).unwrap().splash(), "{line:?} shows the cover");
        }
        for line in [
            "--play",
            "--host 127.0.0.1:5000",
            "--join 127.0.0.1:5000",
            "--smoke",
            "--tour",
            "--photos",
            "--trailer",
            "--menu-shots",
            "--host 127.0.0.1:5000 --net-smoke",
            "--join 127.0.0.1:5000 --net-smoke --headless",
        ] {
            assert!(!args(line).unwrap().splash(), "{line:?} goes straight in");
        }
    }

    #[test]
    fn a_restored_window_fits_the_screen_it_is_on() {
        let asked = (1600, 900);
        // A 1080p screen at 100 %: room enough, the window is as asked.
        assert_eq!(
            fit_window(asked, Some(Vec2::new(1920.0, 1080.0))),
            Vec2::new(1600.0, 900.0)
        );
        // At 125 % (1536×864 logical) and 150 %: smaller, same shape, inside.
        for screen in [
            Vec2::new(1536.0, 864.0),
            Vec2::new(1280.0, 720.0),
            Vec2::new(900.0, 1600.0),
        ] {
            let got = fit_window(asked, Some(screen));
            assert!(got.x < screen.x && got.y < screen.y, "{got} inside {screen}");
            assert!((got.x / got.y - 16.0 / 9.0).abs() < 0.01, "{got} keeps its shape");
        }
        // An unknown or nonsense screen leaves the asked size.
        assert_eq!(fit_window(asked, None), Vec2::new(1600.0, 900.0));
        assert_eq!(fit_window(asked, Some(Vec2::ZERO)), Vec2::new(1600.0, 900.0));
    }

    /// The primary window after one pass of `apply_display`: it starts in
    /// `mode` on a 1080p screen at 125 % scaling, and the player wants
    /// `choice` on a launch from `line`.
    fn displayed(mode: WindowMode, choice: DisplayMode, line: &str) -> Window {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let screen = world
            .spawn((
                Monitor {
                    name: None,
                    physical_height: 1080,
                    physical_width: 1920,
                    physical_position: IVec2::ZERO,
                    refresh_rate_millihertz: None,
                    scale_factor: 1.25,
                    video_modes: Vec::new(),
                },
                PrimaryMonitor,
            ))
            .id();
        let mut window = Window { mode, ..default() };
        window.resolution.set_scale_factor(1.25);
        window.resolution.set_physical_resolution(1920, 1080);
        let id = world.spawn((window, PrimaryWindow, OnMonitor(screen))).id();
        world.insert_resource(Settings {
            display_mode: choice,
            ..Settings::default()
        });
        world.insert_resource(args(line).unwrap());
        world.run_system_once(apply_display).expect("one primary window");
        world.get::<Window>(id).expect("still there").clone()
    }

    #[test]
    fn the_window_follows_the_display_choice() {
        let full = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);
        // Made fullscreen and still wanted so: untouched (not moved from the
        // primary screen to the current one, not resized).
        let kept = displayed(full, DisplayMode::Fullscreen, "");
        assert_eq!(kept.mode, full);
        assert_eq!(kept.position, WindowPosition::Automatic);
        assert_eq!(kept.resolution.physical_size(), UVec2::new(1920, 1080));
        // Leaving it: a centred window of the launch size, fitted to the
        // 1536×864 logical screen it was on.
        let left = displayed(full, DisplayMode::Window, "");
        assert_eq!(left.mode, WindowMode::Windowed);
        assert_eq!(left.position, WindowPosition::Centered(MonitorSelection::Current));
        let want = fit_window((1600, 900), Some(Vec2::new(1536.0, 864.0)));
        assert!(
            (left.resolution.size() - want).abs().max_element() <= 1.0,
            "{}",
            left.resolution.size()
        );
        assert!(left.resolution.width() < 1536.0 && left.resolution.height() < 864.0);
        // A window asked back to fullscreen goes where it is now.
        let entered = displayed(WindowMode::Windowed, DisplayMode::Fullscreen, "");
        assert_eq!(
            entered.mode,
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        );
        // `--windowed` and the debug drivers keep their window whatever is saved.
        for line in ["--windowed", "--menu-shots", "--smoke"] {
            let held = displayed(WindowMode::Windowed, DisplayMode::Fullscreen, line);
            assert_eq!(held.mode, WindowMode::Windowed, "{line}");
            assert_eq!(held.resolution.physical_size(), UVec2::new(1920, 1080), "{line}");
        }
    }

    #[test]
    fn no_sub_slider_quiets_the_master_bus() {
        let subs = [Bus::Music, Bus::Ambience, Bus::Effects];
        let mut s = Settings::default();
        let master = s.gain(Bus::Master);
        let db = 20.0 * master.log10();
        assert!((-14.0..=-10.0).contains(&db), "default {db:.1} dB");
        assert!(subs.iter().all(|&b| s.gain(b) <= master));
        (s.music, s.ambience, s.effects) = (0.0, 0.0, 0.0);
        assert_eq!(s.gain(Bus::Master), master, "the master bus follows the master alone");
        assert!(subs.iter().all(|&b| s.gain(b) == 0.0), "a bus at 0% is silent");
        s.master = 0.0;
        assert_eq!(s.gain(Bus::Master), 0.0, "0% master is silence");
    }
}
