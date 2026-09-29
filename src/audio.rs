//! Sound: the night ambience, rain and thunder, the non-spatial whistle
//! phrases chosen by the perception layer, footsteps, machines and party
//! cues. Every voice is mono and plays without panning or distance
//! attenuation; gains are gentle and follow the master volume. Nothing here
//! reads the Silbón's position — only this player's own body, the shared
//! world state and this frame's events.

use std::collections::BTreeMap;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::app::{
    EncounterMsg, Flow, GameSet, LayoutRes, RunReset, Settings, StormClock, Truth, TuningRes, WhistleMsg,
};
use crate::net::Network;
use crate::perception::WhistleVariant;
use crate::player::Player;
use crate::sim::{Event, ThreatState};
use crate::storm;

/// Ground the footfalls land on, in the order of `Sounds::steps`.
#[derive(Clone, Copy)]
enum Surface {
    Dirt,
    Grass,
    Wood,
    Water,
}

#[derive(Resource)]
struct Sounds {
    whistles: [Handle<AudioSource>; 3],
    thunder: [Handle<AudioSource>; 2],
    steps: [[Handle<AudioSource>; 3]; 4],
    ambience: Handle<AudioSource>,
    rain: Handle<AudioSource>,
    heartbeat: Handle<AudioSource>,
    engine: Handle<AudioSource>,
    crank: Handle<AudioSource>,
    bones: Handle<AudioSource>,
    bones_set: Handle<AudioSource>,
    restitution: Handle<AudioSource>,
    caught: Handle<AudioSource>,
    dawn: Handle<AudioSource>,
    cattle: Handle<AudioSource>,
    engine_start: Handle<AudioSource>,
    power_on: Handle<AudioSource>,
    susto: Handle<AudioSource>,
    revive: Handle<AudioSource>,
    pray: Handle<AudioSource>,
    aji: Handle<AudioSource>,
    beacon: Handle<AudioSource>,
    counting: Handle<AudioSource>,
    ping: Handle<AudioSource>,
    check_warn: Handle<AudioSource>,
    check_great: Handle<AudioSource>,
    check_miss: Handle<AudioSource>,
    radio: Handle<AudioSource>,
    sting_caught: Handle<AudioSource>,
    sting_reveal: Handle<AudioSource>,
    sting_phantom: Handle<AudioSource>,
    sting_hunt: Handle<AudioSource>,
    omen_bones: Handle<AudioSource>,
    omen_lamps: Handle<AudioSource>,
    omen_swell: Handle<AudioSource>,
    dread: Handle<AudioSource>,
    lock_rattle: Handle<AudioSource>,
    lock_open: Handle<AudioSource>,
    tell_weeping: Handle<AudioSource>,
    tell_whip: Handle<AudioSource>,
    tell_bottles: Handle<AudioSource>,
    banished: Handle<AudioSource>,
    dog_growl: Handle<AudioSource>,
    dog_bark: Handle<AudioSource>,
    theme: Handle<AudioSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VoiceKind {
    /// The always-running loops, mixed by `mix`.
    Ambience,
    Rain,
    Heartbeat,
    Engine,
    Crank,
    Radio,
    Dread,
    /// The title screen's cuatro.
    Theme,
    Whistle,
    Effect,
}

impl VoiceKind {
    fn is_loop(self) -> bool {
        !matches!(self, Self::Whistle | Self::Effect)
    }
}

/// A playing sound and its base gain before master volume.
#[derive(Component)]
struct Voice {
    gain: f32,
    kind: VoiceKind,
}

/// The single night-ambience loop (also counted by the debug census).
#[derive(Component)]
pub struct AmbienceLoop;

/// Smoothed ambience multiplier: the insects hush while he warns or hunts.
#[derive(Resource)]
struct Hush(f32);

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Hush(1.0))
            .add_systems(Startup, load_sounds)
            .add_systems(
                Update,
                (
                    stop_voices_on_restart.in_set(GameSet::Control),
                    (
                        play_whistles,
                        play_effects,
                        play_stings,
                        play_pings,
                        footsteps,
                        thunder,
                        mix,
                    )
                        .chain()
                        .in_set(GameSet::Present),
                ),
            )
            .add_systems(OnEnter(Flow::Paused), pause_all)
            .add_systems(OnExit(Flow::Paused), resume_all);
    }
}

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>, tuning: Res<TuningRes>, settings: Res<Settings>) {
    let a = |name: &str| assets.load::<AudioSource>(format!("audio/{name}.wav"));
    let steps = |kind: &str| {
        [
            a(&format!("step_{kind}_0")),
            a(&format!("step_{kind}_1")),
            a(&format!("step_{kind}_2")),
        ]
    };
    let sounds = Sounds {
        whistles: [a("whistle_loud"), a("whistle_mid"), a("whistle_faint")],
        thunder: [a("thunder_a"), a("thunder_b")],
        steps: [steps("dirt"), steps("grass"), steps("wood"), steps("water")],
        ambience: a("ambience_llano"),
        rain: a("rain_loop"),
        heartbeat: a("heartbeat"),
        engine: a("engine_loop"),
        crank: a("pump_crank"),
        bones: a("bones_rattle"),
        bones_set: a("bones_set"),
        restitution: a("restitution"),
        caught: a("caught"),
        dawn: a("dawn"),
        cattle: a("cattle"),
        engine_start: a("engine_start"),
        power_on: a("power_on"),
        susto: a("susto"),
        revive: a("revive"),
        pray: a("pray"),
        aji: a("aji_scatter"),
        check_warn: a("check_warn"),
        check_great: a("check_great"),
        check_miss: a("check_miss"),
        radio: a("radio_broadcast"),
        sting_caught: a("sting_caught"),
        sting_reveal: a("sting_reveal"),
        sting_phantom: a("sting_phantom"),
        sting_hunt: a("sting_hunt"),
        omen_bones: a("omen_bones"),
        omen_lamps: a("omen_lamps"),
        omen_swell: a("omen_swell"),
        dread: a("dread_drone"),
        lock_rattle: a("lock_rattle"),
        lock_open: a("lock_open"),
        tell_weeping: a("tell_weeping"),
        tell_whip: a("tell_whip"),
        tell_bottles: a("tell_bottles"),
        banished: a("banished"),
        dog_growl: a("dog_growl"),
        dog_bark: a("dog_bark"),
        beacon: a("beacon_flare"),
        counting: a("counting"),
        ping: a("ping"),
        theme: a("title_theme"),
    };
    let t = &tuning.0;
    let spawn_loop = |commands: &mut Commands,
                      name: &'static str,
                      clip: &Handle<AudioSource>,
                      gain: f32,
                      kind: VoiceKind,
                      on: bool| {
        let mut e = commands.spawn((
            Name::new(name),
            AudioPlayer::new(clip.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(if on { gain * settings.volume } else { 0.0 })),
            Voice { gain, kind },
        ));
        if kind == VoiceKind::Ambience {
            e.insert(AmbienceLoop);
        }
    };
    spawn_loop(
        &mut commands,
        "ambience loop",
        &sounds.ambience,
        t.ambience_gain,
        VoiceKind::Ambience,
        true,
    );
    spawn_loop(
        &mut commands,
        "rain loop",
        &sounds.rain,
        t.rain_gain,
        VoiceKind::Rain,
        true,
    );
    spawn_loop(
        &mut commands,
        "heartbeat loop",
        &sounds.heartbeat,
        t.sfx_gain * 1.4,
        VoiceKind::Heartbeat,
        false,
    );
    spawn_loop(
        &mut commands,
        "engine loop",
        &sounds.engine,
        t.sfx_gain * 0.9,
        VoiceKind::Engine,
        false,
    );
    spawn_loop(
        &mut commands,
        "title theme",
        &sounds.theme,
        t.ambience_gain * 1.2,
        VoiceKind::Theme,
        false,
    );
    spawn_loop(
        &mut commands,
        "dread loop",
        &sounds.dread,
        t.ambience_gain * 1.1,
        VoiceKind::Dread,
        false,
    );
    spawn_loop(
        &mut commands,
        "radio loop",
        &sounds.radio,
        t.sfx_gain * 0.8,
        VoiceKind::Radio,
        false,
    );
    spawn_loop(
        &mut commands,
        "pump crank loop",
        &sounds.crank,
        t.sfx_gain * 0.85,
        VoiceKind::Crank,
        false,
    );
    commands.insert_resource(sounds);
}

fn one_shot(commands: &mut Commands, clip: &Handle<AudioSource>, gain: f32, speed: f32, kind: VoiceKind, master: f32) {
    commands.spawn((
        AudioPlayer::new(clip.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(gain * master))
            .with_speed(speed),
        Voice { gain, kind },
    ));
}

fn play_whistles(
    mut commands: Commands,
    mut phrases: MessageReader<WhistleMsg>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    state: Res<State<Flow>>,
    net: Res<Network>,
) {
    // The dead hear no more of him; the paused hear nothing at all. The
    // title screen's night has its own faint whistles.
    let heard = matches!(state.get(), Flow::Playing | Flow::Title);
    if !heard || net.status() == 2 {
        phrases.clear();
        return;
    }
    for WhistleMsg(p) in phrases.read() {
        let clip = match p.variant {
            WhistleVariant::Loud => &sounds.whistles[0],
            WhistleVariant::Middling => &sounds.whistles[1],
            WhistleVariant::Faint => &sounds.whistles[2],
        };
        one_shot(
            &mut commands,
            clip,
            p.gain,
            p.speed,
            VoiceKind::Whistle,
            settings.volume,
        );
    }
}

/// What the frights ask to be heard: stingers and omens.
fn play_stings(
    mut commands: Commands,
    mut stings: MessageReader<crate::world::omen::Sting>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
) {
    use crate::world::omen::Sting;
    let g = tuning.0.sfx_gain;
    for sting in stings.read() {
        let (clip, gain, speed) = match sting {
            Sting::Caught => (&sounds.sting_caught, g * 1.5, 1.0),
            Sting::Reveal => (&sounds.sting_reveal, g * 1.1, 1.0),
            Sting::Phantom => (&sounds.sting_phantom, g * 0.8, 1.0),
            Sting::Bones => (&sounds.omen_bones, g * 0.7, 1.0),
            Sting::Lamps => (&sounds.omen_lamps, g * 0.6, 1.0),
            Sting::Swell => (&sounds.omen_swell, g * 0.8, 1.0),
            Sting::Clack => (&sounds.omen_bones, g * 0.45, 1.25),
        };
        one_shot(&mut commands, clip, gain, speed, VoiceKind::Effect, settings.volume);
    }
}

fn play_effects(
    mut commands: Commands,
    mut events: MessageReader<EncounterMsg>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
) {
    let g = tuning.0.sfx_gain;
    for EncounterMsg(e) in events.read() {
        let (clip, gain, speed) = match e {
            Event::RelicTaken => (&sounds.bones, g, 1.0),
            Event::RelicDropped => (&sounds.bones_set, g * 0.8, 1.0),
            Event::RelicDelivered => (&sounds.bones_set, g * 0.9, 0.85),
            Event::AllBonesHome => (&sounds.restitution, g, 1.0),
            Event::PowerRestored => (&sounds.power_on, g, 1.0),
            Event::TruckStarted => (&sounds.engine_start, g, 1.0),
            Event::AjiTaken => (&sounds.aji, g * 0.45, 1.35),
            Event::AjiUsed => (&sounds.aji, g, 1.0),
            Event::BatteriesTaken => (&sounds.aji, g * 0.35, 1.8),
            Event::SkillCheck => (&sounds.check_warn, g * 0.8, 1.0),
            Event::SkillGreat => (&sounds.check_great, g * 0.7, 1.0),
            Event::SkillMissed => (&sounds.check_miss, g, 1.0),
            Event::HuntBegan => (&sounds.sting_hunt, g * 0.9, 1.0),
            Event::LockRattle => (&sounds.lock_rattle, g * 0.9, 1.0),
            Event::KeyFound => (&sounds.lock_open, g, 1.0),
            Event::Weeping => (&sounds.tell_weeping, g * 0.8, 1.0),
            Event::Hauled => (&sounds.bones, g * 1.2, 0.7),
            Event::DogFreed => (&sounds.dog_bark, g * 0.35, 1.2),
            Event::DogGrowl => (&sounds.dog_growl, g * 0.9, 1.0),
            Event::DogBark => (&sounds.dog_bark, g * 1.1, 1.0),
            Event::SackDropped => (&sounds.bones_set, g, 0.8),
            Event::Taken => (&sounds.caught, g, 0.7),
            Event::WhipCrack => (&sounds.tell_whip, g * 0.8, 1.0),
            Event::BottleClink => (&sounds.tell_bottles, g * 0.8, 1.0),
            Event::Banished => (&sounds.banished, g, 1.0),
            Event::NameWrong => (&sounds.sting_reveal, g * 1.1, 0.8),
            Event::CountingBegan => (&sounds.counting, g * 0.8, 1.0),
            Event::Susto => (&sounds.susto, g, 1.0),
            Event::Revived => (&sounds.revive, g, 1.0),
            Event::Downed => (&sounds.caught, g * 0.8, 1.2),
            Event::Died => (&sounds.caught, g, 0.85),
            Event::CattleSpooked => (&sounds.cattle, g, 1.0),
            Event::BeaconLit => (&sounds.beacon, g, 1.0),
            Event::Prayed => (&sounds.pray, g * 0.8, 1.0),
            Event::Escaped => (&sounds.dawn, g * 0.9, 1.0),
            _ => continue,
        };
        one_shot(&mut commands, clip, gain, speed, VoiceKind::Effect, settings.volume);
    }
}

/// A tick when anyone marks a spot.
fn play_pings(
    mut commands: Commands,
    net: Res<Network>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    mut seen: Local<BTreeMap<u64, f32>>,
) {
    let pings = net.snapshot().map_or(&[][..], |s| s.pings.as_slice());
    seen.retain(|by, _| pings.iter().any(|p| p.by == *by));
    for p in pings {
        let fresh = seen.get(&p.by).is_none_or(|left| p.left > *left);
        seen.insert(p.by, p.left);
        if fresh {
            one_shot(
                &mut commands,
                &sounds.ping,
                tuning.0.sfx_gain * 0.7,
                1.0,
                VoiceKind::Effect,
                settings.volume,
            );
        }
    }
}

/// Footfalls follow the ground this player really covers, on the surface
/// underfoot, with the same stride the body model uses for noise.
#[allow(clippy::too_many_arguments)]
fn footsteps(
    mut commands: Commands,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    settings: Res<Settings>,
    sounds: Res<Sounds>,
    net: Res<Network>,
    state: Res<State<Flow>>,
    player: Single<&Player>,
    mut last: Local<Option<Vec2>>,
    mut carried: Local<f32>,
    mut count: Local<usize>,
) {
    let pos = player.pose.pos;
    let moved = last.map_or(0.0, |p| p.distance(pos));
    *last = Some(pos);
    if *state.get() != Flow::Playing || !net.active() || net.stunned() || moved > 2.0 {
        return;
    }
    *carried += moved;
    let stride = tuning.0.stride;
    if *carried < stride {
        return;
    }
    *carried %= stride;
    let (crouch, sprint) = net.me().map_or((false, false), |p| (p.crouch, p.sprint));
    let d = &layout.0.district;
    let surface = if layout.0.wading(pos) {
        Surface::Water
    } else if d.surface_at(pos).is_some() {
        Surface::Wood
    } else if d.grass.iter().any(|r| r.contains(pos)) {
        Surface::Grass
    } else {
        Surface::Dirt
    };
    let gait = if sprint {
        1.0
    } else if crouch {
        0.3
    } else {
        0.62
    };
    let load = 1.0 + 0.12 * net.carrying() as f32;
    *count += 1;
    const PITCH: [f32; 5] = [0.96, 1.04, 1.0, 0.92, 1.08];
    let clip = &sounds.steps[surface as usize][*count % 3];
    one_shot(
        &mut commands,
        clip,
        tuning.0.sfx_gain * gait * load * 0.8,
        PITCH[*count % PITCH.len()],
        VoiceKind::Effect,
        settings.volume,
    );
}

/// Thunder rolls in after each flash, late by the storm's own distance.
fn thunder(
    mut commands: Commands,
    clock: Res<StormClock>,
    tuning: Res<TuningRes>,
    settings: Res<Settings>,
    sounds: Res<Sounds>,
) {
    if clock.t <= clock.prev || clock.t - clock.prev > 1.0 {
        return;
    }
    if let Some(power) = storm::thunder_onset(tuning.0.seed, clock.prev, clock.t) {
        let variant = ((clock.t * 7.0) as usize) % 2;
        one_shot(
            &mut commands,
            &sounds.thunder[variant],
            tuning.0.sfx_gain * (0.55 + 0.75 * power.clamp(0.0, 1.0)),
            1.0,
            VoiceKind::Effect,
            settings.volume,
        );
    }
}

/// Apply master volume and every loop's dynamic level to each live sink.
#[allow(clippy::too_many_arguments)]
fn mix(
    time: Res<Time<Real>>,
    truth: Res<Truth>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    clock: Res<StormClock>,
    mut hush: ResMut<Hush>,
    mut sinks: Query<(&Voice, &mut AudioSink)>,
    state: Res<State<Flow>>,
    net: Res<Network>,
    note: Res<crate::encounter::NoteOpen>,
    fright: Res<crate::world::omen::Fright>,
) {
    let tense = matches!(
        truth.encounter.threat.state,
        ThreatState::Warning | ThreatState::Hunting
    );
    let target = if tense { tuning.0.ambience_hush } else { 1.0 };
    let dt = time.delta_secs();
    hush.0 += (target - hush.0) * (dt * 0.8).min(1.0);

    let snap = net.snapshot();
    let me = snap.map(|s| s.me).unwrap_or_default();
    let world = snap.map(|s| s.world).unwrap_or_default();
    let dazed = net.status() == 1 || me.stun > 0.0;
    let rain = storm::rain(tuning.0.seed, clock.t);
    let over = truth.encounter.outcome.is_over();

    // Fear you can hear: quiet under a threshold, then a drum that speeds up.
    let heart_level = if net.status() == 1 {
        1.0
    } else {
        ((me.fear - 0.3) / 0.55).clamp(0.0, 1.0)
    };
    let heart_speed = if net.status() == 1 {
        0.7
    } else {
        0.85 + 0.85 * me.fear.clamp(0.0, 1.0)
    };
    let engine_on = world.truck >= 1.0 && !over;
    let crank_on = me.hold_kind == 3;
    // Dread swells with the night, with every bundle taken from him, and
    // when he is on you.
    let dread = snap.map_or(0.0, |s| {
        let rite = if s.world.total > 0 {
            s.world.delivered as f32 / s.world.total as f32
        } else {
            0.0
        };
        let chased = if matches!(s.danger, 1..=3) { 0.35 } else { 0.0 };
        let awake = if s.stats[0] > 0 || s.relics.iter().any(|r| r.state != 0) {
            1.0
        } else {
            0.3
        };
        ((0.25 * s.world.night + 0.45 * rite + chased) * awake).clamp(0.0, 1.0)
    });
    let hush_omen = fright.hush();
    let radio_on = note
        .0
        .is_some_and(|id| crate::lore::note(id).medium == crate::lore::Medium::Radio)
        && *state.get() == Flow::Playing;
    // The night draws back while he whistles.
    let whistling = sinks
        .iter()
        .any(|(voice, sink)| voice.kind == VoiceKind::Whistle && !sink.is_paused() && !sink.empty());
    let duck = if whistling { tuning.0.whistle_duck } else { 1.0 };

    for (voice, mut sink) in &mut sinks {
        if *state.get() == Flow::Paused {
            sink.pause();
        }
        let mut v = voice.gain * settings.volume;
        match voice.kind {
            VoiceKind::Ambience => {
                v *= hush_omen * duck * hush.0 * if dazed { 0.5 } else { 1.0 } * (1.0 - 0.25 * (rain - 0.6) / 0.4);
            }
            VoiceKind::Rain => v *= hush_omen * duck * rain * if dazed { 0.65 } else { 1.0 },
            VoiceKind::Dread => v *= if over { 0.0 } else { dread },
            VoiceKind::Theme => v *= if *state.get() == Flow::Title { 1.0 } else { 0.0 },
            VoiceKind::Heartbeat => {
                v *= heart_level * heart_level;
                sink.set_speed(heart_speed);
            }
            VoiceKind::Engine => {
                v *= if engine_on { 0.6 + 0.4 * world.warm } else { 0.0 };
                sink.set_speed(0.92 + 0.16 * world.warm);
            }
            VoiceKind::Crank => v *= if crank_on { 1.0 } else { 0.0 },
            VoiceKind::Radio => v *= if radio_on { 1.0 } else { 0.0 },
            VoiceKind::Whistle if net.status() == 2 => v = 0.0,
            VoiceKind::Whistle | VoiceKind::Effect => {}
        }
        let now = sink.volume().to_linear();
        // Loops glide to their level; one-shots keep theirs.
        let v = if voice.kind.is_loop() {
            now + (v - now) * (dt * 5.0).min(1.0)
        } else {
            v
        };
        if (now - v).abs() > 0.002 || (v == 0.0 && now != 0.0) {
            sink.set_volume(Volume::Linear(v));
        }
    }
}

fn pause_all(sinks: Query<&AudioSink>) {
    for sink in &sinks {
        sink.pause();
    }
}

fn resume_all(sinks: Query<&AudioSink>) {
    for sink in &sinks {
        sink.play();
    }
}

/// A new run: stop every whistle and effect; the loops stay (one each).
fn stop_voices_on_restart(
    mut commands: Commands,
    mut requests: MessageReader<RunReset>,
    voices: Query<(Entity, &Voice)>,
    mut hush: ResMut<Hush>,
) {
    if requests.read().count() == 0 {
        return;
    }
    for (entity, voice) in &voices {
        if !voice.kind.is_loop() {
            commands.entity(entity).despawn();
        }
    }
    hush.0 = 1.0;
}
