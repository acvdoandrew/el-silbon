//! La Voz del Llano: the shelf radio's dial and its numbers hour. Pure and
//! headless.
//!
//! The dial has six stops plus off. The seed deals them: one numbers station,
//! one tale station, one joropo and three of static. The numbers station
//! reads the padlock's code forever, in a cycle that never changes length:
//! an ident, then each digit in its own window (d short pips, a zero one long
//! tone), two seconds between digits and a five-second rest. Lightning can
//! swallow one digit in a cycle, never the same one two cycles running, so
//! any two cycles in a row carry every digit.
//!
//! Nothing here knows where he is: the radio never reacts to him.

use crate::rng::Rng;

/// The dial's stops in kilocycles; the dial reads 0 for off, `i + 1` for
/// `STOPS[i]`.
pub const STOPS: [u16; 6] = [540, 620, 710, 880, 1010, 1270];

/// The ident that opens every cycle.
pub const IDENT: f32 = 1.6;
/// Silence between the ident and a digit, and between digits.
pub const GAP: f32 = 2.0;
/// One short pip, and the step from one pip to the next.
pub const PIP: f32 = 0.12;
pub const PIP_STEP: f32 = 0.4;
/// A zero: one long tone.
pub const LONG: f32 = 1.2;
/// Every digit has the same window, long enough for nine pips, so a swallowed
/// digit's static lasts as long whatever the digit.
pub const SLOT: f32 = 3.6;
/// The rest after the last digit.
pub const REST: f32 = 5.0;
/// One full reading of the code.
pub const CYCLE: f32 = IDENT + 3.0 * (GAP + SLOT) + REST;

/// What a stop carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    Numbers,
    /// The tale station (a placeholder broadcast for now).
    Tale,
    Joropo,
    Static,
}

/// Tonight's stations, one per stop.
pub fn stations(seed: u64) -> [Station; 6] {
    use Station::*;
    let mut s = [Numbers, Tale, Joropo, Static, Static, Static];
    let mut rng = Rng::fork(seed, 0x4AD1_0000);
    for i in (1..s.len()).rev() {
        s.swap(i, rng.below(i + 1));
    }
    s
}

/// The stop (0..6) of tonight's numbers station: the tag on the padlock.
pub fn numbers_stop(seed: u64) -> usize {
    stations(seed).iter().position(|&s| s == Station::Numbers).unwrap_or(0)
}

/// The frequency a dial setting shows (None: off).
pub fn frequency(dial: u8) -> Option<u16> {
    (dial as usize).checked_sub(1).and_then(|i| STOPS.get(i).copied())
}

/// What the radio plays at a dial setting (None: off).
pub fn station(seed: u64, dial: u8) -> Option<Station> {
    (dial as usize)
        .checked_sub(1)
        .and_then(|i| stations(seed).get(i).copied())
}

/// The next setting when someone turns the dial: off, then each stop, then
/// off again.
pub fn turned(dial: u8) -> u8 {
    if dial as usize >= STOPS.len() { 0 } else { dial + 1 }
}

/// A sound the numbers station makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ident,
    Pip,
    Long,
    /// A digit swallowed by lightning: crackle for its whole window.
    Static,
}

/// One sound: when it starts (seconds of the run), what it is, which
/// reading of the code it belongs to, and which part of it (0 the ident,
/// 1..=3 the digits).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beat {
    pub at: f64,
    pub tone: Tone,
    pub cycle: u64,
    pub slot: u8,
}

/// Where digit window `i` of reading `cycle` opens and closes, in seconds of
/// the run.
pub fn window(cycle: u64, i: usize) -> (f64, f64) {
    let open = cycle as f64 * CYCLE as f64 + slot_start(i) as f64;
    (open, open + SLOT as f64)
}

/// When digit window `i` (0..3) opens, from the start of a cycle.
pub fn slot_start(i: usize) -> f32 {
    IDENT + GAP + i as f32 * (SLOT + GAP)
}

/// One clear cycle of the code: (seconds into the cycle, tone, slot).
pub fn numbers_hour(code: [u8; 3]) -> Vec<(f32, Tone, u8)> {
    let mut out = vec![(0.0, Tone::Ident, 0)];
    for (i, &d) in code.iter().enumerate() {
        let start = slot_start(i);
        let slot = i as u8 + 1;
        if d == 0 {
            out.push((start, Tone::Long, slot));
        } else {
            out.extend((0..d.min(9)).map(|k| (start + k as f32 * PIP_STEP, Tone::Pip, slot)));
        }
    }
    out
}

/// Which digit (0..3), if any, lightning swallows in cycle `cycle`. Strikes
/// walk round the three digits, so two cycles running never lose the same
/// one.
pub fn dropped(seed: u64, cycle: u64) -> Option<usize> {
    let offset = Rng::fork(seed, 0x5717_C0DE).below(3) as u64;
    let strikes = Rng::fork(seed ^ 0x11A7_5A11, cycle).below(4) == 0;
    strikes.then_some(((offset + cycle) % 3) as usize)
}

/// Every sound of the numbers station that starts in `[t0, t1)` seconds of
/// the run. Any split of the run's time into consecutive frames hears each
/// beat exactly once.
pub fn beats_between(seed: u64, code: [u8; 3], t0: f64, t1: f64) -> Vec<Beat> {
    let mut out = Vec::new();
    if t1 <= t0 || t1 <= 0.0 || t0.is_nan() {
        return out;
    }
    let c = CYCLE as f64;
    let first = ((t0.max(0.0) / c).floor() as u64).saturating_sub(1);
    let last = (t1 / c).floor() as u64;
    let hour = numbers_hour(code);
    for n in first..=last {
        let base = n as f64 * c;
        let lost = dropped(seed, n);
        for &(at, tone, slot) in &hour {
            let swallowed = slot > 0 && lost == Some(slot as usize - 1);
            let tone = match (swallowed, tone) {
                (false, t) => t,
                // The whole window crackles once, from where it opens.
                (true, _) if at == slot_start(slot as usize - 1) => Tone::Static,
                (true, _) => continue,
            };
            let at = base + at as f64;
            if at >= t0 && at < t1 {
                out.push(Beat {
                    at,
                    tone,
                    cycle: n,
                    slot,
                });
            }
        }
    }
    out
}

/// Someone counting the pips: hears beats in order and works the digits out
/// from what they heard. The station reads each digit in its own window at
/// fixed times, so a digit counts once its whole window was heard: pips
/// counted (a long tone is a zero), and static leaves it for another round.
/// Tuning in halfway through a window, or looking away, loses that window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listener {
    /// Heard without a break from `.0` up to `.1`.
    span: Option<(f64, f64)>,
    /// The window being counted for each digit.
    tally: [Option<Tally>; 3],
    pub digits: [Option<u8>; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Tally {
    cycle: u64,
    pips: u8,
    zero: bool,
    lost: bool,
}

impl Listener {
    /// Hear the beats of `[from, now)`. A gap since the last call starts the
    /// count of any open window over.
    pub fn hear(&mut self, from: f64, now: f64, beats: &[Beat]) {
        let since = match self.span {
            Some((since, to)) if to == from => since,
            _ => {
                self.tally = [None; 3];
                from
            }
        };
        self.span = Some((since, now));
        for b in beats.iter().filter(|b| b.slot > 0) {
            let i = (b.slot as usize - 1).min(2);
            if self.tally[i].is_none_or(|t| t.cycle != b.cycle) {
                self.settle(i, since);
                self.tally[i] = Some(Tally {
                    cycle: b.cycle,
                    ..Tally::default()
                });
            }
            if let Some(t) = self.tally[i].as_mut() {
                match b.tone {
                    Tone::Pip => t.pips = t.pips.saturating_add(1),
                    Tone::Long => t.zero = true,
                    Tone::Static => t.lost = true,
                    Tone::Ident => {}
                }
            }
        }
        for i in 0..3 {
            if self.tally[i].is_some_and(|t| window(t.cycle, i).1 <= now) {
                self.settle(i, since);
            }
        }
    }

    /// A closed window: keep its digit if it was heard whole and clear.
    fn settle(&mut self, i: usize, since: f64) {
        let Some(t) = self.tally[i].take() else { return };
        if t.lost || window(t.cycle, i).0 < since {
            return;
        }
        if t.zero {
            self.digits[i] = Some(0);
        } else if t.pips > 0 {
            self.digits[i] = Some(t.pips);
        }
    }

    /// All three digits, once each has been heard clearly.
    pub fn code(&self) -> Option<[u8; 3]> {
        match self.digits {
            [Some(a), Some(b), Some(c)] => Some([a, b, c]),
            _ => None,
        }
    }
}

/// The dots caption: what the numbers station has said so far this cycle,
/// the ident as a ring, each pip a dot, a zero a dash, static as waves, and
/// the digits apart.
pub fn dots(seed: u64, code: [u8; 3], now: f64) -> String {
    let c = CYCLE as f64;
    let start = (now.max(0.0) / c).floor() * c;
    let mut out = String::new();
    let mut slot = 0;
    for b in beats_between(seed, code, start, now) {
        if b.slot != slot {
            slot = b.slot;
            if !out.is_empty() {
                out.push_str("   ");
            }
        } else {
            out.push(' ');
        }
        out.push_str(match b.tone {
            Tone::Ident => "((o))",
            Tone::Pip => "•",
            Tone::Long => "———",
            Tone::Static => "~~~",
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::lock_code;

    #[test]
    fn one_numbers_station_per_night_and_it_moves_between_seeds() {
        let mut stops = std::collections::BTreeSet::new();
        for seed in 0..200 {
            let s = stations(seed);
            let count = |k| s.iter().filter(|&&x| x == k).count();
            assert_eq!(count(Station::Numbers), 1, "seed {seed}: {s:?}");
            assert_eq!(count(Station::Tale), 1, "seed {seed}: {s:?}");
            assert_eq!(count(Station::Joropo), 1, "seed {seed}: {s:?}");
            assert_eq!(count(Station::Static), 3, "seed {seed}: {s:?}");
            assert_eq!(s, stations(seed), "the same seed deals the same dial");
            let stop = numbers_stop(seed);
            assert_eq!(station(seed, stop as u8 + 1), Some(Station::Numbers));
            assert!(frequency(stop as u8 + 1).is_some());
            stops.insert(stop);
        }
        assert_eq!(stops.len(), STOPS.len(), "the numbers station moves between nights");
        // The dial goes round: off, every stop once, and off again.
        let mut dial = 0;
        let mut seen = Vec::new();
        for _ in 0..=STOPS.len() {
            dial = turned(dial);
            seen.push(dial);
        }
        assert_eq!(seen, vec![1, 2, 3, 4, 5, 6, 0]);
        assert_eq!(station(7, 0), None);
    }

    #[test]
    fn the_pips_reproduce_the_code_for_every_night() {
        const { assert!(CYCLE <= 25.0, "a cycle of at most 25 s") };
        for seed in 0..200 {
            let code = lock_code(seed);
            // A listener tuning in somewhere in the middle of a cycle.
            let mut rng = Rng::fork(seed, 99);
            let mut t = rng.range(0.0, 3.0 * CYCLE) as f64;
            let begin = t;
            let mut ear = Listener::default();
            while ear.code().is_none() {
                let dt = rng.range(0.005, 0.5) as f64;
                ear.hear(t, t + dt, &beats_between(seed, code, t, t + dt));
                t += dt;
                assert!(
                    t - begin < 4.0 * CYCLE as f64,
                    "seed {seed}: still counting after {:.0} s ({:?})",
                    t - begin,
                    ear.digits
                );
            }
            assert_eq!(ear.code(), Some(code), "seed {seed}");
        }
        // Tuning in halfway through a window never counts it short.
        let code = [9, 9, 9];
        let (open, _) = window(3, 0);
        let mut ear = Listener::default();
        let mut t = open + 1.0;
        while t < open + CYCLE as f64 {
            ear.hear(t, t + 0.1, &beats_between(7, code, t, t + 0.1));
            t += 0.1;
        }
        assert!(ear.digits[0].is_none_or(|d| d == 9), "{:?}", ear.digits);
        // Looking away mid-window, likewise.
        let mut ear = Listener::default();
        let (a, b) = window(5, 1);
        ear.hear(a - 1.0, a + 1.0, &beats_between(7, code, a - 1.0, a + 1.0));
        ear.hear(a + 1.5, b + 1.0, &beats_between(7, code, a + 1.5, b + 1.0));
        assert_eq!(ear.digits[1], None, "a window with a gap in it is not counted");
    }

    #[test]
    fn each_beat_is_heard_exactly_once_at_any_frame_rate() {
        let span = 10.0 * CYCLE as f64;
        for seed in [1u64, 7, 42, 1234] {
            let code = lock_code(seed);
            let all = beats_between(seed, code, 0.0, span);
            // Every cycle opens with its ident and ends in the rest.
            assert_eq!(all.iter().filter(|b| b.tone == Tone::Ident).count(), 10);
            for dt in [1.0 / 144.0, 1.0 / 60.0, 1.0 / 30.0, 0.25, 1.0, 7.3] {
                let mut t = 0.0;
                let mut heard = Vec::new();
                while t < span {
                    let next = (t + dt).min(span);
                    heard.extend(beats_between(seed, code, t, next));
                    t = next;
                }
                assert_eq!(heard, all, "seed {seed} at dt {dt}");
            }
            let mut rng = Rng::fork(seed, 5);
            let mut t = 0.0;
            let mut heard = Vec::new();
            while t < span {
                let next = (t + rng.range(0.001, 2.0) as f64).min(span);
                heard.extend(beats_between(seed, code, t, next));
                t = next;
            }
            assert_eq!(heard, all, "seed {seed} at a ragged frame rate");
        }
    }

    #[test]
    fn two_cycles_running_carry_every_digit() {
        let mut strikes = 0;
        for seed in 0..200 {
            for n in 0..200u64 {
                let (a, b) = (dropped(seed, n), dropped(seed, n + 1));
                if a.is_some() {
                    strikes += 1;
                }
                assert!(a.is_none() || a != b, "seed {seed}: digit {a:?} lost twice running");
                // Each digit is clear in at least one of the two cycles.
                let code = [3, 0, 7];
                let heard: Vec<u8> = beats_between(seed, code, n as f64 * CYCLE as f64, (n + 2) as f64 * CYCLE as f64)
                    .iter()
                    .filter(|b| matches!(b.tone, Tone::Pip | Tone::Long))
                    .map(|b| b.slot)
                    .collect();
                for slot in 1..=3 {
                    assert!(
                        heard.contains(&slot),
                        "seed {seed} cycles {n}, {}: digit {slot} never heard",
                        n + 1
                    );
                }
            }
        }
        assert!(strikes > 0, "lightning never swallows a digit");
    }

    #[test]
    fn digits_are_two_seconds_apart_and_the_cycle_rests() {
        for code in [[9, 9, 9], [1, 0, 1], [0, 0, 0]] {
            let hour = numbers_hour(code);
            let end = |slot: u8| {
                hour.iter()
                    .filter(|b| b.2 == slot)
                    .map(|&(at, tone, _)| {
                        at + if tone == Tone::Long {
                            LONG
                        } else if tone == Tone::Ident {
                            IDENT
                        } else {
                            PIP
                        }
                    })
                    .fold(0.0f32, f32::max)
            };
            let begin = |slot: u8| {
                hour.iter()
                    .filter(|b| b.2 == slot)
                    .map(|b| b.0)
                    .fold(f32::MAX, f32::min)
            };
            for slot in 1..=3u8 {
                assert!(
                    begin(slot) - end(slot - 1) >= GAP - 1e-4,
                    "{code:?}: digit {slot} crowds the one before"
                );
            }
            assert!(CYCLE - end(3) >= REST - 1e-4, "{code:?}: no rest");
        }
    }

    #[test]
    fn the_dots_show_what_was_said_this_cycle() {
        let seed = (0..).find(|&s| dropped(s, 0).is_none()).unwrap();
        let code = [4, 0, 2];
        let full = dots(seed, code, CYCLE as f64 - 0.1);
        let groups: Vec<&str> = full.split("   ").collect();
        assert_eq!(groups.len(), 4, "{full}");
        assert_eq!(groups[1].matches('•').count(), 4);
        assert_eq!(groups[2].matches('•').count(), 0);
        assert_eq!(groups[3].matches('•').count(), 2);
        assert!(
            dots(seed, code, CYCLE as f64 + 0.5).len() < full.len(),
            "a new cycle starts afresh"
        );
    }
}
