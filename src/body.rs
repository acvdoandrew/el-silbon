//! The player's body: gait, stamina, footstep noise and fear ("susto").
//!
//! Pure and headless. The host session steps one [`Body`] per participant;
//! the client reads the same numbers back from snapshots. Nothing here knows
//! where the Silbón is: fear reacts to darkness, company, load and to what
//! the player can perceive, never to his true distance.

use crate::tuning::Tuning;

/// Whether a player can act, is on the ground, or is gone for this run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Active,
    /// Caught: crawling, waiting for a teammate. Dies when `bleed` runs out.
    Downed {
        bleed: f32,
    },
    Dead,
}

impl Status {
    pub fn is_active(self) -> bool {
        matches!(self, Status::Active)
    }
    pub fn is_downed(self) -> bool {
        matches!(self, Status::Downed { .. })
    }
    pub fn code(self) -> u8 {
        match self {
            Status::Active => 0,
            Status::Downed { .. } => 1,
            Status::Dead => 2,
        }
    }
}

/// What the player asked their body to do this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyInput {
    pub crouch: bool,
    pub sprint: bool,
    /// Ground metres actually covered this tick.
    pub moved: f32,
}

/// The surface underfoot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ground {
    pub wading: bool,
    pub planks: bool,
}

/// Everything fear listens to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FearInput {
    /// Standing in the glow of a burning lamp.
    pub lit: bool,
    /// Another active player is close.
    pub teammate_near: bool,
    /// The party has company, but none of it is close.
    pub isolated: bool,
    /// Bone bundles carried.
    pub carried: usize,
    pub praying: bool,
    /// 0..1: how much of a hunt this player has sat through.
    pub exposure: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub crouching: bool,
    pub sprinting: bool,
    /// 0..1.
    pub stamina: f32,
    /// Seconds since the last sprint.
    rest: f32,
    /// 0..1; at 1 the player has a susto.
    pub fear: f32,
    /// Seconds still frozen by a susto.
    pub stun: f32,
    /// Ground covered toward the next footstep.
    stride: f32,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            crouching: false,
            sprinting: false,
            stamina: 1.0,
            rest: 10.0,
            fear: 0.0,
            stun: 0.0,
            stride: 0.0,
        }
    }
}

impl Body {
    /// Movement speed right now (metres per second).
    pub fn speed(&self, t: &Tuning, carried: usize, ground: Ground, downed: bool) -> f32 {
        if self.stun > 0.0 {
            return 0.0;
        }
        let wade = if ground.wading { t.wade_factor } else { 1.0 };
        if downed {
            return t.crawl_speed * wade;
        }
        let gait = if self.sprinting {
            t.sprint_factor
        } else if self.crouching {
            t.crouch_factor
        } else {
            1.0
        };
        let load = (1.0 - t.carry_slow * carried as f32).max(t.carry_floor);
        t.walk_speed * gait * load * wade
    }

    /// Audible radius of one footstep right now, before weather masking.
    pub fn step_radius(&self, t: &Tuning, carried: usize, ground: Ground, downed: bool) -> f32 {
        if downed {
            return t.noise_crouch * 0.6;
        }
        let mut r = if self.sprinting {
            t.noise_sprint
        } else if self.crouching {
            t.noise_crouch
        } else {
            t.noise_walk
        };
        let quiet = |k: f32| if self.crouching { k } else { 1.0 };
        if ground.wading {
            r += t.noise_wade * quiet(0.4);
        }
        if ground.planks {
            r += t.noise_plank * quiet(0.3);
        }
        if !self.crouching {
            r += t.noise_carry * carried as f32;
        }
        r
    }

    /// Advance gait, stamina and footsteps. Returns the radius of the
    /// loudest footstep that fell this tick, if any.
    pub fn advance(
        &mut self,
        t: &Tuning,
        input: BodyInput,
        carried: usize,
        ground: Ground,
        downed: bool,
        dt: f32,
    ) -> Option<f32> {
        self.stun = (self.stun - dt).max(0.0);
        self.crouching = input.crouch && !downed;
        let moving = input.moved > 1e-4;
        let want_sprint = input.sprint && moving && !self.crouching && !downed && self.stun <= 0.0;
        if self.sprinting {
            self.sprinting = want_sprint && self.stamina > 0.0;
        } else {
            self.sprinting = want_sprint && self.stamina >= t.sprint_min;
        }
        if self.sprinting {
            self.stamina = (self.stamina - t.stamina_drain * dt).max(0.0);
            self.rest = 0.0;
        } else {
            self.rest += dt;
            if self.rest >= t.stamina_delay {
                self.stamina = (self.stamina + t.stamina_regen * dt).min(1.0);
            }
        }
        self.stride += input.moved;
        let mut loudest = None;
        while self.stride >= t.stride {
            self.stride -= t.stride;
            loudest = Some(self.step_radius(t, carried, ground, downed));
        }
        if !moving {
            self.stride = self.stride.min(t.stride * 0.5);
        }
        loudest
    }

    /// Advance fear. Returns true when a susto strikes.
    pub fn tick_fear(&mut self, t: &Tuning, f: FearInput, dt: f32) -> bool {
        let mut rate = 0.0;
        if !f.lit {
            rate += t.fear_dark;
            if f.isolated {
                rate += t.fear_alone;
            }
        }
        rate += t.fear_carry * f.carried as f32;
        rate += t.fear_exposure * f.exposure;
        if f.teammate_near {
            rate -= t.fear_team;
        }
        if f.lit {
            rate -= t.fear_light;
        }
        if f.praying {
            rate -= t.fear_pray;
        }
        self.fear = (self.fear + rate * dt).clamp(0.0, 1.0);
        self.check_susto(t)
    }

    /// A one-off scare (he warned; a thin whistle was heard). Returns true on susto.
    pub fn startle(&mut self, t: &Tuning, amount: f32) -> bool {
        self.fear = (self.fear + amount).clamp(0.0, 1.0);
        self.check_susto(t)
    }

    /// Calm outright (prayer finished, revived).
    pub fn calm_to(&mut self, level: f32) {
        self.fear = self.fear.min(level);
    }

    fn check_susto(&mut self, t: &Tuning) -> bool {
        if self.fear >= 1.0 {
            self.fear = t.susto_reset;
            self.stun = t.susto_stun;
            self.sprinting = false;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(body: &mut Body, t: &Tuning, input: BodyInput, carried: usize, ground: Ground, secs: f32) -> Vec<f32> {
        let mut steps = Vec::new();
        let dt = 1.0 / 60.0;
        for _ in 0..(secs / dt) as usize {
            let speed = body.speed(t, carried, ground, false);
            let moved = BodyInput {
                moved: speed * dt,
                ..input
            };
            if let Some(r) = body.advance(t, moved, carried, ground, false, dt) {
                steps.push(r);
            }
        }
        steps
    }

    #[test]
    fn quieter_gaits_are_slower_and_noise_scales_with_gait_load_and_ground() {
        let t = Tuning::default();
        let ground = Ground::default();
        let mut b = Body::default();
        let crouch = BodyInput {
            crouch: true,
            ..Default::default()
        };
        let sprint = BodyInput {
            sprint: true,
            ..Default::default()
        };
        let stand = BodyInput::default();
        let speeds: Vec<f32> = [crouch, stand, sprint]
            .iter()
            .map(|i| {
                let mut b = Body::default();
                // One tick to settle the gait flags.
                b.advance(&t, BodyInput { moved: 0.01, ..*i }, 0, ground, false, 0.016);
                b.speed(&t, 0, ground, false)
            })
            .collect();
        assert!(speeds[0] < speeds[1] && speeds[1] < speeds[2], "{speeds:?}");
        let quiet = walk(&mut b, &t, crouch, 0, ground, 2.0);
        let mut b = Body::default();
        let normal = walk(&mut b, &t, stand, 0, ground, 2.0);
        let mut b = Body::default();
        let loud = walk(&mut b, &t, sprint, 0, ground, 2.0);
        let max = |v: &[f32]| v.iter().copied().fold(0.0, f32::max);
        assert!(max(&quiet) < max(&normal) && max(&normal) < max(&loud));
        // Load rattles and slows; planks and shallows add to the noise.
        let mut b = Body::default();
        let heavy = walk(&mut b, &t, stand, 3, ground, 2.0);
        assert!(max(&heavy) > max(&normal));
        let mut b = Body::default();
        let wet = walk(
            &mut b,
            &t,
            stand,
            0,
            Ground {
                wading: true,
                planks: false,
            },
            2.0,
        );
        assert!(max(&wet) > max(&normal));
        assert!(
            Body::default().speed(&t, 3, Ground::default(), false)
                < Body::default().speed(&t, 0, Ground::default(), false)
        );
        assert!(
            Body::default().speed(
                &t,
                0,
                Ground {
                    wading: true,
                    planks: false
                },
                false
            ) < Body::default().speed(&t, 0, Ground::default(), false)
        );
        // A crouched load is silent enough to sneak: no rattle.
        let mut b = Body::default();
        let sneaking = walk(&mut b, &t, crouch, 4, ground, 2.0);
        assert!(max(&sneaking) <= t.noise_crouch + 1e-3);
    }

    #[test]
    fn footsteps_fall_once_per_stride() {
        let t = Tuning::default();
        let mut b = Body::default();
        let mut steps = 0;
        let mut total = 0.0;
        for _ in 0..600 {
            let moved = 0.05;
            total += moved;
            if b.advance(
                &t,
                BodyInput {
                    moved,
                    ..Default::default()
                },
                0,
                Ground::default(),
                false,
                1.0 / 60.0,
            )
            .is_some()
            {
                steps += 1;
            }
        }
        assert_eq!(steps as f32, (total / t.stride).floor());
        // Standing still makes no sound.
        let mut b = Body::default();
        for _ in 0..600 {
            assert!(
                b.advance(&t, BodyInput::default(), 0, Ground::default(), false, 1.0 / 60.0)
                    .is_none()
            );
        }
    }

    #[test]
    fn sprinting_burns_stamina_stops_when_empty_and_recovers_after_a_rest() {
        let t = Tuning::default();
        let mut b = Body::default();
        let sprint = BodyInput {
            sprint: true,
            ..Default::default()
        };
        walk(&mut b, &t, sprint, 0, Ground::default(), 1.0);
        assert!(b.sprinting && b.stamina < 1.0);
        walk(&mut b, &t, sprint, 0, Ground::default(), 6.0);
        assert!(!b.sprinting, "must run out of breath");
        assert!(b.stamina < t.sprint_min);
        // Cannot restart while winded, even holding the key.
        walk(&mut b, &t, sprint, 0, Ground::default(), 0.5);
        assert!(!b.sprinting);
        // Rest, then the breath returns.
        let stand = BodyInput::default();
        walk(&mut b, &t, stand, 0, Ground::default(), t.stamina_delay + 0.2 + 3.0);
        assert!(b.stamina > 0.3);
        walk(&mut b, &t, stand, 0, Ground::default(), 6.0);
        assert!((b.stamina - 1.0).abs() < 1e-3);
    }

    #[test]
    fn a_downed_player_only_crawls_and_cannot_sprint_or_crouch() {
        let t = Tuning::default();
        let mut b = Body::default();
        b.advance(
            &t,
            BodyInput {
                crouch: true,
                sprint: true,
                moved: 0.05,
            },
            0,
            Ground::default(),
            true,
            0.016,
        );
        assert!(!b.crouching && !b.sprinting);
        assert!((b.speed(&t, 0, Ground::default(), true) - t.crawl_speed).abs() < 1e-6);
    }

    #[test]
    fn fear_climbs_in_the_dark_alone_and_falls_with_company_light_and_prayer() {
        let t = Tuning::default();
        let dt = 0.1;
        let run = |input: FearInput, secs: f32, start: f32| {
            let mut b = Body {
                fear: start,
                ..Body::default()
            };
            for _ in 0..(secs / dt) as usize {
                b.tick_fear(&t, input, dt);
            }
            b.fear
        };
        let alone = FearInput {
            isolated: true,
            ..Default::default()
        };
        let dark = FearInput::default();
        assert!(run(alone, 20.0, 0.0) > run(dark, 20.0, 0.0), "isolation is scarier");
        let together = FearInput {
            teammate_near: true,
            ..Default::default()
        };
        assert!(run(together, 20.0, 0.5) < 0.5, "company calms");
        let lit = FearInput {
            lit: true,
            ..Default::default()
        };
        assert!(run(lit, 20.0, 0.5) < 0.5, "lamplight calms");
        let praying = FearInput {
            praying: true,
            ..Default::default()
        };
        assert!(run(praying, 5.0, 0.9) < 0.1, "prayer clears fear");
        let burdened = FearInput {
            carried: 5,
            ..Default::default()
        };
        assert!(
            run(burdened, 20.0, 0.0) > run(dark, 20.0, 0.0),
            "the bones weigh on you"
        );
    }

    #[test]
    fn susto_freezes_the_player_then_leaves_them_shaken_not_calm() {
        let t = Tuning::default();
        let mut b = Body {
            fear: 0.99,
            ..Body::default()
        };
        assert!(b.startle(&t, 0.05));
        assert!(b.stun > 0.0 && (b.fear - t.susto_reset).abs() < 1e-6);
        assert_eq!(b.speed(&t, 0, Ground::default(), false), 0.0);
        let mut later = b.clone();
        later.advance(
            &t,
            BodyInput::default(),
            0,
            Ground::default(),
            false,
            t.susto_stun + 0.1,
        );
        assert!(later.speed(&t, 0, Ground::default(), false) > 0.0);
        // A smaller scare does not trigger one.
        let mut calm = Body::default();
        assert!(!calm.startle(&t, t.fear_warn));
    }
}
