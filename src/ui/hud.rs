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
        let names = ["hacienda", "corral", "bridge"];
        let lines: Vec<&str> = (0..3)
            .filter(|i| w.circuits & (1 << i) != 0)
            .map(|i| names[i])
            .collect();
        format!(
            "{} Power restored — lamps on the {} (switch lines at the windmill panel)",
            tick(true),
            lines.join(" and ")
        )
    } else if w.power > 0.0 {
        format!("{} Restore power at the windmill: {:.0}%", tick(false), w.power * 100.0)
    } else {
        format!("{} Restore power at the windmill", tick(false))
    };
    let truck = if ready {
        format!("{} The truck is warm — everyone aboard!", tick(false))
    } else if running {
        format!("{} Engine warming: {:.0}%", tick(false), w.warm * 100.0)
    } else if bones_home && power_on && !w.key {
        format!(
            "{} Open the key box at the windmill (three numbers, written in the pages)",
            tick(false)
        )
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
        "The bones are home. Now the windmill: hold E at the pump to bring the lights back. It is loud. \
         Or, if you know which of him walks tonight, name him at the ceiba (N)."
    } else if !w.key {
        "The truck key is padlocked in a box on the crates by the windmill. The Madrina, the foreman and the tower guard each wrote down one number."
    } else if !running {
        "Hold E at the truck's ignition. The engine will roar, and he will come."
    } else if !ready {
        "Survive. Keep watch while the engine warms."
    } else if net.is_shared() {
        "Everyone standing: get into the truck zone! Or, aboard, press X to drive off without the others."
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
    // Only a shared night has a party to list (hosting can begin at runtime).
    let playing = matches!(*state.get(), Flow::Playing | Flow::Paused) && net.is_shared();
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
    let (f, stamina, aji, battery) = net.snapshot().map_or((0.0, 1.0, 0, 1.0), |s| {
        (s.me.fear, s.me.stamina, s.me.aji, s.me.battery)
    });
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
    if battery <= 0.0 {
        parts.push("Torch: dead".to_string());
    } else if battery < 0.5 {
        parts.push(format!("Torch {:.0}%", battery * 100.0));
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
        7 => "Untying Tureco…",
        _ => "",
    }
}

fn prompt_for(t: &Target, note_open: bool, carrying: usize, bones_home: bool) -> String {
    let close = !t.ready();
    let text = match t.kind {
        TargetKind::Relic(_) => "[E] Take the bones — heavy, and they rattle",
        TargetKind::Aji(_) => "[E] Take the peppers",
        TargetKind::Batteries(_) => "[E] Take the spare batteries",
        TargetKind::Note(_) if note_open => "[E] Put the note down",
        TargetKind::Note(_) => "[E] Read the note",
        TargetKind::Altar if carrying > 0 => "[Hold E] Lay the bones down",
        TargetKind::Altar if bones_home => "[Hold E] Pray   ·   [N] Name which of him walks tonight",
        TargetKind::Altar => "[Hold E] Pray at the roots — it steadies you, but the ceiba hears",
        TargetKind::Pump => "[Hold E] Crank the pump — loud!",
        TargetKind::Ignition => match t.blocked {
            Some(Blocked::NeedBones) => "The engine will not turn while the bones are unrested",
            Some(Blocked::NeedPower) => "Nothing turns over — the power is out",
            Some(Blocked::NeedKey) => "No key in the ignition — it is padlocked in the box at the windmill",
            None => "[Hold E] Start the truck — the roar will carry",
        },
        TargetKind::Beacon => "[Hold E] Light the beacon — he will come to the light",
        TargetKind::Lockbox => "[E] The key box — a three-number padlock",
        TargetKind::Dog => "[Hold E] Untie Tureco — he fears nothing, and HE fears dogs",
        TargetKind::Panel => "[E] Switch the lamp lines — the old dynamo carries only two",
        TargetKind::Body(_) => "[Hold E] Help them up",
    };
    if close && !matches!(t.kind, TargetKind::Ignition if t.blocked.is_some()) {
        let name = text.split_once(']').map_or(text, |(_, rest)| rest.trim_start());
        match t.kind {
            TargetKind::Relic(_) => "Bones — move closer".into(),
            TargetKind::Aji(_) => "Peppers — move closer".into(),
            TargetKind::Batteries(_) => "Batteries — move closer".into(),
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
            Some(t) => prompt_for(
                &t,
                note.0.is_some(),
                net.carrying(),
                net.snapshot()
                    .is_some_and(|s| s.world.total > 0 && s.world.delivered >= s.world.total),
            ),
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

/// The skill check in flight: the zone where the host put it, the needle
/// where it has swept to by now (the snapshot's, carried on by its age).
pub(crate) fn skill_bar(
    time: Res<Time<Real>>,
    net: Res<Network>,
    tuning: Res<TuningRes>,
    state: Res<State<Flow>>,
    mut vis: ParamSet<(
        Query<&mut Visibility, With<SkillBar>>,
        Query<&mut Visibility, With<SkillKey>>,
        Query<&mut Visibility, With<SkillNeedle>>,
    )>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<SkillZone>>,
        Query<&mut Node, With<SkillGreat>>,
        Query<&mut Node, With<SkillNeedle>>,
    )>,
    mut border: Query<&mut BorderColor, With<SkillBar>>,
) {
    let t = &tuning.0;
    let check = (*state.get() == Flow::Playing)
        .then(|| net.snapshot().and_then(|s| s.me.check))
        .flatten();
    let needle = net.needle(t).map_or(0.0, |(_, n)| n);
    let shown = check.is_some();
    for mut v in &mut vis.p0() {
        set_vis(&mut v, shown);
    }
    for mut v in &mut vis.p1() {
        set_vis(&mut v, shown);
    }
    for mut v in &mut vis.p2() {
        set_vis(&mut v, shown && needle >= 0.0);
    }
    let Some(c) = check else {
        return;
    };
    let span = |n: &mut Node, from: f32, width: f32| {
        let (l, w) = (percent(from * 100.0), percent(width * 100.0));
        if n.left != l || n.width != w {
            n.left = l;
            n.width = w;
        }
    };
    for mut n in &mut nodes.p0() {
        span(&mut n, c.zone, t.check_zone.min(1.0 - c.zone));
    }
    for mut n in &mut nodes.p1() {
        span(&mut n, c.zone, t.check_great);
    }
    for mut n in &mut nodes.p2() {
        span(&mut n, needle.clamp(0.0, 0.99), 0.008);
    }
    // The warning: the frame pulses before the needle moves.
    let glow = if needle < 0.0 {
        0.5 + 0.5 * (time.elapsed_secs() * 18.0).sin()
    } else {
        0.6
    };
    for mut b in &mut border {
        let want = BorderColor::all(Color::srgba(1.0, 0.85, 0.6, glow));
        if *b != want {
            *b = want;
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
        (1, Some(p)) if p.hauled => (
            "IN HIS SACK\nThe bones press on you in the dark. Only ají in his path will make him drop you.".to_string(),
            0.72,
            [0.08, 0.04, 0.02],
        ),
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
    state: Res<State<Flow>>,
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
            Event::SkillCheck => {
                if !hint.taught_skill {
                    hint.taught_skill = true;
                    show(
                        &mut hint,
                        "Keep the rhythm: press Space as the needle crosses the marked zone. Miss, and it screeches.",
                        7.0,
                        3,
                    );
                }
            }
            Event::OmenSilence => show(&mut hint, "Even the frogs have stopped.", 5.0, 1),
            Event::Weeping => show(&mut hint, "Across the llano, a grown man is weeping.", 6.0, 2),
            Event::DogFreed => show(&mut hint, "Tureco shakes himself and falls in at your heels.", 6.0, 2),
            Event::DogGrowl => show(
                &mut hint,
                "Tureco growls low at the dark. He is near — whatever the whistle says.",
                6.0,
                2,
            ),
            Event::DogBark => show(
                &mut hint,
                "Tureco barks — and out in the dark, something flinches away.",
                6.0,
                3,
            ),
            Event::Hauled => {
                if net.status() == 0 {
                    show(
                        &mut hint,
                        "He stuffed them into his sack and walks off! Get ají in his path before he is gone.",
                        8.0,
                        3,
                    );
                }
            }
            Event::SackDropped => show(
                &mut hint,
                "He drops the sack to count his bones — get them up, now!",
                6.0,
                3,
            ),
            Event::Taken => show(&mut hint, "He is gone into the grass. And so are they.", 7.0, 3),
            Event::WhipCrack => show(&mut hint, "A whip cracks somewhere out in the dark.", 5.0, 1),
            Event::BottleClink => show(&mut hint, "Glass knocks against glass, out in the grass.", 5.0, 1),
            Event::NameWrong => show(
                &mut hint,
                "Wrong name. The ceiba shudders — and he comes, furious.",
                6.0,
                3,
            ),
            Event::KeyFound => show(&mut hint, "The padlock gives. The truck key is ours.", 6.0, 3),
            Event::LinesSwitched => show(
                &mut hint,
                "The dynamo groans as the lines change. Somewhere, lamps die; somewhere else, they wake.",
                5.0,
                2,
            ),
            Event::LockRattle => show(
                &mut hint,
                "Wrong numbers. The padlock rattles, loud in the quiet.",
                4.0,
                2,
            ),
            Event::OmenDrag => show(
                &mut hint,
                "Something heavy was dragged through the mud here. Recently.",
                5.0,
                1,
            ),
            Event::SkillMissed => show(&mut hint, "It screeches across the llano. He heard that.", 5.0, 2),
            Event::BatteriesTaken => show(
                &mut hint,
                "Spare batteries. The beam steadies — and it is the brightest thing on the llano.",
                5.0,
                1,
            ),
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
                if net.status() == 1 && net.is_solo() {
                    show(&mut hint, "He has you.", 6.0, 3);
                } else if net.status() == 1 {
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
        if p.phantom {
            // Fear put it there: the caption cannot be sure either.
            caption.text = "A whistle…? Or only the blood in your ears.";
            caption.timer = 4.0;
            continue;
        }
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
    // Only over play: a menu (pause, outcome) has the screen to itself.
    let playing = *state.get() == Flow::Playing;
    let hint_text = if hint.timer > 0.0 && playing { hint.text } else { "" };
    let caption_text = if caption.timer > 0.0 && settings.captions && playing {
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

pub(crate) fn name_panel(
    state: Res<State<Flow>>,
    naming: Res<crate::encounter::NamePanel>,
    mut panel: Query<&mut Visibility, With<NamePanelUi>>,
    mut choices: Query<&mut Text, With<NameChoices>>,
) {
    let open = *state.get() == Flow::Playing && naming.open;
    for mut v in &mut panel {
        set_vis(&mut v, open);
    }
    if !open {
        return;
    }
    let names = [
        "1  El Borracho — the drunkard's return",
        "2  El Hijo — the son himself",
        "3  El Arriero — the drover",
    ];
    let text = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i as u8 == naming.choice {
                format!("› {n} ‹")
            } else {
                n.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    for mut t in &mut choices {
        set_text(&mut t, &text);
    }
}

pub(crate) fn lock_panel(
    state: Res<State<Flow>>,
    lock: Res<crate::encounter::LockPanel>,
    mut panel: Query<&mut Visibility, With<LockPanelUi>>,
    mut digits: Query<&mut Text, With<LockDigits>>,
) {
    let open = *state.get() == Flow::Playing && lock.open;
    for mut v in &mut panel {
        set_vis(&mut v, open);
    }
    if open {
        let [a, b, c] = lock.dials;
        let text = format!("{a}  {b}  {c}");
        for mut t in &mut digits {
            set_text(&mut t, &text);
        }
    }
}

pub(crate) fn note_panel(
    tuning: Res<TuningRes>,
    state: Res<State<Flow>>,
    note: Res<NoteOpen>,
    read: Res<crate::encounter::PagesRead>,
    mut panel: Query<&mut Visibility, With<NotePanel>>,
    mut paper: Query<&mut BackgroundColor, With<NotePaper>>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<NoteEs>>,
        Query<(&mut Text, &mut TextColor), With<NoteEn>>,
        Query<(&mut Text, &mut TextColor), With<NoteBy>>,
        Query<(&mut Text, &mut TextColor), With<NoteTitle>>,
        Query<(&mut Text, &mut TextColor), With<NoteCount>>,
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
    let code = crate::sim::lock_code(tuning.0.seed);
    let (es, en) = (crate::lore::fill(page.es, code), crate::lore::fill(page.en, code));
    let [r, g, b] = page.medium.paper();
    for mut bg in &mut paper {
        let want = Color::srgba(r, g, b, 0.97);
        if bg.0 != want {
            bg.0 = want;
        }
    }
    // Light ink on the dark radio card, dark ink on paper.
    let dark = page.medium == crate::lore::Medium::Radio;
    let ink = if dark {
        Color::srgb(0.9, 0.84, 0.7)
    } else {
        Color::srgb(0.16, 0.12, 0.1)
    };
    let soft = if dark {
        Color::srgb(0.75, 0.7, 0.6)
    } else {
        Color::srgb(0.25, 0.2, 0.16)
    };
    let title = format!("{}\n{}", page.medium.label(), page.title);
    let count = format!(
        "[E] put the page down   ·   pages found {}/{}",
        read.0.len(),
        crate::lore::PAGES
    );
    let lines: [(&str, Color); 5] = [(&es, ink), (&en, soft), (page.by, ink), (&title, soft), (&count, soft)];
    let apply = |i: usize, t: &mut Text, c: &mut TextColor| {
        set_text(t, lines[i].0);
        if c.0 != lines[i].1 {
            c.0 = lines[i].1;
        }
    };
    for (mut t, mut c) in &mut texts.p0() {
        apply(0, &mut t, &mut c);
    }
    for (mut t, mut c) in &mut texts.p1() {
        apply(1, &mut t, &mut c);
    }
    for (mut t, mut c) in &mut texts.p2() {
        apply(2, &mut t, &mut c);
    }
    for (mut t, mut c) in &mut texts.p3() {
        apply(3, &mut t, &mut c);
    }
    for (mut t, mut c) in &mut texts.p4() {
        apply(4, &mut t, &mut c);
    }
}
