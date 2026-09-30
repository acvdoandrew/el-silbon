//! The mix in numbers: what a volume slider's position means in loudness,
//! how the master and a bus combine, and how a looped voice glides to its
//! level. Pure and headless; `audio` applies it to the sinks.
//!
//! A slider is even in decibels: 100% is 0 dB, each 5% step is 2 dB, the
//! first step above silence is -38 dB, and 0% is true silence. The whole
//! mix then sits `MIX_HEADROOM_DB` below full scale, so the default (80%)
//! is -12 dB. `tools/whistle_lab.py` mirrors these numbers (each constant
//! is a plain literal on its own line).

/// The slider's curve: a position `p` above 0 is `SLIDER_FLOOR_DB * (1 - p)`
/// decibels, rising evenly to 0 dB at 100% (0% itself is silence).
pub const SLIDER_FLOOR_DB: f32 = -40.0;
/// The whole mix sits this far below full scale.
pub const MIX_HEADROOM_DB: f32 = -4.0;
/// One press of a slider.
pub const SLIDER_STEP: f32 = 0.05;
/// The master slider on a fresh profile.
pub const DEFAULT_MASTER: f32 = 0.8;
/// How fast a looped voice glides to its level, per second.
pub const GLIDE_RATE: f32 = 5.0;

/// A glide lands on its target once this close (absolute, and as a share
/// of the target): far below hearing, so silence is reached exactly.
const SNAP_FLOOR: f32 = 1e-5;
const SNAP_SHARE: f32 = 0.01;

/// Where a voice's level comes from besides its own gain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    /// Master alone: what no other slider may hide.
    Master,
    Music,
    Ambience,
    Effects,
}

/// Decibels as a linear gain.
pub fn from_db(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// A slider's position (0..1) as a linear gain: even in decibels, silent at 0.
pub fn slider_gain(pos: f32) -> f32 {
    if pos <= 0.0 {
        0.0
    } else {
        from_db(SLIDER_FLOOR_DB * (1.0 - pos.min(1.0)))
    }
}

/// A slider moved one press in `dir`, kept on its steps and in range.
pub fn step(pos: f32, dir: i32) -> f32 {
    (((pos / SLIDER_STEP).round() + dir as f32) * SLIDER_STEP).clamp(0.0, 1.0)
}

/// The gain of a voice on a bus whose slider is at `bus`, under `master`
/// (a voice on the master alone passes `bus` 1).
pub fn gain(master: f32, bus: f32) -> f32 {
    from_db(MIX_HEADROOM_DB) * slider_gain(master) * slider_gain(bus)
}

/// One frame of a glide from `now` toward `target` at `rate` per second.
/// The same curve at any frame rate, never past the target, and landing
/// on it exactly.
pub fn glide(now: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let v = now + (target - now) * (1.0 - (-rate * dt.max(0.0)).exp());
    if (v - target).abs() <= SNAP_FLOOR + SNAP_SHARE * target.abs() {
        target
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(gain: f32) -> f32 {
        20.0 * gain.log10()
    }

    /// Every slider position a player can reach, from 0% up.
    fn positions() -> Vec<f32> {
        let mut at = vec![0.0];
        for _ in 0..40 {
            let next = step(*at.last().unwrap(), 1);
            if next == *at.last().unwrap() {
                break;
            }
            at.push(next);
        }
        at
    }

    #[test]
    fn a_slider_is_silent_at_zero_and_every_step_is_heard() {
        let at = positions();
        assert_eq!(slider_gain(0.0), 0.0, "0% is true silence");
        assert_eq!(slider_gain(*at.last().unwrap()), 1.0, "the top is unity");
        assert!(at.len() >= 20, "fine enough to set by ear: {} positions", at.len());
        for w in at.windows(2).skip(1) {
            let (a, b) = (slider_gain(w[0]), slider_gain(w[1]));
            let rise = db(b) - db(a);
            assert!(
                (1.5..=3.0).contains(&rise),
                "{:.0}% -> {:.0}% rises {rise:.2} dB",
                w[0] * 100.0,
                w[1] * 100.0
            );
        }
        // The lowest audible step is already quiet.
        assert!(db(slider_gain(at[1])) <= -30.0);
    }

    #[test]
    fn a_slider_stays_in_range_and_comes_back_where_it_was() {
        assert_eq!(step(0.0, -1), 0.0);
        assert_eq!(step(1.0, 1), 1.0);
        let mut p = DEFAULT_MASTER;
        for _ in 0..7 {
            p = step(p, -1);
        }
        for _ in 0..7 {
            p = step(p, 1);
        }
        assert_eq!(p, DEFAULT_MASTER);
    }

    #[test]
    fn the_default_mix_is_quieter_and_leaves_headroom() {
        let default = db(gain(DEFAULT_MASTER, 1.0));
        assert!((-14.0..=-10.0).contains(&default), "default master {default:.1} dB");
        assert!(db(gain(1.0, 1.0)) <= -3.0, "even 100% keeps headroom");
        for &m in &positions() {
            for &b in &positions() {
                let g = gain(m, b);
                assert!(g <= gain(m, 1.0) && g <= 1.0, "a bus never lifts over its master");
            }
            assert_eq!(gain(m, 0.0), 0.0, "a bus at 0% is silent");
        }
    }

    /// Glide from `from` toward `to` at `fps` for `secs`: every frame's level.
    fn run(from: f32, to: f32, fps: f32, secs: f32) -> Vec<f32> {
        let mut v = from;
        (0..(secs * fps).round() as usize)
            .map(|_| {
                v = glide(v, to, GLIDE_RATE, 1.0 / fps);
                v
            })
            .collect()
    }

    // The title theme's level leaving the title, the bed going silent, the
    // omen silence's depth under the bed, and a quiet placed loop starting.
    const GLIDES: [(f32, f32); 5] = [(0.307, 0.0), (0.256, 0.0), (0.256, 0.0205), (0.0, 1e-3), (0.0, 0.3)];

    #[test]
    fn a_looped_voice_reaches_its_level_at_any_frame_rate() {
        for fps in [30.0, 60.0, 144.0, 240.0] {
            for (from, to) in GLIDES {
                let path = run(from, to, fps, 3.0);
                assert_eq!(
                    *path.last().unwrap(),
                    to,
                    "{from} -> {to} at {fps} fps stopped at {}",
                    path.last().unwrap()
                );
                let (lo, hi) = (from.min(to), from.max(to));
                assert!(
                    path.iter().all(|v| (lo..=hi).contains(v)),
                    "{from} -> {to} at {fps} fps overshoots"
                );
            }
        }
    }

    #[test]
    fn a_glide_sounds_the_same_at_any_frame_rate() {
        for (from, to) in GLIDES {
            let half: Vec<f32> = [30.0, 60.0, 144.0, 240.0]
                .map(|fps| *run(from, to, fps, 0.5).last().unwrap())
                .to_vec();
            let (lo, hi) = half
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            assert!(lo > 0.0 && db(hi) - db(lo) < 1.0, "{from} -> {to} at 0.5 s: {half:?}");
        }
    }
}
