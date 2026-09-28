//! HUD and menus: objective panel, controls, whistle legend, threat status,
//! crosshair prompt and restitution progress, perceived-whistle captions,
//! teaching hints, exposure vignette, and the briefing / pause (settings) /
//! outcome / note overlays. All nodes are spawned once; states only toggle
//! visibility and text, so restarts never accumulate UI.

use bevy::prelude::*;

use crate::app::{EncounterMsg, Flow, GameSet, RestartRequest, Settings, Truth, TuningRes, WhistleMsg};
use crate::control::TargetKind;
use crate::encounter::{CurrentTarget, NoteOpen};
use crate::geometry::AimStatus;
use crate::perception::WhistleVariant;
use crate::sim::{Event, Objective, Presence, ThreatState};

const PANEL_BG: Color = Color::srgba(0.018, 0.022, 0.03, 0.5);
const INK: Color = Color::srgb(0.93, 0.9, 0.84);
const DIM: Color = Color::srgb(0.66, 0.66, 0.64);
const AMBER: Color = Color::srgb(0.98, 0.72, 0.36);
const RED: Color = Color::srgb(1.0, 0.42, 0.34);
const PALE_BLUE: Color = Color::srgb(0.7, 0.8, 0.95);
const BUTTON: Color = Color::srgba(0.12, 0.11, 0.1, 0.92);
const BUTTON_HOVER: Color = Color::srgba(0.24, 0.2, 0.15, 0.95);
const BUTTON_PRESS: Color = Color::srgba(0.42, 0.3, 0.16, 0.95);

#[derive(Resource)]
struct Fonts {
    sans: Handle<Font>,
    serif: Handle<Font>,
    italic: Handle<Font>,
}

#[derive(Resource, Default)]
struct Hint {
    text: &'static str,
    timer: f32,
    priority: u8,
    taught_loud: bool,
    taught_faint: bool,
}

#[derive(Resource, Default)]
struct CaptionLine {
    text: &'static str,
    timer: f32,
}

#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct ObjectiveTitle;
#[derive(Component)]
struct ObjectiveDetail;
#[derive(Component)]
struct LegendText;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct PromptText;
#[derive(Component)]
struct ProgressOuter;
#[derive(Component)]
struct ProgressFill;
#[derive(Component)]
struct CaptionText;
#[derive(Component)]
struct HintText;
#[derive(Component)]
struct CarryTag;
#[derive(Component)]
struct Vignette;
#[derive(Component)]
struct BriefingPanel;
#[derive(Component)]
struct PausePanel;
#[derive(Component)]
struct OutcomePanel;
#[derive(Component)]
struct OutcomeTitle;
#[derive(Component)]
struct OutcomeBody;
#[derive(Component)]
struct NotePanel;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum SettingLabel {
    Volume,
    Sensitivity,
    Captions,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum MenuAction {
    Begin,
    Resume,
    Restart,
    Quit,
    VolumeDown,
    VolumeUp,
    SensitivityDown,
    SensitivityUp,
    ToggleCaptions,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hint>()
            .init_resource::<CaptionLine>()
            .add_systems(Startup, spawn_ui)
            .add_systems(OnEnter(Flow::Outcome), fill_outcome)
            .add_systems(
                Update,
                (
                    reset_ui.in_set(GameSet::Control),
                    (
                        buttons,
                        panels,
                        objective_text,
                        status_text,
                        prompt,
                        carry_tag,
                        vignette,
                        hints_and_captions,
                        settings_text,
                    )
                        .chain()
                        .in_set(GameSet::Present),
                ),
            );
    }
}

fn font(h: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: h.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn set_text(text: &mut Text, s: &str) {
    if text.0 != s {
        text.0 = s.to_string();
    }
}

fn set_vis(v: &mut Visibility, show: bool) {
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *v != want {
        *v = want;
    }
}

fn label(fonts: &Fonts, text: &str, size: f32, color: Color, italic: bool) -> impl Bundle {
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

fn spawn_ui(mut commands: Commands, assets: Res<AssetServer>) {
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
            // Exposure vignette, behind everything else.
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

            // Objective and whistle legend (compact, top left).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(18),
                    top: px(16),
                    max_width: px(370),
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
                p.spawn((ObjectiveTitle, label(f, "", 19.0, INK, false)));
                p.spawn((ObjectiveDetail, label(f, "", 14.0, DIM, false)));
                p.spawn((
                    LegendText,
                    Visibility::Hidden,
                    label(
                        f,
                        "Loud whistle = he is far · Faint whistle = he is near · Solid walls break his sight",
                        13.0,
                        PALE_BLUE,
                        false,
                    ),
                ));
            });

            // Controls (bottom right, always readable, out of the way).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(18),
                    bottom: px(16),
                    max_width: px(560),
                    padding: UiRect::axes(px(10), px(6)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                children![label(
                    f,
                    "WASD move · Mouse look · E / click interact (hold at the ceiba) · F flashlight · Esc pause · F12 screenshot",
                    12.5,
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
                    max_width: px(420),
                    padding: UiRect::axes(px(12), px(8)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                Visibility::Hidden,
                StatusText,
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

            // Prompt and restitution progress (just below centre).
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: percent(54),
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    ..default()
                },
            ))
            .with_children(|p| {
                p.spawn((PromptText, Text::new(""), font(&f.sans, 18.0), TextColor(INK), TextShadow::default()));
                p.spawn((
                    ProgressOuter,
                    Visibility::Hidden,
                    Node {
                        width: px(260),
                        height: px(8),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(1.0, 0.85, 0.6, 0.6)),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    children![(
                        ProgressFill,
                        Node {
                            width: percent(0),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.95, 0.72, 0.42)),
                    )],
                ));
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

            // Carry tag (bottom left).
            root.spawn((
                CarryTag,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22),
                    bottom: px(20),
                    padding: UiRect::axes(px(12), px(7)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                children![(
                    Text::new("Carrying the bones — you move slower"),
                    font(&f.sans, 15.0),
                    TextColor(AMBER)
                )],
            ));

            // --- Overlays.
            root.spawn((BriefingPanel, overlay(), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)), GlobalZIndex(10)))
                .with_children(|o| {
                    o.spawn(card(780.0)).with_children(|c| {
                        c.spawn((Text::new("EL SILBÓN"), font(&f.serif, 64.0), TextColor(INK)));
                        c.spawn((Text::new("The Return — a first local encounter"), font(&f.italic, 22.0), TextColor(AMBER)));
                        c.spawn((
                            Text::new(
                                "Los Llanos, 1998. The truck died a few kilometres back. By the road: a fenced hato, \
                                 an old ceiba, and a house where a lamp still burns. Something in that house was taken \
                                 from the tree. Carry it back to the ceiba's roots, then return to the road.",
                            ),
                            font(&f.sans, 17.0),
                            TextColor(INK),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            Text::new(
                                "The whistle lies. When it sounds loud and close, he is far away. \
                                 When it sounds thin and far away, he is near. Solid walls break his sight.",
                            ),
                            font(&f.italic, 18.0),
                            TextColor(PALE_BLUE),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn((
                            Text::new(
                                "WASD move · Mouse look · E or left click interact (hold at the ceiba) · F flashlight\n\
                                 Esc pause & settings (volume, sensitivity, captions) · F12 screenshot",
                            ),
                            font(&f.sans, 15.0),
                            TextColor(DIM),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn(button(f, "Begin — click to capture the mouse", MenuAction::Begin, 420.0));
                        c.spawn(button(f, "Quit", MenuAction::Quit, 160.0));
                    });
                });

            root.spawn((PausePanel, overlay(), Visibility::Hidden, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)), GlobalZIndex(10)))
                .with_children(|o| {
                    o.spawn(card(560.0)).with_children(|c| {
                        c.spawn((Text::new("Paused"), font(&f.serif, 40.0), TextColor(INK)));
                        c.spawn((Text::new("The encounter is frozen. The mouse is free."), font(&f.sans, 15.0), TextColor(DIM)));
                        for (kind, down, up) in [
                            (SettingLabel::Volume, MenuAction::VolumeDown, MenuAction::VolumeUp),
                            (SettingLabel::Sensitivity, MenuAction::SensitivityDown, MenuAction::SensitivityUp),
                        ] {
                            c.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(8),
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn(button(f, "−", down, 52.0));
                                row.spawn((
                                    kind,
                                    Text::new(""),
                                    font(&f.sans, 18.0),
                                    TextColor(INK),
                                    Node {
                                        width: px(280),
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    TextLayout::justify(Justify::Center),
                                ));
                                row.spawn(button(f, "+", up, 52.0));
                            });
                        }
                        c.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((SettingLabel::Captions, Text::new(""), font(&f.sans, 18.0), TextColor(INK)));
                            row.spawn(button(f, "Toggle", MenuAction::ToggleCaptions, 110.0));
                        });
                        c.spawn(button(f, "Resume", MenuAction::Resume, 300.0));
                        c.spawn(button(f, "Restart from the road", MenuAction::Restart, 300.0));
                        c.spawn(button(f, "Quit", MenuAction::Quit, 300.0));
                        c.spawn((Text::new("Esc also resumes."), font(&f.sans, 13.0), TextColor(DIM)));
                    });
                });

            root.spawn((OutcomePanel, overlay(), Visibility::Hidden, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)), GlobalZIndex(10)))
                .with_children(|o| {
                    o.spawn(card(680.0)).with_children(|c| {
                        c.spawn((OutcomeTitle, Text::new(""), font(&f.serif, 44.0), TextColor(INK)));
                        c.spawn((
                            OutcomeBody,
                            Text::new(""),
                            font(&f.sans, 17.0),
                            TextColor(INK),
                            TextLayout::justify(Justify::Center),
                        ));
                        c.spawn(button(f, "Play again (R)", MenuAction::Restart, 300.0));
                        c.spawn(button(f, "Quit", MenuAction::Quit, 300.0));
                    });
                });

            root.spawn((NotePanel, overlay(), Visibility::Hidden, GlobalZIndex(9))).with_children(|o| {
                o.spawn((
                    Node {
                        width: px(560),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(30)),
                        row_gap: px(12),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.78, 0.72, 0.58, 0.96)),
                ))
                .with_children(|c| {
                    let ink = Color::srgb(0.16, 0.12, 0.1);
                    c.spawn((
                        Text::new(
                            "Si lo oyes cerca, está lejos.\n\
                             Si lo oyes lejos, ya está aquí.\n\
                             Que no te vea: ponte tras las paredes.\n\
                             Los huesos van a la ceiba, a sus raíces.",
                        ),
                        font(&f.italic, 23.0),
                        TextColor(ink),
                    ));
                    c.spawn((Text::new("— M."), font(&f.italic, 18.0), TextColor(ink)));
                    c.spawn((
                        Text::new(
                            "If you hear him close, he is far. If you hear him far, he is already here. \
                             Don't let him see you: get behind the walls. The bones go to the ceiba, to its roots.",
                        ),
                        font(&f.sans, 15.0),
                        TextColor(Color::srgb(0.25, 0.2, 0.16)),
                    ));
                    c.spawn((Text::new("[E] put the note down"), font(&f.sans, 14.0), TextColor(Color::srgb(0.3, 0.24, 0.18))));
                });
            });
        });
    commands.insert_resource(fonts);
}

fn buttons(
    mut query: Query<(&Interaction, &MenuAction, &mut BackgroundColor), Changed<Interaction>>,
    mut next: ResMut<NextState<Flow>>,
    mut restart: MessageWriter<RestartRequest>,
    mut exit: MessageWriter<AppExit>,
    mut settings: ResMut<Settings>,
    tuning: Res<TuningRes>,
) {
    for (interaction, action, mut bg) in &mut query {
        match interaction {
            Interaction::Pressed => {
                bg.0 = BUTTON_PRESS;
                let (smin, smax) = tuning.0.sensitivity_range;
                match action {
                    MenuAction::Begin | MenuAction::Resume => next.set(Flow::Playing),
                    MenuAction::Restart => {
                        restart.write(RestartRequest);
                    }
                    MenuAction::Quit => {
                        exit.write(AppExit::Success);
                    }
                    MenuAction::VolumeDown => settings.volume = ((settings.volume - 0.1) * 10.0).round() / 10.0,
                    MenuAction::VolumeUp => settings.volume = ((settings.volume + 0.1) * 10.0).round() / 10.0,
                    MenuAction::SensitivityDown => {
                        settings.sensitivity = ((settings.sensitivity - 0.1) * 10.0).round() / 10.0;
                    }
                    MenuAction::SensitivityUp => {
                        settings.sensitivity = ((settings.sensitivity + 0.1) * 10.0).round() / 10.0;
                    }
                    MenuAction::ToggleCaptions => settings.captions = !settings.captions,
                }
                settings.volume = settings.volume.clamp(0.0, 1.0);
                settings.sensitivity = settings.sensitivity.clamp(smin, smax);
            }
            Interaction::Hovered => bg.0 = BUTTON_HOVER,
            Interaction::None => bg.0 = BUTTON,
        }
    }
}

fn panels(
    state: Res<State<Flow>>,
    note: Res<NoteOpen>,
    mut q: ParamSet<(
        Query<&mut Visibility, With<BriefingPanel>>,
        Query<&mut Visibility, With<PausePanel>>,
        Query<&mut Visibility, With<OutcomePanel>>,
        Query<&mut Visibility, With<NotePanel>>,
    )>,
) {
    let s = *state.get();
    for mut v in &mut q.p0() {
        set_vis(&mut v, s == Flow::Briefing);
    }
    for mut v in &mut q.p1() {
        set_vis(&mut v, s == Flow::Paused);
    }
    for mut v in &mut q.p2() {
        set_vis(&mut v, s == Flow::Outcome);
    }
    for mut v in &mut q.p3() {
        set_vis(&mut v, s == Flow::Playing && note.0);
    }
}

fn objective_text(
    truth: Res<Truth>,
    mut q: ParamSet<(
        Query<&mut Text, With<ObjectiveTitle>>,
        Query<&mut Text, With<ObjectiveDetail>>,
        Query<&mut Visibility, With<LegendText>>,
    )>,
) {
    let (title, detail) = match truth.encounter.objective {
        Objective::FindSatchel => (
            "Find the bone satchel in the house",
            "A lamp still burns inside. It should be on the table.",
        ),
        Objective::ReturnBones => (
            "Return the bones to the ceiba",
            "Carry them to the great tree behind the house and hold E at the hollow in its roots.",
        ),
        Objective::Escape => ("Go back to the road", "Out through the gate. It is over."),
        Objective::Won => ("You reached the road", ""),
        Objective::Failed => ("He found you", ""),
    };
    for mut t in &mut q.p0() {
        set_text(&mut t, title);
    }
    for mut t in &mut q.p1() {
        set_text(&mut t, detail);
    }
    let legend = truth.encounter.objective != Objective::FindSatchel;
    for mut v in &mut q.p2() {
        set_vis(&mut v, legend);
    }
}

fn status_text(
    truth: Res<Truth>,
    mut panel: Query<(&mut Visibility, &Children), With<StatusText>>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    let th = &truth.encounter.threat;
    let present = matches!(th.presence, Presence::Present | Presence::Rising { .. });
    let status: Option<(&str, Color)> = if truth.encounter.objective.is_over() {
        None
    } else {
        match th.state {
            ThreatState::Warning => Some(("He has seen you — break his line of sight", AMBER)),
            ThreatState::Hunting if th.has_sight => Some(("He is coming — get behind solid walls!", RED)),
            ThreatState::Hunting => Some(("Out of his sight… stay hidden", PALE_BLUE)),
            ThreatState::Stalking if present => Some(("He is out there on the llano", DIM)),
            _ => None,
        }
    };
    for (mut vis, children) in &mut panel {
        set_vis(&mut vis, status.is_some());
        if let Some((s, c)) = status {
            let kids: &[Entity] = children;
            for &child in kids {
                if let Ok((mut t, mut color)) = texts.get_mut(child) {
                    set_text(&mut t, s);
                    if color.0 != c {
                        color.0 = c;
                    }
                }
            }
        }
    }
}

fn prompt(
    truth: Res<Truth>,
    target: Res<CurrentTarget>,
    note: Res<NoteOpen>,
    state: Res<State<Flow>>,
    mut q: ParamSet<(
        Query<&mut Text, With<PromptText>>,
        Query<&mut Visibility, With<ProgressOuter>>,
        Query<&mut Node, With<ProgressFill>>,
    )>,
) {
    let enc = &truth.encounter;
    let playing = *state.get() == Flow::Playing;
    let restituting = enc.objective == Objective::ReturnBones && enc.restitution > 0.0;
    let text = if !playing {
        ""
    } else if restituting && enc.restituting {
        "Returning the bones…"
    } else {
        match target.0 {
            Some(t) => match (t.kind, t.status) {
                (TargetKind::Satchel, AimStatus::Ready { .. }) => "[E] Take the bone satchel",
                (TargetKind::Satchel, _) => "The bone satchel — move closer",
                (TargetKind::Note, AimStatus::Ready { .. }) if note.0 => "[E] Put the note down",
                (TargetKind::Note, AimStatus::Ready { .. }) => "[E] Read the note",
                (TargetKind::Note, _) => "A note — move closer",
                (TargetKind::Offering, AimStatus::Ready { .. }) => "[Hold E] Return the bones to the ceiba",
                (TargetKind::Offering, _) => "The hollow in the roots — move closer",
            },
            None if restituting => "Paused — hold E at the hollow in the roots",
            None => "",
        }
    };
    for mut t in &mut q.p0() {
        set_text(&mut t, text);
    }
    for mut v in &mut q.p1() {
        set_vis(&mut v, playing && restituting);
    }
    let w = percent((enc.restitution * 100.0).round());
    for mut n in &mut q.p2() {
        if n.width != w {
            n.width = w;
        }
    }
}

fn carry_tag(truth: Res<Truth>, state: Res<State<Flow>>, mut q: Query<&mut Visibility, With<CarryTag>>) {
    let show = truth.encounter.carrying() && *state.get() != Flow::Briefing;
    for mut v in &mut q {
        set_vis(&mut v, show);
    }
}

fn vignette(
    time: Res<Time>,
    truth: Res<Truth>,
    mut q: Query<&mut BackgroundGradient, With<Vignette>>,
    mut shown: Local<f32>,
) {
    let th = &truth.encounter.threat;
    let pulse = if th.state == ThreatState::Warning {
        0.18 + 0.08 * (time.elapsed_secs() * 3.0).sin()
    } else {
        0.0
    };
    let target = (th.exposure * 0.85).max(pulse).min(0.9);
    let k = (time.delta_secs() * 6.0).min(1.0);
    let mut a = *shown + (target - *shown) * k;
    if a < 0.01 {
        a = 0.0;
    }
    if a == *shown || ((a - *shown).abs() < 0.004 && a != 0.0) {
        return;
    }
    *shown = a;
    let edge = Color::srgba(0.06, 0.0, 0.0, a);
    for mut g in &mut q {
        *g = BackgroundGradient::from(RadialGradient::new(
            UiPosition::CENTER,
            RadialGradientShape::FarthestCorner,
            vec![ColorStop::percent(Color::NONE, 45), ColorStop::percent(edge, 100)],
        ));
    }
}

fn hints_and_captions(
    time: Res<Time>,
    settings: Res<Settings>,
    mut hint: ResMut<Hint>,
    mut caption: ResMut<CaptionLine>,
    mut events: MessageReader<EncounterMsg>,
    mut phrases: MessageReader<WhistleMsg>,
    mut q: ParamSet<(Query<&mut Text, With<HintText>>, Query<&mut Text, With<CaptionText>>)>,
) {
    let dt = time.delta_secs();
    let show = |hint: &mut Hint, text: &'static str, secs: f32, priority: u8| {
        if hint.timer <= 0.0 || priority >= hint.priority {
            hint.text = text;
            hint.timer = secs;
            hint.priority = priority;
        }
    };
    for EncounterMsg(e) in events.read() {
        match e {
            Event::SatchelTaken => show(
                &mut hint,
                "The satchel is heavier than it looks. Something out on the llano knows.",
                6.0,
                1,
            ),
            Event::WarningBegan => show(
                &mut hint,
                "He has seen you. Get solid walls between you before he comes.",
                6.0,
                2,
            ),
            Event::HuntBegan => show(&mut hint, "He is coming. Break his line of sight!", 4.0, 2),
            Event::WarningAverted => show(&mut hint, "He lost sight of you.", 4.0, 2),
            Event::LostTrack => show(
                &mut hint,
                "He lost your trail and sinks into the grass. He will rise somewhere else.",
                6.0,
                2,
            ),
            Event::RestitutionComplete => {
                show(
                    &mut hint,
                    "The bones are home. The whistling stops. Go back to the road.",
                    7.0,
                    3,
                );
            }
            _ => {}
        }
    }
    for WhistleMsg(p) in phrases.read() {
        caption.text = p.variant.caption();
        caption.timer = 4.5;
        match p.variant {
            WhistleVariant::Loud if !hint.taught_loud => {
                hint.taught_loud = true;
                show(
                    &mut hint,
                    "A loud whistle, as if right beside you. The old rule: when he sounds near, he is far.",
                    8.0,
                    3,
                );
            }
            WhistleVariant::Faint if !hint.taught_faint => {
                hint.taught_faint = true;
                show(
                    &mut hint,
                    "A thin whistle, far, far away… so he is NEAR. Get solid walls between you.",
                    8.0,
                    3,
                );
            }
            _ => {}
        }
    }
    hint.timer -= dt;
    caption.timer -= dt;
    let hint_text = if hint.timer > 0.0 { hint.text } else { "" };
    let caption_text = if caption.timer > 0.0 && settings.captions {
        caption.text
    } else {
        ""
    };
    for mut t in &mut q.p0() {
        set_text(&mut t, hint_text);
    }
    for mut t in &mut q.p1() {
        set_text(&mut t, caption_text);
    }
}

fn settings_text(settings: Res<Settings>, mut q: Query<(&SettingLabel, &mut Text)>, mut first: Local<bool>) {
    if !settings.is_changed() && *first {
        return;
    }
    *first = true;
    for (kind, mut t) in &mut q {
        let s = match kind {
            SettingLabel::Volume => format!("Volume: {:.0}%", settings.volume * 100.0),
            SettingLabel::Sensitivity => format!("Mouse sensitivity: {:.1}×", settings.sensitivity),
            SettingLabel::Captions => {
                format!("Whistle captions: {}", if settings.captions { "On" } else { "Off" })
            }
        };
        set_text(&mut t, &s);
    }
}

fn fill_outcome(
    truth: Res<Truth>,
    mut q: ParamSet<(
        Query<&mut Text, With<OutcomeTitle>>,
        Query<&mut Text, With<OutcomeBody>>,
    )>,
) {
    let enc = &truth.encounter;
    let secs = enc.elapsed.max(0.0) as u32;
    let stats = format!(
        "Time: {}:{:02}   ·   Warnings: {}   ·   Times you slipped his sight: {}",
        secs / 60,
        secs % 60,
        enc.stats.warnings,
        enc.stats.recoveries
    );
    let (title, body) = if enc.objective == Objective::Won {
        (
            "You made it back to the road.",
            format!("The bones rest in the ceiba's roots. Behind you the llano is quiet again.\n\n{stats}"),
        )
    } else {
        (
            "He found you.",
            format!(
                "The whistle had gone thin and far away — he was already near.\n\
                 Next time, put solid walls between you as soon as it fades.\n\n{stats}"
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

fn reset_ui(mut requests: MessageReader<RestartRequest>, mut hint: ResMut<Hint>, mut caption: ResMut<CaptionLine>) {
    if requests.read().count() > 0 {
        *hint = Hint::default();
        *caption = CaptionLine::default();
    }
}
