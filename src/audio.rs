//! Sound: the night ambience, rain and thunder, the non-spatial whistle
//! phrases chosen by the perception layer, footsteps, machines and party
//! cues.
//!
//! Every voice follows the master and one bus (music, ambience or effects)
//! on the decibel curve in `mix`, with headroom under full scale. The
//! whistle and the catch follow the master alone, so no other slider can
//! hide him. The loops glide to their levels (`mix::glide`), landing on
//! them exactly, so a silence is silent at any frame rate.
//!
//! The llano's own things are *placed*: the machines, the frogs and the
//! windmill, Tureco, teammates' footsteps, marks, a fallen friend's groans
//! and cries for help, the altar and the key box, a bolt's thunder, an
//! omen's clatter behind you. They pan to where they
//! are and fade with distance and behind walls (`Tuning::heard`). The
//! whistle and everything whose source is him (stings, his signs, the hunt)
//! stay unplaced, and nothing here reads the Silbón's position — only this
//! player's own body, the shared world state and this frame's events.

use std::collections::BTreeMap;
use std::time::Duration;

use bevy::audio::{AudioSinkPlayback, SpatialScale, Volume};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::app::{
    EncounterMsg, Flow, GameSet, LayoutRes, RunReset, Settings, StormClock, Truth, TuningRes, VolumePreview, WhistleMsg,
};
use crate::mix::{Bus, GLIDE_RATE, glide};
use crate::net::Network;
use crate::net::protocol::CallKind;
use crate::perception::{WHISTLE_TAKES, WhistleVariant};
use crate::player::Player;
use crate::sim::{Event, ThreatState};
use crate::storm;
use crate::survivor::Survivor;

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
    /// `[variant][take]`: loud, middling, faint.
    whistles: [[Handle<AudioSource>; WHISTLE_TAKES as usize]; 3],
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
    ear_whistle: Handle<AudioSource>,
    ringing: Handle<AudioSource>,
    /// A fallen friend's groans, and their cry for help.
    groans: [Handle<AudioSource>; 3],
    call_help: Handle<AudioSource>,
}

/// Where a placed sound truly comes from.
#[derive(Component, Clone, Copy)]
struct Anchor {
    at: Vec3,
    /// Direction only (a bolt's thunder): its level is its own.
    far: bool,
}

impl Anchor {
    fn at(at: Vec3) -> Self {
        Self { at, far: false }
    }
}

/// Placed emitters stand this far out from the listener in their true
/// direction, at a spatial scale that keeps rodio's own distance gain at
/// one: rodio only pans them, and distance and walls are `Tuning::heard`.
const EMITTER_RADIUS: f32 = 2.0;
const EMITTER_SCALE: f32 = 0.4;
/// The listener's ears (metres apart).
const EAR_GAP: f32 = 0.22;

fn emitter_at(eye: Vec3, at: Vec3) -> Vec3 {
    eye + (at - eye).normalize_or(Vec3::NEG_Z) * EMITTER_RADIUS
}

/// How the listener hears a placed sound: where their head is, and what
/// stands between.
#[derive(SystemParam)]
struct Ears<'w, 's> {
    layout: Res<'w, LayoutRes>,
    tuning: Res<'w, TuningRes>,
    head: Query<'w, 's, &'static Transform, With<Player>>,
}

impl Ears<'_, '_> {
    fn eye(&self) -> Vec3 {
        self.head.iter().next().map_or(Vec3::ZERO, |t| t.translation)
    }

    fn heard(&self, anchor: &Anchor) -> f32 {
        if anchor.far {
            return 1.0;
        }
        heard_at(&self.layout.0, &self.tuning.0, self.eye(), anchor.at)
    }
}

fn heard_at(layout: &crate::geometry::Layout, tuning: &crate::tuning::Tuning, eye: Vec3, at: Vec3) -> f32 {
    let (e, a) = (Vec2::new(eye.x, eye.z), Vec2::new(at.x, at.z));
    tuning.heard(eye.distance(at), !layout.line_of_sight(e, a))
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
    /// Placed loops: frogs at the water, the windmill's creak, the dynamo's
    /// hum once the power is back.
    Frogs,
    Windmill,
    Hum,
    /// The catch's own sounds: heard through its silence and its black.
    Catch,
    Whistle,
    Effect,
    /// The truths about him besides the whistle: Tureco's growl and bark and
    /// the hunt's sting. Played like an effect, but no slider but the master
    /// may hide them.
    Cue,
    /// The omens' unplaced stingers: the night's music.
    Sting,
    /// A bolt's thunder.
    Weather,
    /// A moved volume slider, heard even in the pause menu.
    Preview(Bus),
}

impl VoiceKind {
    /// The loops run all along and glide to their level; every other voice
    /// plays once at its own.
    fn is_loop(self) -> bool {
        matches!(
            self,
            Self::Ambience
                | Self::Rain
                | Self::Heartbeat
                | Self::Engine
                | Self::Crank
                | Self::Radio
                | Self::Dread
                | Self::Theme
                | Self::Frogs
                | Self::Windmill
                | Self::Hum
        )
    }

    /// The slider a voice follows besides the master. His whistle, the dog's
    /// warnings, the hunt and the catch follow the master alone: no other
    /// slider may hide him.
    fn bus(self) -> Bus {
        match self {
            Self::Whistle | Self::Cue | Self::Catch => Bus::Master,
            Self::Theme | Self::Dread | Self::Sting => Bus::Music,
            Self::Ambience | Self::Rain | Self::Frogs | Self::Windmill | Self::Weather => Bus::Ambience,
            Self::Heartbeat | Self::Engine | Self::Crank | Self::Radio | Self::Hum | Self::Effect => Bus::Effects,
            Self::Preview(bus) => bus,
        }
    }
}

/// A voice's level before anything the night does to it: its gain on its
/// bus, never over full scale.
fn base_level(settings: &Settings, gain: f32, kind: VoiceKind) -> f32 {
    (gain * settings.gain(kind.bus())).min(1.0)
}

/// A playing sound and its base gain before its bus and the master.
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
                        attach_listener,
                        play_whistles,
                        play_effects,
                        play_stings,
                        play_pings,
                        play_calls,
                        footsteps,
                        party_steps,
                        party_cries,
                        thunder,
                        play_preview,
                        place_emitters,
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

fn load_sounds(
    mut commands: Commands,
    assets: Res<AssetServer>,
    tuning: Res<TuningRes>,
    layout: Res<LayoutRes>,
    settings: Res<Settings>,
) {
    let a = |name: &str| assets.load::<AudioSource>(format!("audio/{name}.wav"));
    let steps = |kind: &str| {
        [
            a(&format!("step_{kind}_0")),
            a(&format!("step_{kind}_1")),
            a(&format!("step_{kind}_2")),
        ]
    };
    let sounds = Sounds {
        whistles: ["loud", "mid", "faint"].map(|v| std::array::from_fn(|k| a(&format!("whistle_{v}_{k}")))),
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
        ear_whistle: a("whistle_ear"),
        ringing: a("ringing"),
        groans: std::array::from_fn(|k| a(&format!("downed_groan_{k}"))),
        call_help: a("call_help"),
    };
    let frogs = a("frogs_loop");
    let windmill = a("windmill_creak");
    let hum = a("dynamo_hum");
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
            PlaybackSettings::LOOP.with_volume(Volume::Linear(if on {
                base_level(&settings, gain, kind)
            } else {
                0.0
            })),
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
    // Placed loops start silent and glide in once `mix` has heard them.
    let l = &layout.0;
    let d = &l.district;
    let on_ground = |p: Vec2, up: f32| Vec3::new(p.x, l.surface_height(p) + up, p.y);
    let radio = d
        .notes
        .iter()
        .find(|n| crate::lore::note(n.id).medium == crate::lore::Medium::Radio)
        .map_or(d.pump, |n| n.pos);
    use crate::geometry::district::LandmarkId;
    let water = |id| on_ground(d.landmark(id).center, 0.3);
    // The creak comes from the top of the water tower's frame.
    let tower = d.landmark(LandmarkId::WaterTower).center;
    let vane = Vec3::new(tower.x, d.tower_ground + d.tower_height, tower.y);
    for (name, clip, gain, kind, at) in [
        (
            "engine loop",
            &sounds.engine,
            t.sfx_gain * 0.9,
            VoiceKind::Engine,
            on_ground(d.truck.center, 1.0),
        ),
        ("radio loop", &sounds.radio, t.sfx_gain * 0.8, VoiceKind::Radio, radio),
        (
            "pump crank loop",
            &sounds.crank,
            t.sfx_gain * 0.85,
            VoiceKind::Crank,
            d.pump,
        ),
        (
            "frogs at the caño",
            &frogs,
            t.ambience_gain * 0.9,
            VoiceKind::Frogs,
            water(LandmarkId::Cano),
        ),
        (
            "frogs at the marsh",
            &frogs,
            t.ambience_gain * 0.8,
            VoiceKind::Frogs,
            water(LandmarkId::Marsh),
        ),
        ("windmill creak", &windmill, t.sfx_gain * 0.7, VoiceKind::Windmill, vane),
        ("dynamo hum", &hum, t.sfx_gain * 0.45, VoiceKind::Hum, d.panel),
    ] {
        commands.spawn((
            Name::new(name),
            AudioPlayer::new(clip.clone()),
            PlaybackSettings::LOOP
                .with_volume(Volume::Linear(0.0))
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(EMITTER_SCALE)),
            Transform::from_translation(at),
            Voice { gain, kind },
            Anchor::at(at),
        ));
    }
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
    commands.insert_resource(sounds);
}

fn one_shot(
    commands: &mut Commands,
    clip: &Handle<AudioSource>,
    gain: f32,
    speed: f32,
    kind: VoiceKind,
    settings: &Settings,
) {
    commands.spawn((
        AudioPlayer::new(clip.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(base_level(settings, gain, kind)))
            .with_speed(speed),
        Voice { gain, kind },
    ));
}

/// A one-shot from a place on the llano (or unplaced, if `at` is none).
fn placed_shot(
    commands: &mut Commands,
    ears: &Ears,
    clip: &Handle<AudioSource>,
    (gain, speed): (f32, f32),
    anchor: Option<Anchor>,
    kind: VoiceKind,
    settings: &Settings,
) {
    let Some(anchor) = anchor else {
        one_shot(commands, clip, gain, speed, kind, settings);
        return;
    };
    let heard = ears.heard(&anchor);
    if heard <= 0.0 {
        return;
    }
    commands.spawn((
        AudioPlayer::new(clip.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear((gain * settings.gain(kind.bus()) * heard).min(1.0)))
            .with_speed(speed)
            .with_spatial(true)
            .with_spatial_scale(SpatialScale::new(EMITTER_SCALE)),
        Transform::from_translation(emitter_at(ears.eye(), anchor.at)),
        Voice { gain, kind },
        anchor,
    ));
}

/// A mark carries farther than other placed sounds: lifted up to threefold,
/// so it still says where it is.
fn mark_lift(heard: f32) -> f32 {
    if heard > 0.0 {
        (0.35 / heard).clamp(1.0, 3.0)
    } else {
        1.0
    }
}

/// The camera carries the listener's ears — deliberately swapped. rodio
/// 0.22's `Spatial::set_positions` gives each channel
/// `((own_dist - other_dist) / gap + 1) / 4 + 0.5`, i.e. *more* volume to
/// the ear farther from the source, which its inverse-square distance term
/// normally outweighs. Placed emitters pin that term at one (see
/// `EMITTER_SCALE`), so with Bevy's ears (left at -X) every sound would
/// pan to the wrong side; with them swapped, a sound on the right is full
/// in the right speaker and half in the left.
fn attach_listener(mut commands: Commands, heads: Query<Entity, (With<Player>, Without<SpatialListener>)>) {
    for head in &heads {
        commands.entity(head).insert(SpatialListener {
            left_ear_offset: Vec3::X * EAR_GAP / 2.0,
            right_ear_offset: Vec3::X * EAR_GAP / -2.0,
        });
    }
}

/// Keep every placed emitter in its true direction as the listener moves.
fn place_emitters(ears: Ears, mut emitters: Query<(&Anchor, &mut Transform), Without<Player>>) {
    let eye = ears.eye();
    for (anchor, mut tf) in &mut emitters {
        let want = emitter_at(eye, anchor.at);
        if tf.translation.distance_squared(want) > 1e-6 {
            tf.translation = want;
        }
    }
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
        let takes = match p.variant {
            WhistleVariant::Loud => &sounds.whistles[0],
            WhistleVariant::Middling => &sounds.whistles[1],
            WhistleVariant::Faint => &sounds.whistles[2],
        };
        let clip = &takes[(p.take % WHISTLE_TAKES) as usize];
        one_shot(&mut commands, clip, p.gain, p.speed, VoiceKind::Whistle, &settings);
    }
}

/// What the frights ask to be heard: stingers and omens.
fn play_stings(
    mut commands: Commands,
    mut stings: MessageReader<crate::world::omen::Sting>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    ears: Ears,
) {
    use crate::world::omen::Sting;
    let g = ears.tuning.0.sfx_gain;
    for sting in stings.read() {
        // Stingers are the night's music, unplaced; only the clatter of
        // bones behind you comes from somewhere (and never from him). A
        // placed clatter is louder at its source, so it lands at the level
        // it had unplaced.
        let (clip, gain, speed, at) = match *sting {
            // The catch's own sounds go through its silence and its black.
            // Its files are mastered hot: the ear whistle lands a little
            // over the loud whistle, not far over the whole night.
            Sting::EarWhistle | Sting::Cut | Sting::Caught => {
                let catch = |commands: &mut Commands, clip: &Handle<AudioSource>, gain: f32| {
                    one_shot(commands, clip, gain, 1.0, VoiceKind::Catch, &settings)
                };
                match *sting {
                    Sting::EarWhistle => catch(&mut commands, &sounds.ear_whistle, g * 0.8),
                    Sting::Caught => catch(&mut commands, &sounds.sting_caught, g * 1.2),
                    _ => {
                        catch(&mut commands, &sounds.bones, g * 1.4);
                        catch(&mut commands, &sounds.ringing, g * 0.9);
                    }
                }
                continue;
            }
            Sting::Reveal => (&sounds.sting_reveal, g * 1.1, 1.0, None),
            Sting::Phantom => (&sounds.sting_phantom, g * 0.8, 1.0, None),
            Sting::Bones(at) => (&sounds.omen_bones, g * 0.7, 1.0, at),
            Sting::Lamps => (&sounds.omen_lamps, g * 0.6, 1.0, None),
            Sting::Swell => (&sounds.omen_swell, g * 0.8, 1.0, None),
            Sting::Clack(at) => (&sounds.omen_bones, g * 0.45, 1.25, at),
            // Nobody's step, at a walker's weight on whatever ground it is.
            Sting::Step(at) => {
                let surface = surface_at(&ears.layout.0, Vec2::new(at.x, at.z));
                let k = (at.x * 7.3 + at.z * 3.1).abs() as usize;
                placed_shot(
                    &mut commands,
                    &ears,
                    &sounds.steps[surface as usize][k % 3],
                    (g * 0.62 * 0.8, [0.97, 1.03, 0.94][k % 3]),
                    Some(Anchor::at(at)),
                    VoiceKind::Effect,
                    &settings,
                );
                continue;
            }
            // A mark's tick, carried like a real one.
            Sting::Mark(at) => (&sounds.ping, g * 0.7, 1.0, Some(at)),
        };
        // The stingers are music; what sounds like a thing of the llano (a
        // clatter, a false mark) is heard as one.
        let kind = match sting {
            Sting::Reveal | Sting::Phantom | Sting::Lamps | Sting::Swell => VoiceKind::Sting,
            _ => VoiceKind::Effect,
        };
        let lift = at.map_or(1.0, |at| {
            let h = heard_at(&ears.layout.0, &ears.tuning.0, ears.eye(), at);
            match sting {
                // A false mark must sound exactly like a real one.
                Sting::Mark(_) => mark_lift(h),
                _ if h > 0.0 => (1.0 / h).min(4.0),
                _ => 1.0,
            }
        });
        placed_shot(
            &mut commands,
            &ears,
            clip,
            (gain * lift, speed),
            at.map(Anchor::at),
            kind,
            &settings,
        );
    }
}

fn play_effects(
    mut commands: Commands,
    mut events: MessageReader<EncounterMsg>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    net: Res<Network>,
    fright: Res<crate::world::omen::Fright>,
    ears: Ears,
) {
    // When this player is the one caught, the lunge owns the moment.
    let caught = fright.caught_now(net.status());
    let g = ears.tuning.0.sfx_gain;
    let l = &ears.layout.0;
    let d = &l.district;
    let on_ground = |p: Vec2, up: f32| Vec3::new(p.x, l.surface_height(p) + up, p.y);
    let dog = net.snapshot().map(|s| on_ground(Vec2::from_array(s.dog.pos), 0.5));
    for EncounterMsg(e) in events.read() {
        if caught && matches!(e, Event::Downed | Event::Died) {
            continue;
        }
        // Where it happens, when that is a thing of the llano's (the actor
        // of a pickup or a revive is not known here: those stay unplaced).
        let at = match e {
            Event::PowerRestored => Some(d.panel),
            Event::TruckStarted => Some(on_ground(d.truck.center, 1.0)),
            Event::RelicDelivered | Event::AllBonesHome | Event::Banished => Some(l.ceiba.offering),
            Event::LockRattle | Event::KeyFound => Some(d.lockbox),
            Event::CattleSpooked => Some(on_ground(
                d.landmark(crate::geometry::district::LandmarkId::Corral).center,
                1.0,
            )),
            Event::BeaconLit => Some(d.beacon),
            Event::DogGrowl | Event::DogBark | Event::DogFreed => dog,
            _ => None,
        };
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
        let kind = match e {
            Event::DogGrowl | Event::DogBark | Event::HuntBegan => VoiceKind::Cue,
            _ => VoiceKind::Effect,
        };
        placed_shot(
            &mut commands,
            &ears,
            clip,
            (gain, speed),
            at.map(Anchor::at),
            kind,
            &settings,
        );
    }
}

/// A tick when anyone marks a spot.
fn play_pings(
    mut commands: Commands,
    net: Res<Network>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    ears: Ears,
    mut seen: Local<BTreeMap<u64, f32>>,
) {
    let pings = net.snapshot().map_or(&[][..], |s| s.pings.as_slice());
    seen.retain(|by, _| pings.iter().any(|p| p.by == *by));
    for p in pings {
        let fresh = seen.get(&p.by).is_none_or(|left| p.left > *left);
        seen.insert(p.by, p.left);
        if fresh {
            // A mark is a signal to the party: it carries farther than other
            // placed sounds (lifted up to threefold) and still says where it is.
            let at = Vec3::from_array(p.pos);
            let lift = mark_lift(heard_at(&ears.layout.0, &ears.tuning.0, ears.eye(), at));
            placed_shot(
                &mut commands,
                &ears,
                &sounds.ping,
                (ears.tuning.0.sfx_gain * 0.7 * lift, 1.0),
                Some(Anchor::at(at)),
                VoiceKind::Effect,
                &settings,
            );
        }
    }
}

/// A call, from where the caller is and in their own voice. It carries like
/// a mark (a call is meant to be heard) and still says where it comes from.
fn play_calls(
    mut commands: Commands,
    net: Res<Network>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    ears: Ears,
    mut seen: Local<BTreeMap<u64, f32>>,
) {
    let Some(s) = net.snapshot() else {
        seen.clear();
        return;
    };
    seen.retain(|by, _| s.calls.iter().any(|c| c.by == *by));
    for c in &s.calls {
        let fresh = seen.get(&c.by).is_none_or(|left| c.left > *left);
        seen.insert(c.by, c.left);
        if !fresh {
            continue;
        }
        let clip = match c.kind {
            CallKind::Help => &sounds.call_help,
        };
        let voice = s.player(c.by).map_or(1.0, |p| Survivor::from_code(p.survivor).voice());
        let at = Vec3::from_array(c.pos) + Vec3::Y * 0.3;
        let lift = mark_lift(heard_at(&ears.layout.0, &ears.tuning.0, ears.eye(), at));
        placed_shot(
            &mut commands,
            &ears,
            clip,
            (ears.tuning.0.sfx_gain * 0.9 * lift, voice),
            Some(Anchor::at(at)),
            VoiceKind::Effect,
            &settings,
        );
    }
}

/// The fallen groan where they lie, now and then and more often as they
/// bleed, each in their own voice: a friend can follow the sound in the
/// dark. Only from their own row and only while they can be found
/// (`PlayerView::findable`): from his sack the sound would be his. These are
/// the party's to hear, never noises he hears.
#[allow(clippy::too_many_arguments)]
fn party_cries(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    sounds: Res<Sounds>,
    net: Res<Network>,
    state: Res<State<Flow>>,
    ears: Ears,
    mut cries: Local<BTreeMap<u64, (f32, u64)>>,
) {
    let Some(s) = net.snapshot().filter(|_| *state.get() == Flow::Playing) else {
        cries.clear();
        return;
    };
    let me = net.id();
    // Whoever gets up, dies or is carried off starts afresh when next found.
    cries.retain(|id, _| s.player(*id).is_some_and(|p| p.findable()));
    let t = &ears.tuning.0;
    for p in s.players.iter().filter(|p| Some(p.id) != me && p.findable()) {
        // Just fallen, or just dropped from his sack: a groan at once.
        let (wait, count) = cries.entry(p.id).or_insert((0.0, 0));
        *wait -= time.delta_secs();
        if *wait > 0.0 {
            continue;
        }
        let mut rng = crate::rng::Rng::fork(t.seed ^ p.id, *count);
        *wait = t.groan_gap(p.bleed, rng.range(0.8, 1.2));
        // From their head, lying ahead of where they fell.
        let pos = Vec2::from_array(p.position) + Vec2::new(-p.yaw.sin(), -p.yaw.cos()) * 1.3;
        let at = Vec3::new(pos.x, ears.layout.0.rest_height(pos) + 0.3, pos.y);
        placed_shot(
            &mut commands,
            &ears,
            &sounds.groans[(*count % 3) as usize],
            (t.sfx_gain * t.groan_gain, Survivor::from_code(p.survivor).voice()),
            Some(Anchor::at(at)),
            VoiceKind::Effect,
            &settings,
        );
        *count += 1;
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
    let surface = surface_at(&layout.0, pos);
    let gait = if sprint {
        1.0
    } else if crouch {
        0.3
    } else {
        0.62
    };
    let load = 1.0 + 0.12 * net.carrying() as f32;
    let (heavy, low) = wading_deep(&layout.0, pos);
    *count += 1;
    const PITCH: [f32; 5] = [0.96, 1.04, 1.0, 0.92, 1.08];
    let clip = &sounds.steps[surface as usize][*count % 3];
    one_shot(
        &mut commands,
        clip,
        tuning.0.sfx_gain * gait * load * heavy * 0.8,
        PITCH[*count % PITCH.len()] * low,
        VoiceKind::Effect,
        &settings,
    );
}

/// Thunder rolls in after each flash, late by the storm's own distance.
fn thunder(mut commands: Commands, clock: Res<StormClock>, settings: Res<Settings>, sounds: Res<Sounds>, ears: Ears) {
    if clock.t <= clock.prev || clock.t - clock.prev > 1.0 {
        return;
    }
    let seed = ears.tuning.0.seed;
    if let Some(strike) = storm::thunder_strike(seed, clock.prev, clock.t) {
        let variant = ((clock.t * 7.0) as usize) % 2;
        // From the bolt everyone saw, far out on the llano.
        let (ground, _) = storm::bolt_ground(seed, strike.at);
        placed_shot(
            &mut commands,
            &ears,
            &sounds.thunder[variant],
            (
                ears.tuning.0.sfx_gain * (0.55 + 0.75 * strike.power.clamp(0.0, 1.0)),
                1.0,
            ),
            Some(Anchor {
                at: Vec3::new(ground.x, 120.0, ground.y),
                far: true,
            }),
            VoiceKind::Weather,
            &settings,
        );
    }
}

/// Teammates' footfalls, from where they walk: the party can be heard
/// coming (and anyone might be mistaken for one). Nobody down, stunned,
/// hauled in his sack or jumping across the map makes a step.
#[allow(clippy::too_many_arguments)]
fn party_steps(
    mut commands: Commands,
    settings: Res<Settings>,
    sounds: Res<Sounds>,
    net: Res<Network>,
    state: Res<State<Flow>>,
    ears: Ears,
    mut walked: Local<BTreeMap<u64, (Vec2, f32, usize)>>,
) {
    let Some(s) = net.snapshot().filter(|_| *state.get() == Flow::Playing) else {
        walked.clear();
        return;
    };
    let me = net.id();
    walked.retain(|id, _| s.players.iter().any(|p| p.id == *id));
    let l = &ears.layout.0;
    let stride = ears.tuning.0.stride;
    for p in &s.players {
        if Some(p.id) == me {
            continue;
        }
        let pos = Vec2::from_array(p.position);
        let entry = walked.entry(p.id).or_insert((pos, 0.0, 0));
        let moved = entry.0.distance(pos);
        entry.0 = pos;
        if p.status != 0 || p.hauled || moved > 2.0 {
            entry.1 = 0.0;
            continue;
        }
        entry.1 += moved;
        if entry.1 < stride {
            continue;
        }
        entry.1 %= stride;
        entry.2 += 1;
        let surface = surface_at(l, pos);
        let gait = if p.sprint {
            1.0
        } else if p.crouch {
            0.3
        } else {
            0.62
        };
        let load = 1.0 + 0.12 * p.carrying as f32;
        let (heavy, low) = wading_deep(l, pos);
        const PITCH: [f32; 5] = [1.02, 0.95, 1.07, 0.98, 0.93];
        placed_shot(
            &mut commands,
            &ears,
            &sounds.steps[surface as usize][entry.2 % 3],
            (
                ears.tuning.0.sfx_gain * gait * load * heavy * 0.8,
                PITCH[entry.2 % PITCH.len()] * low,
            ),
            Some(Anchor::at(Vec3::new(pos.x, l.surface_height(pos) + 0.1, pos.y))),
            VoiceKind::Effect,
            &settings,
        );
    }
}

/// A survivor's step waist-deep in the caño: the water clips, heavier and
/// lower (gain and pitch factors). Only ever a survivor's: he wades silent,
/// since a splash would place him.
fn wading_deep(layout: &crate::geometry::Layout, pos: Vec2) -> (f32, f32) {
    if layout.wade(pos) == crate::geometry::Wade::Deep {
        (1.4, 0.85)
    } else {
        (1.0, 1.0)
    }
}

fn surface_at(layout: &crate::geometry::Layout, pos: Vec2) -> Surface {
    let d = &layout.district;
    if layout.wading(pos) {
        Surface::Water
    } else if d.surface_at(pos).is_some() {
        Surface::Wood
    } else if d.grass.iter().any(|r| r.contains(pos)) {
        Surface::Grass
    } else {
        Surface::Dirt
    }
}

/// A volume slider moved: a moment of that bus at its new level, heard even
/// in the pause menu (the last move of a frame wins, and cuts the one
/// before). On the title screen the theme and the night already play, so
/// only the tick is needed there. Never a whistle: a menu must not cue him.
fn play_preview(
    mut commands: Commands,
    mut moved: MessageReader<VolumePreview>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    state: Res<State<Flow>>,
    voices: Query<(Entity, &Voice)>,
) {
    let Some(&VolumePreview(bus)) = moved.read().last() else {
        return;
    };
    let t = &tuning.0;
    let (clip, gain, seconds) = match bus {
        Bus::Master | Bus::Effects => (&sounds.ping, t.sfx_gain * 0.7, None),
        Bus::Ambience if *state.get() == Flow::Paused => (&sounds.ambience, t.ambience_gain, Some(1.5)),
        Bus::Music if *state.get() != Flow::Title => (&sounds.theme, t.ambience_gain * 1.2, Some(2.0)),
        Bus::Ambience | Bus::Music => return,
    };
    for (entity, voice) in &voices {
        if matches!(voice.kind, VoiceKind::Preview(_)) {
            commands.entity(entity).try_despawn();
        }
    }
    let kind = VoiceKind::Preview(bus);
    let mut playback = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(base_level(&settings, gain, kind)));
    if let Some(s) = seconds {
        playback = playback.with_duration(Duration::from_secs_f32(s));
    }
    commands.spawn((
        Name::new("volume preview"),
        AudioPlayer::new(clip.clone()),
        playback,
        Voice { gain, kind },
    ));
}

/// Apply the sliders and every loop's dynamic level to each live sink.
#[allow(clippy::too_many_arguments)]
fn mix(
    time: Res<Time<Real>>,
    truth: Res<Truth>,
    settings: Res<Settings>,
    tuning: Res<TuningRes>,
    clock: Res<StormClock>,
    mut hush: ResMut<Hush>,
    mut sinks: Query<(&Voice, &mut AudioSink)>,
    mut placed: Query<(&Voice, &Anchor, &mut SpatialAudioSink)>,
    state: Res<State<Flow>>,
    net: Res<Network>,
    note: Res<crate::encounter::NoteOpen>,
    fright: Res<crate::world::omen::Fright>,
    ears: Ears,
) {
    let tense = matches!(
        truth.encounter.threat.state,
        ThreatState::Warning | ThreatState::Hunting
    );
    let target = if tense { tuning.0.ambience_hush } else { 1.0 };
    let dt = time.delta_secs();
    hush.0 = glide(hush.0, target, 0.8, dt);

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

    let paused = *state.get() == Flow::Paused;
    let powered = world.power >= 1.0 && !over;
    let level = |voice: &Voice, sink: &mut dyn AudioSinkPlayback| -> f32 {
        let mut v = voice.gain * settings.gain(voice.kind.bus());
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
            VoiceKind::Frogs => v *= hush_omen * duck * hush.0,
            VoiceKind::Windmill => v *= duck * (0.7 + 0.3 * rain),
            VoiceKind::Hum => v *= if powered { duck } else { 0.0 },
            VoiceKind::Engine => {
                v *= if engine_on { 0.6 + 0.4 * world.warm } else { 0.0 };
                sink.set_speed(0.92 + 0.16 * world.warm);
            }
            VoiceKind::Crank => v *= if crank_on { 1.0 } else { 0.0 },
            VoiceKind::Radio => v *= if radio_on { 1.0 } else { 0.0 },
            VoiceKind::Whistle if net.status() == 2 => v = 0.0,
            VoiceKind::Whistle
            | VoiceKind::Effect
            | VoiceKind::Cue
            | VoiceKind::Catch
            | VoiceKind::Sting
            | VoiceKind::Weather
            | VoiceKind::Preview(_) => {}
        }
        v
    };
    // The catch holds the world: silent before him, a murmur in the black.
    // Its own sounds are heard through all of it, and so is the menu's.
    let world_level = fright.world_level();
    let apply = |voice: &Voice, sink: &mut dyn AudioSinkPlayback, v: f32| {
        let menu = matches!(voice.kind, VoiceKind::Preview(_));
        if paused && !menu {
            sink.pause();
        }
        let held = !menu && voice.kind != VoiceKind::Catch;
        // No one voice goes over full scale (a lifted, placed sound near).
        let v = if held { v * world_level } else { v }.min(1.0);
        let now = sink.volume().to_linear();
        // Loops glide to their level and land on it; one-shots keep theirs,
        // and the catch's silence falls at once.
        let v = if voice.kind.is_loop() && !(held && world_level == 0.0) {
            glide(now, v, GLIDE_RATE, dt)
        } else {
            v
        };
        if v != now {
            sink.set_volume(Volume::Linear(v));
        }
    };
    for (voice, mut sink) in &mut sinks {
        let v = level(voice, &mut *sink);
        apply(voice, &mut *sink, v);
    }
    for (voice, anchor, mut sink) in &mut placed {
        let v = level(voice, &mut *sink) * ears.heard(anchor);
        apply(voice, &mut *sink, v);
    }
}

fn pause_all(sinks: Query<&AudioSink>, placed: Query<&SpatialAudioSink>) {
    for sink in &sinks {
        sink.pause();
    }
    for sink in &placed {
        sink.pause();
    }
}

fn resume_all(sinks: Query<&AudioSink>, placed: Query<&SpatialAudioSink>) {
    for sink in &sinks {
        sink.play();
    }
    for sink in &placed {
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
