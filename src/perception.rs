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

/// Performances of the phrase recorded at each distance
/// (`whistle_{loud,mid,faint}_{take}.wav`, see `tools/gen_audio.py`).
pub const WHISTLE_TAKES: u8 = 4;

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
    /// Heard only in the listener's fear: there was no whistle at all.
    pub phantom: bool,
    /// Which performance (`0..WHISTLE_TAKES`): how he whistles it this time,
    /// never where he is.
    pub take: u8,
}

/// Inversion: truly near → seems far (0), truly far → seems close (1).
pub fn seeming_closeness(true_distance: f32, tuning: &Tuning) -> f32 {
    let span = (tuning.cue_far_distance - tuning.cue_near_distance).max(1e-3);
    ((true_distance - tuning.cue_near_distance) / span).clamp(0.0, 1.0)
}

pub fn variant_for(seeming: f32, tuning: &Tuning) -> WhistleVariant {
    if seeming >= tuning.cue_loud_above {
        WhistleVariant::Loud
    } else if seeming >= tuning.cue_mid_above {
        WhistleVariant::Middling
    } else {
        WhistleVariant::Faint
    }
}

/// What Tureco makes of him. A dog knows where he truly is: the one cue on
/// the llano that does not lie, and it only works up close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DogSense {
    Calm,
    /// He is near: a low growl at the dark.
    Growl,
    /// He is right there: the dog barks him off, if it has the courage.
    Bark,
}

pub fn dog_senses(true_distance: f32, tuning: &Tuning) -> DogSense {
    if true_distance <= tuning.bark_range {
        DogSense::Bark
    } else if true_distance <= tuning.growl_range {
        DogSense::Growl
    } else {
        DogSense::Calm
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
    /// Seconds since the last phrase that was only in the listener's head.
    phantom_quiet: f32,
    last_take: Option<u8>,
    /// El Velo: nobody can see him, so his whistle keeps closer company
    /// (still inverted, still never placed).
    veiled: bool,
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
            phantom_quiet: 0.0,
            last_take: None,
            veiled: false,
        }
    }

    /// El Velo falls or lifts (the session says so each tick). As it falls,
    /// a long silence already under way is cut to `veil_gap_max`: while
    /// nobody can see him, the whistle is all anyone has of him.
    pub fn veil(&mut self, veiled: bool, tuning: &Tuning) {
        if veiled && !self.veiled {
            self.countdown = self.countdown.min(tuning.veil_gap_max);
        }
        self.veiled = veiled;
    }

    /// A performance other than the last one: he never whistles it the same
    /// way twice running.
    fn take(&mut self) -> u8 {
        let take = match self.last_take {
            None => self.rng.below(WHISTLE_TAKES as usize) as u8,
            Some(last) => {
                let t = self.rng.below(WHISTLE_TAKES as usize - 1) as u8;
                if t >= last { t + 1 } else { t }
            }
        };
        self.last_take = Some(take);
        take
    }

    fn speed(&mut self, tuning: &Tuning) -> f32 {
        let (lo, hi) = tuning.whistle_speed;
        self.rng.range(lo, hi)
    }

    /// Seconds until the next phrase while he stalks: no steady rhythm to
    /// settle into. Usually a while; sometimes he answers himself almost at
    /// once; sometimes the llano goes quiet for a long time. The more of his
    /// bones are taken, the sooner (up to a fifth) — except an answer. Each
    /// bundle laid to rest crowds him further (`Tuning::at_rage`): sooner,
    /// answered more, and the long silences grow rare. Veiled, the long
    /// silences are rarer still and no gap outlasts `veil_gap_max`.
    fn stalk_gap(&mut self, tuning: &Tuning, pressure: f32, rage: u8) -> f32 {
        let raged = tuning.at_rage(rage);
        let tuning = &raged;
        let silence = if self.veiled {
            tuning.stalk_silence_chance * tuning.veil_silence
        } else {
            tuning.stalk_silence_chance
        };
        let roll = self.rng.f32();
        let sooner = 1.0 - 0.2 * pressure.clamp(0.0, 1.0);
        let gap = if roll < tuning.stalk_answer_chance {
            self.rng.range(tuning.stalk_answer.0, tuning.stalk_answer.1)
        } else if roll < tuning.stalk_answer_chance + silence {
            self.rng.range(tuning.stalk_silence.0, tuning.stalk_silence.1) * sooner
        } else {
            let (lo, hi) = tuning.stalk_phrase_interval;
            self.rng.range(lo, hi) * sooner
        };
        if self.veiled { gap.min(tuning.veil_gap_max) } else { gap }
    }

    /// A whistle only a badly frightened listener hears: any of the three
    /// timbres, slightly slurred. Rare, spaced, never below `phantom_whistle_fear`,
    /// and it says nothing about where he is.
    pub fn phantom(&mut self, fear: f32, tuning: &Tuning, dt: f32) -> Option<WhistlePhrase> {
        self.phantom_quiet += dt;
        if fear < tuning.phantom_whistle_fear || self.phantom_quiet < tuning.phantom_whistle_every * 0.5 {
            return None;
        }
        if self.rng.f32() >= dt / tuning.phantom_whistle_every {
            return None;
        }
        self.phantom_quiet = 0.0;
        let variant = match self.rng.below(3) {
            0 => WhistleVariant::Loud,
            1 => WhistleVariant::Middling,
            _ => WhistleVariant::Faint,
        };
        Some(WhistlePhrase {
            variant,
            gain: variant.gain(tuning) * 0.7,
            speed: 0.9 + 0.05 * self.rng.f32(),
            seeming_closeness: 0.5,
            phantom: true,
            take: self.take(),
        })
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
                speed: self.speed(tuning),
                seeming_closeness: if variant == WhistleVariant::Loud { 1.0 } else { 0.5 },
                phantom: false,
                take: self.take(),
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
        let jitter =
            |d: &mut Self, base: f32| base * d.rng.range(1.0 - tuning.phrase_jitter, 1.0 + tuning.phrase_jitter);
        self.countdown = match th.state {
            ThreatState::Warning => jitter(self, tuning.warn_phrase_interval),
            ThreatState::Hunting => jitter(self, tuning.hunt_phrase_interval),
            _ => self.stalk_gap(tuning, enc.pressure, enc.progress.rage()),
        };
        let seeming = seeming_closeness(th.pos.distance(listener), tuning);
        let variant = variant_for(seeming, tuning);
        // The Drunkard's return whistles slurred: slower, and never twice alike.
        let speed = if crate::sim::Variant::of(tuning.seed) == crate::sim::Variant::Borracho {
            self.speed(tuning) - 0.08 + 0.1 * self.rng.f32()
        } else {
            self.speed(tuning)
        };
        let phrase = WhistlePhrase {
            variant,
            gain: variant.gain(tuning),
            speed,
            seeming_closeness: seeming,
            phantom: false,
            take: self.take(),
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
    fn only_deep_fear_hears_whistles_that_are_not_there_and_rarely() {
        let t = Tuning::default();
        let count = |fear: f32| {
            let mut d = CueDirector::new(21);
            (0..(600.0 / 0.05) as usize)
                .filter_map(|_| d.phantom(fear, &t, 0.05))
                .inspect(|p| assert!(p.phantom, "an imagined whistle is marked as one"))
                .count()
        };
        assert_eq!(count(0.3), 0, "a calm mind hears nothing that is not there");
        let frightened = count(0.95);
        assert!(
            (5..=40).contains(&frightened),
            "ten minutes of terror: {frightened} imagined whistles"
        );
    }

    #[test]
    fn whistle_perception_is_inverted_from_true_distance() {
        let t = Tuning::default();
        // Truly close: seems faint and far away.
        assert_eq!(seeming_closeness(2.0, &t), 0.0);
        assert_eq!(variant_for(seeming_closeness(6.0, &t), &t), WhistleVariant::Faint);
        // Truly far: seems loud, right beside you.
        assert_eq!(seeming_closeness(60.0, &t), 1.0);
        assert_eq!(variant_for(seeming_closeness(45.0, &t), &t), WhistleVariant::Loud);
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

    #[test]
    fn a_stalking_whistle_keeps_no_steady_rhythm_and_is_never_performed_twice_alike() {
        let layout = Layout::new();
        let t = Tuning::default();
        let mut enc = Encounter::new(&layout);
        enc.threat.state = ThreatState::Stalking;
        enc.threat.presence = Presence::Present;
        enc.threat.pos = Vec2::new(0.0, -20.0);
        let mut cue = CueDirector::new(5);
        let listener = Vec2::ZERO;
        let dt = 1.0 / 60.0;
        let mut heard = Vec::new();
        for i in 0..(30 * 60 * 60) {
            if let Some(p) = cue.tick(dt, &enc, listener, &t) {
                heard.push((i as f32 * dt, p));
            }
        }
        assert!(heard.len() > 60, "half an hour of stalking: {} phrases", heard.len());
        let gaps: Vec<f32> = heard.windows(2).map(|w| w[1].0 - w[0].0).collect();
        // Sometimes he answers himself at once; sometimes he falls quiet.
        assert!(gaps.iter().any(|&g| g < t.stalk_answer.1 + 0.1), "no quick answers");
        assert!(
            gaps.iter().any(|&g| g > t.stalk_phrase_interval.1 + 1.0),
            "no long silences"
        );
        // Every performance comes up, and never the same one twice running.
        for w in heard.windows(2) {
            assert_ne!(w[0].1.take, w[1].1.take);
        }
        for take in 0..WHISTLE_TAKES {
            assert!(heard.iter().any(|(_, p)| p.take == take), "take {take} never heard");
        }
        // Nor at one speed (which is also its pitch).
        let speeds = heard.iter().map(|(_, p)| p.speed);
        let (lo, hi) = speeds.fold((f32::MAX, f32::MIN), |(lo, hi), s| (lo.min(s), hi.max(s)));
        assert!(hi - lo > 0.08, "speeds {lo}..{hi}");
        // None of it tells where he is: the same distance, the same variant.
        assert!(heard.iter().all(|(_, p)| p.variant == heard[0].1.variant));
    }

    #[test]
    fn his_anger_crowds_the_stalking_whistle_but_keeps_no_steady_rhythm() {
        let layout = Layout::new();
        let t = Tuning::default();
        let dt = 1.0 / 60.0;
        let gaps = |stage: usize| {
            let mut enc = Encounter::new(&layout);
            for r in enc.progress.relics.iter_mut().take(stage) {
                *r = crate::sim::Relic::Delivered;
            }
            enc.threat.state = ThreatState::Stalking;
            enc.threat.presence = Presence::Present;
            enc.threat.pos = Vec2::new(0.0, -20.0);
            let mut cue = CueDirector::new(5);
            let (mut last, mut gaps) = (None, Vec::new());
            for i in 0..(30 * 60 * 60) {
                if cue.tick(dt, &enc, Vec2::ZERO, &t).is_some() {
                    let now = i as f32 * dt;
                    if let Some(before) = last {
                        gaps.push(now - before);
                    }
                    last = Some(now);
                }
            }
            gaps
        };
        let mean = |g: &[f32]| g.iter().sum::<f32>() / g.len() as f32;
        let (calm, angry) = (gaps(0), gaps(5));
        assert!(
            mean(&angry) < mean(&calm) * 0.9,
            "every bundle laid crowds him: {:.1} s against {:.1} s",
            mean(&angry),
            mean(&calm)
        );
        // Still no rhythm to settle into: quick answers and long gaps both.
        let (lo, hi) = angry
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &g| (lo.min(g), hi.max(g)));
        assert!(lo < t.stalk_answer.1 + 0.1, "no quick answers: {lo}");
        assert!(hi > lo * 2.5, "a steady rhythm: {lo}..{hi}");
    }

    /// El Velo: while nobody can see him his whistle keeps closer company.
    /// No gap outlasts `veil_gap_max`, a silence under way is cut short as
    /// the veil falls, and the whistle still keeps no rhythm and still says
    /// only what the inversion says.
    #[test]
    fn veiled_he_is_never_silent_for_long_and_still_keeps_no_rhythm() {
        let layout = Layout::new();
        let t = Tuning::default();
        let dt = 1.0 / 60.0;
        let mut enc = Encounter::new(&layout);
        enc.threat.state = ThreatState::Stalking;
        enc.threat.presence = Presence::Present;
        enc.threat.pos = Vec2::new(0.0, -20.0);
        let run = |veiled: bool| {
            let mut cue = CueDirector::new(5);
            let (mut last, mut gaps, mut heard) = (None, Vec::new(), Vec::new());
            for i in 0..(30 * 60 * 60) {
                cue.veil(veiled, &t);
                if let Some(p) = cue.tick(dt, &enc, Vec2::ZERO, &t) {
                    let now = i as f32 * dt;
                    if let Some(before) = last {
                        gaps.push(now - before);
                    }
                    last = Some(now);
                    heard.push(p);
                }
            }
            (gaps, heard)
        };
        let (seen, _) = run(false);
        let (veiled, heard) = run(true);
        let most = |g: &[f32]| g.iter().copied().fold(f32::MIN, f32::max);
        let least = |g: &[f32]| g.iter().copied().fold(f32::MAX, f32::min);
        let mean = |g: &[f32]| g.iter().sum::<f32>() / g.len() as f32;
        assert!(
            most(&seen) > t.veil_gap_max,
            "seen, the llano still falls quiet for long"
        );
        assert!(
            most(&veiled) <= t.veil_gap_max + 2.0 * dt,
            "veiled, a silence of {:.1} s",
            most(&veiled)
        );
        assert!(mean(&veiled) < mean(&seen), "veiled, he whistles more often");
        assert!(least(&veiled) < t.stalk_answer.1 + 0.1, "no quick answers");
        assert!(most(&veiled) > least(&veiled) * 2.5, "a steady rhythm");
        // The veil changes nothing of what a phrase says: the same distance,
        // the same inverted timbre.
        let expected = variant_for(seeming_closeness(20.0, &t), &t);
        assert!(heard.iter().all(|p| p.variant == expected && !p.phantom));
        // A long silence under way when the veil falls is cut short.
        let mut cue = CueDirector::new(5);
        let mut waited = 0;
        while cue.countdown <= t.veil_gap_max + 1.0 {
            cue.tick(dt, &enc, Vec2::ZERO, &t);
            waited += 1;
            assert!(waited < 30 * 60 * 60, "no long silence in half an hour");
        }
        cue.veil(true, &t);
        let mut quiet = 0.0;
        while cue.tick(dt, &enc, Vec2::ZERO, &t).is_none() {
            quiet += dt;
            assert!(quiet <= t.veil_gap_max + dt, "the silence ran on under the veil");
        }
    }
}
