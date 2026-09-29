//! HUD and menus: objective checklist, vitals (fear, breath, peppers, load),
//! crosshair prompts with hold progress, threat status, party roster and
//! world markers, the map, notes, teaching hints, whistle captions and the
//! briefing / pause / outcome overlays. All nodes are spawned once; states
//! only toggle visibility and text, so restarts never accumulate UI.

mod hud;
mod map;
pub(crate) mod menu;
pub(crate) mod title;

use bevy::prelude::*;

use crate::app::{Flow, GameSet, Launch, LayoutRes, RunReset, Settings, Truth, TuningRes};
use crate::net::Network;

pub(crate) const PANEL_BG: Color = Color::srgba(0.018, 0.022, 0.03, 0.5);
pub(crate) const INK: Color = Color::srgb(0.93, 0.9, 0.84);
pub(crate) const DIM: Color = Color::srgb(0.66, 0.66, 0.64);
pub(crate) const AMBER: Color = Color::srgb(0.98, 0.72, 0.36);
pub(crate) const RED: Color = Color::srgb(1.0, 0.42, 0.34);
pub(crate) const PALE_BLUE: Color = Color::srgb(0.7, 0.8, 0.95);
pub(crate) const GREEN: Color = Color::srgb(0.55, 0.85, 0.5);
const BUTTON: Color = Color::srgba(0.12, 0.11, 0.1, 0.92);
const BUTTON_HOVER: Color = Color::srgba(0.24, 0.2, 0.15, 0.95);
const BUTTON_PRESS: Color = Color::srgba(0.42, 0.3, 0.16, 0.95);

#[derive(Resource)]
pub(crate) struct Fonts {
    pub sans: Handle<Font>,
    pub serif: Handle<Font>,
    pub italic: Handle<Font>,
}

#[derive(Resource, Default)]
pub(crate) struct Hint {
    pub text: &'static str,
    pub timer: f32,
    pub priority: u8,
    pub taught_loud: bool,
    pub taught_faint: bool,
    pub taught_crouch: bool,
    pub taught_skill: bool,
}

#[derive(Resource, Default)]
pub(crate) struct CaptionLine {
    pub text: &'static str,
    pub timer: f32,
}

/// The map overlay is up (M).
#[derive(Resource, Default)]
pub(crate) struct MapOpen(pub bool);

#[derive(Component)]
pub(crate) struct HudRoot;
/// One line of the objective checklist.
#[derive(Component)]
pub(crate) struct ObjectiveLine(pub usize);
#[derive(Component)]
pub(crate) struct ObjectiveHint;
#[derive(Component)]
pub(crate) struct StatusPanel;
#[derive(Component)]
pub(crate) struct RosterLine(pub usize);

/// The briefing's line naming tonight (chosen in the menu, so it changes).
#[derive(Component)]
pub(crate) struct BriefingNight;

/// The black that falls at the end of the catch (spawned once).
#[derive(Component)]
pub(crate) struct CatchVeil;

#[derive(Component)]
pub(crate) struct FearOuter;
#[derive(Component)]
pub(crate) struct FearFill;
#[derive(Component)]
pub(crate) struct BreathOuter;
#[derive(Component)]
pub(crate) struct BreathFill;
#[derive(Component)]
pub(crate) struct VitalsText;
#[derive(Component)]
pub(crate) struct PromptText;
#[derive(Component)]
pub(crate) struct ProgressOuter;
#[derive(Component)]
pub(crate) struct ProgressFill;
#[derive(Component)]
pub(crate) struct ProgressLabel;
/// The skill check's track, its zone and great sliver, the needle and the key.
#[derive(Component)]
pub(crate) struct SkillBar;
#[derive(Component)]
pub(crate) struct SkillZone;
#[derive(Component)]
pub(crate) struct SkillGreat;
#[derive(Component)]
pub(crate) struct SkillNeedle;
#[derive(Component)]
pub(crate) struct SkillKey;
#[derive(Component)]
pub(crate) struct CaptionText;
#[derive(Component)]
pub(crate) struct HintText;
#[derive(Component)]
pub(crate) struct Vignette;
/// Full-screen colour wash: susto flashes, the downed grey-red.
#[derive(Component)]
pub(crate) struct Tint;
#[derive(Component)]
pub(crate) struct DownedPanel;
#[derive(Component)]
pub(crate) struct DownedText;
#[derive(Component)]
pub(crate) struct BriefingPanel;
#[derive(Component)]
pub(crate) struct OutcomePanel;
#[derive(Component)]
pub(crate) struct OutcomeTitle;
#[derive(Component)]
pub(crate) struct OutcomeBody;
#[derive(Component)]
pub(crate) struct NotePanel;
#[derive(Component)]
pub(crate) struct NoteEs;
#[derive(Component)]
pub(crate) struct NoteEn;
#[derive(Component)]
pub(crate) struct NoteBy;
/// Naming him at the ceiba: the panel and its three names.
#[derive(Component)]
pub(crate) struct NamePanelUi;
#[derive(Component)]
pub(crate) struct NameChoices;
/// The key box's padlock: its panel and the three dials.
#[derive(Component)]
pub(crate) struct LockPanelUi;
#[derive(Component)]
pub(crate) struct LockDigits;
/// The page's medium and title, its paper and the pages-found count.
#[derive(Component)]
pub(crate) struct NoteTitle;
#[derive(Component)]
pub(crate) struct NotePaper;
#[derive(Component)]
pub(crate) struct NoteCount;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MenuAction {
    Begin,
    Title,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        menu::plugin(app);
        title::plugin(app);
        app.init_resource::<Hint>()
            .init_resource::<CaptionLine>()
            .init_resource::<MapOpen>()
            .add_systems(Startup, (map::build_map_image, spawn_ui).chain())
            .add_systems(OnEnter(Flow::Outcome), fill_outcome)
            .add_systems(
                Update,
                (
                    reset_ui.in_set(GameSet::Control),
                    (
                        buttons,
                        panels,
                        hud::objectives,
                        hud::status_text,
                        hud::roster,
                        hud::vitals,
                        hud::prompt,
                        hud::skill_bar,
                        hud::vignette,
                        hud::downed_panel,
                        hud::hints_and_captions,
                        hud::note_panel,
                        hud::lock_panel,
                        hud::name_panel,
                        map::toggle_map,
                        map::update_map,
                        map::update_markers,
                    )
                        .chain()
                        .in_set(GameSet::Present),
                ),
            );
    }
}

/// One colour per party slot, shared by the roster, the map and the avatars.
pub(crate) fn player_color(slot: usize) -> Color {
    [
        Color::srgb(0.98, 0.72, 0.36),
        Color::srgb(0.35, 0.8, 0.85),
        Color::srgb(0.92, 0.48, 0.58),
        Color::srgb(0.62, 0.86, 0.42),
    ][slot % 4]
}

pub(crate) fn font(h: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: h.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    }
}

pub(crate) fn set_text(text: &mut Text, s: &str) {
    if text.0 != s {
        text.0 = s.to_string();
    }
}

pub(crate) fn set_vis(v: &mut Visibility, show: bool) {
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *v != want {
        *v = want;
    }
}

pub(crate) fn label(fonts: &Fonts, text: &str, size: f32, color: Color, italic: bool) -> impl Bundle {
    (
        Text::new(text),
        font(if italic { &fonts.italic } else { &fonts.sans }, size),
        TextColor(color),
    )
}

fn button(fonts: &Fonts, text: &str, action: MenuAction, width: f32) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(width),
            padding: UiRect::axes(px(14), px(9)),
            margin: UiRect::all(px(4)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.8, 0.62, 0.38, 0.5)),
        BackgroundColor(BUTTON),
        children![(Text::new(text), font(&fonts.sans, 17.0), TextColor(INK))],
    )
}

fn overlay() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn card(width: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(px(28)),
            row_gap: px(10),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.022, 0.028, 0.9)),
        BorderColor::all(Color::srgba(0.8, 0.62, 0.38, 0.35)),
    )
}

fn bar(width: f32, height: f32, fill: Color) -> (Node, BorderColor, BackgroundColor, Visibility) {
    let _ = fill;
    (
        Node {
            width: px(width),
            height: px(height),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BorderColor::all(Color::srgba(1.0, 0.85, 0.6, 0.45)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Visibility::Hidden,
    )
}

fn fill_node(color: Color) -> impl Bundle {
    (
        Node {
            width: percent(0),
            height: percent(100),
            border_radius: BorderRadius::all(px(2)),
            ..default()
        },
        BackgroundColor(color),
    )
}

fn spawn_ui(mut commands: Commands, assets: Res<AssetServer>, map_image: Res<map::MapImage>, layout: Res<LayoutRes>) {
    commands.spawn((
        Name::new("catch veil"),
        CatchVeil,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.0)),
        GlobalZIndex(18),
        Visibility::Hidden,
    ));
    let fonts = Fonts {
        sans: assets.load("fonts/NotoSans-Regular.ttf"),
        serif: assets.load("fonts/NotoSerif-Regular.ttf"),
        italic: assets.load("fonts/NotoSerif-Italic.ttf"),
    };
    let f = &fonts;

    commands
        .spawn((
            Name::new("hud root"),
            HudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .with_children(|root| {
            // Fear and exposure vignette, behind everything else.
            root.spawn((
                Vignette,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::FarthestCorner,
                    vec![ColorStop::percent(Color::NONE, 45), ColorStop::percent(Color::NONE, 100)],
                )),
            ));
            // Full-screen wash (susto flash, downed).
            root.spawn((
                Tint,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));

            // Objective checklist (compact, top left).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(18),
                    top: px(16),
                    max_width: px(400),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(px(12), px(10)),
                    row_gap: px(4),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .with_children(|p| {
                p.spawn((Text::new("EL SILBÓN — THE RETURN"), font(&f.serif, 12.5), TextColor(AMBER)));
                for i in 0..3 {
                    p.spawn((ObjectiveLine(i), label(f, "", 16.0, INK, false)));
                }
                p.spawn((ObjectiveHint, label(f, "", 13.0, PALE_BLUE, false)));
            });

            // Party roster (top centre, empty unless the night is shared).
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(12),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            })
            .with_children(|r| {
                for i in 0..4 {
                    r.spawn((RosterLine(i), label(f, "", 14.0, INK, false)));
                }
            });

            // Controls (bottom right, always readable, out of the way).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(18),
                    bottom: px(16),
                    max_width: px(640),
                    padding: UiRect::axes(px(10), px(6)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                children![label(
                    f,
                    "WASD move · Shift run · Ctrl crouch · E use / hold · F light · G drop · Q ají · V mark · M map · Esc",
                    12.0,
                    Color::srgb(0.62, 0.62, 0.58),
                    false,
                )],
            ));

            // Threat status (top right).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(22),
                    top: px(20),
                    max_width: px(440),
                    padding: UiRect::axes(px(12), px(8)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                Visibility::Hidden,
                StatusPanel,
                children![(Text::new(""), font(&f.sans, 17.0), TextColor(INK))],
            ));

            // Teaching hint (upper centre).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: percent(17),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    HintText,
                    Text::new(""),
                    font(&f.italic, 21.0),
                    TextColor(INK),
                    TextShadow::default(),
                    TextLayout::justify(Justify::Center),
                    Node {
                        max_width: px(820),
                        ..default()
                    },
                )],
            ));

            // Crosshair.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    width: px(6),
                    height: px(6),
                    margin: UiRect::new(px(-3), px(0), px(-3), px(0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.97, 0.9, 0.7)),
            ));

            // Prompt and hold progress (just below centre).
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: percent(54),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8),
                ..default()
            })
            .with_children(|p| {
                p.spawn((
                    PromptText,
                    Text::new(""),
                    font(&f.sans, 18.0),
                    TextColor(INK),
                    TextShadow::default(),
                ));
                p.spawn((ProgressLabel, Text::new(""), font(&f.italic, 15.0), TextColor(AMBER)));
                p.spawn((ProgressOuter, bar(260.0, 8.0, AMBER))).with_children(|b| {
                    b.spawn((ProgressFill, fill_node(Color::srgb(0.95, 0.72, 0.42))));
                });
                // Skill check: press Space as the needle crosses the zone.
                p.spawn((
                    SkillKey,
                    Text::new("SPACE"),
                    font(&f.serif, 13.0),
                    TextColor(AMBER),
                    Visibility::Hidden,
                ));
                p.spawn((
                    SkillBar,
                    Node {
                        width: px(340),
                        height: px(16),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(1.0, 0.85, 0.6, 0.6)),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    Visibility::Hidden,
                ))
                .with_children(|b| {
                    let mark = |color: Color| {
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                top: px(0),
                                height: percent(100),
                                left: percent(0),
                                width: percent(0),
                                ..default()
                            },
                            BackgroundColor(color),
                        )
                    };
                    b.spawn((SkillZone, mark(Color::srgba(0.95, 0.66, 0.3, 0.75))));
                    b.spawn((SkillGreat, mark(Color::srgba(1.0, 0.97, 0.88, 0.95))));
                    b.spawn((SkillNeedle, mark(Color::srgb(0.92, 0.2, 0.14))));
                });
            });

            // Perceived whistle caption (bottom centre).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: percent(8),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    CaptionText,
                    Text::new(""),
                    font(&f.italic, 19.0),
                    TextColor(PALE_BLUE),
                    TextShadow::default(),
                )],
            ));

            // Vitals (bottom left): susto, breath, peppers and load.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22),
                    bottom: px(20),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: UiRect::axes(px(12), px(8)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .with_children(|v| {
                v.spawn((Text::new("SUSTO"), font(&f.serif, 12.0), TextColor(RED)));
                v.spawn((FearOuter, bar(190.0, 8.0, RED))).with_children(|b| {
                    b.spawn((FearFill, fill_node(Color::srgb(0.85, 0.35, 0.28))));
                });
                v.spawn((BreathOuter, bar(190.0, 5.0, PALE_BLUE))).with_children(|b| {
                    b.spawn((BreathFill, fill_node(Color::srgb(0.7, 0.8, 0.95))));
                });
                v.spawn((VitalsText, label(f, "", 14.0, AMBER, false)));
            });

            // Downed / dead overlay.
            root.spawn((DownedPanel, overlay(), Visibility::Hidden, GlobalZIndex(6)))
                .with_children(|o| {
                    o.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(6),
                        margin: UiRect::top(px(-120)),
                        ..default()
                    })
                    .with_children(|c| {
                        c.spawn((
                            DownedText,
                            Text::new(""),
                            font(&f.serif, 30.0),
                            TextColor(RED),
                            TextShadow::default(),
                            TextLayout::justify(Justify::Center),
                        ));
                    });
                });

            // The map (M).
            map::spawn_map_panel(root, f, &map_image, &layout.0);

            // --- Overlays.
            root.spawn((BriefingPanel, overlay(), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)), GlobalZIndex(10)))
                .with_children(|o| {
                    o.spawn(card(860.0)).with_children(|c| {
                        c.spawn((Text::new("EL SILBÓN"), font(&f.serif, 64.0), TextColor(INK)));
                        c.spawn((
                            Text::new("The Return — some whistles should never be followed"),
                            font(&f.italic, 22.0),
                            TextColor(AMBER),
                        ));
                        c.spawn((
                            Text::new(
                                "Los Llanos, 1998. Your truck died at the river bridge and the road home is thirty \
                                 kilometres of dark. Ahead: a hacienda with a lamp still burning, a windmill that \
                                 could bring the power back, and five bundles of bones taken from El Silbón's sack. \
                                 He is out in the rain, he wants them back, and he listens to everything.",
                            ),
                            font(&f.sans, 16.0),
                            TextColor(INK),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            Text::new(
                                "The whistle lies: loud means he is far, thin means he is near. Walls, trunks and tall \
                                 grass break his sight. Everything you do makes a sound — crouch to sneak, run to be \
                                 heard; rain and thunder hide your steps. Fear grows in the dark and alone.",
                            ),
                            font(&f.italic, 17.0),
                            TextColor(PALE_BLUE),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            Text::new(
                                "Find the five bundles and lay them at the ceiba · restore power at the windmill · open \
                                 the padlocked key box (its numbers are in the pages) · start the truck and survive its \
                                 roar. Or learn which of him walks tonight and name him at the ceiba. Keep the rhythm \
                                 of the work (Space). Your torch runs down and its beam draws him. Ají stops him for a \
                                 while; Tureco, if you untie him, knows where he is. Get the fallen out of his sack.",
                            ),
                            font(&f.sans, 15.0),
                            TextColor(AMBER),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            Text::new(
                                "WASD move · Mouse look · Shift run · Ctrl/C crouch · E or click use (hold at sites) · F flashlight\n\
                                 Space skill check · G put a bundle down · Q scatter ají · V mark a spot · N name him (at the ceiba) · X drive off (with friends)\n\
                                 M map · Esc pause · F12 screenshot",
                            ),
                            font(&f.sans, 14.0),
                            TextColor(DIM),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            BriefingNight,
                            Text::new(""),
                            font(&f.italic, 14.0),
                            TextColor(DIM),
                        ));
                        c.spawn(button(f, "Begin — click to capture the mouse", MenuAction::Begin, 420.0));
                        c.spawn(button(f, "Back to the title", MenuAction::Title, 240.0));
                    });
                });

            root.spawn((OutcomePanel, overlay(), Visibility::Hidden, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)), GlobalZIndex(10)))
                .with_children(|o| {
                    o.spawn(card(700.0)).with_children(|c| {
                        c.spawn((OutcomeTitle, Text::new(""), font(&f.serif, 44.0), TextColor(INK)));
                        c.spawn((
                            OutcomeBody,
                            Text::new(""),
                            font(&f.sans, 17.0),
                            TextColor(INK),
                            TextLayout::justify(Justify::Center),
                        ));
                    });
                });

            // The key box's padlock: three dials, low on the screen so the
            // night stays in view while you fiddle with it.
            root.spawn((
                LockPanelUi,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: percent(14),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Visibility::Hidden,
                GlobalZIndex(8),
            ))
            .with_children(|o| {
                o.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(26), px(14)),
                        row_gap: px(6),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.8, 0.8, 0.82, 0.5)),
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.055, 0.9)),
                ))
                .with_children(|c| {
                    c.spawn((
                        Text::new("CANDADO · PADLOCK"),
                        font(&f.serif, 13.0),
                        TextColor(Color::srgb(0.75, 0.75, 0.78)),
                    ));
                    c.spawn((
                        LockDigits,
                        Text::new("0  0  0"),
                        font(&f.serif, 40.0),
                        TextColor(Color::srgb(0.92, 0.9, 0.84)),
                    ));
                    c.spawn((
                        Text::new("1 · 2 · 3 turn the dials (Shift back)   ·   Enter tries   ·   E closes\nA wrong try rattles — and the llano hears."),
                        font(&f.sans, 13.0),
                        TextColor(Color::srgb(0.7, 0.68, 0.64)),
                        TextLayout::justify(Justify::Center),
                    ));
                });
            });
            // Naming him at the ceiba.
            root.spawn((
                NamePanelUi,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: percent(14),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Visibility::Hidden,
                GlobalZIndex(8),
            ))
            .with_children(|o| {
                o.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(26), px(14)),
                        row_gap: px(8),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.85, 0.7, 0.45, 0.5)),
                    BackgroundColor(Color::srgba(0.05, 0.035, 0.02, 0.92)),
                ))
                .with_children(|c| {
                    c.spawn((
                        Text::new("¿CUÁL DE ÉL CAMINA ESTA NOCHE? · WHICH OF HIM WALKS TONIGHT?"),
                        font(&f.serif, 13.0),
                        TextColor(Color::srgb(0.85, 0.72, 0.5)),
                    ));
                    c.spawn((
                        NameChoices,
                        Text::new(""),
                        font(&f.italic, 20.0),
                        TextColor(Color::srgb(0.95, 0.9, 0.8)),
                        TextLayout::justify(Justify::Center),
                    ));
                    c.spawn((
                        Text::new("1 · 2 · 3 choose   ·   Enter names him   ·   N closes\nA wrong name enrages him, and the ceiba will not listen for a while."),
                        font(&f.sans, 13.0),
                        TextColor(Color::srgb(0.7, 0.66, 0.6)),
                        TextLayout::justify(Justify::Center),
                    ));
                });
            });
            root.spawn((NotePanel, overlay(), Visibility::Hidden, GlobalZIndex(9))).with_children(|o| {
                o.spawn((
                    NotePaper,
                    Node {
                        width: px(640),
                        max_height: percent(92),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(30)),
                        row_gap: px(12),
                        border_radius: BorderRadius::all(px(3)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.78, 0.72, 0.58, 0.96)),
                ))
                .with_children(|c| {
                    let ink = Color::srgb(0.16, 0.12, 0.1);
                    c.spawn((NoteTitle, Text::new(""), font(&f.serif, 14.0), TextColor(ink)));
                    c.spawn((NoteEs, Text::new(""), font(&f.italic, 21.0), TextColor(ink)));
                    c.spawn((NoteBy, Text::new(""), font(&f.italic, 17.0), TextColor(ink)));
                    c.spawn((NoteEn, Text::new(""), font(&f.sans, 15.0), TextColor(Color::srgb(0.25, 0.2, 0.16))));
                    c.spawn((
                        NoteCount,
                        Text::new("[E] put the page down"),
                        font(&f.sans, 14.0),
                        TextColor(Color::srgb(0.3, 0.24, 0.18)),
                    ));
                });
            });
        });
    commands.insert_resource(fonts);
}

fn buttons(
    mut query: Query<(&Interaction, &MenuAction, &mut BackgroundColor), Changed<Interaction>>,
    mut next: ResMut<NextState<Flow>>,
    mut leave: MessageWriter<crate::net::LeaveRun>,
) {
    for (interaction, action, mut bg) in &mut query {
        match interaction {
            Interaction::Pressed => {
                bg.0 = BUTTON_PRESS;
                match action {
                    MenuAction::Begin => next.set(Flow::Playing),
                    MenuAction::Title => {
                        leave.write(crate::net::LeaveRun);
                    }
                }
            }
            Interaction::Hovered => bg.0 = BUTTON_HOVER,
            Interaction::None => bg.0 = BUTTON,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn panels(
    state: Res<State<Flow>>,
    menu: Res<menu::Menu>,
    launch: Res<Launch>,
    fright: Res<crate::world::omen::Fright>,
    mut q: ParamSet<(
        Query<&mut Visibility, With<BriefingPanel>>,
        Query<&mut Visibility, With<HudRoot>>,
        Query<&mut Visibility, With<OutcomePanel>>,
        Query<(&mut BackgroundColor, &mut Visibility), With<CatchVeil>>,
    )>,
    mut night: Query<&mut Text, With<BriefingNight>>,
) {
    // The catch has the screen: no HUD over him, and its black at the end.
    let black = fright.lunge.map_or(0.0, crate::world::silbon::lunge_black);
    let want = Color::BLACK.with_alpha(black);
    for (mut veil_bg, mut veil_vis) in &mut q.p3() {
        set_vis(&mut veil_vis, black > 0.0);
        if veil_bg.0 != want {
            veil_bg.0 = want;
        }
    }
    let s = *state.get();
    for mut v in &mut q.p0() {
        set_vis(&mut v, s == Flow::Briefing);
    }
    if s == Flow::Briefing {
        let line = format!(
            "Night #{} ({}) — the bundles, the padlock and which of him walks change with every night.",
            launch.seed,
            launch.night.label()
        );
        for mut t in &mut night {
            set_text(&mut t, &line);
        }
    }
    // The title screen shows the llano and its menus, no HUD (the photo
    // driver manages the HUD itself).
    if !launch.photos {
        for mut v in &mut q.p1() {
            set_vis(&mut v, s != Flow::Title && fright.lunge.is_none());
        }
    }
    // The outcome card steps aside for a confirmation.
    for mut v in &mut q.p2() {
        set_vis(&mut v, s == Flow::Outcome && menu.page == menu::Page::Outcome);
    }
}

/// The night's awards, one line per player who earned any.
fn night_awards(net: &Network) -> String {
    use crate::awards::Award;
    let Some(s) = net.snapshot().filter(|s| !s.deeds.is_empty()) else {
        return String::new();
    };
    let night = crate::awards::Night {
        party: &s.deeds,
        left_behind: &s.left_behind,
        dog_friend: (s.dog.owner != 0).then_some(s.dog.owner),
    };
    let given = crate::awards::awards(&night);
    let label = |a: Award| match a {
        Award::LeftBehind => "Left behind on the llano",
        Award::SackRider => "Rode in his sack",
        Award::FirstToFall => "First to fall",
        Award::Screamer => "Screamed the most",
        Award::Butterfingers => "Butterfingers (dropped the bones)",
        Award::HisFavourite => "His favourite (warned the most)",
        Award::GuardianAngel => "Guardian angel (got someone up)",
        Award::BoneBearer => "Bone bearer (laid the most)",
        Award::PepperHand => "Ají in every pocket",
        Award::Stampede => "Started a stampede",
        Award::TurecosFriend => "Tureco's friend",
        Award::Untouched => "Untouched (never warned, never scared)",
    };
    let me = net.id();
    let solo = s.deeds.len() < 2;
    let mut lines = Vec::new();
    for (slot, p) in s.players.iter().enumerate() {
        let mine: Vec<&str> = given
            .iter()
            .filter(|(id, _)| *id == p.id)
            .map(|(_, a)| label(*a))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let who = if solo {
            "You".to_string()
        } else if Some(p.id) == me {
            format!("P{} (you)", slot + 1)
        } else {
            format!("P{}", slot + 1)
        };
        lines.push(format!("{who}: {}", mine.join("  ·  ")));
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("\n\nTonight's awards\n{}", lines.join("\n"))
    }
}

fn fill_outcome(
    truth: Res<Truth>,
    net: Res<Network>,
    tuning: Res<TuningRes>,
    read: Res<crate::encounter::PagesRead>,
    mut q: ParamSet<(
        Query<&mut Text, With<OutcomeTitle>>,
        Query<&mut Text, With<OutcomeBody>>,
    )>,
) {
    let enc = &truth.encounter;
    let secs = enc.elapsed.max(0.0) as u32;
    let (home, total) = net.snapshot().map_or((0, 5), |s| (s.world.delivered, s.world.total));
    let stats = format!(
        "Time: {}:{:02}  ·  Bones at rest: {home}/{total}  ·  Times he warned: {}  ·  Slipped his sight: {}\n\
         Times downed: {}  ·  Revived: {}  ·  Pages of the tale found: {}/{}",
        secs / 60,
        secs % 60,
        enc.stats.warnings,
        enc.stats.recoveries,
        enc.stats.downs,
        enc.stats.revives,
        read.0.len(),
        crate::lore::PAGES,
    );
    // Every page read: the tale is whole, and it says so.
    let stats = if read.0.len() >= crate::lore::PAGES as usize {
        format!(
            "{stats}\n\nThe whole tale is told: the son, the deer, the father, the grandfather's curse, \
             the torn sack and the ranch that tried to lay the father down. \
             Somewhere the radio still says: if you hear the whistle, remember…"
        )
    } else {
        stats
    };
    let banished = net.snapshot().is_some_and(|s| s.world.banished);
    // Someone drove off early: who was left, and was it me.
    let left = net.snapshot().map_or(0, |s| s.left_behind.len());
    let abandoned = net
        .snapshot()
        .zip(net.id())
        .is_some_and(|(s, me)| s.left_behind.contains(&me));
    // Who walked tonight, told afterwards: the signs are there to learn.
    let who = match crate::sim::Variant::of(tuning.0.seed) {
        crate::sim::Variant::Borracho => "the drunkard's return (El Borracho)",
        crate::sim::Variant::Hijo => "the son himself (El Hijo)",
        crate::sim::Variant::Arriero => "the drover (El Arriero)",
    };
    let won = enc.outcome == crate::sim::Outcome::Won;
    let mut marks: Vec<&str> = Vec::new();
    if won && enc.stats.warnings == 0 {
        marks.push("Silent as the grass (he never saw you)");
    }
    if won && enc.stats.downs == 0 {
        marks.push("Unbroken (nobody fell)");
    }
    if won && enc.stats.revives > 0 {
        marks.push("Nobody left behind (the fallen got up)");
    }
    if banished {
        marks.push("The one who named him");
    }
    if won && secs < 8 * 60 {
        marks.push("Quick hands (out before eight minutes)");
    }
    if read.0.len() >= crate::lore::PAGES as usize {
        marks.push("Keeper of the tale (every page read)");
    }
    if won && left > 0 && !abandoned {
        marks.push("Every one for themselves (drove off early)");
    }
    let marks = if marks.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", marks.join("  ·  "))
    };
    let stats = format!(
        "{stats}\n\nTonight it was {who}. Night #{}.{marks}{}",
        tuning.0.seed,
        night_awards(&net)
    );
    let (title, body) = if enc.outcome == crate::sim::Outcome::Won && banished {
        (
            "He is laid to rest.",
            format!(
                "You named him at the roots of the ceiba, with his father's bones all home. The whistle unwinds, \
                 lower and lower, into the rain, and the llano is only the llano again.\n\n{stats}"
            ),
        )
    } else if won && abandoned {
        (
            "They left without you.",
            format!(
                "The truck's lights shrink down the road and the engine fades into the rain. The llano is very \
                 quiet. Somewhere a whistle starts, thin and far away…\n\n{stats}"
            ),
        )
    } else if won && left > 0 {
        (
            "The truck pulls away.",
            format!(
                "You did not wait. In the mirror the llano closes over the ones you left, and somewhere out there \
                 a whistle goes thin and far away…\n\n{stats}"
            ),
        )
    } else if enc.outcome == crate::sim::Outcome::Won {
        (
            "The truck pulls away.",
            format!(
                "Behind you the rain hushes the llano. The bones rest in the ceiba's roots, and somewhere out there \
                 a whistle goes thin and far away… for now.\n\n{stats}"
            ),
        )
    } else {
        (
            "He found you.",
            format!(
                "The whistle had gone thin and far away — he was already near.\n\
                 Next time: stay together, stay in the light, and put walls between you when it fades.\n\n{stats}"
            ),
        )
    };
    for mut t in &mut q.p0() {
        set_text(&mut t, title);
    }
    for mut t in &mut q.p1() {
        set_text(&mut t, &body);
    }
}

fn reset_ui(
    mut requests: MessageReader<RunReset>,
    mut hint: ResMut<Hint>,
    mut caption: ResMut<CaptionLine>,
    mut map: ResMut<MapOpen>,
) {
    if requests.read().count() > 0 {
        let taught = (
            hint.taught_loud,
            hint.taught_faint,
            hint.taught_crouch,
            hint.taught_skill,
        );
        *hint = Hint::default();
        // Lessons already learned stay learned across restarts.
        (
            hint.taught_loud,
            hint.taught_faint,
            hint.taught_crouch,
            hint.taught_skill,
        ) = taught;
        *caption = CaptionLine::default();
        map.0 = false;
    }
}
