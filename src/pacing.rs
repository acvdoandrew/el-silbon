//! El Respiro, the pacing director: when the night lets him press and when
//! it lets everyone breathe.
//!
//! Headless and deterministic. `net::session` feeds it session truth each
//! tick (who he sees, who is near him, torches, Tureco, fear) and applies its
//! orders to the threat before he moves: a floor under his warning cooldown,
//! a leash that keeps him away from everyone standing, and a withdraw when a
//! Peak fades. The phase is hidden AI state: it never goes on the wire.
//!
//! The beats: after the first pickup, **Grace** (he cannot warn anyone and
//! keeps his distance); then a cycle of **Build** (he stalks), **Peak**
//! (warnings and hunts, within a budget per rolling ten minutes), **Fade**
//! (he sinks, to rise far off) and **Relax** (leashed far away; only a push
//! after its floor ends it early).

use crate::rng::Rng;
use crate::tuning::Night;

/// Seconds of grace after the first pickup, per night.
pub fn grace(night: Night) -> f32 {
    match night {
        Night::Gentle => 240.0,
        Night::Normal => 120.0,
        Night::Hard => 45.0,
    }
}

/// Committed hunts allowed per rolling `HUNT_WINDOW`.
pub fn hunt_budget(night: Night) -> usize {
    match night {
        Night::Gentle => 1,
        Night::Normal => 2,
        Night::Hard => 3,
    }
}

/// How many bundles one player may carry at once.
pub fn carry_max(night: Night) -> usize {
    match night {
        Night::Gentle => 3,
        Night::Normal | Night::Hard => 2,
    }
}

/// The rooster crows at this many night lengths.
pub const DAWN: f32 = 1.5;

/// Seconds into the run when the rooster crows and he sinks for the night.
pub fn dawn(night_length: f32) -> f32 {
    DAWN * night_length
}

/// How much longer or shorter a Relax is on this night.
fn relax_scale(night: Night) -> f32 {
    match night {
        Night::Gentle => 1.3,
        Night::Normal => 1.0,
        Night::Hard => 0.7,
    }
}

pub const HUNT_WINDOW: f32 = 600.0;
/// While leashed he keeps at least this far from every standing player.
pub const LEASH_GRACE: f32 = 30.0;
pub const LEASH_RELAX: f32 = 45.0;
/// Whenever else his warning is held (a Build, a Fade, a Peak with the hunt
/// budget spent) he keeps out past his notice range, so he never loiters at
/// his standoff in plain view and warns from point-blank the moment he may.
pub const LEASH_HELD: f32 = 30.0;
/// Build lasts this long before the Peak; a push ends it only after the low end.
pub const BUILD: (f32, f32) = (60.0, 120.0);
/// Relax lasts this long before bones laid, night and floor.
pub const RELAX: (f32, f32) = (75.0, 120.0);
/// Nothing ends a Relax before this, and no Relax is shorter.
pub const RELAX_FLOOR: f32 = 40.0;
/// The longest Peak.
pub const PEAK_MAX: f32 = 120.0;
/// A Peak also ends after menace has stayed at `MENACE_HIGH` this long.
pub const MENACE_HIGH: f32 = 80.0;
pub const MENACE_HOLD: f32 = 45.0;
pub const MENACE_MAX: f32 = 100.0;
/// Menace per second from each kind of truth, and its decay.
pub const MENACE_SEEN: f32 = 12.0;
pub const MENACE_NEAR: f32 = 8.0;
pub const MENACE_LURE: f32 = 5.0;
pub const MENACE_GROWL: f32 = 4.0;
pub const MENACE_AFRAID: f32 = 3.0;
pub const MENACE_DECAY: f32 = 4.0;
/// Inside this distance of him a player adds `MENACE_NEAR`.
pub const NEAR: f32 = 16.0;
/// A fear at least this high adds `MENACE_AFRAID`.
pub const AFRAID: f32 = 0.7;
/// A noise at least this loud is a push.
pub const PUSH_NOISE: f32 = 34.0;
/// Anyone this close to him is a push.
pub const PUSH_NEAR: f32 = 30.0;
/// The warning floor held while warnings are not allowed. More than any
/// tick, since the threat counts its cooldown down before it looks.
const HOLD: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// He has not risen yet.
    #[default]
    Dormant,
    Grace,
    Build,
    Peak,
    Fade,
    Relax,
}

/// What one active player is doing to the night, read from session truth.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Menace {
    pub id: u64,
    /// He sees them.
    pub seen: bool,
    /// They are inside `NEAR` of him.
    pub near: bool,
    /// Their torch is drawing him.
    pub lure: bool,
    /// Tureco growls at him beside them.
    pub growl: bool,
    /// Their fear is at `AFRAID` or more.
    pub afraid: bool,
}

impl Menace {
    fn rate(&self) -> f32 {
        let add = |on: bool, r: f32| if on { r } else { 0.0 };
        add(self.seen, MENACE_SEEN)
            + add(self.near, MENACE_NEAR)
            + add(self.lure, MENACE_LURE)
            + add(self.growl, MENACE_GROWL)
            + add(self.afraid, MENACE_AFRAID)
    }
}

/// One tick of truth for the director.
#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    /// Every active player.
    pub players: &'a [Menace],
    /// His distance to the nearest active player (infinite when he is not
    /// there to be walked up to).
    pub nearest: f32,
    /// He is stalking: not warning, hunting, counting or hauling.
    pub calm: bool,
    /// He stands in the world (risen, or rising), so a Fade must sink him.
    pub present: bool,
    /// Bundles laid at the ceiba.
    pub laid: usize,
}

/// What the session does to him this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Orders {
    /// His warning cooldown is at least this.
    pub floor: Option<f32>,
    /// He keeps at least this far from every standing player.
    pub leash: Option<f32>,
    /// He sinks now, to rise at the node farthest from everyone.
    pub withdraw: bool,
}

/// The night's beats so far, for the smoke log and the first timed nights.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Beats {
    pub grace: f32,
    pub build: f32,
    pub peak: f32,
    pub fade: f32,
    pub relax: f32,
    pub peaks: u32,
    pub hunts: u32,
    /// Relaxes a push ended before their time.
    pub early_exits: u32,
}

#[derive(Clone, Debug)]
pub struct Respiro {
    night: Night,
    rng: Rng,
    phase: Phase,
    /// Seconds in the current phase, and its planned length.
    in_phase: f32,
    length: f32,
    /// Worst menace over the active players, and each one's.
    menace: f32,
    each: Vec<(u64, f32)>,
    /// Seconds menace has stayed high.
    high: f32,
    /// A hunt is on (begun, not yet resolved).
    hunting: bool,
    resolved: bool,
    /// When each hunt in the rolling window began.
    hunts: Vec<f32>,
    clock: f32,
    beats: Beats,
}

impl Respiro {
    pub fn new(night: Night, seed: u64) -> Self {
        Self {
            night,
            rng: Rng::fork(seed, 0x5E5B_1170),
            phase: Phase::Dormant,
            in_phase: 0.0,
            length: 0.0,
            menace: 0.0,
            each: Vec::new(),
            high: 0.0,
            hunting: false,
            resolved: false,
            hunts: Vec::new(),
            clock: 0.0,
            beats: Beats::default(),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn menace(&self) -> f32 {
        self.menace
    }
    pub fn beats(&self) -> Beats {
        self.beats
    }
    /// Hunts begun inside the rolling window.
    pub fn hunts_in_window(&self) -> usize {
        self.hunts.len()
    }

    /// He has risen for the first time: the grace begins.
    pub fn begin(&mut self) {
        if self.phase == Phase::Dormant {
            self.enter(Phase::Grace, 0);
        }
    }

    /// A committed hunt began.
    pub fn hunt_began(&mut self) {
        self.hunts.push(self.clock);
        self.hunting = true;
        self.beats.hunts += 1;
    }

    /// The hunt resolved (lost track, or three averts into a withdraw).
    pub fn hunt_resolved(&mut self) {
        if self.phase == Phase::Peak {
            self.resolved = true;
        }
        self.hunting = false;
    }

    /// A push: laying a bundle, cranking, the ignition, the beacon, a loud
    /// noise, someone walking up to him. It ends a Relax after its floor,
    /// and a Build after its shortest length. Carrying bones is no push.
    pub fn push_forward(&mut self, laid: usize) {
        match self.phase {
            Phase::Relax if self.in_phase >= RELAX_FLOOR => {
                self.beats.early_exits += 1;
                self.enter(Phase::Build, laid);
            }
            Phase::Build if self.in_phase >= BUILD.0 => self.enter(Phase::Peak, laid),
            _ => {}
        }
    }

    fn enter(&mut self, phase: Phase, laid: usize) {
        self.phase = phase;
        self.in_phase = 0.0;
        self.resolved = false;
        self.length = match phase {
            Phase::Grace => grace(self.night),
            Phase::Build => self.rng.range(BUILD.0, BUILD.1),
            Phase::Peak => {
                self.beats.peaks += 1;
                self.high = 0.0;
                PEAK_MAX
            }
            Phase::Relax => {
                let bones = (1.0 - 0.1 * laid as f32).max(0.0);
                (self.rng.range(RELAX.0, RELAX.1) * bones * relax_scale(self.night)).max(RELAX_FLOOR)
            }
            Phase::Dormant | Phase::Fade => 0.0,
        };
    }

    /// Advance one tick and say what happens to him.
    pub fn tick(&mut self, input: &Inputs, dt: f32) -> Orders {
        self.clock += dt;
        let window = self.clock - HUNT_WINDOW;
        self.hunts.retain(|&t| t > window);
        // Menace: each active player's own, the worst of them counts.
        let mut each = Vec::with_capacity(input.players.len());
        for m in input.players {
            let before = self.each.iter().find(|(id, _)| *id == m.id).map_or(0.0, |e| e.1);
            let now = (before + (m.rate() - MENACE_DECAY) * dt).clamp(0.0, MENACE_MAX);
            each.push((m.id, now));
        }
        self.each = each;
        self.menace = self.each.iter().map(|e| e.1).fold(0.0, f32::max);
        self.high = if self.menace >= MENACE_HIGH {
            self.high + dt
        } else {
            0.0
        };
        if self.phase == Phase::Dormant {
            return Orders::default();
        }
        self.in_phase += dt;
        let b = &mut self.beats;
        match self.phase {
            Phase::Grace => b.grace += dt,
            Phase::Build => b.build += dt,
            Phase::Peak => b.peak += dt,
            Phase::Fade => b.fade += dt,
            Phase::Relax => b.relax += dt,
            Phase::Dormant => {}
        }
        if self.hunting && input.calm {
            // Whatever ended it (a down, the dog, pepper), it is over.
            self.hunt_resolved();
        }
        if input.nearest < PUSH_NEAR {
            self.push_forward(input.laid);
        }
        let mut withdraw = false;
        match self.phase {
            Phase::Grace | Phase::Build | Phase::Relax if self.in_phase >= self.length => {
                let next = if self.phase == Phase::Build {
                    Phase::Peak
                } else {
                    Phase::Build
                };
                self.enter(next, input.laid);
            }
            Phase::Peak if self.resolved || (input.calm && (self.in_phase >= PEAK_MAX || self.high >= MENACE_HOLD)) => {
                self.enter(Phase::Fade, input.laid);
            }
            Phase::Fade if input.calm => {
                withdraw = input.present;
                self.enter(Phase::Relax, input.laid);
            }
            _ => {}
        }
        let floor = match self.phase {
            Phase::Dormant => None,
            Phase::Grace => Some((self.length - self.in_phase).max(HOLD)),
            Phase::Peak if self.hunts.len() < hunt_budget(self.night) => None,
            _ => Some(HOLD),
        };
        let leash = match self.phase {
            Phase::Dormant => None,
            Phase::Grace => Some(LEASH_GRACE),
            Phase::Relax => Some(LEASH_RELAX),
            _ if floor.is_some() => Some(LEASH_HELD),
            _ => None,
        };
        Orders { floor, leash, withdraw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.1;
    const NIGHTS: [Night; 3] = [Night::Gentle, Night::Normal, Night::Hard];

    /// A stand-in threat: while warnings are allowed he warns after a
    /// moment, commits a hunt after the warning and the hunt resolves after
    /// a while; pushes arrive at random.
    struct Mock {
        rng: Rng,
        warn: f32,
        hunt: f32,
        present: bool,
    }

    impl Mock {
        fn new(seed: u64) -> Self {
            Self {
                rng: Rng::fork(seed, 7),
                warn: -1.0,
                hunt: -1.0,
                present: true,
            }
        }

        /// One tick; returns the phase after it.
        fn step(&mut self, d: &mut Respiro, pushes: f32, t: f32) -> (Phase, Orders) {
            if self.rng.f32() < pushes * DT {
                d.push_forward(self.rng.below(6));
            }
            let calm = self.warn < 0.0 && self.hunt < 0.0;
            let menace = [Menace {
                id: 1,
                seen: !calm,
                near: self.rng.f32() < 0.3,
                ..Menace::default()
            }];
            let input = Inputs {
                players: &menace,
                nearest: self.rng.range(10.0, 90.0),
                calm,
                present: self.present,
                laid: ((t / 200.0) as usize).min(5),
            };
            let o = d.tick(&input, DT);
            if o.withdraw {
                self.present = false;
            } else if !self.present && self.rng.f32() < 0.05 {
                self.present = true;
            }
            // The threat reads the floor the way `sim` does.
            if self.warn >= 0.0 {
                self.warn += DT;
                if self.warn > 2.5 {
                    self.warn = -1.0;
                    self.hunt = 0.0;
                    d.hunt_began();
                }
            } else if self.hunt >= 0.0 {
                self.hunt += DT;
                if self.hunt > self.rng.range(5.0, 30.0) {
                    self.hunt = -1.0;
                }
            } else if o.floor.is_none() && self.present && self.rng.f32() < 0.2 * DT {
                self.warn = 0.0;
            }
            (d.phase(), o)
        }
    }

    #[test]
    fn a_relax_of_at_least_forty_seconds_always_follows_a_peak() {
        for night in NIGHTS {
            for seed in 0..12 {
                let mut d = Respiro::new(night, seed);
                let mut m = Mock::new(seed);
                d.begin();
                let (mut last, mut relax, mut peaks, mut relaxes) = (Phase::Grace, 0.0, 0, 0);
                let mut t = 0.0;
                while t < 3600.0 {
                    let (phase, _) = m.step(&mut d, 0.2, t);
                    if phase != last {
                        if last == Phase::Peak {
                            peaks += 1;
                            assert_eq!(phase, Phase::Fade, "{night:?} {seed}: a Peak fades");
                        }
                        if last == Phase::Fade {
                            assert_eq!(phase, Phase::Relax, "{night:?} {seed}: a Fade relaxes");
                        }
                        if last == Phase::Relax {
                            relaxes += 1;
                            assert!(
                                relax >= RELAX_FLOOR - 0.5 * DT,
                                "{night:?} {seed}: a Relax of {relax:.1} s"
                            );
                        }
                        if phase == Phase::Relax {
                            relax = 0.0;
                        }
                        last = phase;
                    }
                    if phase == Phase::Relax {
                        relax += DT;
                    }
                    t += DT;
                }
                assert!(
                    peaks >= 3 && relaxes >= 3,
                    "{night:?} {seed}: {peaks} peaks, {relaxes} relaxes"
                );
            }
        }
    }

    #[test]
    fn hunts_in_any_rolling_ten_minutes_stay_within_the_budget() {
        for night in NIGHTS {
            for seed in 0..12 {
                let mut d = Respiro::new(night, seed);
                let mut m = Mock::new(seed);
                d.begin();
                let mut began: Vec<f32> = Vec::new();
                let mut t = 0.0;
                while t < 5400.0 {
                    let before = d.beats().hunts;
                    m.step(&mut d, 0.3, t);
                    if d.beats().hunts > before {
                        began.push(t);
                        let recent = began.iter().filter(|&&b| b > t - HUNT_WINDOW).count();
                        assert!(
                            recent <= hunt_budget(night),
                            "{night:?} {seed}: {recent} hunts inside ten minutes at {t:.0} s"
                        );
                    }
                    t += DT;
                }
                assert!(!began.is_empty(), "{night:?} {seed}: he never hunted");
            }
        }
    }

    #[test]
    fn a_push_ends_a_relax_only_after_its_floor() {
        for night in NIGHTS {
            let mut d = Respiro::new(night, 3);
            d.begin();
            let calm = Inputs {
                players: &[],
                nearest: f32::INFINITY,
                calm: true,
                present: true,
                laid: 0,
            };
            // Through the grace and the build into the Peak, then fade.
            while d.phase() != Phase::Peak {
                d.tick(&calm, DT);
            }
            d.hunt_began();
            d.hunt_resolved();
            let o = d.tick(&calm, DT);
            assert_eq!(d.phase(), Phase::Fade);
            assert!(!o.withdraw);
            let o = d.tick(&calm, DT);
            assert!(o.withdraw, "the fade sinks him");
            assert_eq!(d.phase(), Phase::Relax);
            assert_eq!(o.leash, Some(LEASH_RELAX));
            // Pushed every tick, and walked up to: it holds for the floor.
            let near = Inputs { nearest: 5.0, ..calm };
            let mut held = 0.0;
            while d.phase() == Phase::Relax {
                d.push_forward(5);
                let o = d.tick(&near, DT);
                if d.phase() == Phase::Relax {
                    assert_eq!(o.floor, Some(HOLD), "no warning during a Relax");
                }
                held += DT;
                assert!(held < 200.0);
            }
            assert!(
                (RELAX_FLOOR - 0.5 * DT..RELAX_FLOOR + 2.0 * DT).contains(&held),
                "{night:?}: pushed out after {held:.1} s"
            );
            assert_eq!(d.beats().early_exits, 1);
            // Unpushed, a Relax runs its own length, never under the floor.
            let mut d = Respiro::new(night, 3);
            d.begin();
            while d.phase() != Phase::Relax {
                if d.phase() == Phase::Peak {
                    d.hunt_began();
                    d.hunt_resolved();
                }
                d.tick(&calm, DT);
            }
            let mut held = 0.0;
            while d.phase() == Phase::Relax {
                d.tick(&calm, DT);
                held += DT;
            }
            assert!(held >= RELAX_FLOOR - 0.5 * DT, "{night:?}: {held:.1} s");
            assert_eq!(d.beats().early_exits, 0);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_beats() {
        for night in NIGHTS {
            let run = |seed: u64| {
                let mut d = Respiro::new(night, seed);
                let mut m = Mock::new(99);
                d.begin();
                let mut phases = Vec::new();
                let mut t = 0.0;
                while t < 2400.0 {
                    let (p, _) = m.step(&mut d, 0.1, t);
                    if phases.last() != Some(&p) {
                        phases.push(p);
                    }
                    t += DT;
                }
                (phases, d.beats())
            };
            assert_eq!(run(5), run(5), "{night:?}");
        }
    }

    #[test]
    fn grace_forbids_warnings_and_keeps_him_away_until_it_ends() {
        for night in NIGHTS {
            let mut d = Respiro::new(night, 1);
            let calm = Inputs {
                players: &[],
                nearest: 5.0,
                calm: true,
                present: true,
                laid: 0,
            };
            assert_eq!(d.tick(&calm, DT), Orders::default(), "nothing before he rises");
            d.begin();
            let mut t = 0.0;
            while t + DT < grace(night) - 0.5 {
                let o = d.tick(&calm, DT);
                t += DT;
                assert_eq!(d.phase(), Phase::Grace, "walking up to him does not end the grace");
                let floor = o.floor.expect("no warning in the grace");
                assert!(floor >= grace(night) - t - 0.01 && floor >= HOLD);
                assert_eq!(o.leash, Some(LEASH_GRACE));
            }
        }
    }

    #[test]
    fn menace_is_the_worst_of_the_party_and_decays() {
        let mut d = Respiro::new(Night::Normal, 1);
        let seen = [
            Menace {
                id: 1,
                seen: true,
                near: true,
                ..Menace::default()
            },
            Menace {
                id: 2,
                ..Menace::default()
            },
        ];
        let input = Inputs {
            players: &seen,
            nearest: 50.0,
            calm: true,
            present: true,
            laid: 0,
        };
        for _ in 0..50 {
            d.tick(&input, DT);
        }
        // (12 + 8 - 4) per second for 5 s.
        assert!((d.menace() - 80.0).abs() < 0.5, "{}", d.menace());
        let quiet = Inputs {
            players: &seen[1..],
            ..input
        };
        for _ in 0..50 {
            d.tick(&quiet, DT);
        }
        assert!(d.menace() < 0.5, "the one who was seen is gone");
    }
}
