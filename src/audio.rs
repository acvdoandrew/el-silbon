//! Sound: the night ambience loop, non-spatial whistle phrases chosen by the
//! perception layer, and a few one-shot effects. Every voice is mono and
//! plays without panning or distance attenuation; gains are gentle and follow
//! the master volume. Nothing here reads the Silbón's position.

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::app::{EncounterMsg, Flow, GameSet, RestartRequest, Settings, Truth, TuningRes, WhistleMsg};
use crate::perception::WhistleVariant;
use crate::sim::{Event, ThreatState};

#[derive(Resource)]
struct Sounds {
    whistle_loud: Handle<AudioSource>,
    whistle_mid: Handle<AudioSource>,
    whistle_faint: Handle<AudioSource>,
    ambience: Handle<AudioSource>,
    bones: Handle<AudioSource>,
    restitution: Handle<AudioSource>,
    caught: Handle<AudioSource>,
    dawn: Handle<AudioSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VoiceKind {
    Ambience,
    Whistle,
    Effect,
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
                    (play_whistles, play_effects, mix).chain().in_set(GameSet::Present),
                ),
            )
            .add_systems(OnEnter(Flow::Paused), pause_all)
            .add_systems(OnExit(Flow::Paused), resume_all);
    }
}

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>, tuning: Res<TuningRes>, settings: Res<Settings>) {
    let sounds = Sounds {
        whistle_loud: assets.load("audio/whistle_loud.wav"),
        whistle_mid: assets.load("audio/whistle_mid.wav"),
        whistle_faint: assets.load("audio/whistle_faint.wav"),
        ambience: assets.load("audio/ambience_llano.wav"),
        bones: assets.load("audio/bones_rattle.wav"),
        restitution: assets.load("audio/restitution.wav"),
        caught: assets.load("audio/caught.wav"),
        dawn: assets.load("audio/dawn.wav"),
    };
    let gain = tuning.0.ambience_gain;
    commands.spawn((
        Name::new("ambience loop"),
        AmbienceLoop,
        AudioPlayer::new(sounds.ambience.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(gain * settings.volume)),
        Voice {
            gain,
            kind: VoiceKind::Ambience,
        },
    ));
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
    net: Res<crate::net::Network>,
) {
    if *state.get() != Flow::Playing || (net.enabled && net.caught()) {
        phrases.clear();
        return;
    }
    for WhistleMsg(p) in phrases.read() {
        let clip = match p.variant {
            WhistleVariant::Loud => &sounds.whistle_loud,
            WhistleVariant::Middling => &sounds.whistle_mid,
            WhistleVariant::Faint => &sounds.whistle_faint,
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

fn play_effects(
    mut commands: Commands,
    mut events: MessageReader<EncounterMsg>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
) {
    let g = tuning.0.sfx_gain;
    for EncounterMsg(e) in events.read() {
        let clip = match e {
            Event::SatchelTaken => Some((&sounds.bones, g)),
            Event::RestitutionComplete => Some((&sounds.restitution, g)),
            Event::Caught => Some((&sounds.caught, g)),
            Event::Escaped => Some((&sounds.dawn, g * 0.9)),
            _ => None,
        };
        if let Some((clip, gain)) = clip {
            one_shot(&mut commands, clip, gain, 1.0, VoiceKind::Effect, settings.volume);
        }
    }
}

/// Apply master volume and the ambience hush to every live sink.
fn mix(
    time: Res<Time<Real>>,
    truth: Res<Truth>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    mut hush: ResMut<Hush>,
    mut sinks: Query<(&Voice, &mut AudioSink)>,
    state: Res<State<Flow>>,
    net: Res<crate::net::Network>,
) {
    let tense = matches!(
        truth.encounter.threat.state,
        ThreatState::Warning | ThreatState::Hunting
    );
    let target = if tense { tuning.0.ambience_hush } else { 1.0 };
    let k = (time.delta_secs() * 0.8).min(1.0);
    hush.0 += (target - hush.0) * k;
    for (voice, mut sink) in &mut sinks {
        if *state.get() == Flow::Paused {
            sink.pause();
        }
        let mut v = voice.gain * settings.volume;
        if net.enabled && net.caught() && voice.kind == VoiceKind::Whistle {
            v = 0.0;
        }
        if voice.kind == VoiceKind::Ambience {
            v *= hush.0;
        }
        if (sink.volume().to_linear() - v).abs() > 0.002 {
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

/// Restart: stop every whistle and effect; the ambience loop stays (one).
fn stop_voices_on_restart(
    mut commands: Commands,
    mut requests: MessageReader<RestartRequest>,
    voices: Query<(Entity, &Voice)>,
    mut hush: ResMut<Hush>,
) {
    if requests.read().count() == 0 {
        return;
    }
    for (entity, voice) in &voices {
        if voice.kind != VoiceKind::Ambience {
            commands.entity(entity).despawn();
        }
    }
    hush.0 = 1.0;
}
