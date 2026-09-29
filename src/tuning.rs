//! Every gameplay number of the run, in one place.
//!
//! Pure data: no ECS, no randomness. The simulation, perception layer, body
//! model, session, debug route and tests all read the same [`Tuning`] so a
//! change here is a change everywhere. Distances are metres, times seconds,
//! speeds metres per second, angles radians.

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
    /// The beacon burns this long, then must cool down.
    pub beacon_burn: f32,
    pub beacon_cooldown: f32,
    /// Peppers a player can carry.
    pub aji_max: u8,
    pub aji_zone_radius: f32,
    pub aji_zone_life: f32,
    /// He stops to count the bones for this long.
    pub counting_time: f32,
    pub ping_cooldown: f32,
    pub ping_life: f32,
    pub ping_range: f32,

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
    /// Pressure per carried bundle, and relief per bundle laid to rest.
    pub pressure_carry: f32,
    pub pressure_relief: f32,
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
    /// Seconds between whistle phrases while stalking (min, max).
    pub stalk_phrase_interval: (f32, f32),
    /// Seconds between phrases during a warning.
    pub warn_phrase_interval: f32,
    /// Seconds between phrases during a hunt.
    pub hunt_phrase_interval: f32,
    /// Seconds from manifestation to the first phrase.
    pub first_phrase_delay: f32,
    /// Seconds between far-off whistles before he has manifested.
    pub prologue_phrase_interval: (f32, f32),
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
    /// Linear gain of the rain loop.
    pub rain_gain: f32,

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
            crouch_lower: 0.62,
            downed_lower: 1.28,
            walk_speed: 3.6,
            crouch_factor: 0.5,
            sprint_factor: 1.45,
            wade_factor: 0.55,
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
            deliver_hold: 1.6,
            pray_hold: 5.0,
            pump_hold: 10.0,
            truck_hold: 6.0,
            truck_warmup: 26.0,
            beacon_hold: 2.0,
            revive_hold: 4.0,
            bleed_out: 60.0,
            beacon_burn: 26.0,
            beacon_cooldown: 70.0,
            aji_max: 3,
            aji_zone_radius: 3.4,
            aji_zone_life: 24.0,
            counting_time: 7.5,
            ping_cooldown: 2.0,
            ping_life: 14.0,
            ping_range: 140.0,

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
            rain_mask: 0.32,
            thunder_mask: 0.3,
            cattle_radius: 10.0,
            cattle_alarm: 5.0,
            cattle_cooldown: 20.0,

            night_length: 840.0,
            pressure_base: 0.12,
            pressure_night: 0.5,
            pressure_carry: 0.07,
            pressure_relief: 0.04,
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
            susto_reset: 0.55,
            susto_stun: 1.1,
            team_radius: 9.0,

            cue_near_distance: 5.0,
            cue_far_distance: 42.0,
            stalk_phrase_interval: (7.0, 10.0),
            warn_phrase_interval: 3.2,
            hunt_phrase_interval: 3.8,
            first_phrase_delay: 1.2,
            prologue_phrase_interval: (28.0, 46.0),
            gain_loud: 0.5,
            gain_mid: 0.34,
            gain_faint: 0.22,

            ambience_gain: 0.32,
            ambience_hush: 0.25,
            sfx_gain: 0.55,
            rain_gain: 0.36,

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

    /// How much farther he hears at this pressure.
    pub fn hearing_gain(&self, pressure: f32) -> f32 {
        1.0 + 0.6 * pressure.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
