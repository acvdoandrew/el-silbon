//! Host-only encounter authority. Reuses the offline collision and threat rules;
//! owns player poses, the single satchel, and all accepted objective changes.
use super::protocol::*;
use crate::{
    control::{Pose, Target, TargetKind},
    geometry::{AimStatus, Layout},
    perception::CueDirector,
    sim::{Encounter, Event, Movement, Objective, Presence, ThreatState},
    tuning::Tuning,
};
use bevy::math::{Vec2, Vec3};
use std::collections::BTreeMap;

pub struct Participant {
    pub pose: Pose,
    pub caught: bool,
    input: Input,
    input_age: f32,
    action_sequence: u64,
    cue: CueDirector,
    escape_armed: bool,
}

pub struct Session {
    pub encounter: Encounter,
    pub players: BTreeMap<PlayerId, Participant>,
    pub satchel: Satchel,
    pub run: u64,
    pub tick: u64,
    pub started: bool,
    target: Option<PlayerId>,
    serial: u64,
    pub outbox: Vec<(PlayerId, ServerMessage)>,
}

/// Same target evaluation on the host and client, with dynamic item placement.
pub fn target(
    layout: &Layout,
    tuning: &Tuning,
    pose: &Pose,
    satchel: Satchel,
    objective: Objective,
    id: PlayerId,
) -> Option<Target> {
    if objective.is_over() {
        return None;
    }
    let candidate = match satchel {
        Satchel::Ground(p) => Some((
            TargetKind::Satchel,
            Vec3::from_array(p),
            layout.satchel_radius,
            tuning.satchel_reach,
        )),
        Satchel::Carried(owner) if owner == id => Some((
            TargetKind::Offering,
            layout.ceiba.offering,
            layout.ceiba.offering_radius,
            tuning.offering_reach,
        )),
        _ => None,
    };
    for (kind, center, radius, reach) in candidate.into_iter().chain(std::iter::once((
        TargetKind::Note,
        layout.note,
        layout.note_radius,
        tuning.note_reach,
    ))) {
        let status = layout.aim(pose.eye(tuning), pose.look_dir(), center, radius, reach);
        if matches!(status, AimStatus::Ready { .. } | AimStatus::OutOfReach { .. }) {
            return Some(Target { kind, status });
        }
    }
    None
}

impl Session {
    pub fn new(layout: &Layout, tuning: &Tuning) -> Self {
        let mut s = Self {
            encounter: Encounter::new(layout),
            players: BTreeMap::new(),
            satchel: Satchel::Ground(layout.satchel.to_array()),
            run: 1,
            tick: 0,
            started: false,
            target: None,
            serial: 0,
            outbox: Vec::new(),
        };
        s.add_player(HOST, layout, tuning).expect("empty host session");
        s
    }
    pub fn add_player(&mut self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        if self.started {
            return Err("Encounter already started. Late joining is not supported; start a new host session.".into());
        }
        if self.players.len() >= MAX_PLAYERS {
            return Err("Session full (two-player checkpoint).".into());
        }
        if self.players.contains_key(&id) {
            return Err("Player identity already connected.".into());
        }
        let mut pose = Pose::spawn(layout);
        pose.pos.x += self.players.len() as f32 * 1.4;
        self.players.insert(
            id,
            Participant {
                pose,
                caught: false,
                input: Input::default(),
                input_age: 1.0,
                action_sequence: 0,
                cue: CueDirector::new(tuning.seed ^ id),
                escape_armed: false,
            },
        );
        Ok(())
    }
    pub fn remove_player(&mut self, id: PlayerId) {
        self.release(id);
        self.players.remove(&id);
        self.outbox.retain(|(recipient, _)| *recipient != id);
        if self.target == Some(id) {
            self.target = None;
            self.withdraw();
        }
        self.check_failure();
    }
    pub fn input(&mut self, id: PlayerId, input: Input, tuning: &Tuning) -> Result<(), String> {
        if input.run != self.run {
            return Err("Input belongs to an old run.".into());
        }
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        if input.sequence <= p.input.sequence {
            return Ok(());
        }
        if !input.axis.iter().all(|n| n.is_finite()) || !input.yaw.is_finite() || !input.pitch.is_finite() {
            return Err("Invalid movement input.".into());
        }
        p.input = input;
        p.input_age = 0.0;
        if !p.caught {
            p.pose.yaw = crate::control::wrap_angle(input.yaw);
            p.pose.pitch = input.pitch.clamp(-tuning.max_pitch, tuning.max_pitch);
        }
        Ok(())
    }
    pub fn command(
        &mut self,
        id: PlayerId,
        run: u64,
        sequence: u64,
        action: Action,
        layout: &Layout,
        tuning: &Tuning,
    ) -> Result<(), String> {
        if run != self.run {
            return Err("Action belongs to an old run.".into());
        }
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        if sequence <= p.action_sequence {
            return Err("Action was already processed.".into());
        }
        p.action_sequence = sequence;
        match action {
            Action::Start => {
                if id != HOST {
                    return Err("Only the host can start.".into());
                }
                if self.started {
                    return Err("Encounter already started.".into());
                }
                if self.players.len() != MAX_PLAYERS {
                    return Err("Waiting for the second player before starting.".into());
                }
                self.started = true;
                Ok(())
            }
            Action::Restart => {
                if id != HOST {
                    return Err("Only the host can restart.".into());
                }
                if !self.started {
                    return Err("Start the encounter first.".into());
                }
                self.restart(layout, tuning);
                Ok(())
            }
            Action::Take | Action::Drop => {
                if !self.started || self.encounter.objective.is_over() {
                    return Err("The encounter is not active.".into());
                }
                if p.caught {
                    return Err("You are caught until the host restarts.".into());
                }
                if matches!(action, Action::Drop) {
                    if self.satchel != Satchel::Carried(id) {
                        return Err("You do not carry the satchel.".into());
                    }
                    self.release(id);
                    return Ok(());
                }
                if !matches!(self.satchel, Satchel::Ground(_)) {
                    return Err("The satchel is already carried or returned.".into());
                }
                if !target(layout, tuning, &p.pose, self.satchel, self.encounter.objective, id)
                    .is_some_and(|t| t.kind == TargetKind::Satchel && t.ready())
                {
                    return Err("Aim at the satchel within reach and without a wall in the way.".into());
                }
                let pos = p.pose.pos;
                if self.encounter.objective == Objective::FindSatchel {
                    let mut events = Vec::new();
                    self.encounter
                        .take_satchel(layout, tuning, pos, &mut events)
                        .map_err(|e| format!("Pickup rejected: {e:?}"))?;
                    self.safe_manifestation(layout);
                }
                self.satchel = Satchel::Carried(id);
                self.emit(id, &[Event::SatchelTaken]);
                Ok(())
            }
        }
    }
    pub fn restart(&mut self, layout: &Layout, tuning: &Tuning) {
        self.run += 1;
        self.tick = 0;
        self.encounter.reset(layout);
        self.satchel = Satchel::Ground(layout.satchel.to_array());
        self.target = None;
        self.outbox.clear();
        for (index, (&id, p)) in self.players.iter_mut().enumerate() {
            p.pose = Pose::spawn(layout);
            p.pose.pos.x += index as f32 * 1.4;
            p.caught = false;
            p.input = Input::default();
            p.input_age = 1.0;
            p.action_sequence = 0;
            p.escape_armed = false;
            p.cue.reset(tuning.seed ^ id);
        }
    }
    fn release(&mut self, id: PlayerId) {
        if self.satchel == Satchel::Carried(id)
            && let Some(p) = self.players.get(&id)
        {
            // Player collision guarantees a reachable standing spot, not a wall.
            self.satchel = Satchel::Ground([p.pose.pos.x, 0.35, p.pose.pos.y]);
        }
    }
    fn emit(&mut self, id: PlayerId, events: &[Event]) {
        if events.is_empty() {
            return;
        }
        self.serial += 1;
        self.outbox.push((
            id,
            ServerMessage::Events {
                run: self.run,
                serial: self.serial,
                events: events.iter().map(|e| *e as u8).collect(),
            },
        ));
    }
    fn broadcast(&mut self, events: &[Event]) {
        let ids: Vec<_> = self.players.keys().copied().collect();
        for id in ids {
            self.emit(id, events);
        }
    }
    fn withdraw(&mut self) {
        let th = &mut self.encounter.threat;
        if th.state == ThreatState::Dormant || th.state == ThreatState::Resolved {
            return;
        }
        th.state = ThreatState::Stalking;
        th.state_time = 0.0;
        th.exposure = 0.0;
        th.unseen = 0.0;
        th.has_sight = false;
        th.cooldown = 8.0;
        th.movement = Movement::Still;
        th.presence = Presence::Sinking { t: 0.0, relocate: true };
    }
    fn check_failure(&mut self) {
        if self.started && !self.encounter.objective.is_over() && self.players.values().all(|p| p.caught) {
            self.encounter.objective = Objective::Failed;
        }
    }
    fn safe_manifestation(&mut self, layout: &Layout) {
        let index = (0..layout.ring.len())
            .max_by(|a, b| {
                let clearance = |i: usize| {
                    self.players
                        .values()
                        .filter(|p| !p.caught)
                        .map(|p| p.pose.pos.distance_squared(layout.ring[i]))
                        .fold(f32::INFINITY, f32::min)
                };
                clearance(*a).total_cmp(&clearance(*b))
            })
            .unwrap_or(0);
        self.encounter.threat.pos = layout.ring[index];
        self.encounter.threat.movement = Movement::AtAnchor(index);
    }
    pub fn step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        self.tick += 1;
        if !self.started || self.encounter.objective.is_over() {
            return;
        }
        let dt = dt.clamp(0.0, tuning.max_step);
        self.encounter.elapsed += dt;
        self.encounter.restituting = false;
        for (&id, p) in &mut self.players {
            p.input_age += dt;
            if p.caught || p.input_age > 0.3 {
                continue;
            }
            let speed = if self.satchel == Satchel::Carried(id) {
                tuning.carry_speed
            } else {
                tuning.walk_speed
            };
            p.pose.walk(layout, tuning, Vec2::from_array(p.input.axis), speed, dt);
        }
        if let Satchel::Carried(id) = self.satchel
            && let Some(p) = self.players.get(&id)
            && !p.caught
            && p.input_age < 0.3
            && p.input.hold
            && target(layout, tuning, &p.pose, self.satchel, self.encounter.objective, id)
                .is_some_and(|t| t.kind == TargetKind::Offering && t.ready())
        {
            let mut events = Vec::new();
            let _ = self
                .encounter
                .hold_offering(layout, tuning, p.pose.pos, dt, &mut events);
            if self.encounter.objective == Objective::Escape {
                self.satchel = Satchel::Returned;
            }
            self.broadcast(&events);
        }
        let locked = matches!(self.encounter.threat.state, ThreatState::Warning | ThreatState::Hunting)
            && self
                .target
                .is_some_and(|id| self.players.get(&id).is_some_and(|p| !p.caught));
        if !locked {
            self.target = match self.satchel {
                Satchel::Carried(id) => Some(id),
                _ => self
                    .players
                    .iter()
                    .filter(|(_, p)| !p.caught)
                    .min_by(|(_, a), (_, b)| {
                        let score = |p: &Participant| {
                            p.pose.pos.distance(self.encounter.threat.pos)
                                + if layout.line_of_sight(p.pose.pos, self.encounter.threat.pos) {
                                    0.0
                                } else {
                                    100.0
                                }
                        };
                        score(a).total_cmp(&score(b))
                    })
                    .map(|(&id, _)| id),
            };
        }
        if let Some(id) = self.target
            && let Some(p) = self.players.get(&id)
        {
            let mut events = Vec::new();
            let objective = self.encounter.objective;
            self.encounter
                .update_threat(layout, tuning, p.pose.pos, dt, &mut events);
            if events.contains(&Event::ThreatReturned) {
                self.safe_manifestation(layout);
            }
            if events.contains(&Event::Caught) {
                // Offline threat rule reports capture via Failed; shared failure
                // is instead computed from all connected, active participants.
                self.encounter.objective = objective;
                self.release(id);
                if let Some(p) = self.players.get_mut(&id) {
                    p.caught = true;
                    p.input = Input::default();
                }
                self.withdraw();
                self.target = None;
            }
            self.emit(id, &events);
        }
        self.check_failure();
        if self.encounter.objective == Objective::Escape {
            for p in self.players.values_mut() {
                if !p.caught && !layout.in_road_goal(p.pose.pos) {
                    p.escape_armed = true;
                }
            }
        }
        if self.encounter.objective == Objective::Escape
            && self
                .players
                .values()
                .any(|p| !p.caught && p.escape_armed && layout.in_road_goal(p.pose.pos))
        {
            self.encounter.objective = Objective::Won;
            self.broadcast(&[Event::Escaped]);
        }
        for (&id, p) in &mut self.players {
            if !p.caught
                && let Some(cue) = p.cue.tick(dt, &self.encounter, p.pose.pos, tuning)
            {
                self.serial += 1;
                self.outbox.push((
                    id,
                    ServerMessage::Cue {
                        run: self.run,
                        serial: self.serial,
                        variant: cue.variant as u8,
                        speed: cue.speed,
                    },
                ));
            }
        }
    }
    pub fn snapshot(&self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Snapshot {
        let local = &self.players[&id];
        let th = &self.encounter.threat;
        let towards = th.pos - local.pose.pos;
        // A slightly wider than camera cone accommodates the visible body's
        // extent, but no transforms are sent for enemies behind walls or behind
        // this listener. Host trust and previously seen positions are not hidden.
        let visible = !local.caught
            && th.visibility(tuning) > 0.0
            && towards.length() < 75.0
            && layout.line_of_sight(local.pose.pos, th.pos)
            && local.pose.forward2().dot(towards.normalize_or(Vec2::Y)) > 0.55;
        let danger = if self.target != Some(id) || local.caught {
            0
        } else {
            match th.state {
                ThreatState::Warning => 1,
                ThreatState::Hunting if th.has_sight => 2,
                ThreatState::Hunting => 3,
                _ => 0,
            }
        };
        Snapshot {
            run: self.run,
            tick: self.tick,
            started: self.started,
            players: self
                .players
                .iter()
                .map(|(&id, p)| PlayerView {
                    id,
                    position: p.pose.pos.to_array(),
                    yaw: p.pose.yaw,
                    pitch: p.pose.pitch,
                    caught: p.caught,
                })
                .collect(),
            satchel: self.satchel,
            objective: self.encounter.objective as u8,
            restitution: self.encounter.restitution,
            restituting: self.encounter.restituting,
            elapsed: self.encounter.elapsed,
            stats: [
                self.encounter.stats.warnings,
                self.encounter.stats.hunts,
                self.encounter.stats.recoveries,
            ],
            threat: visible.then(|| VisibleThreat {
                position: th.pos.to_array(),
                facing: th.facing.to_array(),
                speed: th.speed,
                visibility: th.visibility(tuning),
                state: th.state as u8,
            }),
            danger,
            exposure: if danger > 0 { th.exposure } else { 0.0 },
        }
    }
}
