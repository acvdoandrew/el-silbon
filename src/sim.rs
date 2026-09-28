//! Encounter truth: objectives, the Silbón's hidden state and transitions.
//!
//! Headless and deterministic. The ECS layer feeds a [`TickInput`] per frame
//! and reacts to the [`Event`]s pushed out; nothing in here knows about
//! rendering, audio or input devices. Offline play advances this encounter
//! directly; `net::session` reuses its threat rules on the authoritative host
//! and sends only player-facing cues and legitimate visible presentation.

use bevy::math::Vec2;

use crate::geometry::{Layout, ground};
use crate::tuning::Tuning;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Objective {
    /// Enter the house and take the bone satchel.
    FindSatchel,
    /// Carry the bones to the ceiba and hold at its roots.
    ReturnBones,
    /// Bones returned: walk back to the road.
    Escape,
    Won,
    Failed,
}

impl Objective {
    pub fn is_over(self) -> bool {
        matches!(self, Objective::Won | Objective::Failed)
    }
}

/// The Silbón's true behavioural state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreatState {
    /// Not yet here; the bones are still on the table.
    Dormant,
    /// Walking his authored route, lurking, creeping toward a visible player.
    Stalking,
    /// He has seen you. Break line of sight before the hunt begins.
    Warning,
    /// Closing in while he can see you; exposure builds.
    Hunting,
    /// The bones are home. He leaves.
    Resolved,
}

/// Whether he is physically present in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Presence {
    Hidden,
    /// Rising out of the grass, `t` seconds in.
    Rising {
        t: f32,
    },
    Present,
    /// Sinking away; `relocate` = re-manifest far from the player afterwards.
    Sinking {
        t: f32,
        relocate: bool,
    },
}

/// How he is moving along the authored anchor ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Movement {
    /// Standing on anchor `i`.
    AtAnchor(usize),
    /// Walking the ring between two anchors.
    Ring { from: usize, to: usize },
    /// Left anchor `home` and walks straight toward a visible player.
    Creep { home: usize },
    /// Walking straight back to anchor `home`.
    Return { home: usize },
    /// Not moving (warning, hunting without sight, rising, sinking).
    Still,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    SatchelTaken,
    ThreatManifested,
    WarningBegan,
    HuntBegan,
    /// Sight broken during a warning: the hunt never starts.
    WarningAverted,
    /// Sight broken long enough during a hunt: he withdraws.
    LostTrack,
    /// He rose again at a distant anchor after withdrawing.
    ThreatReturned,
    RestitutionComplete,
    ThreatResolved,
    Escaped,
    Caught,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    /// The action does not belong to the current objective.
    WrongObjective,
    /// The player is not where the action can be performed.
    OutOfReach,
    /// The encounter already ended.
    Finished,
}

/// Everything the truth layer needs from one player for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickInput {
    pub dt: f32,
    pub player: Vec2,
    /// The client pressed interact while aiming at a reachable satchel.
    pub take_satchel: bool,
    /// The client is holding interact while aiming at the reachable hollow.
    pub hold_offering: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub warnings: u32,
    pub hunts: u32,
    /// Averted warnings plus lost-track withdrawals.
    pub recoveries: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Threat {
    pub state: ThreatState,
    pub presence: Presence,
    pub pos: Vec2,
    /// Unit direction he faces on the ground plane.
    pub facing: Vec2,
    /// Ground speed this tick (for the walk animation).
    pub speed: f32,
    pub movement: Movement,
    /// Seconds in the current state.
    pub state_time: f32,
    /// While > 0 he cannot begin a warning.
    pub cooldown: f32,
    /// 0..1; reaching 1 during a hunt ends the encounter.
    pub exposure: f32,
    /// Seconds without sight during a warning or hunt.
    pub unseen: f32,
    /// Seconds lurking at the right anchor without sight.
    pub lurk_time: f32,
    /// He currently has line of sight to the player.
    pub has_sight: bool,
    /// Out of patience: walking the whole ring instead of waiting.
    pub circling: bool,
}

impl Threat {
    fn dormant(layout: &Layout) -> Self {
        Self {
            state: ThreatState::Dormant,
            presence: Presence::Hidden,
            pos: layout.ring[0],
            facing: Vec2::new(0.0, 1.0),
            speed: 0.0,
            movement: Movement::Still,
            state_time: 0.0,
            cooldown: 0.0,
            exposure: 0.0,
            unseen: 0.0,
            lurk_time: 0.0,
            has_sight: false,
            circling: false,
        }
    }

    /// 0 = hidden underground, 1 = fully standing.
    pub fn visibility(&self, tuning: &Tuning) -> f32 {
        match self.presence {
            Presence::Hidden => 0.0,
            Presence::Rising { t } => (t / tuning.rise_time).clamp(0.0, 1.0),
            Presence::Present => 1.0,
            Presence::Sinking { t, .. } => 1.0 - (t / tuning.sink_time).clamp(0.0, 1.0),
        }
    }

    fn set_state(&mut self, state: ThreatState) {
        self.state = state;
        self.state_time = 0.0;
        self.unseen = 0.0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub objective: Objective,
    pub threat: Threat,
    /// Restitution progress 0..1 (pauses, never resets, when interrupted).
    pub restitution: f32,
    /// Restitution advanced during the last tick.
    pub restituting: bool,
    pub elapsed: f32,
    pub stats: Stats,
}

impl Encounter {
    pub fn new(layout: &Layout) -> Self {
        Self {
            objective: Objective::FindSatchel,
            threat: Threat::dormant(layout),
            restitution: 0.0,
            restituting: false,
            elapsed: 0.0,
            stats: Stats::default(),
        }
    }

    /// Restart: every truth value and timer back to its initial state.
    pub fn reset(&mut self, layout: &Layout) {
        *self = Self::new(layout);
    }

    pub fn carrying(&self) -> bool {
        self.objective == Objective::ReturnBones
    }

    pub fn player_speed(&self, tuning: &Tuning) -> f32 {
        if self.carrying() {
            tuning.carry_speed
        } else {
            tuning.walk_speed
        }
    }

    /// Take the satchel. Validates objective and reach on the truth side.
    pub fn take_satchel(
        &mut self,
        layout: &Layout,
        tuning: &Tuning,
        player: Vec2,
        events: &mut Vec<Event>,
    ) -> Result<(), ActionError> {
        if self.objective.is_over() {
            return Err(ActionError::Finished);
        }
        if self.objective != Objective::FindSatchel {
            return Err(ActionError::WrongObjective);
        }
        if player.distance(ground(layout.satchel)) > tuning.satchel_reach + tuning.reach_slack {
            return Err(ActionError::OutOfReach);
        }
        self.objective = Objective::ReturnBones;
        events.push(Event::SatchelTaken);

        // Controlled manifestation: the anchor farthest from the player.
        let (anchor, _) = layout.farthest_anchor(player);
        let th = &mut self.threat;
        th.set_state(ThreatState::Stalking);
        th.presence = Presence::Rising { t: 0.0 };
        th.pos = layout.ring[anchor];
        th.facing = (player - th.pos).normalize_or(Vec2::Y);
        th.movement = Movement::AtAnchor(anchor);
        th.cooldown = tuning.first_warn_delay;
        th.exposure = 0.0;
        th.lurk_time = 0.0;
        th.circling = false;
        events.push(Event::ThreatManifested);
        Ok(())
    }

    /// Hold at the ceiba's hollow for `dt`. Validates objective and reach.
    pub fn hold_offering(
        &mut self,
        layout: &Layout,
        tuning: &Tuning,
        player: Vec2,
        dt: f32,
        events: &mut Vec<Event>,
    ) -> Result<(), ActionError> {
        if self.objective.is_over() {
            return Err(ActionError::Finished);
        }
        if self.objective != Objective::ReturnBones {
            return Err(ActionError::WrongObjective);
        }
        if player.distance(ground(layout.ceiba.offering)) > tuning.offering_reach + tuning.reach_slack {
            return Err(ActionError::OutOfReach);
        }
        self.restituting = true;
        self.restitution = (self.restitution + dt / tuning.restitution_hold).min(1.0);
        if self.restitution >= 1.0 {
            self.objective = Objective::Escape;
            events.push(Event::RestitutionComplete);
            let th = &mut self.threat;
            th.set_state(ThreatState::Resolved);
            th.exposure = 0.0;
            th.movement = Movement::Still;
            th.speed = 0.0;
            th.presence = match th.presence {
                Presence::Hidden => Presence::Hidden,
                Presence::Sinking { t, .. } => Presence::Sinking { t, relocate: false },
                Presence::Rising { t } => {
                    let v = (t / tuning.rise_time).clamp(0.0, 1.0);
                    Presence::Sinking {
                        t: (1.0 - v) * tuning.sink_time,
                        relocate: false,
                    }
                }
                Presence::Present => Presence::Sinking {
                    t: 0.0,
                    relocate: false,
                },
            };
            events.push(Event::ThreatResolved);
        }
        Ok(())
    }

    /// Advance the encounter by one tick. Events are appended to `events`.
    pub fn step(&mut self, layout: &Layout, tuning: &Tuning, input: TickInput, events: &mut Vec<Event>) {
        if self.objective.is_over() {
            self.restituting = false;
            self.threat.speed = 0.0;
            return;
        }
        let dt = input.dt.clamp(0.0, tuning.max_step);
        self.elapsed += dt;
        self.restituting = false;

        // Client claims are re-validated inside; invalid claims change nothing.
        if input.take_satchel {
            let _ = self.take_satchel(layout, tuning, input.player, events);
        }
        if input.hold_offering {
            let _ = self.hold_offering(layout, tuning, input.player, dt, events);
        }

        self.update_threat(layout, tuning, input.player, dt, events);
        if self.objective.is_over() {
            return;
        }

        if self.objective == Objective::Escape && layout.in_road_goal(input.player) {
            self.objective = Objective::Won;
            events.push(Event::Escaped);
        }
    }

    pub(crate) fn update_threat(
        &mut self,
        layout: &Layout,
        tuning: &Tuning,
        player: Vec2,
        dt: f32,
        events: &mut Vec<Event>,
    ) {
        let th = &mut self.threat;
        th.speed = 0.0;
        match th.presence {
            Presence::Hidden => return,
            Presence::Rising { t } => {
                let t = t + dt;
                th.facing = (player - th.pos).normalize_or(th.facing);
                th.presence = if t >= tuning.rise_time {
                    Presence::Present
                } else {
                    Presence::Rising { t }
                };
                th.has_sight = false;
                th.cooldown = (th.cooldown - dt).max(0.0);
                return;
            }
            Presence::Sinking { t, relocate } => {
                let t = t + dt;
                th.has_sight = false;
                if t < tuning.sink_time {
                    th.presence = Presence::Sinking { t, relocate };
                } else if relocate && th.state != ThreatState::Resolved {
                    let (anchor, _) = layout.farthest_anchor(player);
                    th.pos = layout.ring[anchor];
                    th.facing = (player - th.pos).normalize_or(th.facing);
                    th.movement = Movement::AtAnchor(anchor);
                    th.presence = Presence::Rising { t: 0.0 };
                    events.push(Event::ThreatReturned);
                } else {
                    th.presence = Presence::Hidden;
                }
                return;
            }
            Presence::Present => {}
        }
        if th.state == ThreatState::Resolved || th.state == ThreatState::Dormant {
            return;
        }

        let to_player = player - th.pos;
        let dist = to_player.length();
        th.has_sight = layout.line_of_sight(th.pos, player);
        th.cooldown = (th.cooldown - dt).max(0.0);
        th.state_time += dt;

        match th.state {
            ThreatState::Stalking => {
                stalk(th, layout, tuning, player, dt);
                let dist = th.pos.distance(player);
                if th.has_sight && dist <= tuning.warn_distance && th.cooldown <= 0.0 {
                    th.set_state(ThreatState::Warning);
                    th.movement = Movement::Still;
                    th.speed = 0.0;
                    self.stats.warnings += 1;
                    events.push(Event::WarningBegan);
                }
            }
            ThreatState::Warning => {
                th.facing = to_player.normalize_or(th.facing);
                if th.has_sight {
                    th.unseen = 0.0;
                } else {
                    th.unseen += dt;
                }
                if th.unseen >= tuning.warn_break_time {
                    th.set_state(ThreatState::Stalking);
                    th.cooldown = tuning.warn_recover_cooldown;
                    th.movement = Movement::Return {
                        home: layout.nearest_anchor(th.pos),
                    };
                    self.stats.recoveries += 1;
                    events.push(Event::WarningAverted);
                } else if th.state_time >= tuning.warn_time && th.has_sight {
                    th.set_state(ThreatState::Hunting);
                    th.exposure = 0.0;
                    self.stats.hunts += 1;
                    events.push(Event::HuntBegan);
                }
            }
            ThreatState::Hunting => {
                if th.has_sight {
                    th.unseen = 0.0;
                    th.facing = to_player.normalize_or(th.facing);
                    let stop = tuning.catch_distance * 0.8;
                    if dist > stop {
                        let step = (tuning.hunt_speed * dt).min(dist - stop);
                        th.pos += to_player / dist * step;
                        th.speed = step / dt.max(1e-6);
                    }
                    let near = 1.0 - (dist / tuning.warn_distance).clamp(0.0, 1.0);
                    th.exposure += dt / tuning.exposure_time * (1.0 + tuning.exposure_near_boost * near);
                    let dist_now = th.pos.distance(player);
                    if th.exposure >= 1.0 || dist_now <= tuning.catch_distance {
                        th.exposure = th.exposure.min(1.0);
                        th.movement = Movement::Still;
                        self.objective = Objective::Failed;
                        events.push(Event::Caught);
                    }
                } else {
                    th.unseen += dt;
                    th.exposure = (th.exposure - tuning.exposure_decay * dt).max(0.0);
                    if th.unseen >= tuning.lose_track_time {
                        th.set_state(ThreatState::Stalking);
                        th.exposure = 0.0;
                        th.cooldown = tuning.recover_cooldown;
                        th.movement = Movement::Still;
                        th.circling = false;
                        th.lurk_time = 0.0;
                        th.presence = Presence::Sinking { t: 0.0, relocate: true };
                        self.stats.recoveries += 1;
                        events.push(Event::LostTrack);
                    }
                }
            }
            ThreatState::Dormant | ThreatState::Resolved => {}
        }
    }
}

/// Move toward `target` by at most `max`; returns true once there.
fn advance(pos: &mut Vec2, facing: &mut Vec2, target: Vec2, max: f32) -> (bool, f32) {
    let d = target - *pos;
    let len = d.length();
    if len <= max || len < 1e-4 {
        *pos = target;
        return (true, len);
    }
    let dir = d / len;
    *pos += dir * max;
    *facing = dir;
    (false, max)
}

/// Stalking: walk the ring to the anchor nearest the player, lurk there,
/// creep straight in only while the player is visible, circle when bored.
fn stalk(th: &mut Threat, layout: &Layout, tuning: &Tuning, player: Vec2, dt: f32) {
    let target = layout.nearest_anchor(player);
    let dist_player = th.pos.distance(player);
    if th.has_sight {
        th.lurk_time = 0.0;
        th.circling = false;
    }
    let mut moved = 0.0;
    th.movement = match th.movement {
        Movement::Still => Movement::Return {
            home: layout.nearest_anchor(th.pos),
        },
        Movement::AtAnchor(i) => {
            if th.circling {
                let n = layout.ring.len();
                Movement::Ring {
                    from: i,
                    to: (i + 1) % n,
                }
            } else if i != target {
                Movement::Ring {
                    from: i,
                    to: layout.ring_step_toward(i, target),
                }
            } else {
                // Lurking at the right anchor.
                th.facing = (player - th.pos).normalize_or(th.facing);
                if th.has_sight && th.cooldown <= 0.0 && dist_player > tuning.warn_distance {
                    Movement::Creep { home: i }
                } else {
                    if !th.has_sight {
                        th.lurk_time += dt;
                        if th.lurk_time >= tuning.patience {
                            th.circling = true;
                        }
                    }
                    Movement::AtAnchor(i)
                }
            }
        }
        Movement::Ring { from, to } => {
            // Turn around if the short way to the target now lies behind him.
            let (from, to) = if !th.circling && layout.ring_hops(from, target) < layout.ring_hops(to, target) {
                (to, from)
            } else {
                (from, to)
            };
            let (arrived, m) = advance(&mut th.pos, &mut th.facing, layout.ring[to], tuning.stalk_speed * dt);
            moved = m;
            if arrived {
                if th.circling {
                    // One anchor further each time; stop circling when he sees you.
                    th.lurk_time = 0.0;
                }
                Movement::AtAnchor(to)
            } else {
                Movement::Ring { from, to }
            }
        }
        Movement::Creep { home } => {
            if !th.has_sight || target != home {
                Movement::Return { home }
            } else {
                let stop = tuning.warn_distance * 0.5;
                if dist_player > stop {
                    let (_, m) = advance(
                        &mut th.pos,
                        &mut th.facing,
                        player,
                        (tuning.creep_speed * dt).min(dist_player - stop),
                    );
                    moved = m;
                }
                Movement::Creep { home }
            }
        }
        Movement::Return { home } => {
            let (arrived, m) = advance(&mut th.pos, &mut th.facing, layout.ring[home], tuning.stalk_speed * dt);
            moved = m;
            if arrived {
                Movement::AtAnchor(home)
            } else {
                Movement::Return { home }
            }
        }
    };
    th.speed = moved / dt.max(1e-6);
    if th.circling && th.has_sight {
        th.circling = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Layout, Tuning, Encounter, Vec<Event>) {
        let layout = Layout::authored();
        let enc = Encounter::new(&layout);
        (layout, Tuning::default(), enc, Vec::new())
    }

    fn tick(enc: &mut Encounter, l: &Layout, t: &Tuning, player: Vec2, ev: &mut Vec<Event>) {
        enc.step(
            l,
            t,
            TickInput {
                dt: 1.0 / 60.0,
                player,
                ..Default::default()
            },
            ev,
        );
    }

    fn run_for(enc: &mut Encounter, l: &Layout, t: &Tuning, player: Vec2, secs: f32, ev: &mut Vec<Event>) {
        for _ in 0..(secs * 60.0) as usize {
            tick(enc, l, t, player, ev);
        }
    }

    /// Put a present, stalking Silbón at `pos` with no cooldown.
    fn place_threat(enc: &mut Encounter, l: &Layout, pos: Vec2) {
        enc.objective = Objective::ReturnBones;
        let th = &mut enc.threat;
        th.state = ThreatState::Stalking;
        th.presence = Presence::Present;
        th.pos = pos;
        th.movement = Movement::AtAnchor(l.nearest_anchor(pos));
        th.cooldown = 0.0;
    }

    #[test]
    fn objective_progresses_find_carry_restitute_escape_win() {
        let (l, t, mut enc, mut ev) = setup();
        let at_table = Vec2::new(2.9, -3.3);
        enc.take_satchel(&l, &t, at_table, &mut ev).unwrap();
        assert_eq!(enc.objective, Objective::ReturnBones);
        assert!(enc.carrying());
        assert!(enc.player_speed(&t) < t.walk_speed);
        assert_eq!(enc.threat.state, ThreatState::Stalking);

        let at_tree = ground(l.ceiba.offering) + Vec2::new(1.0, 1.0);
        let mut held = 0.0;
        while enc.objective == Objective::ReturnBones {
            enc.hold_offering(&l, &t, at_tree, 0.1, &mut ev).unwrap();
            held += 0.1;
            assert!(held < t.restitution_hold + 0.2);
        }
        assert_eq!(enc.objective, Objective::Escape);
        assert_eq!(enc.threat.state, ThreatState::Resolved);
        assert!(ev.contains(&Event::RestitutionComplete));
        assert_eq!(enc.player_speed(&t), t.walk_speed);

        // Not yet on the road: nothing happens.
        run_for(&mut enc, &l, &t, Vec2::new(0.0, 10.0), 1.0, &mut ev);
        assert_eq!(enc.objective, Objective::Escape);
        tick(&mut enc, &l, &t, Vec2::new(0.0, 30.0), &mut ev);
        assert_eq!(enc.objective, Objective::Won);
        assert!(ev.contains(&Event::Escaped));
    }

    #[test]
    fn invalid_transitions_are_rejected_and_change_nothing() {
        let (l, t, mut enc, mut ev) = setup();
        let before = enc.clone();
        // Restitution before carrying anything.
        let at_tree = ground(l.ceiba.offering) + Vec2::new(1.0, 1.0);
        assert_eq!(
            enc.hold_offering(&l, &t, at_tree, 1.0, &mut ev),
            Err(ActionError::WrongObjective)
        );
        // Taking the satchel from the road.
        assert_eq!(enc.take_satchel(&l, &t, l.spawn, &mut ev), Err(ActionError::OutOfReach));
        assert_eq!(enc, before);
        assert!(ev.is_empty());
        // Reaching the road first does not win.
        run_for(&mut enc, &l, &t, l.spawn, 0.5, &mut ev);
        assert_eq!(enc.objective, Objective::FindSatchel);

        enc.take_satchel(&l, &t, Vec2::new(2.9, -3.3), &mut ev).unwrap();
        // Taking it twice.
        assert_eq!(
            enc.take_satchel(&l, &t, Vec2::new(2.9, -3.3), &mut ev),
            Err(ActionError::WrongObjective)
        );
        // Holding far from the tree.
        assert_eq!(
            enc.hold_offering(&l, &t, Vec2::new(0.0, 0.0), 1.0, &mut ev),
            Err(ActionError::OutOfReach)
        );
        assert_eq!(enc.restitution, 0.0);
        // Road while carrying: no escape.
        let mut far = enc.clone();
        tick(&mut far, &l, &t, Vec2::new(0.0, 30.0), &mut ev);
        assert_eq!(far.objective, Objective::ReturnBones);

        // After the end nothing is accepted.
        enc.objective = Objective::Failed;
        assert_eq!(
            enc.hold_offering(&l, &t, at_tree, 1.0, &mut ev),
            Err(ActionError::Finished)
        );
        let frozen = enc.clone();
        run_for(&mut enc, &l, &t, at_tree, 1.0, &mut ev);
        assert_eq!(enc.elapsed, frozen.elapsed);
    }

    #[test]
    fn restitution_pauses_when_interrupted_and_resumes() {
        let (l, t, mut enc, mut ev) = setup();
        enc.take_satchel(&l, &t, Vec2::new(2.9, -3.3), &mut ev).unwrap();
        let at_tree = ground(l.ceiba.offering) + Vec2::new(1.0, 1.0);
        enc.hold_offering(&l, &t, at_tree, 1.0, &mut ev).unwrap();
        let partial = enc.restitution;
        assert!(partial > 0.0 && partial < 1.0);
        // Let go for a while: progress stays where it was.
        run_for(&mut enc, &l, &t, at_tree, 2.0, &mut ev);
        assert_eq!(enc.restitution, partial);
        assert!(!enc.restituting);
        enc.hold_offering(&l, &t, at_tree, t.restitution_hold, &mut ev).unwrap();
        assert_eq!(enc.objective, Objective::Escape);
    }

    #[test]
    fn manifestation_is_always_at_a_safe_distance() {
        let l = Layout::authored();
        let t = Tuning::default();
        // Anywhere the player could stand when taking the satchel — and
        // anywhere in the paddock when he re-manifests after losing track.
        let mut x = l.bounds.min.x;
        while x <= l.bounds.max.x {
            let mut z = -56.0;
            while z <= 22.0 {
                let p = Vec2::new(x, z);
                let (_, d) = l.farthest_anchor(p);
                assert!(d >= t.manifest_min_distance, "unsafe manifestation {d} m from {p:?}");
                z += 2.0;
            }
            x += 2.0;
        }
        let (l, t, mut enc, mut ev) = setup();
        let at_table = Vec2::new(2.9, -3.3);
        enc.take_satchel(&l, &t, at_table, &mut ev).unwrap();
        assert!(enc.threat.pos.distance(at_table) >= t.manifest_min_distance);
    }

    #[test]
    fn warning_then_hunt_when_he_keeps_seeing_you() {
        let (l, t, mut enc, mut ev) = setup();
        let player = Vec2::new(0.0, 9.0);
        place_threat(&mut enc, &l, Vec2::new(14.0, 14.0));
        tick(&mut enc, &l, &t, player, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Warning);
        assert!(ev.contains(&Event::WarningBegan));
        run_for(&mut enc, &l, &t, player, t.warn_time + 0.1, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        assert!(ev.contains(&Event::HuntBegan));
        // Standing in the open: exposure fills and the encounter fails.
        let mut guard = 0;
        while enc.objective != Objective::Failed {
            tick(&mut enc, &l, &t, player, &mut ev);
            guard += 1;
            assert!(guard < 60 * 30, "never caught while exposed");
        }
        assert!(ev.contains(&Event::Caught));
        // Fair: the hunt lasted at least a few seconds before the catch.
        assert!(guard as f32 / 60.0 > 2.0);
    }

    #[test]
    fn breaking_sight_during_warning_averts_the_hunt() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, Vec2::new(14.0, 14.0));
        tick(&mut enc, &l, &t, Vec2::new(0.0, 9.0), &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Warning);
        // Duck inside the house, away from door and window lines.
        let hidden = Vec2::new(-3.8, -4.9);
        assert!(!l.line_of_sight(enc.threat.pos, hidden));
        run_for(&mut enc, &l, &t, hidden, t.warn_break_time + 0.1, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert!(ev.contains(&Event::WarningAverted));
        assert!(!ev.contains(&Event::HuntBegan));
        assert!(enc.threat.cooldown > 0.0);
    }

    #[test]
    fn breaking_sight_during_hunt_makes_him_lose_track_and_withdraw() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, Vec2::new(14.0, 14.0));
        let exposed = Vec2::new(0.0, 9.0);
        run_for(&mut enc, &l, &t, exposed, t.warn_time + 0.2, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        run_for(&mut enc, &l, &t, exposed, 1.0, &mut ev);
        let exposure_seen = enc.threat.exposure;
        assert!(exposure_seen > 0.0);

        let hidden = Vec2::new(-3.8, -4.9);
        assert!(!l.line_of_sight(enc.threat.pos, hidden));
        run_for(&mut enc, &l, &t, hidden, 1.0, &mut ev);
        // Still hunting, but no longer approaching and exposure recovering.
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        assert!(enc.threat.exposure < exposure_seen);
        let held = enc.threat.pos;
        run_for(&mut enc, &l, &t, hidden, 0.2, &mut ev);
        assert_eq!(enc.threat.pos, held);

        run_for(&mut enc, &l, &t, hidden, t.lose_track_time, &mut ev);
        assert!(ev.contains(&Event::LostTrack));
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert_eq!(enc.threat.exposure, 0.0);
        // He sinks and rises again far away, never on top of the player.
        run_for(&mut enc, &l, &t, hidden, t.sink_time + t.rise_time + 0.2, &mut ev);
        assert!(ev.contains(&Event::ThreatReturned));
        assert!(enc.threat.pos.distance(hidden) >= t.manifest_min_distance);
        assert_eq!(enc.threat.presence, Presence::Present);
        assert_ne!(enc.objective, Objective::Failed);
    }

    #[test]
    fn he_never_approaches_through_walls() {
        let (l, t, mut enc, mut ev) = setup();
        let hidden = Vec2::new(-3.8, -4.9);
        place_threat(&mut enc, &l, Vec2::new(14.0, 14.0));
        enc.threat.state = ThreatState::Hunting;
        let start = enc.threat.pos;
        run_for(&mut enc, &l, &t, hidden, 1.5, &mut ev);
        assert_eq!(enc.threat.pos, start);
        assert_eq!(enc.threat.exposure, 0.0);
    }

    #[test]
    fn restart_resets_all_truth_and_timers() {
        let (l, t, mut enc, mut ev) = setup();
        let pristine = enc.clone();
        enc.take_satchel(&l, &t, Vec2::new(2.9, -3.3), &mut ev).unwrap();
        run_for(&mut enc, &l, &t, Vec2::new(0.0, 9.0), 60.0, &mut ev);
        let at_tree = ground(l.ceiba.offering) + Vec2::new(1.0, 1.0);
        let _ = enc.hold_offering(&l, &t, at_tree, 1.0, &mut ev);
        assert_ne!(enc, pristine);
        enc.reset(&l);
        assert_eq!(enc, pristine);
        assert_eq!(enc.elapsed, 0.0);
        assert_eq!(enc.threat.exposure, 0.0);
        assert_eq!(enc.threat.cooldown, 0.0);
        assert_eq!(enc.stats, Stats::default());
    }
}
