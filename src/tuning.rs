//! Every gameplay number of the encounter, in one place.
//!
//! Pure data: no ECS, no randomness. The simulation, perception layer, player
//! controller, debug route and tests all read the same [`Tuning`] so a change
//! here is a change everywhere. Distances are metres, times seconds, speeds
//! metres per second, angles radians.

/// Default world/cue seed. Fixed so every run (and every screenshot) is the
/// same unless `--seed` is passed.
pub const DEFAULT_SEED: u64 = 1997;

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// Seed for procedural scatter (grass, props) and whistle phrase jitter.
    pub seed: u64,

    // ---------------------------------------------------------------- player
    /// Collision radius of the player on the ground plane.
    pub player_radius: f32,
    /// Camera height above the flat ground.
    pub eye_height: f32,
    /// Normal walking speed.
    pub walk_speed: f32,
    /// Speed while carrying the bone satchel (the readable burden penalty).
    pub carry_speed: f32,
    /// Maximum look up/down angle.
    pub max_pitch: f32,
    /// Radians of turn per mouse count at sensitivity multiplier 1.0.
    pub mouse_radians_per_count: f32,
    /// Allowed sensitivity multiplier range (settings menu clamps to this).
    pub sensitivity_range: (f32, f32),

    // ----------------------------------------------------------- interaction
    /// Eye-to-target reach for taking the satchel.
    pub satchel_reach: f32,
    /// Eye-to-target reach for reading the note.
    pub note_reach: f32,
    /// Eye-to-target reach for the ceiba's root hollow.
    pub offering_reach: f32,
    /// Extra ground distance the truth layer tolerates when re-validating a
    /// client's claim that it is in reach (authoritative-style double check).
    pub reach_slack: f32,
    /// Seconds of continuous holding needed to return the bones.
    pub restitution_hold: f32,

    // ---------------------------------------------------------------- threat
    /// He never manifests closer than this to the player.
    pub manifest_min_distance: f32,
    /// Seconds to rise out of the grass when he manifests.
    pub rise_time: f32,
    /// Seconds to sink away when he loses track or is resolved.
    pub sink_time: f32,
    /// After the satchel is taken, no warning can start before this.
    pub first_warn_delay: f32,
    /// Walking speed along the authored anchor ring.
    pub stalk_speed: f32,
    /// Speed when he leaves an anchor and creeps straight toward a visible player.
    pub creep_speed: f32,
    /// Speed while hunting (always slower than `carry_speed`).
    pub hunt_speed: f32,
    /// Warning begins when he sees the player within this distance.
    pub warn_distance: f32,
    /// Seconds of warning before the hunt begins if he still sees the player.
    pub warn_time: f32,
    /// Seconds out of sight during a warning that avert it.
    pub warn_break_time: f32,
    /// Seconds of continuous sight during a hunt that end the encounter
    /// (at the far edge of `warn_distance`; faster when he is close).
    pub exposure_time: f32,
    /// Extra exposure rate at point-blank range (0 = none, 1 = double).
    pub exposure_near_boost: f32,
    /// Exposure lost per second while out of his sight.
    pub exposure_decay: f32,
    /// Seconds out of sight during a hunt before he loses track and withdraws.
    pub lose_track_time: f32,
    /// After losing track: seconds before he may warn again.
    pub recover_cooldown: f32,
    /// After an averted warning: seconds before he may warn again.
    pub warn_recover_cooldown: f32,
    /// Hunting and within this distance while seeing the player = caught.
    pub catch_distance: f32,
    /// Seconds lurking at an anchor without sight before he starts circling.
    pub patience: f32,

    // ------------------------------------------------------------ perception
    /// True distance at (and below) which the whistle seems FAINTEST.
    pub cue_near_distance: f32,
    /// True distance at (and above) which the whistle seems LOUDEST.
    pub cue_far_distance: f32,
    /// Seconds between whistle phrases while stalking (min, max).
    pub stalk_phrase_interval: (f32, f32),
    /// Seconds between phrases during a warning.
    pub warn_phrase_interval: f32,
    /// Seconds between phrases during a hunt.
    pub hunt_phrase_interval: f32,
    /// Seconds from manifestation to the first phrase.
    pub first_phrase_delay: f32,
    /// Linear playback gains of the three perceived variants (gentle).
    pub gain_loud: f32,
    pub gain_mid: f32,
    pub gain_faint: f32,

    // ----------------------------------------------------------------- audio
    /// Linear gain of the night ambience loop.
    pub ambience_gain: f32,
    /// Ambience multiplier while he warns or hunts (the insects hush).
    pub ambience_hush: f32,
    /// Linear gain of one-shot effects.
    pub sfx_gain: f32,

    // ------------------------------------------------------------ simulation
    /// Largest step the truth layer integrates at once (hitches are clamped).
    pub max_step: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,

            player_radius: 0.3,
            eye_height: 1.62,
            walk_speed: 3.6,
            carry_speed: 2.7,
            max_pitch: 1.45,
            mouse_radians_per_count: 0.0022,
            sensitivity_range: (0.2, 3.0),

            satchel_reach: 2.4,
            note_reach: 2.4,
            offering_reach: 3.0,
            reach_slack: 0.35,
            restitution_hold: 3.0,

            manifest_min_distance: 30.0,
            rise_time: 1.6,
            sink_time: 1.4,
            first_warn_delay: 9.0,
            stalk_speed: 2.2,
            creep_speed: 1.2,
            hunt_speed: 2.3,
            warn_distance: 22.0,
            warn_time: 3.5,
            warn_break_time: 0.75,
            exposure_time: 7.0,
            exposure_near_boost: 0.8,
            exposure_decay: 0.35,
            lose_track_time: 2.5,
            recover_cooldown: 12.0,
            warn_recover_cooldown: 6.0,
            catch_distance: 1.5,
            patience: 20.0,

            cue_near_distance: 5.0,
            cue_far_distance: 42.0,
            stalk_phrase_interval: (7.0, 10.0),
            warn_phrase_interval: 3.2,
            hunt_phrase_interval: 3.8,
            first_phrase_delay: 1.2,
            gain_loud: 0.5,
            gain_mid: 0.34,
            gain_faint: 0.22,

            ambience_gain: 0.32,
            ambience_hush: 0.25,
            sfx_gain: 0.55,

            max_step: 0.1,
        }
    }
}

impl Tuning {
    /// Default tuning with a different seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }
}
