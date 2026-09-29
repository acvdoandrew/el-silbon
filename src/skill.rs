//! Skill checks: the rhythm a long task asks of the hands.
//!
//! Cranking the windmill pump, turning the truck's engine over and laying
//! bones at the roots are long holds. Every so often, while one is worked, a
//! check comes: a short warning, then a needle sweeps across a track once,
//! and the player presses when it crosses the marked zone. A press at the very
//! start of the zone is great and speeds the work; inside it is good; outside,
//! or no press at all, is a miss: the machinery screeches, the bones clatter,
//! and he hears it.
//!
//! Headless and deterministic: the schedule comes from a per-player [`Rng`]
//! stream. The host owns every check; a client only claims where it saw the
//! needle when it pressed, and the host refuses claims from the future or
//! from too long ago.

use crate::rng::Rng;
use crate::tuning::Tuning;

/// One check in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Check {
    /// Unique per player within a run.
    pub id: u32,
    /// Seconds since the needle started; negative during the warning.
    pub t: f32,
    /// Where the zone starts along the sweep (0..1).
    pub zone: f32,
}

impl Check {
    /// Where the needle is (0..1 while sweeping; negative before it starts).
    pub fn needle(&self, tuning: &Tuning) -> f32 {
        self.t / tuning.check_sweep
    }

    /// The verdict for a press with the needle at `at`.
    pub fn judge(&self, tuning: &Tuning, at: f32) -> Verdict {
        let into = at - self.zone;
        if (0.0..tuning.check_great).contains(&into) {
            Verdict::Great
        } else if (0.0..tuning.check_zone).contains(&into) {
            Verdict::Good
        } else {
            Verdict::Miss
        }
    }

    /// The needle has swept past and no press can still be on its way.
    pub fn expired(&self, tuning: &Tuning) -> bool {
        self.t > tuning.check_sweep + tuning.check_latency
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Great,
    Good,
    Miss,
}

/// What working a task did to the rhythm this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulse {
    Quiet,
    /// A new check began (its warning sounds now).
    Started,
    /// The check swept past without a press.
    Missed,
}

/// One player's schedule of checks.
#[derive(Clone, Debug, PartialEq)]
pub struct Rhythm {
    pub check: Option<Check>,
    /// Seconds of work until the next check.
    wait: f32,
    /// The task being worked (a `Vitals::hold_kind`), 0 for none.
    kind: u8,
    next_id: u32,
    rng: Rng,
}

impl Rhythm {
    pub fn new(seed: u64) -> Self {
        Self {
            check: None,
            wait: 0.0,
            kind: 0,
            next_id: 1,
            rng: Rng::fork(seed, 0x5C11),
        }
    }

    fn schedule(&mut self, tuning: &Tuning, first: Option<f32>) {
        let (lo, hi) = tuning.check_every;
        self.wait = first.unwrap_or_else(|| self.rng.range(lo, hi));
    }

    /// Task `kind` was worked for `dt`. `first`: when a fresh bout of this
    /// task asks for its first check (a short rite), else the usual spacing.
    pub fn work(&mut self, kind: u8, first: Option<f32>, tuning: &Tuning, dt: f32) -> Pulse {
        if kind != self.kind {
            self.kind = kind;
            self.check = None;
            self.schedule(tuning, first);
        }
        if let Some(c) = &mut self.check {
            c.t += dt;
            if c.expired(tuning) {
                self.check = None;
                self.schedule(tuning, None);
                return Pulse::Missed;
            }
            return Pulse::Quiet;
        }
        self.wait -= dt;
        if self.wait > 0.0 {
            return Pulse::Quiet;
        }
        let (lo, hi) = tuning.check_zone_at;
        self.check = Some(Check {
            id: self.next_id,
            t: -tuning.check_warn,
            zone: self.rng.range(lo, hi),
        });
        self.next_id += 1;
        Pulse::Started
    }

    /// Nothing is being worked: a check in flight is simply gone (letting go
    /// is never punished), and the next bout starts its schedule afresh.
    pub fn rest(&mut self) {
        self.kind = 0;
        self.check = None;
    }

    /// The task in hand (0 for none).
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// A press claiming the needle stood at `needle` for check `id`. Refused
    /// for another check, before the needle moved, or a claim the host's own
    /// needle cannot support (ahead of it, or older than the latency allows).
    pub fn press(&mut self, id: u32, needle: f32, tuning: &Tuning) -> Result<Verdict, &'static str> {
        let Some(c) = self.check else {
            return Err("No check is under way.");
        };
        if c.id != id {
            return Err("That check is over.");
        }
        if !needle.is_finite() {
            return Err("Invalid press.");
        }
        let now = c.needle(tuning);
        let slack = tuning.check_slack;
        let oldest = now - tuning.check_latency / tuning.check_sweep - slack;
        if needle > now + slack || needle < oldest || needle < 0.0 {
            return Err("The press does not match the needle.");
        }
        let verdict = c.judge(tuning, needle.min(1.0));
        self.check = None;
        self.schedule(tuning, None);
        Ok(verdict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(r: &mut Rhythm, t: &Tuning, kind: u8) -> Check {
        for _ in 0..(20.0 / 0.01) as usize {
            if r.work(kind, None, t, 0.01) == Pulse::Started {
                return r.check.expect("a check began");
            }
        }
        panic!("no check within 20 s of work");
    }

    #[test]
    fn checks_come_at_seeded_intervals_while_working_and_vanish_when_you_let_go() {
        let t = Tuning::default();
        let (mut a, mut b) = (Rhythm::new(7), Rhythm::new(7));
        let ca = start(&mut a, &t, 3);
        let cb = start(&mut b, &t, 3);
        assert_eq!(ca, cb, "same seed, same schedule");
        assert!(ca.t < 0.0, "a warning comes before the needle moves");
        assert!((t.check_zone_at.0..t.check_zone_at.1).contains(&ca.zone));
        a.rest();
        assert!(a.check.is_none(), "letting go is never punished");
        // A short rite asks for its first check at once.
        let mut c = Rhythm::new(9);
        assert_eq!(c.work(1, Some(0.0), &t, 0.01), Pulse::Started);
    }

    #[test]
    fn a_sweep_without_a_press_is_a_miss() {
        let t = Tuning::default();
        let mut r = Rhythm::new(3);
        start(&mut r, &t, 3);
        let mut missed = false;
        for _ in 0..((t.check_warn + t.check_sweep + t.check_latency + 0.2) / 0.01) as usize {
            missed |= r.work(3, None, &t, 0.01) == Pulse::Missed;
        }
        assert!(missed);
        assert!(r.check.is_none());
    }

    #[test]
    fn presses_are_judged_by_the_zone_and_impossible_claims_are_refused() {
        let t = Tuning::default();
        let mut r = Rhythm::new(5);
        let c = start(&mut r, &t, 4);
        // Before the needle moves, nothing can be claimed.
        assert!(r.press(c.id, 0.0, &t).is_err());
        // Advance the needle to the middle of the zone.
        let mid = c.zone + t.check_zone * 0.5;
        while r.check.unwrap().needle(&t) < mid {
            r.work(4, None, &t, 0.01);
        }
        assert!(r.press(c.id + 1, mid, &t).is_err(), "another check");
        assert!(r.press(c.id, mid + 0.3, &t).is_err(), "a needle from the future");
        assert!(r.press(c.id, mid - 0.6, &t).is_err(), "too long ago");
        assert_eq!(r.press(c.id, mid, &t), Ok(Verdict::Good));
        assert!(
            r.check.is_none() && r.press(c.id, mid, &t).is_err(),
            "one press per check"
        );
        let c = start(&mut r, &t, 4);
        while r.check.unwrap().needle(&t) < c.zone + 0.01 {
            r.work(4, None, &t, 0.01);
        }
        assert_eq!(r.press(c.id, c.zone + 0.01, &t), Ok(Verdict::Great));
        let c = start(&mut r, &t, 4);
        while r.check.unwrap().needle(&t) < c.zone - 0.05 {
            r.work(4, None, &t, 0.01);
        }
        assert_eq!(r.press(c.id, c.zone - 0.05, &t), Ok(Verdict::Miss), "too early");
    }
}
