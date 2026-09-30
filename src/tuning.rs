//! Every gameplay number of the run, in one place.
//!
//! Pure data: no ECS, no randomness. The simulation, perception layer, body
//! model, session, debug route and tests all read the same [`Tuning`] so a
//! change here is a change everywhere. Distances are metres, times seconds,
//! speeds metres per second, angles radians.

/// Default world/cue seed. Fixed so every run (and every screenshot) is the
/// same unless `--seed` is passed.
pub const DEFAULT_SEED: u64 = 1997;

/// La Rabia's last stage: every bundle laid to rest.
pub const MAX_RAGE: u8 = 5;

/// How hard the night is. Normal is the tuned game; the others scale it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Night {
    /// For learning the llano: he notices you later, his gaze is slower,
    /// the torch lasts, the rites anger him less, the rhythm is forgiving.
    Gentle,
    #[default]
    Normal,
    /// He notices you farther off, his gaze fills faster, the torch dies
    /// sooner, every rite angers him more, omens crowd you, the rhythm is
    /// tight and he is slower to tire of waiting over you.
    Hard,
}

impl Night {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "gentle" => Some(Night::Gentle),
            "normal" => Some(Night::Normal),
            "hard" => Some(Night::Hard),
            _ => None,
        }
    }

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        [Night::Gentle, Night::Normal, Night::Hard]
            .into_iter()
            .find(|n| n.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Night::Gentle => "gentle",
            Night::Normal => "normal",
            Night::Hard => "hard",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// Seed for procedural scatter (grass, props) and whistle phrase jitter.
    pub seed: u64,
    /// How hard the night is (already applied to the numbers below).
    pub night: Night,

    // ---------------------------------------------------------------- player
    /// Collision radius of the player on the ground plane.
    pub player_radius: f32,
    /// Standing camera height above the ground.
    pub eye_height: f32,
    /// How far the eye drops when crouching.
    pub crouch_lower: f32,
    /// How far the eye drops when downed.
    pub downed_lower: f32,
    /// Normal walking speed.
    pub walk_speed: f32,
    /// Crouch-walking speed as a fraction of walking.
    pub crouch_factor: f32,
    /// Sprint speed as a fraction of walking.
    pub sprint_factor: f32,
    /// Wading speed as a fraction of the current gait.
    pub wade_factor: f32,
    /// Speed waist-deep in a channel, as a fraction of the current gait; no
    /// sprinting there.
    pub deep_wade_factor: f32,
    /// Crawling speed of a downed player.
    pub crawl_speed: f32,
    /// Speed lost per carried bone bundle (fraction of walking).
    pub carry_slow: f32,
    /// The slowest a heavy load can make you (fraction of walking).
    pub carry_floor: f32,
    /// Stamina (0..1) drained per second of sprinting.
    pub stamina_drain: f32,
    pub stamina_regen: f32,
    /// Seconds after sprinting before stamina returns.
    pub stamina_delay: f32,
    /// Minimum stamina needed to start a sprint.
    pub sprint_min: f32,
    /// Maximum look up/down angle.
    pub max_pitch: f32,
    /// Radians of turn per mouse count at sensitivity multiplier 1.0.
    pub mouse_radians_per_count: f32,
    /// Allowed sensitivity multiplier range (settings menu clamps to this).
    pub sensitivity_range: (f32, f32),

    // ----------------------------------------------------------- interaction
    /// Eye-to-target reach for bone bundles and peppers.
    pub relic_reach: f32,
    /// Eye-to-target reach for reading notes.
    pub note_reach: f32,
    /// Eye-to-target reach for the altar in the ceiba's roots.
    pub altar_reach: f32,
    /// Eye-to-target reach for the pump crank, the ignition and the beacon.
    pub site_reach: f32,
    /// Reach for reviving a downed teammate.
    pub body_reach: f32,
    /// Extra ground distance the truth layer tolerates when re-validating a
    /// client's claim that it is in reach.
    pub reach_slack: f32,
    /// Seconds of holding at the altar to lay down one bundle.
    pub deliver_hold: f32,
    /// Seconds of holding at the altar with empty hands to pray.
    pub pray_hold: f32,
    /// Seconds of cranking (summed over players) to restore power.
    pub pump_hold: f32,
    /// Seconds of holding the ignition to start the truck.
    pub truck_hold: f32,
    /// The running engine needs this long before the truck can leave; he
    /// comes for the noise in the meantime.
    pub truck_warmup: f32,
    /// Seconds of holding to light the watchtower beacon.
    pub beacon_hold: f32,
    /// Seconds of holding beside a downed teammate to revive them.
    pub revive_hold: f32,
    /// A downed player dies after this long without help.
    pub bleed_out: f32,
    /// Dropped from his sack, the fallen have at least this long left: they
    /// lie where nobody saw them fall, and must be found.
    pub bleed_after_sack: f32,
    /// The beacon burns this long, then must cool down.
    pub beacon_burn: f32,
    pub beacon_cooldown: f32,
    /// Seconds of light in a fresh torch battery.
    pub battery_life: f32,
    /// Fraction of a full charge one pair of spare batteries restores.
    pub battery_pickup: f32,
    /// Below this charge the beam gutters (and you cannot take spares above
    /// `battery_full`: the torch is fresh).
    pub battery_low: f32,
    pub battery_full: f32,
    /// A lit torch he can see within this distance draws him to look; it
    /// pulses his attention every `light_lure_pulse` seconds.
    pub light_lure_range: f32,
    pub light_lure_pulse: f32,
    /// Seconds of work between skill checks (min, max), and how soon a
    /// fresh laying of bones asks for its first.
    pub check_every: (f32, f32),
    pub check_first_rite: f32,
    /// Warning before the needle moves; the needle's sweep.
    pub check_warn: f32,
    pub check_sweep: f32,
    /// Width of the zone and of its great start (fractions of the sweep),
    /// and where along the sweep the zone may start.
    pub check_zone: f32,
    pub check_great: f32,
    pub check_zone_at: (f32, f32),
    /// Seconds a press may take to reach the host after the sweep, and how
    /// far ahead of the host's needle a claim may be (fraction of the sweep).
    pub check_latency: f32,
    pub check_slack: f32,
    /// Seconds of work a miss throws back: more than a whole check lasts, so
    /// a miss always ends behind where its warning found the work.
    pub check_setback: f32,
    /// Seconds of holding on after a miss that do nothing: the crank kicks
    /// back, the engine floods, the bones slip.
    pub check_stall: f32,
    /// Work gained by a great press (a fraction of the task).
    pub check_bonus: f32,
    /// Peppers a player can carry.
    pub aji_max: u8,
    pub aji_zone_radius: f32,
    pub aji_zone_life: f32,
    /// He stops to count the bones for this long.
    pub counting_time: f32,
    /// Walking pace while he hauls a fallen player off in his sack (slower
    /// than a walking teammate), and how long before they are gone.
    pub haul_speed: f32,
    pub haul_time: f32,
    /// Tureco the dog: seconds to untie him, how close he trails the one he
    /// follows, his top speed, the distances at which he growls at him and
    /// barks him off, how long his courage takes to come back, and how far
    /// his bark carries.
    pub untie_hold: f32,
    pub dog_trail: f32,
    pub dog_speed: f32,
    pub growl_range: f32,
    pub bark_range: f32,
    pub dog_courage: f32,
    pub noise_bark: f32,
    /// After a wrong name at the ceiba, seconds before it hears another.
    pub naming_cooldown: f32,
    /// Seconds between the night's signs of which of him walks (min, max).
    pub tell_every: (f32, f32),
    pub ping_cooldown: f32,
    pub ping_life: f32,
    pub ping_range: f32,
    /// A call out loud (the downed player's cry for help): seconds before
    /// the same voice calls again, and how long the call shows.
    pub call_cooldown: f32,
    pub call_life: f32,
    /// The fallen see him when the friend they watch does, through that
    /// friend's eyes (off: they watch their friend panic at nothing).
    pub anima_sight: bool,

    // ---------------------------------------------------------------- threat
    /// He never manifests closer than this to a player.
    pub manifest_min_distance: f32,
    /// Seconds to rise out of the grass when he manifests.
    pub rise_time: f32,
    /// Seconds to sink away when he loses track.
    pub sink_time: f32,
    /// After the first bundle is taken, no warning can start before this.
    pub first_warn_delay: f32,
    /// Walking speed along the patrol graph.
    pub stalk_speed: f32,
    /// Speed when he leaves a node and creeps straight toward a visible player.
    pub creep_speed: f32,
    /// Speed while hunting (always slower than a walking player).
    pub hunt_speed: f32,
    /// Warning begins when he sees a player within this distance.
    pub warn_distance: f32,
    /// Seconds of warning before the hunt begins if he still sees the player.
    pub warn_time: f32,
    /// Seconds out of sight during a warning that avert it.
    pub warn_break_time: f32,
    /// Seconds of continuous sight during a hunt that down the player
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
    /// Seconds lurking without sight before he starts roaming the whole map.
    pub patience: f32,
    /// Warnings averted in a row after which he tires of waiting over his
    /// prey: he sinks into the grass and rises somewhere far off.
    pub averts_to_withdraw: u32,
    /// He waits at least this far from the player he is stalking.
    pub standoff: f32,
    /// Seconds he searches the spot of a noise before giving up.
    pub search_time: f32,
    /// How long a heard noise stays in his mind.
    pub noise_memory: f32,
    /// Sight multiplier when the player crouches, holds a light, or hides in
    /// tall grass (multiplies the warning distance).
    pub sight_crouch: f32,
    pub sight_light: f32,
    /// Crouched in tall grass, he cannot see you beyond this distance.
    pub grass_sight: f32,

    // ----------------------------------------------------------------- noise
    /// Ground covered per footstep noise.
    pub stride: f32,
    /// Audible radius of one step by gait.
    pub noise_crouch: f32,
    pub noise_walk: f32,
    pub noise_sprint: f32,
    /// Added radius when wading or crossing planks.
    pub noise_wade: f32,
    pub noise_plank: f32,
    /// Added radius of a step waist-deep in a channel: a walker there is
    /// heard as far as a sprinter.
    pub noise_deep: f32,
    /// Added radius per carried bone bundle (they rattle) while not crouched.
    pub noise_carry: f32,
    pub noise_pump: f32,
    pub noise_altar: f32,
    pub noise_truck: f32,
    pub noise_beacon: f32,
    pub noise_cattle: f32,
    pub noise_susto: f32,
    pub noise_revive: f32,
    pub noise_pickup: f32,
    pub noise_drop: f32,
    pub noise_aji: f32,
    /// A cry for help from the ground: he hears it too.
    pub noise_call: f32,
    /// A missed skill check: the windmill screeches, the engine backfires,
    /// the bones clatter. At least `noise_miss`, and always louder than the
    /// work it spoils (its own noise × `noise_miss_over`).
    pub noise_miss: f32,
    pub noise_miss_over: f32,
    /// A wrong combination rattles the padlock; the right one clunks open.
    pub noise_rattle: f32,
    pub noise_unlock: f32,
    /// How much heavy rain shrinks every noise radius (0..1).
    pub rain_mask: f32,
    /// Extra shrink during a thunder roll.
    pub thunder_mask: f32,
    /// How near a walking player must be to the herd to unsettle it.
    pub cattle_radius: f32,
    pub cattle_alarm: f32,
    pub cattle_cooldown: f32,

    // ------------------------------------------------------- night pressure
    /// Seconds for the night to reach its worst.
    pub night_length: f32,
    pub pressure_base: f32,
    pub pressure_night: f32,
    /// Pressure per carried bundle, and per bundle laid to rest: every
    /// bundle returned to the roots is one less in his sack, and he feels it.
    pub pressure_carry: f32,
    pub pressure_rite: f32,
    pub pressure_max: f32,

    // ------------------------------------------------------------------ fear
    /// Fear per second of standing in the dark.
    pub fear_dark: f32,
    /// Extra fear per second in the dark with no teammate near.
    pub fear_alone: f32,
    /// Fear per second per carried bundle.
    pub fear_carry: f32,
    /// Fear lost per second beside a teammate, in lamplight, and praying.
    pub fear_team: f32,
    pub fear_light: f32,
    pub fear_pray: f32,
    /// Fear per second while he is on you (times exposure).
    pub fear_exposure: f32,
    /// One-off fear when he warns, and when a whistle sounds faint.
    pub fear_warn: f32,
    pub fear_faint: f32,
    /// Fear from your own missed skill check.
    pub fear_miss: f32,
    /// Fear a susto resets to, and how long it freezes you.
    pub susto_reset: f32,
    pub susto_stun: f32,
    /// Teammates within this distance calm each other.
    pub team_radius: f32,

    // ------------------------------------------------------------ perception
    /// True distance at (and below) which the whistle seems FAINTEST.
    pub cue_near_distance: f32,
    /// True distance at (and above) which the whistle seems LOUDEST.
    pub cue_far_distance: f32,
    /// Seemingly-close thresholds above which a phrase is heard as loud,
    /// and as middling (below both: faint).
    pub cue_loud_above: f32,
    pub cue_mid_above: f32,
    /// Seconds between whistle phrases while stalking (min, max), usually…
    pub stalk_phrase_interval: (f32, f32),
    /// …but sometimes he answers himself almost at once (chance, seconds)…
    pub stalk_answer_chance: f32,
    pub stalk_answer: (f32, f32),
    /// …and sometimes the llano goes quiet for a long while (chance, seconds).
    pub stalk_silence_chance: f32,
    pub stalk_silence: (f32, f32),
    /// El Velo: while nobody can see him, the long silences come this much
    /// rarer (a multiplier on `stalk_silence_chance`) and no stalking gap
    /// runs past `veil_gap_max`: then the whistle is all anyone has of him.
    pub veil_silence: f32,
    pub veil_gap_max: f32,
    /// Seconds between phrases during a warning.
    pub warn_phrase_interval: f32,
    /// Seconds between phrases during a hunt.
    pub hunt_phrase_interval: f32,
    /// Warning and hunt intervals vary by up to this fraction either way.
    pub phrase_jitter: f32,
    /// Playback speed range of a phrase (it also moves the pitch).
    pub whistle_speed: (f32, f32),
    /// Seconds from manifestation to the first phrase.
    pub first_phrase_delay: f32,
    /// Seconds between far-off whistles before he has manifested.
    pub prologue_phrase_interval: (f32, f32),
    /// Quiet seconds (min, max) before the night sends a player an omen
    /// (shorter as pressure rises, longer before he wakes).
    pub omen_quiet: (f32, f32),
    /// Fear at which a player may glimpse him where he is not, and the rage
    /// stage (bones laid) from which a torch may move far off that is
    /// nobody's.
    pub phantom_fear: f32,
    pub stolen_light_rage: u8,
    /// Fear at which a player may hear a whistle that is not there, and the
    /// mean seconds between such whistles.
    pub phantom_whistle_fear: f32,
    pub phantom_whistle_every: f32,
    /// Linear playback gains of the three perceived variants. The files are
    /// loudness-matched, so these are the whole level difference: about
    /// 20 dB between "right beside you" and "far, far away" (the timbres
    /// carry the rest). `tools/whistle_lab.py` plays them at these levels.
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
    /// Linear gain of the rain loop.
    pub rain_gain: f32,
    /// Ambience and rain multiplier while a whistle sounds, so a faint one
    /// is heard as faint rather than lost.
    pub whistle_duck: f32,
    /// How the llano's placed sounds (machines, the dog, teammates, frogs;
    /// never the whistle) fall off: full within `sound_near` metres, then
    /// inversely with distance, silent by `sound_far`; a wall or bank
    /// between leaves `sound_occluded` of it.
    pub sound_near: f32,
    pub sound_far: f32,
    pub sound_occluded: f32,
    /// Seconds between a downed friend's groans: at the last of their bleed
    /// and with all of it left (they groan more often as it runs out), and
    /// the groans' gain on the effects.
    pub groan_every: (f32, f32),
    pub groan_gain: f32,

    // ------------------------------------------------------------ simulation
    /// Largest step the truth layer integrates at once (hitches are clamped).
    pub max_step: f32,

    // ------------------------------------------------------------ the radio
    /// How far the dial's squeal carries (metres), and how near the radio
    /// the dots caption shows its pips.
    pub noise_dial: f32,
    pub radio_reach: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,
            night: Night::Normal,

            player_radius: 0.3,
            eye_height: 1.62,
            crouch_lower: 0.62,
            downed_lower: 1.28,
            walk_speed: 3.6,
            crouch_factor: 0.5,
            sprint_factor: 1.45,
            wade_factor: 0.55,
            deep_wade_factor: 0.35,
            crawl_speed: 0.9,
            carry_slow: 0.06,
            carry_floor: 0.62,
            stamina_drain: 0.24,
            stamina_regen: 0.17,
            stamina_delay: 1.1,
            sprint_min: 0.12,
            max_pitch: 1.45,
            mouse_radians_per_count: 0.0022,
            sensitivity_range: (0.2, 3.0),

            relic_reach: 2.4,
            note_reach: 2.4,
            altar_reach: 3.0,
            site_reach: 2.6,
            body_reach: 2.4,
            reach_slack: 0.35,
            deliver_hold: 3.0,
            pray_hold: 5.0,
            pump_hold: 12.0,
            truck_hold: 6.0,
            truck_warmup: 26.0,
            beacon_hold: 2.0,
            revive_hold: 4.0,
            bleed_out: 60.0,
            bleed_after_sack: 30.0,
            beacon_burn: 26.0,
            beacon_cooldown: 70.0,
            battery_life: 300.0,
            battery_pickup: 0.6,
            battery_low: 0.18,
            battery_full: 0.95,
            light_lure_range: 45.0,
            light_lure_pulse: 2.0,
            check_every: (2.5, 5.0),
            check_first_rite: 0.5,
            check_warn: 0.6,
            check_sweep: 1.1,
            check_zone: 0.14,
            check_great: 0.04,
            check_zone_at: (0.45, 0.82),
            check_latency: 0.35,
            check_slack: 0.06,
            check_setback: 2.5,
            check_stall: 1.8,
            check_bonus: 0.04,
            aji_max: 3,
            aji_zone_radius: 3.4,
            aji_zone_life: 24.0,
            counting_time: 7.5,
            untie_hold: 1.5,
            dog_trail: 1.8,
            dog_speed: 5.5,
            growl_range: 22.0,
            bark_range: 7.0,
            dog_courage: 90.0,
            noise_bark: 30.0,
            naming_cooldown: 60.0,
            haul_speed: 1.7,
            haul_time: 35.0,
            tell_every: (60.0, 110.0),
            ping_cooldown: 2.0,
            ping_life: 14.0,
            ping_range: 140.0,
            call_cooldown: 8.0,
            call_life: 3.0,
            anima_sight: true,

            manifest_min_distance: 30.0,
            rise_time: 1.6,
            sink_time: 1.4,
            first_warn_delay: 9.0,
            stalk_speed: 2.2,
            creep_speed: 1.2,
            hunt_speed: 2.3,
            warn_distance: 24.0,
            warn_time: 3.5,
            warn_break_time: 0.75,
            exposure_time: 7.0,
            exposure_near_boost: 0.8,
            exposure_decay: 0.35,
            lose_track_time: 4.0,
            recover_cooldown: 12.0,
            warn_recover_cooldown: 6.0,
            catch_distance: 1.5,
            patience: 20.0,
            averts_to_withdraw: 4,
            standoff: 13.0,
            search_time: 6.0,
            noise_memory: 14.0,
            sight_crouch: 0.6,
            sight_light: 1.3,
            grass_sight: 5.0,

            stride: 1.4,
            noise_crouch: 2.5,
            noise_walk: 8.0,
            noise_sprint: 22.0,
            noise_wade: 8.0,
            noise_plank: 6.0,
            noise_deep: 14.0,
            noise_carry: 1.6,
            noise_pump: 34.0,
            noise_altar: 22.0,
            noise_truck: 75.0,
            noise_beacon: 95.0,
            noise_cattle: 48.0,
            noise_susto: 34.0,
            noise_revive: 4.0,
            noise_pickup: 5.0,
            noise_drop: 11.0,
            noise_aji: 7.0,
            noise_call: 18.0,
            noise_miss: 40.0,
            noise_miss_over: 1.6,
            noise_rattle: 12.0,
            noise_unlock: 6.0,
            rain_mask: 0.32,
            thunder_mask: 0.3,
            cattle_radius: 10.0,
            cattle_alarm: 5.0,
            cattle_cooldown: 20.0,

            night_length: 1200.0,
            pressure_base: 0.10,
            pressure_night: 0.2,
            pressure_carry: 0.04,
            pressure_rite: 0.12,
            pressure_max: 0.9,

            fear_dark: 0.008,
            fear_alone: 0.012,
            fear_carry: 0.003,
            fear_team: 0.05,
            fear_light: 0.06,
            fear_pray: 0.22,
            fear_exposure: 0.14,
            fear_warn: 0.12,
            fear_faint: 0.05,
            fear_miss: 0.12,
            susto_reset: 0.55,
            susto_stun: 1.1,
            team_radius: 9.0,

            cue_near_distance: 5.0,
            cue_far_distance: 42.0,
            cue_loud_above: 0.62,
            cue_mid_above: 0.3,
            stalk_phrase_interval: (6.0, 13.0),
            stalk_answer_chance: 0.15,
            stalk_answer: (1.8, 3.4),
            stalk_silence_chance: 0.2,
            stalk_silence: (15.0, 26.0),
            veil_silence: 0.25,
            veil_gap_max: 10.0,
            warn_phrase_interval: 3.2,
            hunt_phrase_interval: 3.8,
            phrase_jitter: 0.35,
            whistle_speed: (0.92, 1.06),
            first_phrase_delay: 1.2,
            prologue_phrase_interval: (28.0, 46.0),
            omen_quiet: (45.0, 85.0),
            phantom_fear: 0.55,
            stolen_light_rage: 2,
            phantom_whistle_fear: 0.7,
            phantom_whistle_every: 25.0,
            gain_loud: 1.0,
            gain_mid: 0.3,
            gain_faint: 0.1,

            ambience_gain: 0.32,
            ambience_hush: 0.25,
            sfx_gain: 0.55,
            rain_gain: 0.36,
            whistle_duck: 0.4,
            sound_near: 3.0,
            sound_far: 90.0,
            sound_occluded: 0.45,
            groan_every: (3.5, 8.0),
            groan_gain: 0.55,

            max_step: 0.1,

            noise_dial: 8.0,
            radio_reach: 6.0,
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

    /// These numbers for a gentler or harder night. His hunt never outpaces
    /// a walking player and his warning always stays readable.
    pub fn with_night(self, night: Night) -> Self {
        let k = match night {
            Night::Gentle => -1.0,
            Night::Normal => return Self { night, ..self },
            Night::Hard => 1.0,
        };
        let scale = |base: f32, harder: f32, gentler: f32| if k > 0.0 { base * harder } else { base * gentler };
        let zone = scale(self.check_zone, 0.75, 1.4);
        Self {
            night,
            warn_distance: scale(self.warn_distance, 1.12, 0.85),
            exposure_time: scale(self.exposure_time, 0.85, 1.3),
            battery_life: scale(self.battery_life, 0.7, 1.5),
            // Carry and rite by one factor: a laying never lowers pressure.
            pressure_carry: scale(self.pressure_carry, 1.3, 0.5),
            pressure_rite: scale(self.pressure_rite, 1.3, 0.5),
            light_lure_range: scale(self.light_lure_range, 1.2, 0.8),
            omen_quiet: (scale(self.omen_quiet.0, 0.7, 1.4), scale(self.omen_quiet.1, 0.7, 1.4)),
            check_zone: zone,
            // A wider zone must still fit on the track.
            check_zone_at: (self.check_zone_at.0, self.check_zone_at.1.min(0.98 - zone)),
            check_great: scale(self.check_great, 0.8, 1.3),
            // One more on a Hard night, one fewer on a Gentle one.
            averts_to_withdraw: if k > 0.0 {
                self.averts_to_withdraw + 1
            } else {
                self.averts_to_withdraw.saturating_sub(1).max(1)
            },
            stalk_speed: scale(self.stalk_speed, 1.08, 0.9),
            ..self
        }
    }

    /// The threat's numbers at a given night pressure (0 calm .. 1 finale).
    /// He is never faster than a walking player, but the night wears you down.
    pub fn at_pressure(&self, pressure: f32) -> Self {
        let p = pressure.clamp(0.0, 1.0);
        let lerp = |a: f32, b: f32| a + (b - a) * p;
        Self {
            stalk_speed: lerp(self.stalk_speed, self.stalk_speed * 1.45),
            creep_speed: lerp(self.creep_speed, self.creep_speed * 1.5),
            hunt_speed: lerp(self.hunt_speed, (self.hunt_speed * 1.35).min(self.walk_speed * 0.9)),
            warn_distance: lerp(self.warn_distance, self.warn_distance * 1.25),
            warn_time: lerp(self.warn_time, self.warn_time * 0.75),
            exposure_time: lerp(self.exposure_time, self.exposure_time * 0.7),
            recover_cooldown: lerp(self.recover_cooldown, self.recover_cooldown * 0.5),
            warn_recover_cooldown: lerp(self.warn_recover_cooldown, self.warn_recover_cooldown * 0.5),
            patience: lerp(self.patience, self.patience * 0.5),
            ..self.clone()
        }
    }

    /// La Rabia: his numbers once `stage` bundles are laid to rest (0..=5,
    /// public as the bones line). Each stage only crowds the night: denser
    /// whistles that fill the silence, sooner omens, a torch that draws him
    /// from farther, and tall grass that hides you only closer. None of it
    /// touches his speeds or his warning, and nothing here moves because a
    /// player fell.
    pub fn at_rage(&self, stage: u8) -> Self {
        let s = f32::from(stage.min(MAX_RAGE));
        let (lo, hi) = self.stalk_phrase_interval;
        let (qlo, qhi) = self.omen_quiet;
        let denser = 1.0 - 0.05 * s;
        let crowded = 1.0 - 0.08 * s;
        Self {
            stalk_phrase_interval: (lo * denser, hi * denser),
            stalk_silence_chance: (self.stalk_silence_chance - 0.04 * s).max(0.0),
            stalk_answer_chance: self.stalk_answer_chance + 0.03 * s,
            omen_quiet: (qlo * crowded, qhi * crowded),
            grass_sight: self.grass_sight + 0.8 * s,
            light_lure_range: self.light_lure_range + 4.0 * s,
            ..self.clone()
        }
    }

    /// How much farther he hears at this pressure.
    pub fn hearing_gain(&self, pressure: f32) -> f32 {
        1.0 + 0.6 * pressure.clamp(0.0, 1.0)
    }

    /// How loud a placed sound is `distance` metres away (0..1): full up
    /// close, inverse distance beyond, faded out towards `sound_far`,
    /// dulled when something stands between.
    pub fn heard(&self, distance: f32, occluded: bool) -> f32 {
        let d = distance.max(self.sound_near);
        let inverse = self.sound_near / d;
        let edge = (1.0 - (d - self.sound_near) / (self.sound_far - self.sound_near)).clamp(0.0, 1.0);
        let fade = edge * edge * (3.0 - 2.0 * edge);
        inverse * fade * if occluded { self.sound_occluded } else { 1.0 }
    }

    /// Seconds until a downed friend with `bleed` seconds left groans again:
    /// more often as the bleed runs out, varied by `jitter` (about 0.8..1.2)
    /// so no two keep time, and never outside `groan_every`.
    pub fn groan_gap(&self, bleed: f32, jitter: f32) -> f32 {
        let (last, full) = self.groan_every;
        let left = (bleed / self.bleed_out).clamp(0.0, 1.0);
        ((last + (full - last) * left) * jitter).clamp(last, full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placed_sounds_fade_with_distance_and_behind_walls() {
        let t = Tuning::default();
        assert_eq!(t.heard(0.0, false), 1.0);
        assert_eq!(t.heard(t.sound_near, false), 1.0);
        let mut prev = 1.0;
        for d in 1..200 {
            let g = t.heard(d as f32 * 0.5, false);
            assert!(g <= prev + 1e-6, "never louder farther away");
            assert!(t.heard(d as f32 * 0.5, true) < g || g == 0.0, "a wall always dulls it");
            prev = g;
        }
        assert_eq!(t.heard(t.sound_far, false), 0.0);
        assert!(t.heard(30.0, false) > 0.02, "a machine across the llano is still heard");
    }

    #[test]
    fn the_fallen_groan_more_often_as_they_bleed() {
        let t = Tuning::default();
        let mut before = f32::MAX;
        for left in (0..=(t.bleed_out as u32)).rev() {
            let gap = t.groan_gap(left as f32, 1.0);
            assert!(gap <= before, "{left} s left: a groan never comes later as they weaken");
            before = gap;
        }
        assert!(t.groan_gap(5.0, 1.0) < t.groan_gap(t.bleed_out - 5.0, 1.0));
        for jitter in [0.8, 1.0, 1.2] {
            for left in [0.0, t.bleed_after_sack, t.bleed_out, t.bleed_out * 2.0] {
                let gap = t.groan_gap(left, jitter);
                assert!(gap >= t.groan_every.0 && gap <= t.groan_every.1, "{gap} s");
            }
        }
    }

    #[test]
    fn every_night_keeps_him_slower_than_a_walker_and_his_warning_readable() {
        for night in [Night::Gentle, Night::Normal, Night::Hard] {
            let t = Tuning::default().with_night(night);
            assert_eq!(t.night, night);
            for i in 0..=20 {
                let scaled = t.at_pressure(i as f32 / 20.0);
                assert!(scaled.hunt_speed < t.walk_speed && scaled.stalk_speed < t.walk_speed);
                assert!(scaled.warn_time > 1.5);
            }
            assert!(t.check_zone > t.check_great && t.check_zone_at.1 + t.check_zone <= 1.0);
            // A miss always ends behind where its warning found the work,
            // stalls the hands, and is louder than the work it spoils.
            assert!(t.check_setback > t.check_warn + t.check_sweep + t.check_latency);
            assert!(t.check_stall > 0.0 && t.noise_miss_over > 1.0);
        }
        let (gentle, hard) = (
            Tuning::default().with_night(Night::Gentle),
            Tuning::default().with_night(Night::Hard),
        );
        assert!(hard.warn_distance > gentle.warn_distance && hard.battery_life < gentle.battery_life);
        // He waits over his prey longer on a harder night, never zero times.
        let normal = Tuning::default();
        assert!(hard.averts_to_withdraw > normal.averts_to_withdraw);
        assert!(normal.averts_to_withdraw > gentle.averts_to_withdraw && gentle.averts_to_withdraw >= 1);
        // Veiled, the whistle comes more often: the veil only ever shortens
        // the stalking gaps, and never below his quickest answer.
        assert!((0.0..1.0).contains(&normal.veil_silence));
        assert!(normal.veil_gap_max < normal.stalk_silence.0 && normal.veil_gap_max > normal.stalk_answer.1);
        assert_eq!(Tuning::default().with_night(Night::Normal), Tuning::default());
        assert_eq!(Night::parse("hard"), Some(Night::Hard));
        assert_eq!(Night::parse("brutal"), None);
    }

    #[test]
    fn pressure_never_makes_him_outrun_a_walking_player() {
        let t = Tuning::default();
        for i in 0..=20 {
            let scaled = t.at_pressure(i as f32 / 20.0);
            assert!(scaled.hunt_speed < t.walk_speed);
            assert!(scaled.stalk_speed < t.walk_speed);
            assert!(scaled.warn_time > 1.5, "warning must stay readable");
        }
        // The night only ever tightens the rules.
        let calm = t.at_pressure(0.0);
        let worst = t.at_pressure(1.0);
        assert!(worst.hunt_speed > calm.hunt_speed);
        assert!(worst.exposure_time < calm.exposure_time);
        assert!(worst.recover_cooldown < calm.recover_cooldown);
    }

    #[test]
    fn rage_only_tightens_and_never_outruns_a_walker() {
        for night in [Night::Gentle, Night::Normal, Night::Hard] {
            let t = Tuning::default().with_night(night);
            for i in 0..=20 {
                let p = i as f32 / 20.0;
                let calm = t.at_rage(0).at_pressure(p);
                let mut before = calm.clone();
                for stage in 0..=MAX_RAGE {
                    let r = t.at_rage(stage).at_pressure(p);
                    // Never faster, never a shorter warning: the stage is not a speed.
                    assert!(
                        r.hunt_speed < t.walk_speed && r.stalk_speed < t.walk_speed,
                        "{night:?} {stage} {p}"
                    );
                    assert!(r.warn_time > 1.5, "{night:?} {stage} {p}: warning must stay readable");
                    assert_eq!(
                        (r.hunt_speed, r.stalk_speed, r.creep_speed, r.warn_time, r.warn_distance),
                        (
                            calm.hunt_speed,
                            calm.stalk_speed,
                            calm.creep_speed,
                            calm.warn_time,
                            calm.warn_distance
                        ),
                        "rage leaves his pace and his warning alone"
                    );
                    // Each stage only crowds the night.
                    assert!(r.stalk_phrase_interval.0 <= before.stalk_phrase_interval.0);
                    assert!(r.stalk_phrase_interval.1 <= before.stalk_phrase_interval.1);
                    assert!(r.stalk_phrase_interval.0 > 0.0 && r.stalk_phrase_interval.0 < r.stalk_phrase_interval.1);
                    assert!(r.stalk_silence_chance <= before.stalk_silence_chance && r.stalk_silence_chance >= 0.0);
                    assert!(r.stalk_answer_chance >= before.stalk_answer_chance);
                    assert!(
                        r.stalk_silence_chance + r.stalk_answer_chance < 1.0,
                        "the usual gap stays usual"
                    );
                    assert!(r.omen_quiet.0 <= before.omen_quiet.0 && r.omen_quiet.1 <= before.omen_quiet.1);
                    assert!(r.omen_quiet.0 > 0.0);
                    assert!(r.grass_sight >= before.grass_sight && r.grass_sight < r.warn_distance * r.sight_crouch);
                    assert!(r.light_lure_range >= before.light_lure_range);
                    before = r;
                }
                assert!(before.light_lure_range > calm.light_lure_range && before.grass_sight > calm.grass_sight);
            }
            // Past the last bundle there is no more anger to find.
            assert_eq!(t.at_rage(MAX_RAGE + 3), t.at_rage(MAX_RAGE));
            assert_eq!(t.at_rage(0), t);
        }
        // The roadmap's marks on a Normal night.
        let t = Tuning::default();
        assert!((t.at_rage(2).light_lure_range - 53.0).abs() < 1e-4);
        assert!((t.at_rage(3).grass_sight - 7.4).abs() < 1e-4);
    }
}
