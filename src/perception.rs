//! The perception layer: what a listener HEARS, derived from hidden truth.
//!
//! Folk rule of the encounter: when the whistle sounds close, he is far;
//! when it sounds faint and far away, he is near. This module is the only
//! place where the Silbón's true distance is turned into a cue, and it does
//! so inverted. Audio playback and captions consume [`WhistlePhrase`] only —
//! they never see a position or a distance, and nothing is spatialized.

use bevy::math::Vec2;

use crate::rng::Rng;
use crate::sim::{Encounter, Presence, ThreatState};
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

    /// How much a listener is frightened by hearing it. Deliberately the
    /// inverse of comfort: thin and far away means he is near.
    pub fn dread(self, tuning: &Tuning) -> f32 {
        match self {
            WhistleVariant::Loud => 0.0,
            WhistleVariant::Middling => tuning.fear_faint * 0.4,
            WhistleVariant::Faint => tuning.fear_faint,
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
    /// Seconds until the next far-off whistle while he is still dormant.
    prologue: f32,
    rng: Rng,
    last_state: ThreatState,
    pub last_phrase: Option<WhistlePhrase>,
    pub phrases: u32,
}

impl CueDirector {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::fork(seed, 0xC0E);
        let t = Tuning::default();
        let prologue = rng.range(t.prologue_phrase_interval.0, t.prologue_phrase_interval.1) * 0.6;
        Self {
            countdown: 0.0,
            prologue,
            rng,
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
        if enc.outcome.is_over() {
            return None;
        }
        // Before the first bundle is touched he is only a rumour: a whistle
        // now and then from far off. He is truly far, so it seems close.
        if th.state == ThreatState::Dormant {
            self.prologue -= dt;
            if self.prologue > 0.0 {
                return None;
            }
            self.prologue = self
                .rng
                .range(tuning.prologue_phrase_interval.0, tuning.prologue_phrase_interval.1);
            let variant = if self.rng.f32() < 0.6 {
                WhistleVariant::Loud
            } else {
                WhistleVariant::Middling
            };
            let phrase = WhistlePhrase {
                variant,
                gain: variant.gain(tuning) * 0.85,
                speed: 0.97 + 0.06 * self.rng.f32(),
                seeming_closeness: if variant == WhistleVariant::Loud { 1.0 } else { 0.5 },
            };
            self.last_phrase = Some(phrase);
            self.phrases += 1;
            return Some(phrase);
        }
        let active_state = matches!(
            th.state,
            ThreatState::Stalking | ThreatState::Warning | ThreatState::Hunting
        );
        let present = matches!(th.presence, Presence::Present | Presence::Rising { .. });
        if !active_state || !present {
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
        // The loud variant is louder than the faint one, and the faint one is
        // the frightening one.
        assert!(WhistleVariant::Loud.gain(&t) > WhistleVariant::Faint.gain(&t));
        assert!(WhistleVariant::Faint.dread(&t) > WhistleVariant::Middling.dread(&t));
        assert_eq!(WhistleVariant::Loud.dread(&t), 0.0);
    }

    #[test]
    fn director_whistles_far_off_before_he_wakes_then_inverted_phrases_while_present() {
        let layout = Layout::new();
        let t = Tuning::default();
        let mut enc = Encounter::new(&layout);
        let mut cue = CueDirector::new(t.seed);
        let listener = Vec2::new(0.0, 9.0);
        // Dormant: only rare far-off whistles, never a close-sounding warning.
        let mut prologue = Vec::new();
        for _ in 0..(150 * 60) {
            if let Some(p) = cue.tick(1.0 / 60.0, &enc, listener, &t) {
                prologue.push(p);
            }
        }
        assert!(
            (2..=6).contains(&prologue.len()),
            "expected a few, got {}",
            prologue.len()
        );
        assert!(prologue.iter().all(|p| p.variant != WhistleVariant::Faint));
        // Present and far away: the phrase seems loud.
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
        // Counting bones: silence.
        enc.threat.state = ThreatState::Counting;
        for _ in 0..1200 {
            assert!(cue.tick(1.0 / 60.0, &enc, listener, &t).is_none());
        }
        // After the run ends: silence.
        enc.threat.state = ThreatState::Stalking;
        enc.outcome = crate::sim::Outcome::Won;
        for _ in 0..1200 {
            assert!(cue.tick(1.0 / 60.0, &enc, listener, &t).is_none());
        }
    }
}
