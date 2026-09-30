//! The front end: the title screen's menus over the llano, the pause menu
//! and the choices after a night.
//!
//! One model drives every page: a page is a heading, some words and a list
//! of rows (buttons, `‹ value ›` choosers and small text fields). The rows
//! are rebuilt when the page changes; their text and focus are refreshed
//! every frame. Keyboard (arrows or WASD, Enter, Esc) and mouse both work,
//! through one rule (`choose`): a click on ‹ is the left key, on › the
//! right one, on the row itself Enter; left and right only turn choosers.
//! Nothing here touches the rules: a night begins or ends through
//! `net::StartRun` / `net::LeaveRun`, restarts through the session command.

use std::net::SocketAddr;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use super::{AMBER, DIM, Fonts, INK, PALE_BLUE, RED, font, set_text, set_vis};
use crate::app::{DisplayMode, Flow, GameSet, Launch, ProfileRes, Settings, Truth, TuningRes, VolumePreview};
use crate::display;
use crate::encounter::PagesRead;
use crate::lang::{Lang, menu as m};
use crate::mix::Bus;
use crate::net::transport::Mode;
use crate::net::{LeaveRun, NetControl, Network, StartRun, protocol::Action};
use crate::tuning::Night;

/// Which page of the menus is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Main,
    Solo,
    Multiplayer,
    Host,
    Join,
    Journal,
    Reading(u8),
    /// The settings, in three groups (each its own page, so none outgrows
    /// the pause card).
    Settings,
    Video,
    Audio,
    Controls,
    /// Brightness and contrast over three hats (`calibrate`); offered once
    /// on the first title screen.
    Calibrate,
    HowTo,
    Credits,
    Pause,
    Outcome,
    ConfirmLeave,
    ConfirmQuit,
    /// One of the Madrina's chapters, kept in the Journal.
    Chapter(u8),
}

/// A text field a row edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Seed,
    HostAddr,
    JoinAddr,
}

/// A setting a row adjusts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Knob {
    Master,
    Music,
    Ambience,
    Effects,
    Sensitivity,
    InvertY,
    Fov,
    Brightness,
    Contrast,
    HeadBob,
    Captions,
    DisplayMode,
    /// English or Spanish (this player's alone).
    Language,
}

impl Knob {
    /// The volume slider this knob is, if it is one.
    fn bus(self) -> Option<Bus> {
        match self {
            Knob::Master => Some(Bus::Master),
            Knob::Music => Some(Bus::Music),
            Knob::Ambience => Some(Bus::Ambience),
            Knob::Effects => Some(Bus::Effects),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    Go(Page),
    Back,
    Difficulty,
    Survivor,
    Edit(Field),
    Adjust(Knob),
    /// Brightness and contrast back to the night as graded.
    PictureDefaults,
    Solo,
    Host,
    Join,
    Read(u8),
    Resume,
    Restart,
    NewNight,
    Leave,
    Title,
    Quit,
    None,
    Chapter(u8),
}

impl Act {
    /// A chooser: its ‹ and › (and the left and right keys) turn it one
    /// step either way. Every other row is a button.
    fn turns(self) -> bool {
        matches!(self, Act::Difficulty | Act::Survivor | Act::Adjust(_))
    }
}

struct Row {
    label: String,
    value: Option<String>,
    act: Act,
    enabled: bool,
}

impl Row {
    fn go(label: &str, act: Act) -> Self {
        Self {
            label: label.to_string(),
            value: None,
            act,
            enabled: true,
        }
    }
    fn value(label: &str, value: String, act: Act) -> Self {
        Self {
            label: label.to_string(),
            value: Some(value),
            act,
            enabled: true,
        }
    }
    fn off(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// A run of a page's rows laid side by side in `cols` columns, each filled
/// top to bottom, so the order of the rows (and Up / Down through them)
/// reads down one column and on into the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Columns {
    start: usize,
    len: usize,
    cols: usize,
}

impl Columns {
    /// Rows in each column.
    fn lines(self) -> usize {
        self.len.div_ceil(self.cols)
    }
    /// Row `i`'s column and line, if it is in this run.
    fn place(self, i: usize) -> Option<(usize, usize)> {
        let k = i.checked_sub(self.start).filter(|&k| k < self.len)?;
        Some((k / self.lines(), k % self.lines()))
    }
}

/// The page's rows that sit in columns: the Journal's twenty pages and six
/// chapters, two columns each (one long list outgrows a 720-px screen).
/// Every other row is a full-width line of its own.
fn columns(page: Page) -> Vec<Columns> {
    let pages = crate::lore::PAGES as usize;
    match page {
        Page::Journal => vec![
            Columns {
                start: 0,
                len: pages,
                cols: 2,
            },
            Columns {
                start: pages,
                len: crate::lore::CHAPTERS as usize,
                cols: 2,
            },
        ],
        _ => Vec::new(),
    }
}

/// Left (-1) or right (+1) from row `focus` across its run of columns: the
/// row that can be chosen in the next column that way, nearest in line
/// (the higher on a tie). Nowhere to go keeps the focus.
fn across(rows: &[Row], runs: &[Columns], focus: usize, dir: i32) -> usize {
    let Some((run, (col, line))) = runs.iter().find_map(|r| r.place(focus).map(|p| (*r, p))) else {
        return focus;
    };
    let Some(to) = col.checked_add_signed(dir.signum() as isize).filter(|&c| c < run.cols) else {
        return focus;
    };
    (run.start..run.start + run.len)
        .filter(|&i| rows.get(i).is_some_and(|r| r.enabled))
        .filter_map(|i| run.place(i).filter(|&(c, _)| c == to).map(|(_, l)| (i, l)))
        .min_by_key(|&(_, l)| l.abs_diff(line))
        .map_or(focus, |(i, _)| i)
}

/// The menus' state: the page, the focused row, where Back goes, and the
/// choices being made on the way into a night.
#[derive(Resource)]
pub(crate) struct Menu {
    pub page: Page,
    focus: usize,
    back: Vec<Page>,
    night: Night,
    seed: String,
    host: String,
    join: String,
    /// A line under the rows: why something could not start, or a hint.
    note: String,
    /// What the screen shows now (page, rows, where), to know when to rebuild.
    built: Option<(Page, usize, Flow)>,
    caret: f32,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            page: Page::Main,
            focus: 0,
            back: Vec::new(),
            night: Night::Normal,
            seed: String::new(),
            host: format!("{}:5000", lan_address()),
            join: String::new(),
            note: String::new(),
            built: None,
            caret: 0.0,
        }
    }
}

impl Menu {
    fn open(&mut self, page: Page) {
        self.back.push(self.page);
        self.show(page);
    }
    fn show(&mut self, page: Page) {
        self.page = page;
        self.focus = 0;
        self.note.clear();
    }
    fn go_back(&mut self) -> bool {
        match self.back.pop() {
            Some(p) => {
                self.page = p;
                self.focus = 0;
                self.note.clear();
                true
            }
            None => false,
        }
    }
    fn reset_to(&mut self, page: Page) {
        self.back.clear();
        self.show(page);
    }
    /// Open a page directly (the `--menu-shots` driver).
    pub(crate) fn jump(&mut self, page: Page) {
        self.reset_to(page);
    }
    /// The calibration page is on screen. `page` outlives a closed menu, so
    /// anything that hides the night for it asks this, never the page alone.
    pub(crate) fn calibrating(&self, flow: Flow) -> bool {
        menu_up(&flow) && self.page == Page::Calibrate
    }
}

/// This machine's address on the local network, as friends would reach it
/// (asks the routing table; nothing is sent). Loopback when there is none.
fn lan_address() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.168.1.1:9")?;
            s.local_addr()
        })
        .ok()
        .map(|a| a.ip())
        .filter(|ip| match ip {
            std::net::IpAddr::V4(v) => v.is_private(),
            _ => false,
        })
        .map_or_else(|| "127.0.0.1".to_string(), |ip| ip.to_string())
}

fn parse_addr(text: &str, l: Lang) -> Result<SocketAddr, String> {
    let text = text.trim();
    let full = if text.contains(':') {
        text.to_string()
    } else {
        format!("{text}:5000")
    };
    let addr: SocketAddr = full.parse().map_err(|_| match l {
        Lang::En => format!("\"{text}\" is not an address like 192.168.1.20:5000"),
        Lang::Es => format!("\"{text}\" no es una dirección como 192.168.1.20:5000"),
    })?;
    crate::net::transport::local_address(addr)
}

/// A night's difficulty, as the menus name it.
pub(crate) fn night_label(n: Night, l: Lang) -> &'static str {
    l.say(match n {
        Night::Gentle => m::GENTLE,
        Night::Normal => m::NORMAL,
        Night::Hard => m::HARD,
    })
}

fn night_blurb(n: Night, l: Lang) -> &'static str {
    l.say(match n {
        Night::Gentle => m::GENTLE_BLURB,
        Night::Normal => m::NORMAL_BLURB,
        Night::Hard => m::HARD_BLURB,
    })
}

/// Everything a menu choice may change.
#[derive(SystemParam)]
struct Effects<'w> {
    next: ResMut<'w, NextState<Flow>>,
    starts: MessageWriter<'w, StartRun>,
    leaves: MessageWriter<'w, LeaveRun>,
    control: MessageWriter<'w, NetControl>,
    exit: MessageWriter<'w, AppExit>,
    settings: ResMut<'w, Settings>,
    profile: ResMut<'w, ProfileRes>,
    tuning: Res<'w, TuningRes>,
    preview: MessageWriter<'w, VolumePreview>,
}

/// What the page says about the world right now.
#[derive(SystemParam)]
struct Context<'w> {
    launch: Res<'w, Launch>,
    read: Res<'w, PagesRead>,
    truth: Res<'w, Truth>,
}

fn rows(menu: &Menu, settings: &Settings, ctx: &Context, profile: &ProfileRes) -> (String, String, Vec<Row>) {
    let l = settings.lang;
    let host_side = ctx.launch.network.is_host();
    let solo = ctx.launch.network.is_solo();
    let field = |text: &str, empty: &str| {
        if text.is_empty() {
            empty.to_string()
        } else {
            text.to_string()
        }
    };
    // Choosers show their value bare: their ‹ and › are buttons of their own
    // (see `build`).
    let on = |b: bool| l.say(if b { m::ON } else { m::OFF }).to_string();
    let brightness = || {
        Row::value(
            l.say(m::BRIGHTNESS),
            format!("{:+.1}", settings.brightness),
            Act::Adjust(Knob::Brightness),
        )
    };
    let contrast = || {
        Row::value(
            l.say(m::CONTRAST),
            format!("{:.2}", settings.contrast),
            Act::Adjust(Knob::Contrast),
        )
    };
    let back = || Row::go(l.say(m::BACK), Act::Back);
    let saved = || l.say(m::SAVED).to_string();
    match menu.page {
        Page::Main => (
            String::new(),
            String::new(),
            vec![
                Row::go(l.say(m::PLAY), Act::Go(Page::Solo)),
                Row::go(l.say(m::PLAY_FRIENDS), Act::Go(Page::Multiplayer)),
                Row::go(l.say(m::JOURNAL), Act::Go(Page::Journal)),
                Row::go(l.say(m::SETTINGS), Act::Go(Page::Settings)),
                Row::go(l.say(m::HOW_TO_PLAY), Act::Go(Page::HowTo)),
                Row::go(l.say(m::CREDITS), Act::Go(Page::Credits)),
                Row::go(l.say(m::QUIT), Act::Go(Page::ConfirmQuit)),
            ],
        ),
        Page::Solo => (
            l.say(m::SOLO_HEADING).into(),
            format!("{}\n{}", night_blurb(menu.night, l), l.say(m::SOLO_BODY)),
            vec![
                Row::value(
                    l.say(m::DIFFICULTY),
                    night_label(menu.night, l).to_string(),
                    Act::Difficulty,
                ),
                Row::value(
                    l.say(m::NIGHT),
                    field(&menu.seed, l.say(m::A_NEW_NIGHT_FIELD)),
                    Act::Edit(Field::Seed),
                ),
                Row::go(l.say(m::BEGIN), Act::Solo),
                back(),
            ],
        ),
        Page::Multiplayer => (
            l.say(m::FRIENDS_HEADING).into(),
            l.say(m::FRIENDS_BODY).into(),
            vec![
                Row::value(
                    l.say(m::WHO_YOU_ARE),
                    format!(
                        "{} · {}",
                        profile.profile.survivor.name(),
                        profile.profile.survivor.role(l)
                    ),
                    Act::Survivor,
                ),
                Row::go(l.say(m::HOST), Act::Go(Page::Host)),
                Row::go(l.say(m::JOIN_FRIEND), Act::Go(Page::Join)),
                back(),
            ],
        ),
        Page::Host => (
            l.say(m::HOST).into(),
            format!("{}\n{}", l.say(m::HOST_BODY), night_blurb(menu.night, l)),
            vec![
                Row::value(l.say(m::YOUR_ADDRESS), menu.host.clone(), Act::Edit(Field::HostAddr)),
                Row::value(
                    l.say(m::DIFFICULTY),
                    night_label(menu.night, l).to_string(),
                    Act::Difficulty,
                ),
                Row::value(
                    l.say(m::NIGHT),
                    field(&menu.seed, l.say(m::A_NEW_NIGHT_FIELD)),
                    Act::Edit(Field::Seed),
                ),
                Row::go(l.say(m::OPEN_SESSION), Act::Host),
                back(),
            ],
        ),
        Page::Join => (
            l.say(m::JOIN_FRIEND).into(),
            l.say(m::JOIN_BODY).into(),
            vec![
                Row::value(
                    l.say(m::HOSTS_ADDRESS),
                    field(&menu.join, l.say(m::TYPE_IT_HERE)),
                    Act::Edit(Field::JoinAddr),
                ),
                Row::go(l.say(m::JOIN), Act::Join),
                back(),
            ],
        ),
        Page::Journal => {
            let t = &profile.profile.tally;
            let fastest = t
                .fastest
                .map_or("—".to_string(), |s| format!("{}:{:02}", s as u32 / 60, s as u32 % 60));
            let mut rows: Vec<Row> = (0..crate::lore::PAGES)
                .map(|id| {
                    let page = crate::lore::note(id);
                    if ctx.read.0.contains(&id) {
                        Row::go(l.say(page.title), Act::Read(id))
                    } else {
                        Row::go(l.say(m::NOT_FOUND), Act::None).off()
                    }
                })
                .collect();
            // The Madrina's tale, as far as it has been told.
            for n in 1..=crate::lore::CHAPTERS {
                match crate::lore::chapter(n) {
                    Some(c) if profile.profile.chapters.contains(&n) => {
                        rows.push(Row::go(l.say(c.title), Act::Chapter(n)))
                    }
                    _ => rows.push(Row::go(l.say(m::NOT_TOLD), Act::None).off()),
                }
            }
            rows.push(back());
            let found = ctx.read.0.len();
            let pages = crate::lore::PAGES;
            let body = match l {
                Lang::En => format!(
                    "Pages of the tale found: {found}/{pages}   ·   Nights: {}   ·   Escapes: {}   ·   Laid to rest: {}   ·   Caught: {}   ·   Lived till dawn: {}   ·   Fastest: {fastest}",
                    t.nights, t.escapes, t.banishments, t.caught, t.dawns
                ),
                Lang::Es => format!(
                    "Páginas del cuento encontradas: {found}/{pages}   ·   Noches: {}   ·   Escapes: {}   ·   Lo hiciste descansar: {}   ·   Atrapado: {}   ·   Hasta el alba: {}   ·   La más rápida: {fastest}",
                    t.nights, t.escapes, t.banishments, t.caught, t.dawns
                ),
            };
            (l.say(m::JOURNAL).into(), body, rows)
        }
        Page::Chapter(n) => {
            let (title, text) = crate::lore::chapter(n).map_or(("", ""), |c| (l.say(c.title), c.text(l)));
            (format!("La Madrina — {title}"), text.to_string(), vec![back()])
        }
        Page::Reading(id) => {
            // No page carries a night's digits: the radio reads them out.
            let page = crate::lore::note(id);
            (
                format!("{} — {}", page.medium.label(l), l.say(page.title)),
                format!("{}\n\n{}", page.text(l), l.say(page.by)),
                vec![back()],
            )
        }
        Page::Settings => (
            l.say(m::SETTINGS).into(),
            saved(),
            vec![
                Row::go(l.say(m::VIDEO), Act::Go(Page::Video)),
                Row::go(l.say(m::AUDIO), Act::Go(Page::Audio)),
                Row::go(l.say(m::CONTROLS), Act::Go(Page::Controls)),
                Row::go(l.say(m::CALIBRATE), Act::Go(Page::Calibrate)),
                // Named in both languages, so it can be found from either.
                Row::value(m::LANGUAGE, l.name().to_string(), Act::Adjust(Knob::Language)),
                back(),
            ],
        ),
        Page::Video => {
            let display = if ctx.launch.windowed {
                // `--windowed` keeps this launch in a window and leaves the
                // saved choice alone, so the row cannot change it.
                Row::value(l.say(m::DISPLAY_MODE), l.say(m::WINDOWED_LAUNCH).into(), Act::None).off()
            } else {
                let mode = match settings.display_mode {
                    DisplayMode::Fullscreen => m::FULLSCREEN,
                    DisplayMode::Window => m::WINDOW,
                };
                Row::value(
                    l.say(m::DISPLAY_MODE),
                    l.say(mode).to_string(),
                    Act::Adjust(Knob::DisplayMode),
                )
            };
            (
                l.say(m::VIDEO).into(),
                saved(),
                vec![
                    display,
                    Row::value(l.say(m::FOV), format!("{:.0}°", settings.fov), Act::Adjust(Knob::Fov)),
                    brightness(),
                    contrast(),
                    Row::go(l.say(m::CALIBRATE), Act::Go(Page::Calibrate)),
                    Row::value(l.say(m::HEAD_BOB), on(settings.head_bob), Act::Adjust(Knob::HeadBob)),
                    back(),
                ],
            )
        }
        Page::Calibrate => (
            l.say(m::BRIGHTNESS).into(),
            format!(
                "{}{}",
                l.say(m::CALIBRATE_BODY),
                if solo { "" } else { l.say(m::SHARED_BEHIND) }
            ),
            vec![
                brightness(),
                contrast(),
                Row::go(l.say(m::DEFAULTS), Act::PictureDefaults),
                Row::go(l.say(m::DONE), Act::Back),
            ],
        ),
        Page::Audio => {
            let level = |v: f32| format!("{:.0}%", v * 100.0);
            (
                l.say(m::AUDIO).into(),
                l.say(m::AUDIO_BODY).into(),
                vec![
                    Row::value(l.say(m::MASTER), level(settings.master), Act::Adjust(Knob::Master)),
                    Row::value(l.say(m::MUSIC), level(settings.music), Act::Adjust(Knob::Music)),
                    Row::value(
                        l.say(m::AMBIENCE),
                        level(settings.ambience),
                        Act::Adjust(Knob::Ambience),
                    ),
                    Row::value(l.say(m::EFFECTS), level(settings.effects), Act::Adjust(Knob::Effects)),
                    Row::value(l.say(m::CAPTIONS), on(settings.captions), Act::Adjust(Knob::Captions)),
                    back(),
                ],
            )
        }
        Page::Controls => (
            l.say(m::CONTROLS).into(),
            saved(),
            vec![
                Row::value(
                    l.say(m::SENSITIVITY),
                    format!("{:.1}×", settings.sensitivity),
                    Act::Adjust(Knob::Sensitivity),
                ),
                Row::value(l.say(m::INVERT_Y), on(settings.invert_y), Act::Adjust(Knob::InvertY)),
                back(),
            ],
        ),
        Page::HowTo => (l.say(m::HOW_TO_PLAY).into(), l.say(m::HOW_TO).into(), vec![back()]),
        Page::Credits => (l.say(m::CREDITS).into(), l.say(m::CREDITS_TEXT).into(), vec![back()]),
        Page::Pause => {
            let mut v = vec![
                Row::go(l.say(m::RESUME), Act::Resume),
                Row::go(l.say(m::SETTINGS), Act::Go(Page::Settings)),
                Row::go(l.say(m::JOURNAL), Act::Go(Page::Journal)),
                Row::go(l.say(m::HOW_TO_PLAY), Act::Go(Page::HowTo)),
            ];
            if host_side {
                v.push(Row::go(l.say(m::RESTART), Act::Restart));
            }
            v.push(Row::go(
                l.say(if solo || !host_side {
                    m::LEAVE_TO_TITLE
                } else {
                    m::END_FOR_ALL
                }),
                Act::Go(Page::ConfirmLeave),
            ));
            (
                l.say(m::PAUSED).into(),
                l.say(if solo { m::PAUSED_SOLO } else { m::PAUSED_SHARED }).into(),
                v,
            )
        }
        Page::Outcome => {
            let mut v = Vec::new();
            if host_side {
                v.push(Row::go(l.say(m::PLAY_AGAIN), Act::Restart));
            }
            if solo {
                v.push(Row::go(l.say(m::NEW_NIGHT), Act::NewNight));
            }
            v.push(Row::go(l.say(m::TITLE_MENU), Act::Title));
            v.push(Row::go(l.say(m::QUIT), Act::Go(Page::ConfirmQuit)));
            let _ = &ctx.truth;
            (String::new(), String::new(), v)
        }
        Page::ConfirmLeave => (
            l.say(m::LEAVE_HEADING).into(),
            l.say(if !solo && host_side {
                m::LEAVE_SHARED
            } else {
                m::LEAVE_SOLO
            })
            .into(),
            vec![Row::go(l.say(m::STAY), Act::Back), Row::go(l.say(m::LEAVE), Act::Leave)],
        ),
        Page::ConfirmQuit => (
            l.say(m::QUIT_HEADING).into(),
            String::new(),
            vec![Row::go(l.say(m::STAY), Act::Back), Row::go(l.say(m::QUIT), Act::Quit)],
        ),
    }
}

// ------------------------------------------------------------------ building

#[derive(Component)]
pub(crate) struct MenuRoot;
#[derive(Component)]
struct MenuHeading;
#[derive(Component)]
struct MenuBody;
#[derive(Component)]
struct MenuNote;
#[derive(Component)]
struct MenuRow(usize);
#[derive(Component)]
struct RowLabel(usize);
#[derive(Component)]
struct RowValue(usize);
/// A chooser's ‹ (`dir` -1) or › (+1): a button of its own inside the row,
/// so a click on it turns the value that way and never presses the row.
#[derive(Component)]
struct RowArrow {
    row: usize,
    dir: i32,
}
/// The glyph on a chooser's arrow, coloured with its row.
#[derive(Component)]
struct ArrowGlyph(usize);

/// One of a chooser's arrows.
fn arrow(fonts: &Fonts, row: usize, dir: i32, color: Color) -> impl Bundle {
    (
        Button,
        RowArrow { row, dir },
        Node {
            padding: UiRect::axes(px(10), px(0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            ArrowGlyph(row),
            Text::new(if dir < 0 { "‹" } else { "›" }),
            font(&fonts.sans, 22.0),
            TextColor(color),
        )],
    )
}

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        Name::new("menus"),
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        Visibility::Hidden,
        GlobalZIndex(20),
    ));
}

/// One row: its label, and its value or chooser (`‹ value ›`) at the right.
/// A row with no `min_width` (the Journal's, in columns) is compact:
/// smaller, and as wide as its column.
fn spawn_row(
    c: &mut ChildSpawnerCommands<'_>,
    fonts: &Fonts,
    i: usize,
    row: &Row,
    focused: bool,
    min_width: Option<f32>,
) {
    let compact = min_width.is_none();
    let (text, bg, edge) = row_colors(focused, row.enabled);
    c.spawn((
        Button,
        MenuRow(i),
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(14), px(if compact { 3.0 } else { 7.0 })),
            border: UiRect::left(px(3)),
            min_width: min_width.map_or(Val::Auto, px),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(edge),
    ))
    .with_children(|row_ui| {
        row_ui.spawn((
            RowLabel(i),
            Text::new(row.label.clone()),
            font(&fonts.sans, if compact { 15.0 } else { 20.0 }),
            TextColor(text),
        ));
        let value = |v: &String| {
            (
                RowValue(i),
                Text::new(v.clone()),
                font(&fonts.sans, 18.0),
                TextColor(text),
            )
        };
        match &row.value {
            // A chooser: ‹ value ›, each arrow its own button.
            Some(v) if row.act.turns() => {
                row_ui
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|chooser| {
                        chooser.spawn(arrow(fonts, i, -1, text));
                        chooser.spawn(value(v));
                        chooser.spawn(arrow(fonts, i, 1, text));
                    });
            }
            Some(v) => {
                row_ui.spawn(value(v));
            }
            None => {}
        }
    });
}

/// The rows' look when focused or not.
fn row_colors(focused: bool, enabled: bool) -> (Color, Color, Color) {
    let text = if !enabled {
        Color::srgba(0.6, 0.58, 0.54, 0.45)
    } else if focused {
        AMBER
    } else {
        INK
    };
    let bg = if focused && enabled {
        Color::srgba(0.35, 0.24, 0.12, 0.45)
    } else {
        Color::NONE
    };
    let edge = if focused && enabled { AMBER } else { Color::NONE };
    (text, bg, edge)
}

#[allow(clippy::too_many_arguments)]
fn build(
    commands: &mut Commands,
    root: Entity,
    fonts: &Fonts,
    menu: &Menu,
    flow: Flow,
    heading: &str,
    body: &str,
    rows: &[Row],
    portrait: Option<(Handle<Image>, crate::survivor::Survivor)>,
    lang: Lang,
) {
    commands.entity(root).despawn_children();
    // The title screen keeps its rail; a paused night shows a card; after a
    // night the choices sit under the outcome (other pages there are cards).
    // The calibration leaves the screen to its hats, anywhere.
    let calibrating = menu.page == Page::Calibrate;
    let title_screen = flow == Flow::Title && !calibrating;
    let in_run_card = !calibrating && (flow == Flow::Paused || (flow == Flow::Outcome && menu.page != Page::Outcome));
    let reading = matches!(
        menu.page,
        Page::Reading(_) | Page::Chapter(_) | Page::HowTo | Page::Credits
    );
    let runs = columns(menu.page);
    let journal = !runs.is_empty();
    // Rails and cards widen for a page to read, and more for columns.
    let wide = if journal {
        860.0
    } else if reading {
        760.0
    } else {
        0.0
    };
    commands.entity(root).with_children(|r| {
        // Where the rows sit: a dark rail on the left of the title screen,
        // a centred card in a run, a row of choices under the outcome.
        let mut column = Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        };
        let mut bg = BackgroundColor(Color::NONE);
        let mut gradient = None;
        if calibrating {
            // Low, dark and out of the way: bright words beside the hats
            // would change what black looks like.
            column.position_type = PositionType::Absolute;
            column.left = percent(50);
            column.bottom = percent(4);
            column.width = px(520);
            column.margin = UiRect::left(px(-260));
            column.padding = UiRect::all(px(18));
            bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85));
            for (i, label) in [m::HAT_VANISH, m::HAT_BARELY, m::HAT_PLAIN]
                .map(|w| lang.say(w))
                .into_iter()
                .enumerate()
            {
                r.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent((1.0 + super::calibrate::HAT_X[i]) * 50.0),
                        top: percent(super::calibrate::LABEL_TOP),
                        width: px(220),
                        margin: UiRect::left(px(-110)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![(
                        Text::new(label),
                        font(&fonts.sans, 15.0),
                        TextColor(DIM),
                        TextLayout::justify(Justify::Center),
                    )],
                ));
            }
        } else if title_screen {
            column.position_type = PositionType::Absolute;
            column.left = px(0);
            column.top = px(0);
            column.height = percent(100);
            column.width = px(if wide > 0.0 { wide } else { 600.0 });
            column.padding = UiRect::new(px(64), px(40), px(56), px(40));
            column.justify_content = JustifyContent::Center;
            gradient = Some(BackgroundGradient::from(LinearGradient::to_right(vec![
                ColorStop::percent(Color::srgba(0.0, 0.0, 0.01, 0.92), 0),
                ColorStop::percent(Color::srgba(0.0, 0.0, 0.01, 0.75), 70),
                ColorStop::percent(Color::srgba(0.0, 0.0, 0.01, 0.0), 100),
            ])));
        } else if in_run_card {
            column.position_type = PositionType::Absolute;
            column.left = percent(50);
            column.top = percent(50);
            let width = if wide > 0.0 { wide } else { 520.0 };
            column.width = px(width);
            column.margin = UiRect::new(px(-width / 2.0), px(0), px(-300), px(0));
            column.padding = UiRect::all(px(28));
            column.border = UiRect::all(px(1));
            column.border_radius = BorderRadius::all(px(8));
            bg = BackgroundColor(Color::srgba(0.02, 0.022, 0.028, 0.94));
        } else {
            // Under the outcome card.
            column.position_type = PositionType::Absolute;
            column.left = percent(50);
            column.bottom = percent(6);
            column.width = px(360);
            column.margin = UiRect::left(px(-180));
        }
        // Who you will be to the others, beside the rail.
        if let Some((image, who)) = &portrait {
            r.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(610),
                top: percent(50),
                margin: UiRect::top(px(-300)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|p| {
                p.spawn((
                    ImageNode::new(image.clone()),
                    Node {
                        width: px(360),
                        height: px(540),
                        ..default()
                    },
                ));
                p.spawn((Text::new(who.name()), font(&fonts.serif, 30.0), TextColor(INK)));
                p.spawn((Text::new(who.role(lang)), font(&fonts.italic, 18.0), TextColor(AMBER)));
            });
        }
        let mut col = r.spawn((column, bg));
        if let Some(g) = gradient {
            col.insert(g);
        }
        if in_run_card {
            col.insert(BorderColor::all(Color::srgba(0.8, 0.62, 0.38, 0.35)));
        }
        col.with_children(|c| {
            if menu.page == Page::Main {
                c.spawn((Text::new("EL SILBÓN"), font(&fonts.serif, 84.0), TextColor(INK)));
                c.spawn((
                    Text::new("The Return"),
                    font(&fonts.italic, 26.0),
                    TextColor(AMBER),
                    Node {
                        margin: UiRect::bottom(px(6)),
                        ..default()
                    },
                ));
                c.spawn((
                    Text::new(lang.say(m::TAGLINE)),
                    font(&fonts.italic, 16.0),
                    TextColor(PALE_BLUE),
                    Node {
                        margin: UiRect::bottom(px(36)),
                        ..default()
                    },
                ));
            } else if !heading.is_empty() {
                c.spawn((
                    MenuHeading,
                    Text::new(heading),
                    font(&fonts.serif, 34.0),
                    TextColor(INK),
                ));
            }
            if !body.is_empty() {
                c.spawn((
                    MenuBody,
                    Text::new(body),
                    font(
                        if reading { &fonts.italic } else { &fonts.sans },
                        if reading { 16.0 } else { 15.0 },
                    ),
                    TextColor(if reading { INK } else { DIM }),
                    Node {
                        margin: UiRect::bottom(px(18)),
                        max_width: px(if wide > 0.0 { wide - 100.0 } else { 470.0 }),
                        ..default()
                    },
                ));
            }
            // Rows in a run of columns sit side by side (see `columns`);
            // the rest are full-width lines.
            let width = (!journal).then_some(if menu.page == Page::Outcome { 360.0 } else { 440.0 });
            let mut i = 0;
            while i < rows.len() {
                let Some(run) = runs.iter().find(|r| r.start == i && r.len > 0) else {
                    spawn_row(c, fonts, i, &rows[i], i == menu.focus, width);
                    i += 1;
                    continue;
                };
                c.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(14),
                    margin: UiRect::bottom(px(8)),
                    ..default()
                })
                .with_children(|grid| {
                    for col in 0..run.cols {
                        let from = run.start + col * run.lines();
                        let to = (from + run.lines()).min(run.start + run.len);
                        grid.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            row_gap: px(2),
                            ..default()
                        })
                        .with_children(|column| {
                            for (k, row) in rows.iter().enumerate().take(to).skip(from) {
                                spawn_row(column, fonts, k, row, k == menu.focus, None);
                            }
                        });
                    }
                });
                i = (run.start + run.len).max(i + 1);
            }
            c.spawn((
                MenuNote,
                Text::new(""),
                font(&fonts.sans, 15.0),
                TextColor(RED),
                Node {
                    margin: UiRect::top(px(12)),
                    max_width: px(470),
                    ..default()
                },
            ));
        });
        if title_screen {
            r.spawn((
                Text::new(format!("alpha {}", env!("CARGO_PKG_VERSION"))),
                font(&fonts.sans, 13.0),
                TextColor(DIM),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(18),
                    bottom: px(14),
                    ..default()
                },
            ));
        }
    });
}

// -------------------------------------------------------------------- systems

/// The menus follow the flow: the title's main page on the title screen, the
/// pause page when paused, the choices after a night. The first title
/// screen a player's profile sees opens on the calibration, once (Done
/// leads on to the main page); the debug drivers never keep a profile.
fn follow_flow(
    state: Res<State<Flow>>,
    mut menu: ResMut<Menu>,
    mut profile: ResMut<ProfileRes>,
    mut last: Local<Option<Flow>>,
) {
    let now = *state.get();
    if *last == Some(now) {
        return;
    }
    *last = Some(now);
    match now {
        Flow::Title => {
            menu.reset_to(Page::Main);
            if profile.persist && !profile.profile.calibrated {
                menu.open(Page::Calibrate);
                profile.profile.calibrated = true;
                profile.save();
            }
        }
        Flow::Paused => menu.reset_to(Page::Pause),
        Flow::Outcome => menu.reset_to(Page::Outcome),
        _ => {}
    }
}

fn menu_up(state: &Flow) -> bool {
    matches!(state, Flow::Title | Flow::Paused | Flow::Outcome)
}

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    time: Res<Time<Real>>,
    state: Res<State<Flow>>,
    fonts: Res<Fonts>,
    settings: Res<Settings>,
    ctx: Context,
    profile: Res<ProfileRes>,
    assets: Res<AssetServer>,
    mut menu: ResMut<Menu>,
    root: Single<(Entity, &mut Visibility), With<MenuRoot>>,
    mut rows_ui: Query<(&MenuRow, &mut BackgroundColor, &mut BorderColor)>,
    mut texts: ParamSet<(
        Query<(&RowLabel, &mut Text, &mut TextColor)>,
        Query<(&RowValue, &mut Text, &mut TextColor)>,
        Query<&mut Text, With<MenuNote>>,
        Query<&mut Text, With<MenuBody>>,
        Query<(&ArrowGlyph, &mut TextColor)>,
    )>,
) {
    let (root, mut vis) = root.into_inner();
    let up = menu_up(state.get());
    set_vis(&mut vis, up);
    if !up {
        // Closed menus keep no rows around (a night's entity count must not
        // grow with every outcome or pause).
        if menu.built.take().is_some() {
            commands.entity(root).despawn_children();
        }
        return;
    }
    menu.caret += time.delta_secs();
    let (heading, body, rows) = rows(&menu, &settings, &ctx, &profile);
    if menu.focus >= rows.len() {
        menu.focus = rows.len().saturating_sub(1);
    }
    // A page opens on its first row; one that cannot be chosen hands the
    // focus to the first that can (Enter must never press a disabled row).
    if rows.get(menu.focus).is_some_and(|r| !r.enabled)
        && let Some(first) = rows.iter().position(|r| r.enabled)
    {
        menu.focus = first;
    }
    let shape = (menu.page, rows.len(), *state.get());
    if menu.built != Some(shape) {
        let portrait = (menu.page == Page::Multiplayer && *state.get() == Flow::Title).then(|| {
            let who = profile.profile.survivor;
            (assets.load(format!("ui/survivors/{}.png", who.key())), who)
        });
        build(
            &mut commands,
            root,
            &fonts,
            &menu,
            *state.get(),
            &heading,
            &body,
            &rows,
            portrait,
            settings.lang,
        );
        menu.built = Some(shape);
        return;
    }
    let caret = if ((menu.caret * 2.0) as u32).is_multiple_of(2) {
        "|"
    } else {
        " "
    };
    for (row, mut bg, mut edge) in &mut rows_ui {
        let Some(r) = rows.get(row.0) else { continue };
        let (_, b, e) = row_colors(row.0 == menu.focus, r.enabled);
        if bg.0 != b {
            bg.0 = b;
        }
        let want = BorderColor::all(e);
        if *edge != want {
            *edge = want;
        }
    }
    for (label, mut t, mut c) in &mut texts.p0() {
        let Some(r) = rows.get(label.0) else { continue };
        set_text(&mut t, &r.label);
        let (color, ..) = row_colors(label.0 == menu.focus, r.enabled);
        if c.0 != color {
            c.0 = color;
        }
    }
    for (value, mut t, mut c) in &mut texts.p1() {
        let Some(r) = rows.get(value.0) else { continue };
        let editing = value.0 == menu.focus && matches!(r.act, Act::Edit(_));
        let shown = match &r.value {
            Some(v) if editing => format!("{}{caret}", field_text(&menu, r.act).unwrap_or(v)),
            Some(v) => v.clone(),
            None => String::new(),
        };
        set_text(&mut t, &shown);
        let (color, ..) = row_colors(value.0 == menu.focus, r.enabled);
        let color = if editing { PALE_BLUE } else { color };
        if c.0 != color {
            c.0 = color;
        }
    }
    for mut t in &mut texts.p2() {
        set_text(&mut t, &menu.note);
    }
    for mut t in &mut texts.p3() {
        set_text(&mut t, &body);
    }
    for (glyph, mut c) in &mut texts.p4() {
        let Some(r) = rows.get(glyph.0) else { continue };
        let (color, ..) = row_colors(glyph.0 == menu.focus, r.enabled);
        if c.0 != color {
            c.0 = color;
        }
    }
}

fn field_text(menu: &Menu, act: Act) -> Option<&str> {
    match act {
        Act::Edit(Field::Seed) => Some(&menu.seed),
        Act::Edit(Field::HostAddr) => Some(&menu.host),
        Act::Edit(Field::JoinAddr) => Some(&menu.join),
        _ => None,
    }
}

/// One frame of the player's hands on a page, as read from the devices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Input {
    /// Left (-1) or right (+1): the arrow keys, or A / D outside a text
    /// field.
    turn: i32,
    /// Enter.
    enter: bool,
    /// The row the pointer came onto (over the row or one of its arrows).
    hover: Option<usize>,
    /// A click on a row, anywhere but its arrows.
    click: Option<usize>,
    /// A click on one of a chooser's arrows: its row, and -1 for ‹ or +1
    /// for ›.
    arrow: Option<(usize, i32)>,
}

/// Where the focus goes and what is done, in which direction, for one
/// frame's input. The mouse and the keys meet here: a click on ‹ is the
/// left key, a click on › the right one, a click on the row is Enter. Left
/// and right (keys or arrows) only turn choosers, both ways; they never
/// press a button. A row that cannot be chosen takes neither focus nor
/// clicks.
fn choose(rows: &[Row], focus: usize, input: Input) -> (usize, Option<(Act, i32)>) {
    let usable = |i: usize| rows.get(i).filter(|r| r.enabled).map(|r| r.act);
    let mut focus = focus;
    let mut chosen = None;
    if let Some(i) = input.hover.filter(|&i| usable(i).is_some()) {
        focus = i;
    }
    if let Some((i, act)) = input.click.and_then(|i| usable(i).map(|a| (i, a))) {
        focus = i;
        chosen = Some((act, 1));
    }
    if let Some((i, dir, act)) = input.arrow.and_then(|(i, d)| usable(i).map(|a| (i, d, a))) {
        focus = i;
        chosen = act.turns().then_some((act, dir.signum()));
    }
    let act = usable(focus).unwrap_or(Act::None);
    if input.turn != 0 && act.turns() {
        chosen = Some((act, input.turn.signum()));
    }
    if input.enter {
        chosen = Some((act, 1));
    }
    (focus, chosen.filter(|(a, _)| *a != Act::None))
}

/// Keyboard and mouse on the menus.
#[allow(clippy::too_many_arguments)]
fn navigate(
    state: Res<State<Flow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    buttons: Query<(&Interaction, &MenuRow), Changed<Interaction>>,
    arrows: Query<(&Interaction, &RowArrow), Changed<Interaction>>,
    ctx: Context,
    mut menu: ResMut<Menu>,
    mut fx: Effects,
) {
    if ctx.launch.driven() || !menu_up(state.get()) {
        typed.clear();
        return;
    }
    let (_, _, rows) = rows(&menu, &fx.settings, &ctx, &fx.profile);
    if rows.is_empty() {
        return;
    }
    let focus_row = rows.get(menu.focus).map(|r| r.act).unwrap_or(Act::None);
    // Typing into the focused field.
    let mut typed_any = false;
    for ev in typed.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        let Act::Edit(field) = focus_row else { continue };
        let text = match field {
            Field::Seed => &mut menu.seed,
            Field::HostAddr => &mut menu.host,
            Field::JoinAddr => &mut menu.join,
        };
        match &ev.logical_key {
            Key::Backspace => {
                text.pop();
                typed_any = true;
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    let ok = match field {
                        Field::Seed => ch.is_ascii_digit() && text.len() < 9,
                        _ => (ch.is_ascii_digit() || ch == '.' || ch == ':') && text.len() < 32,
                    };
                    if ok {
                        text.push(ch);
                        typed_any = true;
                    }
                }
            }
            _ => {}
        }
    }
    // Moving the focus (skipping rows that cannot be chosen).
    let step = |menu: &mut Menu, dir: i32| {
        let n = rows.len() as i32;
        let mut i = menu.focus as i32;
        for _ in 0..n {
            i = (i + dir).rem_euclid(n);
            if rows[i as usize].enabled {
                break;
            }
        }
        menu.focus = i as usize;
    };
    let editing = matches!(focus_row, Act::Edit(_));
    if keys.any_just_pressed([KeyCode::ArrowDown]) || (!editing && keys.just_pressed(KeyCode::KeyS)) {
        step(&mut menu, 1);
    }
    if keys.any_just_pressed([KeyCode::ArrowUp]) || (!editing && keys.just_pressed(KeyCode::KeyW)) {
        step(&mut menu, -1);
    }
    let mut input = Input::default();
    for (interaction, row) in &buttons {
        match interaction {
            Interaction::Hovered => input.hover = Some(row.0),
            Interaction::Pressed => input.click = Some(row.0),
            Interaction::None => {}
        }
    }
    // An arrow is a button inside its row: pressed, the row is not.
    for (interaction, arrow) in &arrows {
        match interaction {
            Interaction::Hovered => input.hover = Some(arrow.row),
            Interaction::Pressed => input.arrow = Some((arrow.row, arrow.dir)),
            Interaction::None => {}
        }
    }
    if !typed_any {
        if keys.any_just_pressed([KeyCode::ArrowLeft]) || (!editing && keys.just_pressed(KeyCode::KeyA)) {
            input.turn = -1;
        }
        if keys.any_just_pressed([KeyCode::ArrowRight]) || (!editing && keys.just_pressed(KeyCode::KeyD)) {
            input.turn = 1;
        }
    }
    // Left and right cross the Journal's columns (its rows turn nothing).
    if input.turn != 0 && !rows.get(menu.focus).is_some_and(|r| r.act.turns()) {
        menu.focus = across(&rows, &columns(menu.page), menu.focus, input.turn);
    }
    input.enter = keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]);
    let (focus, mut chosen) = choose(&rows, menu.focus, input);
    menu.focus = focus;
    if keys.just_pressed(KeyCode::Escape) {
        match menu.page {
            Page::Pause => chosen = Some((Act::Resume, 1)),
            Page::Main | Page::Outcome => {}
            _ => chosen = Some((Act::Back, 1)),
        }
    }
    if let Some((a, dir)) = chosen {
        activate(a, dir, &mut menu, &ctx, &mut fx);
    }
}

fn random_seed() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    1 + now % 1_000_000
}

fn chosen_seed(menu: &Menu) -> u64 {
    menu.seed
        .parse::<u64>()
        .ok()
        .filter(|&s| s > 0)
        .unwrap_or_else(random_seed)
}

fn activate(act: Act, dir: i32, menu: &mut Menu, ctx: &Context, fx: &mut Effects) {
    match act {
        Act::Go(page) => menu.open(page),
        Act::Back => {
            if !menu.go_back() && menu.page == Page::ConfirmQuit {
                menu.show(Page::Main);
            }
        }
        Act::Difficulty => {
            let all = [Night::Gentle, Night::Normal, Night::Hard];
            let at = all.iter().position(|&n| n == menu.night).unwrap_or(1) as i32;
            menu.night = all[(at + dir).clamp(0, 2) as usize];
        }
        Act::Survivor => {
            fx.profile.profile.survivor = fx.profile.profile.survivor.cycle(dir);
            fx.profile.save();
            // The portrait changes with them.
            menu.built = None;
        }
        Act::Edit(_) => {}
        Act::PictureDefaults => {
            let fresh = Settings::default();
            fx.settings.brightness = fresh.brightness;
            fx.settings.contrast = fresh.contrast;
        }
        Act::Adjust(knob) => {
            adjust(knob, dir, &mut fx.settings, &fx.tuning);
            // Heard at once, even at the end of its travel.
            if let Some(bus) = knob.bus() {
                fx.preview.write(VolumePreview(bus));
            }
            // Every word on the page changes with the language.
            if knob == Knob::Language {
                menu.built = None;
            }
        }
        Act::Solo => {
            fx.starts.write(StartRun {
                mode: Mode::Solo,
                seed: chosen_seed(menu),
                night: menu.night,
            });
        }
        Act::Host => match parse_addr(&menu.host, fx.settings.lang) {
            Ok(addr) => {
                fx.starts.write(StartRun {
                    mode: Mode::Host(addr),
                    seed: chosen_seed(menu),
                    night: menu.night,
                });
            }
            Err(e) => menu.note = e,
        },
        Act::Join => match parse_addr(&menu.join, fx.settings.lang) {
            Ok(addr) => {
                fx.profile.profile.join = menu.join.trim().to_string();
                fx.profile.save();
                // The host's night arrives with the session (see `net`).
                fx.starts.write(StartRun {
                    mode: Mode::Join(addr),
                    seed: ctx.launch.seed,
                    night: ctx.launch.night,
                });
            }
            Err(e) => menu.note = e,
        },
        Act::Read(id) => menu.open(Page::Reading(id)),
        Act::Chapter(n) => menu.open(Page::Chapter(n)),
        Act::Resume => fx.next.set(Flow::Playing),
        Act::Restart => {
            fx.control.write(NetControl::Action(Action::Restart));
            fx.next.set(Flow::Playing);
        }
        Act::NewNight => {
            fx.starts.write(StartRun {
                mode: Mode::Solo,
                seed: random_seed(),
                night: ctx.launch.night,
            });
        }
        Act::Leave | Act::Title => {
            fx.leaves.write(LeaveRun);
        }
        Act::Quit => {
            fx.exit.write(AppExit::Success);
        }
        Act::None => {}
    }
}

fn adjust(knob: Knob, dir: i32, s: &mut Settings, tuning: &TuningRes) {
    let d = dir as f32;
    let (smin, smax) = tuning.0.sensitivity_range;
    match knob {
        Knob::Master => s.master = crate::mix::step(s.master, dir),
        Knob::Music => s.music = crate::mix::step(s.music, dir),
        Knob::Ambience => s.ambience = crate::mix::step(s.ambience, dir),
        Knob::Effects => s.effects = crate::mix::step(s.effects, dir),
        Knob::Sensitivity => {
            s.sensitivity = (((s.sensitivity + 0.1 * d) * 10.0).round() / 10.0).clamp(smin, smax);
        }
        Knob::InvertY => s.invert_y = !s.invert_y,
        Knob::Fov => s.fov = (s.fov + 4.0 * d).clamp(56.0, 100.0),
        Knob::Brightness => {
            s.brightness = display::step(s.brightness, dir, display::BRIGHTNESS_STEP, display::BRIGHTNESS_RANGE);
        }
        Knob::Contrast => s.contrast = display::step(s.contrast, dir, display::CONTRAST_STEP, display::CONTRAST_RANGE),
        Knob::HeadBob => s.head_bob = !s.head_bob,
        Knob::Captions => s.captions = !s.captions,
        // The window follows in `app::apply_display`.
        Knob::DisplayMode => s.display_mode = s.display_mode.toggled(),
        Knob::Language => s.lang = s.lang.toggled(),
    }
}

/// The profile follows the settings and the pages read, and the journal
/// counts every night that ends.
#[allow(clippy::too_many_arguments)]
fn remember(
    settings: Res<Settings>,
    mut read: ResMut<PagesRead>,
    mut profile: ResMut<ProfileRes>,
    state: Res<State<Flow>>,
    truth: Res<Truth>,
    net: Res<Network>,
    mut started: Local<bool>,
    mut last: Local<Option<Flow>>,
) {
    if !*started {
        // Pages found on earlier nights are already in the journal.
        *started = true;
        read.0.extend(profile.profile.pages.iter().copied());
    }
    let mut dirty = false;
    if settings.is_changed() && profile.profile.settings != *settings {
        profile.profile.settings = settings.clone();
        dirty = true;
    }
    if read.0.iter().any(|p| !profile.profile.pages.contains(p)) {
        profile.profile.pages.extend(read.0.iter().copied());
        dirty = true;
    }
    // The Madrina's chapters, as the shared count of bones laid tells them.
    if let Some(s) = net.snapshot() {
        for n in 1..=crate::lore::chapters_told(s.world.delivered, s.world.total) {
            dirty |= profile.profile.chapters.insert(n);
        }
    }
    // Each night that ends is counted once, as its outcome comes up.
    let now = *state.get();
    let arrived = now == Flow::Outcome && *last != Some(Flow::Outcome);
    *last = Some(now);
    if arrived
        && let Some(s) = net.snapshot()
        && s.outcome().is_over()
    {
        {
            let seconds = truth.encounter.elapsed;
            let left_behind = net.id().is_some_and(|me| s.left_behind.contains(&me));
            let ending = if s.outcome() == crate::sim::Outcome::Failed || left_behind {
                crate::profile::Ending::Caught
            } else if s.outcome() == crate::sim::Outcome::Dawn {
                crate::profile::Ending::Dawn
            } else if s.world.banished {
                crate::profile::Ending::Banished { seconds }
            } else {
                crate::profile::Ending::Escaped { seconds }
            };
            profile.profile.tally.record(ending);
            dirty = true;
        }
    }
    if dirty {
        profile.save();
    }
}

/// Seed the join field with the last address used.
fn prime(mut menu: ResMut<Menu>, profile: Res<ProfileRes>, launch: Res<Launch>) {
    menu.join = profile.profile.join.clone();
    menu.night = launch.night;
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .add_systems(Startup, (spawn_menu, prime))
        .add_systems(
            Update,
            (
                follow_flow.in_set(GameSet::Control),
                navigate.in_set(GameSet::Control).after(follow_flow),
                (draw, remember).in_set(GameSet::Present),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page with a slider, a chooser, a button and a row that cannot be
    /// chosen.
    fn page() -> Vec<Row> {
        vec![
            Row::value("volume", "100%".into(), Act::Adjust(Knob::Master)),
            Row::value("difficulty", "Normal".into(), Act::Difficulty),
            Row::go("back", Act::Back),
            Row::value("display", "Window".into(), Act::None).off(),
        ]
    }

    fn with(f: impl FnOnce(&mut Input)) -> Input {
        let mut input = Input::default();
        f(&mut input);
        input
    }

    #[test]
    fn a_click_on_an_arrow_is_the_key_that_way_and_a_click_on_the_row_is_enter() {
        let rows = page();
        for row in [0, 1] {
            let act = rows[row].act;
            for dir in [-1, 1] {
                // From anywhere on the page: the arrow takes its row's focus
                // and turns it exactly as the key does from there.
                let clicked = choose(&rows, 2, with(|i| i.arrow = Some((row, dir))));
                assert_eq!(clicked, (row, Some((act, dir))), "row {row}, arrow {dir}");
                assert_eq!(
                    clicked,
                    choose(&rows, row, with(|i| i.turn = dir)),
                    "row {row}, key {dir}"
                );
            }
            let on_row = choose(&rows, 2, with(|i| i.click = Some(row)));
            assert_eq!(on_row, (row, Some((act, 1))));
            assert_eq!(on_row, choose(&rows, row, with(|i| i.enter = true)));
        }
    }

    #[test]
    fn left_and_right_never_press_a_button_and_a_dead_row_takes_nothing() {
        let rows = page();
        for dir in [-1, 1] {
            assert_eq!(
                choose(&rows, 2, with(|i| i.turn = dir)),
                (2, None),
                "key {dir} on a button"
            );
        }
        assert_eq!(choose(&rows, 2, with(|i| i.enter = true)), (2, Some((Act::Back, 1))));
        assert_eq!(choose(&rows, 2, with(|i| i.click = Some(2))), (2, Some((Act::Back, 1))));
        // The row that cannot be chosen takes neither the pointer nor a click.
        assert_eq!(choose(&rows, 2, with(|i| i.hover = Some(3))), (2, None));
        assert_eq!(choose(&rows, 2, with(|i| i.click = Some(3))), (2, None));
        // The pointer alone only moves the focus.
        assert_eq!(choose(&rows, 2, with(|i| i.hover = Some(0))), (0, None));
    }

    #[test]
    fn left_and_right_cross_the_journal_columns_to_a_page_that_was_found() {
        let runs = columns(Page::Journal);
        let total = runs.iter().map(|r| r.len).sum::<usize>() + 1;
        let found = [2, 11, 14, 17];
        let rows: Vec<Row> = (0..total)
            .map(|i| {
                let row = Row::go("", Act::Read(i as u8));
                if found.contains(&i) || i >= crate::lore::PAGES as usize {
                    row
                } else {
                    row.off()
                }
            })
            .collect();
        let pages = runs[0];
        let line = |i: usize| pages.place(i).unwrap().1;
        // From the left column to the nearest found page on the right.
        let right = across(&rows, &runs, 2, 1);
        assert_eq!(pages.place(right).unwrap().0, 1);
        assert!(found.contains(&right));
        let nearest = found[1..].iter().map(|&i| line(i).abs_diff(line(2))).min().unwrap();
        assert_eq!(line(right).abs_diff(line(2)), nearest);
        // And back again; the outer edges and full-width rows stay put.
        assert_eq!(across(&rows, &runs, right, -1), 2);
        assert_eq!(across(&rows, &runs, 2, -1), 2);
        assert_eq!(across(&rows, &runs, right, 1), right);
        assert_eq!(across(&rows, &runs, total - 1, 1), total - 1);
        // The chapters cross their own two columns, never into the pages.
        let chapters = runs[1];
        let first = chapters.start;
        let other = across(&rows, &runs, first, 1);
        assert_eq!(chapters.place(other), Some((1, 0)));
        assert_eq!(across(&rows, &runs, other, -1), first);
    }

    #[test]
    fn the_arrows_lower_and_raise_every_setting_from_either_end() {
        let tuning = TuningRes(crate::tuning::Tuning::default());
        let press = |s: &mut Settings, knob: Knob, dir: i32| {
            let row = [Row::value("", String::new(), Act::Adjust(knob))];
            match choose(&row, 0, with(|i| i.arrow = Some((0, dir)))) {
                (_, Some((Act::Adjust(k), d))) => adjust(k, d, s, &tuning),
                other => panic!("{knob:?}: the arrow did nothing ({other:?})"),
            }
        };
        let sliders: [(Knob, fn(&Settings) -> f32); 8] = [
            (Knob::Master, |s| s.master),
            (Knob::Music, |s| s.music),
            (Knob::Ambience, |s| s.ambience),
            (Knob::Effects, |s| s.effects),
            (Knob::Sensitivity, |s| s.sensitivity),
            (Knob::Fov, |s| s.fov),
            (Knob::Brightness, |s| s.brightness),
            (Knob::Contrast, |s| s.contrast),
        ];
        for (knob, read) in sliders {
            let mut s = Settings::default();
            for _ in 0..200 {
                press(&mut s, knob, 1);
            }
            let top = read(&s);
            press(&mut s, knob, -1);
            assert!(read(&s) < top, "{knob:?}: ‹ lowers it from the top");
            for _ in 0..200 {
                press(&mut s, knob, -1);
            }
            let bottom = read(&s);
            assert!(bottom < top, "{knob:?}");
            press(&mut s, knob, 1);
            assert!(read(&s) > bottom, "{knob:?}: › raises it from the bottom");
        }
        // Two-way choices flip whichever arrow is pressed.
        for knob in [
            Knob::InvertY,
            Knob::HeadBob,
            Knob::Captions,
            Knob::DisplayMode,
            Knob::Language,
        ] {
            for dir in [-1, 1] {
                let mut s = Settings::default();
                press(&mut s, knob, dir);
                assert_ne!(s, Settings::default(), "{knob:?}, arrow {dir}");
                press(&mut s, knob, dir);
                assert_eq!(s, Settings::default(), "{knob:?}, arrow {dir}");
            }
        }
    }
}
