//! The front end: the title screen's menus over the llano, the pause menu
//! and the choices after a night.
//!
//! One model drives every page: a page is a heading, some words and a list
//! of rows (buttons, `‹ value ›` choosers and small text fields). The rows
//! are rebuilt when the page changes; their text and focus are refreshed
//! every frame. Keyboard (arrows or WASD, Enter, Esc) and mouse both work.
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

fn parse_addr(text: &str) -> Result<SocketAddr, String> {
    let text = text.trim();
    let full = if text.contains(':') {
        text.to_string()
    } else {
        format!("{text}:5000")
    };
    let addr: SocketAddr = full
        .parse()
        .map_err(|_| format!("\"{text}\" is not an address like 192.168.1.20:5000"))?;
    crate::net::transport::local_address(addr)
}

fn night_label(n: Night) -> &'static str {
    match n {
        Night::Gentle => "Gentle",
        Night::Normal => "Normal",
        Night::Hard => "Hard",
    }
}

fn night_blurb(n: Night) -> &'static str {
    match n {
        Night::Gentle => "For learning the llano: he notices you later, the torch lasts, the rhythm forgives.",
        Night::Normal => "The night as it was meant to be.",
        Night::Hard => "He notices you farther off, the torch dies sooner, every rite angers him more.",
    }
}

const HOW_TO: &str = "\
Lay the five bundles of the father's bones at the ceiba's roots, bring the power back at the windmill, \
open the padlocked key box (its three numbers are written in the pages you find) and start the truck \
at the bridge. Survive its warm-up, everyone aboard. Or learn which of him walks tonight and name him \
at the ceiba (N) once every bone is home.\n\n\
The whistle lies: loud means he is far, thin and faint means he is near. Walls, trunks and tall grass \
break his sight; crouch to move quietly, running is heard far away. Your torch runs down, and its beam \
draws him. Lamplight and company calm fear; the dark and being alone feed it.\n\n\
While you lay bones, crank or turn the engine over, press Space as the needle crosses the marked zone. \
Ají stops him to count his bones. Tureco, tied behind the house, knows where he truly is. Caught with \
friends standing, you are carried off in his sack: pepper in his path drops you. With friends, whoever \
is aboard the ready truck can drive off without the others (X).\n\n\
WASD move · mouse look · Shift run · Ctrl/C crouch · E use (hold at sites) · F torch · Space rhythm · \
G drop a bundle · Q ají · V mark · M map · N name him · X drive off · Esc menu · F12 screenshot";

const CREDITS: &str = "\
EL SILBÓN — The Return\n\n\
A game by Andrew Acevedo Mirena, made with Claude Code.\n\n\
Engine: Bevy 0.19 · Networking: Renet\n\
Fonts: Noto Sans and Noto Serif (SIL Open Font License)\n\
Every mesh, texture and sound is original, generated in code.\n\n\
The legend of El Silbón belongs to the llanos of Venezuela and Colombia. The Hacienda Santa Rosa, its \
people and every page in this game are fiction inspired by it.\n\n\
Thank you for playing.";

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
    let host_side = ctx.launch.network.is_host();
    let solo = ctx.launch.network.is_solo();
    let field = |text: &str, empty: &str| {
        if text.is_empty() {
            empty.to_string()
        } else {
            text.to_string()
        }
    };
    let on = |b: bool| if b { "‹ On ›" } else { "‹ Off ›" };
    let brightness = || {
        Row::value(
            "Brightness",
            format!("‹ {:+.1} ›", settings.brightness),
            Act::Adjust(Knob::Brightness),
        )
    };
    let contrast = || {
        Row::value(
            "Contrast",
            format!("‹ {:.2} ›", settings.contrast),
            Act::Adjust(Knob::Contrast),
        )
    };
    match menu.page {
        Page::Main => (
            String::new(),
            String::new(),
            vec![
                Row::go("Play", Act::Go(Page::Solo)),
                Row::go("Play with friends", Act::Go(Page::Multiplayer)),
                Row::go("Journal", Act::Go(Page::Journal)),
                Row::go("Settings", Act::Go(Page::Settings)),
                Row::go("How to play", Act::Go(Page::HowTo)),
                Row::go("Credits", Act::Go(Page::Credits)),
                Row::go("Quit", Act::Go(Page::ConfirmQuit)),
            ],
        ),
        Page::Solo => (
            "A night alone".into(),
            format!(
                "{}\nLeave the night blank for a new one, or type a night's number to play it again (friends can share numbers).",
                night_blurb(menu.night)
            ),
            vec![
                Row::value(
                    "Difficulty",
                    format!("‹ {} ›", night_label(menu.night)),
                    Act::Difficulty,
                ),
                Row::value("Night", field(&menu.seed, "a new night"), Act::Edit(Field::Seed)),
                Row::go("Begin the night", Act::Solo),
                Row::go("Back", Act::Back),
            ],
        ),
        Page::Multiplayer => (
            "With friends".into(),
            "Up to four players on the same local network, or far apart on a private VPN such as Tailscale. \
             One hosts the night; the others join with the host's address and receive the host's night and \
             difficulty.\nChoose who the others will see. \
             If a friend is already them, you are someone else (F7 in the lobby changes)."
                .into(),
            vec![
                Row::value(
                    "Who you are",
                    format!(
                        "‹ {} · {} ›",
                        profile.profile.survivor.name(),
                        profile.profile.survivor.role()
                    ),
                    Act::Survivor,
                ),
                Row::go("Host a night", Act::Go(Page::Host)),
                Row::go("Join a friend", Act::Go(Page::Join)),
                Row::go("Back", Act::Back),
            ],
        ),
        Page::Host => (
            "Host a night".into(),
            format!(
                "Give your friends this address. When everyone is in, press Enter in the night to begin.\n{}",
                night_blurb(menu.night)
            ),
            vec![
                Row::value("Your address", menu.host.clone(), Act::Edit(Field::HostAddr)),
                Row::value(
                    "Difficulty",
                    format!("‹ {} ›", night_label(menu.night)),
                    Act::Difficulty,
                ),
                Row::value("Night", field(&menu.seed, "a new night"), Act::Edit(Field::Seed)),
                Row::go("Open the session", Act::Host),
                Row::go("Back", Act::Back),
            ],
        ),
        Page::Join => (
            "Join a friend".into(),
            "Type the address your host gave you (for example 192.168.1.20:5000). Their night and difficulty \
             come with the session."
                .into(),
            vec![
                Row::value(
                    "Host's address",
                    field(&menu.join, "type it here"),
                    Act::Edit(Field::JoinAddr),
                ),
                Row::go("Join", Act::Join),
                Row::go("Back", Act::Back),
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
                        Row::go(page.title, Act::Read(id))
                    } else {
                        Row::go("· · ·  not yet found", Act::None).off()
                    }
                })
                .collect();
            rows.push(Row::go("Back", Act::Back));
            (
                "Journal".into(),
                format!(
                    "Pages of the tale found: {}/{}   ·   Nights: {}   ·   Escapes: {}   ·   Laid to rest: {}   ·   Caught: {}   ·   Lived till dawn: {}   ·   Fastest: {fastest}",
                    ctx.read.0.len(),
                    crate::lore::PAGES,
                    t.nights,
                    t.escapes,
                    t.banishments,
                    t.caught,
                    t.dawns
                ),
                rows,
            )
        }
        Page::Reading(id) => {
            // The kept copy has no digits: a page read on an earlier night
            // must not tell tonight's padlock.
            let page = crate::lore::note(id);
            (
                format!("{} — {}", page.medium.label(), page.title),
                format!(
                    "{}\n\n{}\n\n{}",
                    crate::lore::keep(page.es),
                    page.by,
                    crate::lore::keep(page.en)
                ),
                vec![Row::go("Back", Act::Back)],
            )
        }
        Page::Settings => (
            "Settings".into(),
            "Saved as you change them.".into(),
            vec![
                Row::go("Video", Act::Go(Page::Video)),
                Row::go("Audio", Act::Go(Page::Audio)),
                Row::go("Controls", Act::Go(Page::Controls)),
                Row::go("Calibrate brightness", Act::Go(Page::Calibrate)),
                Row::go("Back", Act::Back),
            ],
        ),
        Page::Video => {
            let display = if ctx.launch.windowed {
                // `--windowed` keeps this launch in a window and leaves the
                // saved choice alone, so the row cannot change it.
                Row::value("Display mode", "Window (--windowed)".into(), Act::None).off()
            } else {
                let mode = match settings.display_mode {
                    DisplayMode::Fullscreen => "Fullscreen",
                    DisplayMode::Window => "Window",
                };
                Row::value("Display mode", format!("‹ {mode} ›"), Act::Adjust(Knob::DisplayMode))
            };
            (
                "Video".into(),
                "Saved as you change them.".into(),
                vec![
                    display,
                    Row::value(
                        "Field of view",
                        format!("‹ {:.0}° ›", settings.fov),
                        Act::Adjust(Knob::Fov),
                    ),
                    brightness(),
                    contrast(),
                    Row::go("Calibrate brightness", Act::Go(Page::Calibrate)),
                    Row::value("Head bob", on(settings.head_bob).into(), Act::Adjust(Knob::HeadBob)),
                    Row::go("Back", Act::Back),
                ],
            )
        }
        Page::Calibrate => (
            "Brightness".into(),
            format!(
                "Raise Brightness until the middle hat is just barely visible. If the left one shows too, \
                 raise Contrast until it is gone. The right one should be plain.{}",
                if solo {
                    ""
                } else {
                    "\nThe shared night goes on behind this page."
                }
            ),
            vec![
                brightness(),
                contrast(),
                Row::go("Defaults", Act::PictureDefaults),
                Row::go("Done", Act::Back),
            ],
        ),
        Page::Audio => {
            let level = |v: f32| format!("‹ {:.0}% ›", v * 100.0);
            (
                "Audio".into(),
                "Saved as you change them. His whistle follows the master volume alone.".into(),
                vec![
                    Row::value("Master volume", level(settings.master), Act::Adjust(Knob::Master)),
                    Row::value("Music", level(settings.music), Act::Adjust(Knob::Music)),
                    Row::value("Ambience", level(settings.ambience), Act::Adjust(Knob::Ambience)),
                    Row::value("Effects", level(settings.effects), Act::Adjust(Knob::Effects)),
                    Row::value(
                        "Whistle captions",
                        on(settings.captions).into(),
                        Act::Adjust(Knob::Captions),
                    ),
                    Row::go("Back", Act::Back),
                ],
            )
        }
        Page::Controls => (
            "Controls".into(),
            "Saved as you change them.".into(),
            vec![
                Row::value(
                    "Mouse sensitivity",
                    format!("‹ {:.1}× ›", settings.sensitivity),
                    Act::Adjust(Knob::Sensitivity),
                ),
                Row::value(
                    "Invert mouse Y",
                    on(settings.invert_y).into(),
                    Act::Adjust(Knob::InvertY),
                ),
                Row::go("Back", Act::Back),
            ],
        ),
        Page::HowTo => ("How to play".into(), HOW_TO.into(), vec![Row::go("Back", Act::Back)]),
        Page::Credits => ("Credits".into(), CREDITS.into(), vec![Row::go("Back", Act::Back)]),
        Page::Pause => {
            let mut v = vec![
                Row::go("Resume", Act::Resume),
                Row::go("Settings", Act::Go(Page::Settings)),
                Row::go("Journal", Act::Go(Page::Journal)),
                Row::go("How to play", Act::Go(Page::HowTo)),
            ];
            if host_side {
                v.push(Row::go("Restart the night", Act::Restart));
            }
            v.push(Row::go(
                if solo || !host_side {
                    "Leave to the title"
                } else {
                    "End the night for everyone"
                },
                Act::Go(Page::ConfirmLeave),
            ));
            (
                "Paused".into(),
                if solo {
                    "The night holds its breath. The mouse is free.".into()
                } else {
                    "Only your controls stop: the shared night goes on.".into()
                },
                v,
            )
        }
        Page::Outcome => {
            let mut v = Vec::new();
            if host_side {
                v.push(Row::go("Play the night again", Act::Restart));
            }
            if solo {
                v.push(Row::go("A new night", Act::NewNight));
            }
            v.push(Row::go("Title menu", Act::Title));
            v.push(Row::go("Quit", Act::Go(Page::ConfirmQuit)));
            let _ = &ctx.truth;
            (String::new(), String::new(), v)
        }
        Page::ConfirmLeave => (
            "Leave the night?".into(),
            if !solo && host_side {
                "Everyone in your session goes back to their title screen.".into()
            } else {
                "This night's progress is lost; pages you read stay in your journal.".into()
            },
            vec![Row::go("Stay", Act::Back), Row::go("Leave", Act::Leave)],
        ),
        Page::ConfirmQuit => (
            "Quit the game?".into(),
            String::new(),
            vec![Row::go("Stay", Act::Back), Row::go("Quit", Act::Quit)],
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
) {
    commands.entity(root).despawn_children();
    // The title screen keeps its rail; a paused night shows a card; after a
    // night the choices sit under the outcome (other pages there are cards).
    // The calibration leaves the screen to its hats, anywhere.
    let calibrating = menu.page == Page::Calibrate;
    let title_screen = flow == Flow::Title && !calibrating;
    let in_run_card = !calibrating && (flow == Flow::Paused || (flow == Flow::Outcome && menu.page != Page::Outcome));
    let reading = matches!(menu.page, Page::Reading(_) | Page::HowTo | Page::Credits);
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
            for (i, label) in ["should vanish", "barely visible", "plainly seen"]
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
            column.width = px(if reading { 760.0 } else { 600.0 });
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
            column.width = px(if reading { 760.0 } else { 520.0 });
            column.margin = UiRect::new(px(if reading { -380.0 } else { -260.0 }), px(0), px(-300), px(0));
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
                p.spawn((Text::new(who.role()), font(&fonts.italic, 18.0), TextColor(AMBER)));
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
                    Text::new("Si lo oyes cerca, está lejos. Si lo oyes lejos, ya está aquí."),
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
                        max_width: px(if reading { 660.0 } else { 470.0 }),
                        ..default()
                    },
                ));
            }
            let journal = menu.page == Page::Journal;
            for (i, row) in rows.iter().enumerate() {
                let (text, bg, edge) = row_colors(i == menu.focus, row.enabled);
                c.spawn((
                    Button,
                    MenuRow(i),
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(14), px(if journal { 3.0 } else { 7.0 })),
                        border: UiRect::left(px(3)),
                        min_width: px(if menu.page == Page::Outcome { 360.0 } else { 440.0 }),
                        ..default()
                    },
                    BackgroundColor(bg),
                    BorderColor::all(edge),
                ))
                .with_children(|row_ui| {
                    row_ui.spawn((
                        RowLabel(i),
                        Text::new(row.label.clone()),
                        font(&fonts.sans, if journal { 15.0 } else { 20.0 }),
                        TextColor(text),
                    ));
                    if let Some(v) = &row.value {
                        row_ui.spawn((
                            RowValue(i),
                            Text::new(v.clone()),
                            font(&fonts.sans, 18.0),
                            TextColor(text),
                        ));
                    }
                });
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
}

fn field_text(menu: &Menu, act: Act) -> Option<&str> {
    match act {
        Act::Edit(Field::Seed) => Some(&menu.seed),
        Act::Edit(Field::HostAddr) => Some(&menu.host),
        Act::Edit(Field::JoinAddr) => Some(&menu.join),
        _ => None,
    }
}

/// Keyboard and mouse on the menus.
#[allow(clippy::too_many_arguments)]
fn navigate(
    state: Res<State<Flow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    buttons: Query<(&Interaction, &MenuRow), Changed<Interaction>>,
    mouse: Res<ButtonInput<MouseButton>>,
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
    let mut chosen: Option<(Act, i32)> = None;
    for (interaction, row) in &buttons {
        match interaction {
            Interaction::Hovered => {
                if rows.get(row.0).is_some_and(|r| r.enabled) {
                    menu.focus = row.0;
                }
            }
            Interaction::Pressed => {
                if let Some(r) = rows.get(row.0).filter(|r| r.enabled) {
                    menu.focus = row.0;
                    chosen = Some((r.act, 1));
                }
            }
            Interaction::None => {}
        }
    }
    let _ = mouse;
    let act = rows.get(menu.focus).map(|r| r.act).unwrap_or(Act::None);
    if !typed_any {
        if keys.any_just_pressed([KeyCode::ArrowLeft]) || (!editing && keys.just_pressed(KeyCode::KeyA)) {
            chosen = Some((act, -1));
        }
        if keys.any_just_pressed([KeyCode::ArrowRight]) || (!editing && keys.just_pressed(KeyCode::KeyD)) {
            chosen = Some((act, 1));
        }
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        chosen = Some((act, 1));
    }
    // Left/right only turn choosers; they do not press buttons.
    if let Some((a, dir)) = chosen
        && dir < 0
        && !matches!(a, Act::Difficulty | Act::Survivor | Act::Adjust(_))
    {
        chosen = None;
    }
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
        }
        Act::Solo => {
            fx.starts.write(StartRun {
                mode: Mode::Solo,
                seed: chosen_seed(menu),
                night: menu.night,
            });
        }
        Act::Host => match parse_addr(&menu.host) {
            Ok(addr) => {
                fx.starts.write(StartRun {
                    mode: Mode::Host(addr),
                    seed: chosen_seed(menu),
                    night: menu.night,
                });
            }
            Err(e) => menu.note = e,
        },
        Act::Join => match parse_addr(&menu.join) {
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
