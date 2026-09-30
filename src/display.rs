//! The picture in numbers: what the Brightness and Contrast settings do to
//! the night's grade. Pure and headless; `player` applies it to the
//! camera's `ColorGrading`, and the calibration page shows its hats
//! through the same grade.
//!
//! Contrast turns every level about the night's fog (`pivot`) in log
//! terms: above it rise, below it fall, the fog stays. Brightness then
//! bends the result toward white, which holds: it lifts the darks by
//! stops, where an exposure offset could only scale every level alike.
//! Together that is one power and one gain, `x^n · 2^e`, which Bevy's
//! sectional CDL carries (the same `gamma` on every section) with the
//! global exposure. The sections' own `contrast` pivots at linear 0.5,
//! far above a night's levels, and crushes them: it is never used. The
//! camera's `Exposure` is not either: it scales lit surfaces but not the
//! sky, the fog or anything unlit. At the defaults the grade is exactly
//! the night as graded, every section at its default, so the renderer
//! never switches its sectional grading on.

/// The night's grade in stops (the look it was lit for).
pub const BASE_EXPOSURE: f32 = 0.3;
/// Brightness: its range and one press.
pub const BRIGHTNESS_RANGE: (f32, f32) = (-1.0, 1.0);
pub const BRIGHTNESS_STEP: f32 = 0.1;
/// Contrast: its range and one press. Kept narrow: the fog sits some
/// eight stops under white, so contrast swings the lamps hard.
pub const CONTRAST_RANGE: (f32, f32) = (0.75, 1.25);
pub const CONTRAST_STEP: f32 = 0.05;
/// How far full brightness bends the levels: their exponent is
/// `2^-BEND_STOPS` at +1 (about 2.5 stops more for him) and its inverse
/// at -1.
const BEND_STOPS: f32 = 0.5;
/// The calibration hats, as shares of the pivot (the fog's level): his
/// shadowed side (should vanish), him under the moon (barely visible),
/// the fog itself (plainly seen). Estimates from the night's lights;
/// check them against a capture of him.
pub const HATS: [f32; 3] = [0.1, 0.45, 1.0];

/// What the camera's grade is set to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    /// `ColorGradingGlobal::exposure`, in stops.
    pub exposure: f32,
    /// Every section's `gamma` (Bevy raises each level to `1 / gamma`).
    pub gamma: f32,
}

/// Rec. 709 luminance of a linear colour.
pub const fn luminance(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// The grade for these settings over a night whose fog sits at `pivot`
/// (linear luminance).
pub fn grade(brightness: f32, contrast: f32, pivot: f32) -> Grade {
    let b = brightness.clamp(BRIGHTNESS_RANGE.0, BRIGHTNESS_RANGE.1);
    let c = contrast.clamp(CONTRAST_RANGE.0, CONTRAST_RANGE.1);
    let bend = (-BEND_STOPS * b).exp2();
    // Contrast about the pivot, then the bend toward white:
    // (p·(x/p)^c)^bend = p^(bend·(1-c)) · x^(bend·c).
    Grade {
        exposure: BASE_EXPOSURE + bend * (1.0 - c) * pivot.max(1e-6).log2(),
        gamma: 1.0 / (bend * c),
    }
}

/// What the grade does to a linear level before tonemapping (Bevy's CDL
/// with gain 1 and lift 0, then the exposure).
pub fn transfer(g: Grade, x: f32) -> f32 {
    x.max(0.0).powf(1.0 / g.gamma) * g.exposure.exp2()
}

/// A setting moved one press in `dir`, kept on its steps and in range.
pub fn step(value: f32, dir: i32, step: f32, (lo, hi): (f32, f32)) -> f32 {
    (((value / step).round() + dir as f32) * step).clamp(lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The night's fog, the pivot the game uses.
    const FOG: f32 = luminance(crate::world::land::HORIZON);

    fn stops(from: f32, to: f32) -> f32 {
        (to / from).log2()
    }

    /// Every brightness and contrast a player can reach.
    fn settings() -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for i in -10..=10 {
            for j in -5..=5 {
                out.push((i as f32 * BRIGHTNESS_STEP, 1.0 + j as f32 * CONTRAST_STEP));
            }
        }
        out
    }

    #[test]
    fn the_default_picture_is_the_night_as_graded() {
        // Exactly, so every section stays at its default and the renderer
        // never switches its sectional grading on.
        for pivot in [FOG, 0.001, 0.18] {
            assert_eq!(
                grade(0.0, 1.0, pivot),
                Grade {
                    exposure: BASE_EXPOSURE,
                    gamma: 1.0
                }
            );
        }
    }

    #[test]
    fn brightness_lifts_the_dark_by_stops_and_holds_white() {
        let him = HATS[1] * FOG;
        let base = grade(0.0, 1.0, FOG);
        for i in 1..=10 {
            let b = i as f32 * BRIGHTNESS_STEP;
            let up = grade(b, 1.0, FOG);
            let down = grade(-b, 1.0, FOG);
            let lift = |g: Grade, x: f32| stops(transfer(base, x), transfer(g, x));
            // The darker a level, the more it rises; a lamp-lit level less.
            assert!(lift(up, him) > lift(up, FOG) && lift(up, FOG) > lift(up, 0.1) && lift(up, 0.1) > 0.0);
            assert!(lift(down, him) < lift(down, 0.1) && lift(down, 0.1) < 0.0);
            // White stays white.
            for g in [up, down] {
                assert!(lift(g, 1.0).abs() < 1e-4, "white moved {} stops at {b}", lift(g, 1.0));
            }
        }
        // Full brightness lifts him out of the near-black by stops (an
        // exposure offset gave him 0.7 at most).
        let full = stops(transfer(base, him), transfer(grade(1.0, 1.0, FOG), him));
        assert!(full >= 2.0, "full brightness lifts him {full:.2} stops");
    }

    #[test]
    fn contrast_turns_about_the_fog_at_any_brightness() {
        for (b, c) in settings() {
            let g = grade(b, c, FOG);
            let flat = grade(b, 1.0, FOG);
            // The fog holds wherever brightness put it.
            let held = stops(transfer(flat, FOG), transfer(g, FOG));
            assert!(held.abs() < 1e-3, "fog moved {held} stops at ({b}, {c})");
            // Above it rises with contrast, below it falls.
            for x in [HATS[0] * FOG, HATS[1] * FOG, 0.1, 1.0] {
                let moved = stops(transfer(flat, x), transfer(g, x));
                let want = (c - 1.0) * (x - FOG).signum();
                assert!(moved * want > 0.0 || c == 1.0, "{x} moved {moved} stops at ({b}, {c})");
            }
            // More contrast takes the shadowed hat down faster than him, so
            // it can vanish while he stays.
            if c > 1.0 {
                let fall = |x: f32| stops(transfer(g, x), transfer(flat, x));
                assert!(fall(HATS[0] * FOG) > fall(HATS[1] * FOG));
            }
        }
    }

    #[test]
    fn the_picture_never_folds() {
        // Any setting keeps every level in order, so the hats keep theirs.
        for (b, c) in settings() {
            let g = grade(b, c, FOG);
            let mut last = 0.0;
            let mut x = 1e-5_f32;
            while x < 100.0 {
                let y = transfer(g, x);
                assert!(y.is_finite() && y > last, "folds at {x} for ({b}, {c})");
                last = y;
                x *= 1.25;
            }
        }
    }

    #[test]
    fn settings_out_of_range_are_held_at_the_ends() {
        assert_eq!(grade(5.0, 9.0, FOG), grade(1.0, CONTRAST_RANGE.1, FOG));
        assert_eq!(grade(-5.0, -9.0, FOG), grade(-1.0, CONTRAST_RANGE.0, FOG));
    }

    #[test]
    fn a_setting_steps_evenly_and_comes_back_where_it_was() {
        for (default, size, range) in [
            (0.0, BRIGHTNESS_STEP, BRIGHTNESS_RANGE),
            (1.0, CONTRAST_STEP, CONTRAST_RANGE),
        ] {
            let mut v = default;
            for _ in 0..3 {
                v = step(v, 1, size, range);
            }
            assert!((v - (default + 3.0 * size)).abs() < 1e-6);
            for _ in 0..6 {
                v = step(v, -1, size, range);
            }
            for _ in 0..3 {
                v = step(v, 1, size, range);
            }
            // Exactly, so the default grade (and its renderer path) returns.
            assert_eq!(v, default);
            // Held at the ends, and put back on the steps.
            assert_eq!(step(range.1, 1, size, range), range.1);
            assert_eq!(step(range.0, -1, size, range), range.0);
            assert!((step(default + size * 0.4, 1, size, range) - (default + size)).abs() < 1e-6);
        }
    }
}
