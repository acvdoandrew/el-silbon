//! HUD update systems: they only read the latest snapshot, the crosshair
//! target and this frame's messages, and toggle text/visibility.

use bevy::prelude::*;

use super::*;
use crate::app::{EncounterMsg, WhistleMsg};
use crate::control::{Blocked, Target, TargetKind};
use crate::encounter::{CurrentTarget, NoteOpen};
use crate::perception::WhistleVariant;
use crate::sim::{Event, ThreatState};

fn tick(done: bool) -> &'static str {
    if done { "[x]" } else { "[ ]" }
}

fn set_color(c: &mut TextColor, color: Color) {
    if c.0 != color {
        c.0 = color;
    }
}

pub(crate) fn objectives(
    net: Res<Network>,
    mut lines: Query<(&ObjectiveLine, &mut Text, &mut TextColor)>,
    mut hint: Query<&mut Text, (With<ObjectiveHint>, Without<ObjectiveLine>)>,
) {
    let Some(s) = net.snapshot() else {
        for (_, mut t, _) in &mut lines {
            set_text(&mut t, "");
        }
        return;
    };
    let w = &s.world;
    let carrying = net.carrying();
    let bones_home = w.delivered >= w.total;
    let power_on = w.power >= 1.0;
    let running = w.truck >= 1.0;
    let ready = running && w.warm >= 1.0;

    let bones = if carrying > 0 {
        format!(
            "{} Lay the bones at the ceiba: {}/{}  (you carry {carrying})",
            tick(bones_home),
            w.delivered,
            w.total
        )
    } else {
        format!(
            "{} Lay the bones at the ceiba: {}/{}",
            tick(bones_home),
            w.delivered,
            w.total
        )
    };
    let power = if power_on {
        format!("{} Power restored", tick(true))
    } else if w.power > 0.0 {
        format!("{} Restore power at the windmill: {:.0}%", tick(false), w.power * 100.0)
    } else {
        format!("{} Restore power at the windmill", tick(false))
    };
    let truck = if ready {
        format!("{} The truck is warm — everyone aboard!", tick(false))
    } else if running {
        format!("{} Engine warming: {:.0}%", tick(false), w.warm * 100.0)
    } else {
        format!("{} Start the truck at the bridge", tick(false))
    };
    let done = [bones_home, power_on, running];
    for (line, mut t, mut c) in &mut lines {
        let text = match line.0 {
            0 => &bones,
            1 => &power,
            _ => &truck,
        };
        set_text(&mut t, text);
        set_color(&mut c, if done[line.0.min(2)] { GREEN } else { INK });
    }

    let guidance = if !s.started {
        "Waiting for the host to begin (Enter)."
    } else if !bones_home {
        "Bundles of bones lie in the landmarks — the ranch, the corral, the fields, the caño, the tower — and the ceiba is where they belong."
    } else if !power_on {
        "The bones are home. Now the windmill: hold E at the pump to bring the lights back. It is loud."
    } else if !running {
        "Hold E at the truck's ignition. The engine will roar, and he will come."
    } else if !ready {
        "Survive. Keep watch while the engine warms."
    } else {
        "Everyone standing: get into the truck zone!"
    };
    for mut t in &mut hint {
        set_text(&mut t, guidance);
    }
}

pub(crate) fn status_text(
    net: Res<Network>,
    state: Res<State<Flow>>,
    mut panel: Query<(&mut Visibility, &Children), With<StatusPanel>>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    let status: Option<(&str, Color)> = match net.snapshot() {
        Some(s) if *state.get() == Flow::Playing && !s.outcome().is_over() => match s.danger {
            1 => Some(("He has seen you — break his line of sight", AMBER)),
            2 => Some(("He is coming — get behind solid walls!", RED)),
            3 => Some(("Out of his sight… stay hidden", PALE_BLUE)),
            4 => Some(("He kneels to count his bones — slip away", PALE_BLUE)),
            _ if s.me.stun > 0.0 => Some(("Frozen with fright…", RED)),
            _ if s.world.cattle > 0.0 => Some(("The cattle are bellowing — the whole llano can hear", AMBER)),
            _ if s.world.truck >= 1.0 && s.world.warm < 1.0 => Some(("The engine roars — it carries for miles", AMBER)),
            _ if s.world.beacon > 0.0 => Some(("The beacon burns — he is drawn to the light", AMBER)),
            _ => None,
        },
        _ => None,
    };
    for (mut vis, children) in &mut panel {
        set_vis(&mut vis, status.is_some());
        if let Some((s, c)) = status {
            let kids: &[Entity] = children;
            for &child in kids {
                if let Ok((mut t, mut color)) = texts.get_mut(child) {
                    set_text(&mut t, s);
                    set_color(&mut color, c);
                }
            }
        }
    }
}

pub(crate) fn roster(
    net: Res<Network>,
    state: Res<State<Flow>>,
    mut lines: Query<(&RosterLine, &mut Text, &mut TextColor)>,
) {
    let playing = matches!(*state.get(), Flow::Playing | Flow::Paused);
    let me = net.id();
    for (line, mut t, mut c) in &mut lines {
        let Some(p) = net.snapshot().filter(|_| playing).and_then(|s| s.players.get(line.0)) else {
            set_text(&mut t, "");
            continue;
        };
        let you = if Some(p.id) == me { " (you)" } else { "" };
        let text = match p.status {
            1 => format!("P{}{you}  DOWN {:.0}s", line.0 + 1, p.bleed.max(0.0)),
            2 => format!("P{}{you}  lost", line.0 + 1),
            _ if p.carrying > 0 => format!("P{}{you}  carrying {}", line.0 + 1, p.carrying),
            _ => format!("P{}{you}", line.0 + 1),
        };
        set_text(&mut t, &text);
        set_color(&mut c, if p.status == 0 { player_color(line.0) } else { RED });
    }
}

pub(crate) fn vitals(
    net: Res<Network>,
    state: Res<State<Flow>>,
    mut bars: ParamSet<(
        Query<&mut Visibility, With<FearOuter>>,
        Query<&mut Node, With<FearFill>>,
        Query<&mut BackgroundColor, With<FearFill>>,
        Query<&mut Visibility, With<BreathOuter>>,
        Query<&mut Node, With<BreathFill>>,
    )>,
    mut text: Query<&mut Text, With<VitalsText>>,
) {
    let playing = *state.get() == Flow::Playing && net.snapshot().is_some();
    let (f, stamina, aji) = net
        .snapshot()
        .map_or((0.0, 1.0, 0), |s| (s.me.fear, s.me.stamina, s.me.aji));
    for mut v in &mut bars.p0() {
        set_vis(&mut v, playing && f > 0.02);
    }
    let w = percent((f.clamp(0.0, 1.0) * 100.0).round());
    for mut n in &mut bars.p1() {
        if n.width != w {
            n.width = w;
        }
    }
    let tone = Color::srgb(0.55 + 0.4 * f, 0.4 - 0.2 * f, 0.34 - 0.1 * f);
    for mut c in &mut bars.p2() {
        if c.0 != tone {
            c.0 = tone;
        }
    }
    for mut v in &mut bars.p3() {
        set_vis(&mut v, playing && stamina < 0.985);
    }
    let w = percent((stamina.clamp(0.0, 1.0) * 100.0).round());
    for mut n in &mut bars.p4() {
        if n.width != w {
            n.width = w;
        }
    }
    let mut parts = Vec::new();
    if aji > 0 {
        parts.push(format!("Ají x{aji}  [Q]"));
    }
    if net.carrying() > 0 {
        parts.push(format!("Bones x{}  [G] drop", net.carrying()));
    }
    let line = parts.join("   ·   ");
    for mut t in &mut text {
        set_text(&mut t, if playing { &line } else { "" });
    }
}

fn hold_label(kind: u8) -> &'static str {
    match kind {
        1 => "Laying the bones down…",
        2 => "Praying at the roots…",
        3 => "Cranking the pump…",
        4 => "Turning the key…",
        5 => "Lighting the beacon…",
        6 => "Helping them up…",
        _ => "",
    }
}

fn prompt_for(t: &Target, note_open: bool, carrying: usize) -> String {
    let close = !t.ready();
    let text = match t.kind {
        TargetKind::Relic(_) => "[E] Take the bones — heavy, and they rattle",
        TargetKind::Aji(_) => "[E] Take the peppers",
        TargetKind::Note(_) if note_open => "[E] Put the note down",
        TargetKind::Note(_) => "[E] Read the note",
        TargetKind::Altar if carrying > 0 => "[Hold E] Lay the bones down",
        TargetKind::Altar => "[Hold E] Pray at the roots — it steadies you, but the ceiba hears",
        TargetKind::Pump => "[Hold E] Crank the pump — loud!",
        TargetKind::Ignition => match t.blocked {
            Some(Blocked::NeedBones) => "The engine will not turn while the bones are unrested",
            Some(Blocked::NeedPower) => "Nothing turns over — the power is out",
            None => "[Hold E] Start the truck — the roar will carry",
        },
        TargetKind::Beacon => "[Hold E] Light the beacon — he will come to the light",
        TargetKind::Body(_) => "[Hold E] Help them up",
    };
    if close && !matches!(t.kind, TargetKind::Ignition if t.blocked.is_some()) {
        let name = text.split_once(']').map_or(text, |(_, rest)| rest.trim_start());
        match t.kind {
            TargetKind::Relic(_) => "Bones — move closer".into(),
            TargetKind::Aji(_) => "Peppers — move closer".into(),
            _ => format!("{name} — move closer"),
        }
    } else {
        text.to_string()
    }
}

pub(crate) fn prompt(
    net: Res<Network>,
    target: Res<CurrentTarget>,
    note: Res<NoteOpen>,
    state: Res<State<Flow>>,
    mut q: ParamSet<(
        Query<&mut Text, With<PromptText>>,
        Query<&mut Text, With<ProgressLabel>>,
        Query<&mut Visibility, With<ProgressOuter>>,
        Query<&mut Node, With<ProgressFill>>,
    )>,
) {
    let playing = *state.get() == Flow::Playing;
    let me = net.snapshot().map(|s| s.me);
    let (kind, hold) = me.map_or((0, 0.0), |m| (m.hold_kind, m.hold));
    let holding = playing && kind != 0 && net.status() == 0;
    let text = if !playing || net.status() != 0 || net.stunned() || holding {
        String::new()
    } else {
        match target.0 {
            Some(t) => prompt_for(&t, note.0.is_some(), net.carrying()),
            None => String::new(),
        }
    };
    for mut t in &mut q.p0() {
        set_text(&mut t, &text);
    }
    for mut t in &mut q.p1() {
        set_text(&mut t, if holding { hold_label(kind) } else { "" });
    }
    for mut v in &mut q.p2() {
        set_vis(&mut v, holding);
    }
    let w = percent((hold.clamp(0.0, 1.0) * 100.0).round());
    for mut n in &mut q.p3() {
        if n.width != w {
            n.width = w;
        }
    }
}

pub(crate) fn vignette(
    time: Res<Time<Real>>,
    truth: Res<Truth>,
    net: Res<Network>,
    mut q: Query<&mut BackgroundGradient, With<Vignette>>,
    mut shown: Local<(f32, f32)>,
) {
    let th = &truth.encounter.threat;
    let fear = net.snapshot().map_or(0.0, |s| s.me.fear);
    let pulse = if th.state == ThreatState::Warning {
        0.18 + 0.08 * (time.elapsed_secs() * 3.0).sin()
    } else {
        0.0
    };
    let hot = (th.exposure * 0.85).max(pulse).min(0.9);
    // Fear closes the edges slowly and throbs with the heartbeat.
    let cold = (fear * 0.75 * (1.0 + 0.12 * (time.elapsed_secs() * (2.0 + fear * 3.0)).sin())).clamp(0.0, 0.85);
    let k = (time.delta_secs() * 6.0).min(1.0);
    let mut a = shown.0 + (hot - shown.0) * k;
    let mut b = shown.1 + (cold - shown.1) * k;
    if a < 0.01 {
        a = 0.0;
    }
    if b < 0.01 {
        b = 0.0;
    }
    if (a - shown.0).abs() < 0.004 && (b - shown.1).abs() < 0.004 && (a != 0.0 || b != 0.0 || *shown == (0.0, 0.0)) {
        return;
    }
    *shown = (a, b);
    let mix = if a + b > 0.0 { a / (a + b) } else { 0.0 };
    let edge = Color::srgba(
        0.07 * mix,
        0.01 + 0.02 * (1.0 - mix),
        0.03 * (1.0 - mix),
        (a + b * 0.9).min(0.92),
    );
    for mut g in &mut q {
        *g = BackgroundGradient::from(RadialGradient::new(
            UiPosition::CENTER,
            RadialGradientShape::FarthestCorner,
            vec![ColorStop::percent(Color::NONE, 42), ColorStop::percent(edge, 100)],
        ));
    }
}

pub(crate) fn downed_panel(
    net: Res<Network>,
    state: Res<State<Flow>>,
    time: Res<Time<Real>>,
    mut panel: Query<&mut Visibility, With<DownedPanel>>,
    mut text: Query<&mut Text, With<DownedText>>,
    mut tint: Query<&mut BackgroundColor, With<Tint>>,
    mut wash: Local<(f32, [f32; 3])>,
) {
    let playing = *state.get() == Flow::Playing;
    let me = net.me();
    let status = if playing { me.map_or(0, |p| p.status) } else { 0 };
    let shared = net.is_shared();
    let (label, want, rgb) = match (status, me) {
        (1, Some(p)) => (
            if shared {
                format!("YOU ARE DOWN\nA friend can help you up — {:.0}s left", p.bleed.max(0.0))
            } else {
                format!("YOU ARE DOWN\n{:.0}s left", p.bleed.max(0.0))
            },
            0.38,
            [0.28, 0.02, 0.02],
        ),
        (2, _) => (
            if shared {
                "YOU DIED\nWatch over your friends".to_string()
            } else {
                "YOU DIED".to_string()
            },
            0.5,
            [0.02, 0.02, 0.03],
        ),
        _ => {
            let stun = net.snapshot().map_or(0.0, |s| s.me.stun);
            (String::new(), (stun.min(1.0) * 0.4).max(0.0), [0.82, 0.88, 1.0])
        }
    };
    for mut v in &mut panel {
        set_vis(&mut v, !label.is_empty());
    }
    for mut t in &mut text {
        set_text(&mut t, &label);
    }
    let k = (time.delta_secs() * 8.0).min(1.0);
    wash.0 += (want - wash.0) * k;
    if want > 0.0 {
        wash.1 = rgb;
    }
    if wash.0 < 0.004 && want == 0.0 {
        wash.0 = 0.0;
    }
    let color = Color::srgba(wash.1[0], wash.1[1], wash.1[2], wash.0);
    for mut bg in &mut tint {
        if bg.0 != color {
            bg.0 = color;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hints_and_captions(
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    net: Res<Network>,
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
            Event::RelicTaken => show(
                &mut hint,
                "The bundle is heavy, and it rattles. Something out on the llano knows.",
                6.0,
                1,
            ),
            Event::RelicDropped => show(&mut hint, "You set the bones down. Gently.", 4.0, 1),
            Event::RelicDelivered => show(&mut hint, "One more bundle at rest. The roots are listening.", 5.0, 2),
            Event::AllBonesHome => show(
                &mut hint,
                "The bones are home. Bring the power back, then start the truck.",
                8.0,
                3,
            ),
            Event::PowerRestored => show(
                &mut hint,
                "The lamps hum back to life. Light steadies the nerves — stay near it.",
                7.0,
                3,
            ),
            Event::TruckStarted => show(
                &mut hint,
                "The engine roars — he heard it. Hold out until it warms up.",
                8.0,
                3,
            ),
            Event::ThreatManifested => {
                if !hint.taught_crouch {
                    hint.taught_crouch = true;
                    show(
                        &mut hint,
                        "Something moved out on the llano. Crouch (Ctrl) to move quietly; running is loud.",
                        8.0,
                        2,
                    );
                }
            }
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
            Event::Downed => {
                if net.status() == 1 {
                    show(&mut hint, "You are down. Hold on — someone may reach you.", 6.0, 3);
                } else {
                    show(
                        &mut hint,
                        "A friend is down! Hold E beside them to help them up.",
                        7.0,
                        3,
                    );
                }
            }
            Event::Revived => show(&mut hint, "Back on your feet. Keep moving.", 4.0, 2),
            Event::Died => show(&mut hint, "Someone did not make it.", 6.0, 3),
            Event::AjiTaken => show(
                &mut hint,
                "Hot peppers. Press Q to scatter them — he stops to count his bones.",
                7.0,
                2,
            ),
            Event::CountingBegan => show(&mut hint, "He kneels to count his bones. Slip away, quietly.", 5.0, 2),
            Event::CountingEnded => show(&mut hint, "He has finished counting.", 3.0, 1),
            Event::Susto => show(
                &mut hint,
                "Susto — fright freezes you. Stay in the light and close to your friends.",
                6.0,
                2,
            ),
            Event::CattleSpooked => show(&mut hint, "The cattle bellow. Everything heard that.", 5.0, 2),
            Event::BeaconLit => show(&mut hint, "The beacon flares. He turns toward the light.", 5.0, 2),
            Event::Prayed => show(&mut hint, "The fear eases.", 3.0, 1),
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

pub(crate) fn note_panel(
    state: Res<State<Flow>>,
    note: Res<NoteOpen>,
    mut panel: Query<&mut Visibility, With<NotePanel>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<NoteEs>>,
        Query<&mut Text, With<NoteEn>>,
        Query<&mut Text, With<NoteBy>>,
    )>,
) {
    let open = if *state.get() == Flow::Playing { note.0 } else { None };
    for mut v in &mut panel {
        set_vis(&mut v, open.is_some());
    }
    let Some(id) = open else {
        return;
    };
    let page = crate::lore::note(id);
    for mut t in &mut texts.p0() {
        set_text(&mut t, page.es);
    }
    for mut t in &mut texts.p1() {
        set_text(&mut t, page.en);
    }
    for mut t in &mut texts.p2() {
        set_text(&mut t, page.by);
    }
}
