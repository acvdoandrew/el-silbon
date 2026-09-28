//! The perception layer: what a listener HEARS, derived from hidden truth.
//!
//! Folk rule of the encounter: when the whistle sounds close, he is far;
//! when it sounds faint and far away, he is near. This module is the only
//! place where the Silbón's true distance is turned into a cue, and it does
//! so inverted. Audio playback and captions consume [`WhistlePhrase`] only —
//! they never see a position or a distance, and nothing is spatialized.

use bevy::math::Vec2;

use crate::rng::Rng;
use crate::sim::{Encounter, Objective, Presence, ThreatState};
use crate::tuning::Tuning;

/// The three recorded timbres of the same phrase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WhistleVariant {
    /// Full, dry, breathy: seems right beside you.
    Loud,
    /// Somewhere across the grass.
    Middling,
    /// Thin and reverberant: seems far, far away.
    Faint,
}

impl WhistleVariant {
    /// Caption text: describes the impression only, never the truth.
    pub fn caption(self) -> &'static str {
        match self {
            WhistleVariant::Loud => "A whistle — loud, as if right beside you.",
            WhistleVariant::Middling => "A whistle — somewhere out across the grass.",
            WhistleVariant::Faint => "A whistle — thin and faint, far, far away…",
        }
    }

    pub fn gain(self, tuning: &Tuning) -> f32 {
        match self {
            WhistleVariant::Loud => tuning.gain_loud,
            WhistleVariant::Middling => tuning.gain_mid,
            WhistleVariant::Faint => tuning.gain_faint,
        }
    }
}

/// One whistle phrase to play, non-spatially, at `gain` and `speed`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WhistlePhrase {
    pub variant: WhistleVariant,
    pub gain: f32,
    pub speed: f32,
    /// How close it SEEMS, 0 = faint and far, 1 = right beside you.
    pub seeming_closeness: f32,
}

/// Inversion: truly near → seems far (0), truly far → seems close (1).
pub fn seeming_closeness(true_distance: f32, tuning: &Tuning) -> f32 {
    let span = (tuning.cue_far_distance - tuning.cue_near_distance).max(1e-3);
    ((true_distance - tuning.cue_near_distance) / span).clamp(0.0, 1.0)
}

pub fn variant_for(seeming: f32) -> WhistleVariant {
    if seeming >= 0.62 {
        WhistleVariant::Loud
    } else if seeming >= 0.3 {
        WhistleVariant::Middling
    } else {
        WhistleVariant::Faint
    }
}

/// Schedules phrases for one listener. Owns no truth; reads it each tick.
#[derive(Clone, Debug, PartialEq)]
pub struct CueDirector {
    countdown: f32,
    rng: Rng,
    last_state: ThreatState,
    pub last_phrase: Option<WhistlePhrase>,
    pub phrases: u32,
}

impl CueDirector {
    pub fn new(seed: u64) -> Self {
        Self {
            countdown: 0.0,
            rng: Rng::fork(seed, 0xC0E),
            last_state: ThreatState::Dormant,
            last_phrase: None,
            phrases: 0,
        }
    }

    pub fn reset(&mut self, seed: u64) {
        *self = Self::new(seed);
    }

    /// Advance by `dt`; returns a phrase when one should start now.
    pub fn tick(&mut self, dt: f32, enc: &Encounter, listener: Vec2, tuning: &Tuning) -> Option<WhistlePhrase> {
        let th = &enc.threat;
        if th.state != self.last_state {
            match th.state {
                ThreatState::Stalking if self.last_state == ThreatState::Dormant => {
                    self.countdown = tuning.first_phrase_delay;
                }
                ThreatState::Warning | ThreatState::Hunting => {
                    self.countdown = self.countdown.min(0.35);
                }
                _ => {}
            }
            self.last_state = th.state;
        }
        let active_state = matches!(
            th.state,
            ThreatState::Stalking | ThreatState::Warning | ThreatState::Hunting
        );
        let present = matches!(th.presence, Presence::Present | Presence::Rising { .. });
        if !active_state || !present || enc.objective == Objective::Failed {
            return None;
        }
        self.countdown -= dt;
        if self.countdown > 0.0 {
            return None;
        }
        self.countdown = match th.state {
            ThreatState::Warning => tuning.warn_phrase_interval,
            ThreatState::Hunting => tuning.hunt_phrase_interval,
            _ => {
                let (lo, hi) = tuning.stalk_phrase_interval;
                self.rng.range(lo, hi)
            }
        };
        let seeming = seeming_closeness(th.pos.distance(listener), tuning);
        let variant = variant_for(seeming);
        let phrase = WhistlePhrase {
            variant,
            gain: variant.gain(tuning),
            speed: 0.97 + 0.06 * self.rng.f32(),
            seeming_closeness: seeming,
        };
        self.last_phrase = Some(phrase);
        self.phrases += 1;
        Some(phrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Layout;
    use crate::sim::Movement;

    #[test]
    fn whistle_perception_is_inverted_from_true_distance() {
        let t = Tuning::default();
        // Truly close: seems faint and far away.
        assert_eq!(seeming_closeness(2.0, &t), 0.0);
        assert_eq!(variant_for(seeming_closeness(6.0, &t)), WhistleVariant::Faint);
        // Truly far: seems loud, right beside you.
        assert_eq!(seeming_closeness(60.0, &t), 1.0);
        assert_eq!(variant_for(seeming_closeness(45.0, &t)), WhistleVariant::Loud);
        // Monotonic: the farther he truly is, the closer he seems.
        let mut prev = -1.0;
        for d in 0..80 {
            let s = seeming_closeness(d as f32, &t);
            assert!(s >= prev);
            prev = s;
        }
        // And the loud variant is louder than the faint one.
        assert!(WhistleVariant::Loud.gain(&t) > WhistleVariant::Faint.gain(&t));
    }

    #[test]
    fn director_emits_inverted_phrases_only_while_he_is_present() {
        let layout = Layout::authored();
        let t = Tuning::default();
        let mut enc = Encounter::new(&layout);
        let mut cue = CueDirector::new(t.seed);
        let listener = Vec2::new(0.0, 9.0);
        // Dormant: silence.
        for _ in 0..600 {
            assert!(cue.tick(1.0 / 60.0, &enc, listener, &t).is_none());
        }
        // Present and far away: the phrase seems loud.
        enc.objective = Objective::ReturnBones;
        enc.threat.state = ThreatState::Stalking;
        enc.threat.presence = Presence::Present;
        enc.threat.movement = Movement::Still;
        enc.threat.pos = Vec2::new(0.0, -40.0);
        let mut far = None;
        for _ in 0..600 {
            if let Some(p) = cue.tick(1.0 / 60.0, &enc, listener, &t) {
                far = Some(p);
                break;
            }
        }
        assert_eq!(far.expect("no phrase while stalking").variant, WhistleVariant::Loud);
        // Warning, truly close: the next phrase comes quickly and seems faint.
        enc.threat.state = ThreatState::Warning;
        enc.threat.pos = Vec2::new(0.0, 4.0);
        let mut near = None;
        for _ in 0..60 {
            if let Some(p) = cue.tick(1.0 / 60.0, &enc, listener, &t) {
                near = Some(p);
                break;
            }
        }
        let near = near.expect("warning must whistle within a second");
        assert_eq!(near.variant, WhistleVariant::Faint);
        assert!(near.gain < far.unwrap().gain);
        // Resolved: silence again.
        enc.threat.state = ThreatState::Resolved;
        for _ in 0..1200 {
            assert!(cue.tick(1.0 / 60.0, &enc, listener, &t).is_none());
        }
    }
}
