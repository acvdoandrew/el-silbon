//! The director: when the night does something to one player.
//!
//! Fear needs quiet to grow in. A player left alone by him for a while, the
//! night fills the silence with an omen: the lamps around them die for a
//! moment, the llano falls silent, bones clatter somewhere, drag marks from
//! a sack cross the mud, a hat lies on the trail. Shaken badly enough, they
//! glimpse him where he is not; late in the night, a torch moves far off
//! that belongs to nobody.
//!
//! Headless and deterministic (a per-player [`Rng`] stream). The director
//! only decides *when* and *which*; every omen is presentation placed by the
//! listener's own client near its own eye. None of them says where he truly
//! is, and none of them changes the rules.

use crate::rng::Rng;
use crate::tuning::Tuning;

/// Something the night shows or plays to one player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Omen {
    /// The lamps around you gutter out, then come back.
    LampsDie,
    /// Insects, frogs and rain fall silent; then one clack of bone.
    Silence,
    /// Bones clattering somewhere off in the dark.
    Bones,
    /// Drag marks from a heavy sack across the mud nearby.
    Drag,
    /// His hat, lying on the trail ahead.
    Hat,
    /// A glimpse of him where he is not (fear only).
    Phantom,
    /// A torch far off that is nobody's (late in the night only).
    StolenLight,
    /// Footsteps in the grass behind you, coming closer; nobody's (fear
    /// only).
    Footsteps,
    /// A teammate's mark where nobody marked (fear only, in company).
    FalseMark,
}

/// What the director knows about one player this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    /// He has manifested (before that, only rare prologue omens).
    pub awake: bool,
    /// He is warning or hunting this player right now.
    pub pursued: bool,
    /// On their feet, the run on.
    pub able: bool,
    pub fear: f32,
    /// The night's pressure, 0..1.
    pub pressure: f32,
    /// Others share the night (a mark can seem to be theirs).
    pub company: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Director {
    /// Quiet seconds so far, and how many are needed before the next omen.
    quiet: f32,
    due: f32,
    /// The last omen, so the next is a different one.
    last: Option<Omen>,
    rng: Rng,
}

impl Director {
    pub fn new(seed: u64) -> Self {
        let mut d = Self {
            quiet: 0.0,
            due: 0.0,
            last: None,
            rng: Rng::fork(seed, 0x0DE7),
        };
        d.due = d.rng.range(40.0, 70.0);
        d
    }

    /// Advance by `dt`; returns an omen when one should happen now.
    pub fn tick(&mut self, mood: Mood, tuning: &Tuning, dt: f32) -> Option<Omen> {
        if !mood.able {
            return None;
        }
        if mood.pursued {
            // Pursuit is its own terror; the quiet starts over after it.
            self.quiet = 0.0;
            return None;
        }
        self.quiet += dt;
        if self.quiet < self.due {
            return None;
        }
        self.quiet = 0.0;
        let p = mood.pressure.clamp(0.0, 1.0);
        let (lo, hi) = tuning.omen_quiet;
        // Before he wakes the night only whispers; later it crowds you.
        self.due = if mood.awake {
            self.rng.range(lo, hi) * (1.0 - 0.4 * p)
        } else {
            self.rng.range(lo, hi) * 1.6
        };
        let mut pool: Vec<(Omen, f32)> = vec![(Omen::Silence, 1.0), (Omen::Bones, 1.0)];
        if mood.awake {
            pool.extend([(Omen::LampsDie, 1.2), (Omen::Drag, 1.0), (Omen::Hat, 0.8)]);
            if mood.fear >= tuning.phantom_fear {
                pool.extend([(Omen::Phantom, 1.6), (Omen::Footsteps, 1.3)]);
                if mood.company {
                    pool.push((Omen::FalseMark, 1.0));
                }
            }
            if p >= tuning.stolen_light_pressure {
                pool.push((Omen::StolenLight, 1.2));
            }
        }
        pool.retain(|(o, _)| Some(*o) != self.last);
        let total: f32 = pool.iter().map(|(_, w)| w).sum();
        let mut pick = self.rng.range(0.0, total);
        let mut chosen = pool[0].0;
        for (o, w) in pool {
            if pick < w {
                chosen = o;
                break;
            }
            pick -= w;
        }
        self.last = Some(chosen);
        Some(chosen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mood(awake: bool, fear: f32, pressure: f32) -> Mood {
        Mood {
            awake,
            pursued: false,
            able: true,
            fear,
            pressure,
            company: false,
        }
    }

    fn omens(d: &mut Director, m: Mood, t: &Tuning, secs: f32) -> Vec<Omen> {
        (0..(secs / 0.1) as usize).filter_map(|_| d.tick(m, t, 0.1)).collect()
    }

    #[test]
    fn omens_need_quiet_are_spaced_and_never_repeat_back_to_back() {
        let t = Tuning::default();
        let mut d = Director::new(11);
        let seen = omens(&mut d, mood(true, 0.2, 0.3), &t, 900.0);
        assert!(seen.len() >= 8, "a quiet quarter hour brings omens: {seen:?}");
        assert!(seen.len() <= 30, "but not a flood: {}", seen.len());
        assert!(seen.windows(2).all(|w| w[0] != w[1]));
        // Pursuit keeps resetting the quiet: nothing comes while he is on you.
        let mut chased = Director::new(11);
        let mut m = mood(true, 0.2, 0.3);
        m.pursued = true;
        assert!(omens(&mut chased, m, &t, 600.0).is_empty());
        // The downed and the dead are left alone.
        m.pursued = false;
        m.able = false;
        assert!(omens(&mut chased, m, &t, 600.0).is_empty());
    }

    #[test]
    fn phantoms_need_fear_stolen_lights_need_the_late_night_and_he_must_be_awake() {
        let t = Tuning::default();
        let calm: Vec<Omen> = omens(&mut Director::new(3), mood(true, 0.1, 0.1), &t, 3000.0);
        assert!(!calm.contains(&Omen::Phantom) && !calm.contains(&Omen::StolenLight));
        assert!(!calm.contains(&Omen::Footsteps) && !calm.contains(&Omen::FalseMark));
        let late: Vec<Omen> = omens(&mut Director::new(3), mood(true, 0.9, 0.9), &t, 3000.0);
        assert!(late.contains(&Omen::Phantom) && late.contains(&Omen::StolenLight));
        assert!(late.contains(&Omen::Footsteps), "fear hears steps behind it");
        assert!(!late.contains(&Omen::FalseMark), "alone, nobody else marks anything");
        let mut together = mood(true, 0.9, 0.9);
        together.company = true;
        let shared: Vec<Omen> = omens(&mut Director::new(3), together, &t, 3000.0);
        assert!(
            shared.contains(&Omen::FalseMark),
            "in company a mark can seem to be a friend's"
        );
        assert!(shared.windows(2).all(|w| w[0] != w[1]));
        let asleep: Vec<Omen> = omens(&mut Director::new(3), mood(false, 0.9, 0.9), &t, 3000.0);
        assert!(!asleep.is_empty(), "the night whispers before he wakes");
        assert!(
            asleep.iter().all(|o| matches!(o, Omen::Silence | Omen::Bones)),
            "{asleep:?}"
        );
        // Same seed, same night.
        assert_eq!(
            omens(&mut Director::new(5), mood(true, 0.5, 0.5), &t, 1200.0),
            omens(&mut Director::new(5), mood(true, 0.5, 0.5), &t, 1200.0)
        );
    }
}
