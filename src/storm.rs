//! The storm: rain swells, lightning and the thunder that follows it, as pure
//! functions of `(seed, seconds into the run)`.
//!
//! The host uses it to mask noise (rain and thunder swallow footsteps); every
//! peer draws the same flashes and plays the same thunder from the run clock
//! in the snapshot, so nothing about the weather is sent over the wire.

use crate::tuning::Tuning;

/// One lightning opportunity per slot.
const SLOT: f32 = 26.0;
/// How long a thunder roll lasts.
const ROLL: f32 = 3.6;
/// How long one strike's flash can last: a 0.23 s gap to the second pulse plus
/// that pulse's 0.6 s.
const FLASH_SPAN: f32 = 0.85;

fn unit(seed: u64, slot: i64, salt: u64) -> f32 {
    let mut z = seed
        .wrapping_add((slot as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// One lightning strike and its thunder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    /// Run time of the flash.
    pub at: f32,
    /// Run time the thunder reaches the players (sound is slower than light).
    pub thunder_at: f32,
    /// 0.5..1: how violent it is.
    pub power: f32,
    /// A second, weaker flash follows the first.
    pub double: bool,
}

pub fn strike(seed: u64, slot: i64) -> Option<Strike> {
    if unit(seed, slot, 1) < 0.14 {
        return None;
    }
    let at = slot as f32 * SLOT + 3.0 + unit(seed, slot, 2) * (SLOT - 9.0);
    Some(Strike {
        at,
        thunder_at: at + 0.6 + unit(seed, slot, 3) * 4.6,
        power: 0.5 + 0.5 * unit(seed, slot, 4),
        double: unit(seed, slot, 5) > 0.45,
    })
}

fn strikes_near(seed: u64, t: f32) -> impl Iterator<Item = Strike> {
    let slot = (t / SLOT).floor() as i64;
    (slot - 1..=slot).filter_map(move |s| strike(seed, s))
}

/// Lightning brightness 0..1 at run time `t` (a sharp flash, a flicker, and
/// sometimes a weaker second strike).
pub fn flash(seed: u64, t: f32) -> f32 {
    let mut level = 0.0_f32;
    for s in strikes_near(seed, t) {
        let pulse = |dt: f32, amp: f32| {
            if !(0.0..0.6).contains(&dt) {
                return 0.0;
            }
            let core = (-(dt / 0.07).powi(2)).exp();
            let flicker = 0.35 * (dt * 55.0).sin().max(0.0) * (-dt * 5.0).exp();
            amp * (core + flicker).min(1.0)
        };
        level = level.max(pulse(t - s.at, s.power));
        if s.double {
            level = level.max(pulse(t - s.at - 0.23, s.power * 0.6));
        }
    }
    level
}

/// Thunder loudness 0..1 at run time `t`: a quick attack and a long rumble.
pub fn thunder(seed: u64, t: f32) -> f32 {
    let mut level = 0.0_f32;
    for s in strikes_near(seed, t + ROLL) {
        let dt = t - s.thunder_at;
        if (0.0..ROLL).contains(&dt) {
            let attack = (dt / 0.18).min(1.0);
            let decay = (1.0 - dt / ROLL).powf(1.6);
            level = level.max(s.power * attack * decay);
        }
    }
    level
}

/// The thunder clap that starts in `(t0, t1]`, if any: its power.
pub fn thunder_onset(seed: u64, t0: f32, t1: f32) -> Option<f32> {
    strikes_near(seed, t1)
        .chain(strikes_near(seed, t0))
        .find(|s| s.thunder_at > t0 && s.thunder_at <= t1)
        .map(|s| s.power)
}

/// The lightning that starts in `(t0, t1]`, if any: its power.
pub fn flash_onset(seed: u64, t0: f32, t1: f32) -> Option<f32> {
    strikes_near(seed, t1)
        .chain(strikes_near(seed, t0))
        .find(|s| s.at > t0 && s.at <= t1)
        .map(|s| s.power)
}

/// The strike whose flash is on screen at `t` (its second pulse included), so
/// anything drawn for that strike can be seeded by its exact `at`.
pub fn active_strike(seed: u64, t: f32) -> Option<Strike> {
    strikes_near(seed, t).find(|s| t >= s.at && t < s.at + FLASH_SPAN)
}

/// Rain intensity 0.6..1: it never really stops, it only swells and eases.
pub fn rain(seed: u64, t: f32) -> f32 {
    let phase = (seed % 97) as f32;
    let slow = (t * 0.021 + phase).sin() * 0.5 + 0.5;
    let gusts = (t * 0.113 + phase * 1.7).sin() * 0.5 + 0.5;
    0.6 + 0.3 * slow + 0.1 * gusts
}

/// Multiplier applied to every noise radius: heavy rain and thunder hide
/// footsteps. 1 = no masking.
pub fn masking(tuning: &Tuning, t: f32) -> f32 {
    let seed = tuning.seed;
    (1.0 - tuning.rain_mask * rain(seed, t) - tuning.thunder_mask * thunder(seed, t)).clamp(0.3, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storm_is_deterministic_and_keeps_its_ranges() {
        let t = Tuning::default();
        for i in 0..4000 {
            let time = i as f32 * 0.25;
            assert_eq!(flash(7, time), flash(7, time));
            assert!((0.0..=1.0).contains(&flash(7, time)));
            assert!((0.0..=1.0).contains(&thunder(7, time)));
            assert!((0.6..=1.0).contains(&rain(7, time)));
            let m = masking(&t, time);
            assert!((0.3..=1.0).contains(&m));
        }
    }

    #[test]
    fn any_visible_flash_belongs_to_an_active_strike() {
        for seed in [3_u64, 7, 4242] {
            let mut time = 0.0;
            while time < 900.0 {
                let strike = active_strike(seed, time);
                if flash(seed, time) > 0.0 {
                    let s = strike.expect("a flash with no strike to draw a bolt for");
                    assert!(time >= s.at && time < s.at + FLASH_SPAN);
                }
                time += 1.0 / 60.0;
            }
        }
    }

    #[test]
    fn every_strike_flashes_before_its_thunder_and_thunder_masks_noise() {
        let t = Tuning::default();
        let mut claps = 0;
        let mut prev = 0.0;
        let mut time = 0.0;
        while time < 600.0 {
            time += 1.0 / 30.0;
            if let Some(power) = thunder_onset(t.seed, prev, time) {
                claps += 1;
                assert!(power >= 0.5);
                // Sound lags light: the flash of the same strike is earlier.
                let s = strikes_near(t.seed, time)
                    .find(|s| s.thunder_at > prev && s.thunder_at <= time)
                    .expect("onset belongs to a strike");
                assert!(s.at < s.thunder_at);
                assert!(flash(t.seed, s.at + 0.02) > 0.3, "no flash at the strike");
                // While it rolls, noise is masked harder than in plain rain.
                let rolling = masking(&t, s.thunder_at + 0.4);
                let plain = 1.0 - t.rain_mask * rain(t.seed, s.thunder_at + 0.4);
                assert!(rolling < plain);
            }
            prev = time;
        }
        // 600 s at one opportunity per 26 s, 86% taken: about twenty claps.
        assert!((15..=25).contains(&claps), "unexpected thunder count {claps}");
    }
}
