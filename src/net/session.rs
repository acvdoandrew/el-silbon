//! Host-only run authority for one to four players. Solo play is a session
//! of one, hosted play a session of several: the rules are the same.
//!
//! The session owns player poses and bodies, bone-bundle ownership, every
//! accepted interaction, noise, the one threat and the outcome. Clients send
//! inputs and actions; they never write truth.
use super::protocol::*;
use crate::{
    body::{Body, BodyInput, FearInput, Ground, Status},
    control::{Pose, SceneData, TargetKind, evaluate_target},
    geometry::{Layout, district::SurfaceKind, ground},
    perception::CueDirector,
    sim::{Encounter, Event, Outcome, Presence, Prey, Relic, ThreatState},
    storm,
    tuning::Tuning,
};
use bevy::math::{Vec2, Vec3};
use std::collections::BTreeMap;

/// Where each player stands relative to the spawn point, by join order.
const SPAWN_OFFSETS: [f32; 4] = [0.0, 1.4, -1.4, 2.8];

pub struct Participant {
    pub pose: Pose,
    pub status: Status,
    pub body: Body,
    /// Peppers held.
    pub aji: u8,
    pub light: bool,
    /// Revive progress while downed, 0..1.
    pub revive: f32,
    /// Progress of the current hold, and what it is (see `Vitals`).
    pub hold: f32,
    pub hold_kind: u8,
    input: Input,
    input_age: f32,
    action_sequence: u64,
    cue: CueDirector,
    ground_height: f32,
    ping_wait: f32,
    prayed: bool,
    praying: bool,
    /// Noise pulse timers: pump, ignition, prayer, revive.
    pulse: [f32; 4],
}

impl Participant {
    fn new(pose: Pose, seed: u64, layout: &Layout) -> Self {
        Self {
            ground_height: layout.surface_height(pose.pos),
            pose,
            status: Status::Active,
            body: Body::default(),
            aji: 0,
            light: true,
            revive: 0.0,
            hold: 0.0,
            hold_kind: 0,
            input: Input::default(),
            input_age: 1.0,
            action_sequence: 0,
            cue: CueDirector::new(seed),
            ping_wait: 0.0,
            prayed: false,
            praying: false,
            pulse: [0.0; 4],
        }
    }

    /// Down or dead: not on their feet.
    pub fn is_down(&self) -> bool {
        !self.status.is_active()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ping {
    pub pos: Vec3,
    pub left: f32,
    pub by: PlayerId,
}

pub struct Session {
    pub encounter: Encounter,
    pub players: BTreeMap<PlayerId, Participant>,
    pub pings: Vec<Ping>,
    pub run: u64,
    pub tick: u64,
    pub started: bool,
    target: Option<PlayerId>,
    serial: u64,
    beacon_pulse: f32,
    noises: Vec<(Vec2, f32)>,
    events: Vec<(Option<PlayerId>, Event)>,
    pub outbox: Vec<(PlayerId, ServerMessage)>,
}

impl Session {
    pub fn new(layout: &Layout, tuning: &Tuning) -> Self {
        let mut s = Self {
            encounter: Encounter::new(layout),
            players: BTreeMap::new(),
            pings: Vec::new(),
            run: 1,
            tick: 0,
            started: false,
            target: None,
            serial: 0,
            beacon_pulse: 0.0,
            noises: Vec::new(),
            events: Vec::new(),
            outbox: Vec::new(),
        };
        s.add_player(HOST, layout, tuning).expect("empty host session");
        s
    }

    fn spawn_pose(layout: &Layout, index: usize) -> Pose {
        let mut pose = Pose::spawn(layout);
        pose.pos.x += SPAWN_OFFSETS[index.min(SPAWN_OFFSETS.len() - 1)];
        pose
    }

    pub fn add_player(&mut self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        if self.started {
            return Err("The run already started. Late joining is not supported; start a new host session.".into());
        }
        if self.players.len() >= MAX_PLAYERS {
            return Err(format!("Session full ({MAX_PLAYERS} players)."));
        }
        if self.players.contains_key(&id) {
            return Err("Player identity already connected.".into());
        }
        let pose = Self::spawn_pose(layout, self.players.len());
        self.players
            .insert(id, Participant::new(pose, tuning.seed ^ id, layout));
        Ok(())
    }

    pub fn remove_player(&mut self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            let at = Vec3::new(p.pose.pos.x, p.ground_height, p.pose.pos.y);
            self.encounter.release_all(id, at);
        }
        self.players.remove(&id);
        self.outbox.retain(|(recipient, _)| *recipient != id);
        self.pings.retain(|p| p.by != id);
        if self.target == Some(id) {
            self.target = None;
            self.encounter.withdraw();
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
        if !matches!(p.status, Status::Dead) {
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
                    return Err("The run already started.".into());
                }
                self.started = true;
                Ok(())
            }
            Action::Restart => {
                if id != HOST {
                    return Err("Only the host can restart.".into());
                }
                if !self.started {
                    return Err("Start the run first.".into());
                }
                self.restart(layout, tuning);
                Ok(())
            }
            Action::Ping { at } => self.ping(id, at, tuning),
            _ => {
                if !self.started || self.encounter.outcome.is_over() {
                    return Err("The run is not active.".into());
                }
                let p = &self.players[&id];
                if !p.status.is_active() {
                    return Err("You are down.".into());
                }
                if p.body.stun > 0.0 {
                    return Err("You are frozen with fear.".into());
                }
                match action {
                    Action::Drop => self.drop_bundle(id, tuning),
                    Action::UseAji => self.use_aji(id, tuning),
                    _ => self.interact(id, layout, tuning),
                }
            }
        }
    }

    fn ping(&mut self, id: PlayerId, at: [f32; 3], tuning: &Tuning) -> Result<(), String> {
        if !self.started || self.encounter.outcome.is_over() {
            return Err("The run is not active.".into());
        }
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        if matches!(p.status, Status::Dead) {
            return Err("You are dead.".into());
        }
        if p.ping_wait > 0.0 {
            return Err("Wait a moment before marking again.".into());
        }
        if !at.iter().all(|n| n.is_finite()) {
            return Err("Invalid ping.".into());
        }
        let pos = Vec3::from_array(at);
        if ground(pos).distance(p.pose.pos) > tuning.ping_range {
            return Err("That is too far to mark.".into());
        }
        p.ping_wait = tuning.ping_cooldown;
        self.pings.retain(|q| q.by != id);
        self.pings.push(Ping {
            pos,
            left: tuning.ping_life,
            by: id,
        });
        Ok(())
    }

    /// What the crosshair may act on for `id`, built from truth.
    pub fn scene_data(&self, id: PlayerId) -> SceneData {
        let p = &self.players[&id];
        let progress = &self.encounter.progress;
        SceneData {
            relics: progress.relics.clone(),
            aji_taken: progress.aji_taken.clone(),
            power_on: progress.power_on(),
            truck_running: progress.truck_running(),
            bones_home: progress.bones_home(),
            beacon_ready: progress.beacon_ready(),
            carrying: progress.carried_by(id),
            aji_held: p.aji,
            bodies: self
                .players
                .iter()
                .filter(|(_, q)| q.status.is_downed())
                .map(|(&qid, q)| (qid, q.pose.pos))
                .collect(),
            me: id,
            acting: p.status.is_active() && p.body.stun <= 0.0 && self.started && !self.encounter.outcome.is_over(),
        }
    }

    fn active_positions(&self) -> Vec<Vec2> {
        self.players
            .values()
            .filter(|p| p.status.is_active())
            .map(|p| p.pose.pos)
            .collect()
    }

    fn interact(&mut self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        let data = self.scene_data(id);
        let pose = self.players[&id].pose;
        let target = evaluate_target(layout, tuning, &pose, &data.scene())
            .filter(|t| t.usable() && !t.kind.is_hold())
            .ok_or("Aim at something within reach and without a wall in the way.")?;
        let mut ev = Vec::new();
        match target.kind {
            TargetKind::Relic(i) => {
                if !self.encounter.take_relic(i as usize, id, &mut ev) {
                    return Err("That bundle is already taken.".into());
                }
                if self.encounter.threat.state == ThreatState::Dormant {
                    let watchers = self.active_positions();
                    self.encounter.manifest(layout, tuning, &watchers, &mut ev);
                }
                self.noises.push((pose.pos, tuning.noise_pickup));
            }
            TargetKind::Aji(i) => {
                if !self.encounter.take_aji(i as usize, &mut ev) {
                    return Err("That pepper is already taken.".into());
                }
                if let Some(p) = self.players.get_mut(&id) {
                    p.aji += 1;
                }
                // Only the finder hears about it.
                self.events.extend(ev.drain(..).map(|e| (Some(id), e)));
            }
            TargetKind::Note(_) => {}
            _ => return Err("Hold interact to use that.".into()),
        }
        self.events.extend(ev.into_iter().map(|e| (None, e)));
        self.flush_events();
        Ok(())
    }

    fn drop_bundle(&mut self, id: PlayerId, tuning: &Tuning) -> Result<(), String> {
        let p = &self.players[&id];
        let at = Vec3::new(p.pose.pos.x, p.ground_height, p.pose.pos.y);
        let mut ev = Vec::new();
        if !self.encounter.drop_relic(id, at, &mut ev) {
            return Err("You carry nothing to put down.".into());
        }
        self.noises.push((p.pose.pos, tuning.noise_drop));
        self.events.extend(ev.into_iter().map(|e| (None, e)));
        self.flush_events();
        Ok(())
    }

    fn use_aji(&mut self, id: PlayerId, tuning: &Tuning) -> Result<(), String> {
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        if p.aji == 0 {
            return Err("You have no pepper.".into());
        }
        p.aji -= 1;
        let at = p.pose.pos + p.pose.forward2() * 1.6;
        let pos = p.pose.pos;
        let mut ev = Vec::new();
        self.encounter.place_aji(tuning, at, &mut ev);
        self.noises.push((pos, tuning.noise_aji));
        self.events.extend(ev.into_iter().map(|e| (None, e)));
        self.flush_events();
        Ok(())
    }

    pub fn restart(&mut self, layout: &Layout, tuning: &Tuning) {
        self.run += 1;
        self.tick = 0;
        self.encounter.reset(layout);
        self.target = None;
        self.outbox.clear();
        self.pings.clear();
        self.events.clear();
        self.noises.clear();
        self.beacon_pulse = 0.0;
        for (index, (&id, p)) in self.players.iter_mut().enumerate() {
            *p = Participant::new(Self::spawn_pose(layout, index), tuning.seed ^ id, layout);
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

    /// Send the queued events: `None` recipients go to everyone.
    fn flush_events(&mut self) {
        let events = std::mem::take(&mut self.events);
        if events.is_empty() {
            return;
        }
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let mine: Vec<Event> = events
                .iter()
                .filter(|(to, _)| to.is_none_or(|t| t == id))
                .map(|(_, e)| *e)
                .collect();
            self.emit(id, &mine);
        }
    }

    fn check_failure(&mut self) {
        if self.started
            && !self.encounter.outcome.is_over()
            && !self.players.is_empty()
            && self.players.values().all(|p| !p.status.is_active())
        {
            self.encounter.outcome = Outcome::Failed;
        }
    }

    // ------------------------------------------------------------------ step

    pub fn step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        self.tick += 1;
        if !self.started || self.encounter.outcome.is_over() {
            return;
        }
        let dt = dt.clamp(0.0, tuning.max_step);
        self.encounter.tick_world(tuning, dt);
        for p in &mut self.pings {
            p.left -= dt;
        }
        self.pings.retain(|p| p.left > 0.0);
        self.move_players(layout, tuning, dt);
        self.resolve_holds(layout, tuning, dt);
        self.world_noise(layout, tuning, dt);
        self.threat_step(layout, tuning, dt);
        self.fear_step(layout, tuning, dt);
        self.outcome_step(layout, tuning);
        self.flush_events();
        self.cues(tuning, dt);
    }

    fn move_players(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let carried = self.encounter.progress.carried_by(id);
            let Some(p) = self.players.get_mut(&id) else {
                continue;
            };
            p.input_age += dt;
            p.ping_wait = (p.ping_wait - dt).max(0.0);
            for t in &mut p.pulse {
                *t = (*t - dt).max(0.0);
            }
            if let Status::Downed { bleed } = &mut p.status {
                *bleed -= dt;
                if *bleed <= 0.0 {
                    p.status = Status::Dead;
                    p.revive = 0.0;
                    self.events.push((None, Event::Died));
                }
            }
            if matches!(p.status, Status::Dead) {
                p.pose.lower = tuning.downed_lower;
                continue;
            }
            let fresh = p.input_age <= 0.3;
            let downed = p.status.is_downed();
            let pos = p.pose.pos;
            let ground_kind = Ground {
                wading: layout.wading(pos),
                planks: layout
                    .district
                    .surface_at(pos)
                    .is_some_and(|s| s.kind == SurfaceKind::Boardwalk),
            };
            let axis = if fresh {
                Vec2::from_array(p.input.axis)
            } else {
                Vec2::ZERO
            };
            let speed = p.body.speed(tuning, carried, ground_kind, downed);
            let moved = p.pose.walk(layout, tuning, axis, speed, dt);
            let input = BodyInput {
                crouch: fresh && p.input.crouch,
                sprint: fresh && p.input.sprint,
                moved,
            };
            if let Some(radius) = p.body.advance(tuning, input, carried, ground_kind, downed, dt) {
                self.noises.push((p.pose.pos, radius));
            }
            p.pose.lower = if downed {
                tuning.downed_lower
            } else if p.body.crouching {
                tuning.crouch_lower
            } else {
                0.0
            };
            p.light = p.input.light;
            p.ground_height = layout.surface_height(p.pose.pos);
        }
    }

    fn resolve_holds(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        let mut reviving: Vec<PlayerId> = Vec::new();
        for id in ids {
            let target = {
                let p = &self.players[&id];
                if p.input.hold && p.input_age <= 0.3 && p.status.is_active() && p.body.stun <= 0.0 {
                    let data = self.scene_data(id);
                    evaluate_target(layout, tuning, &p.pose, &data.scene()).filter(|t| t.usable() && t.kind.is_hold())
                } else {
                    None
                }
            };
            let carried = self.encounter.progress.carried_by(id);
            let mut kind = 0u8;
            let mut progress = None;
            let mut ev = Vec::new();
            if let Some(t) = target {
                let p = self.players.get_mut(&id).expect("id from keys");
                let pos = p.pose.pos;
                match t.kind {
                    TargetKind::Altar if carried > 0 => {
                        kind = 1;
                        if p.hold_kind != kind {
                            p.hold = 0.0;
                        }
                        p.hold += dt / tuning.deliver_hold;
                        if p.hold >= 1.0 {
                            p.hold = 0.0;
                            self.encounter.deliver_relic(id, &mut ev);
                            self.noises.push((ground(layout.ceiba.offering), tuning.noise_altar));
                        }
                        progress = Some(p.hold);
                    }
                    TargetKind::Altar => {
                        kind = 2;
                        if p.hold_kind != kind {
                            p.hold = 0.0;
                            p.prayed = false;
                        }
                        p.praying = true;
                        p.hold = (p.hold + dt / tuning.pray_hold).min(1.0);
                        if p.pulse[2] <= 0.0 {
                            p.pulse[2] = 2.0;
                            self.noises.push((pos, tuning.noise_altar));
                        }
                        if p.hold >= 1.0 && !p.prayed {
                            p.prayed = true;
                            p.body.calm_to(0.0);
                            self.events.push((Some(id), Event::Prayed));
                        }
                        progress = Some(p.hold);
                    }
                    TargetKind::Pump => {
                        kind = 3;
                        self.encounter.work_pump(tuning, dt, &mut ev);
                        if p.pulse[0] <= 0.0 {
                            p.pulse[0] = 1.5;
                            self.noises.push((ground(layout.district.pump), tuning.noise_pump));
                        }
                        progress = Some(self.encounter.progress.power);
                    }
                    TargetKind::Ignition => {
                        kind = 4;
                        self.encounter.work_truck(tuning, dt, &mut ev);
                        if p.pulse[1] <= 0.0 {
                            p.pulse[1] = 1.0;
                            self.noises.push((ground(layout.district.ignition), tuning.noise_truck));
                        }
                        progress = Some(self.encounter.progress.truck);
                    }
                    TargetKind::Beacon => {
                        kind = 5;
                        if p.hold_kind != kind {
                            p.hold = 0.0;
                        }
                        p.hold += dt / tuning.beacon_hold;
                        if p.hold >= 1.0 {
                            p.hold = 0.0;
                            self.encounter.light_beacon(tuning, &mut ev);
                        }
                        progress = Some(p.hold);
                    }
                    TargetKind::Body(victim_id) => {
                        kind = 6;
                        if p.pulse[3] <= 0.0 {
                            p.pulse[3] = 2.0;
                            self.noises.push((pos, tuning.noise_revive));
                        }
                        reviving.push(victim_id);
                        if let Some(v) = self.players.get_mut(&victim_id) {
                            v.revive += dt / tuning.revive_hold;
                            if v.revive >= 1.0 && v.status.is_downed() {
                                v.revive = 0.0;
                                v.status = Status::Active;
                                v.body.calm_to(0.5);
                                v.body.stamina = 1.0;
                                self.encounter.stats.revives += 1;
                                ev.push(Event::Revived);
                            }
                            progress = Some(v.revive);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.praying = kind == 2;
                p.hold_kind = kind;
                if kind == 0 {
                    p.hold = 0.0;
                    p.prayed = false;
                } else if let Some(v) = progress {
                    p.hold = v;
                }
            }
            if ev.contains(&Event::TruckStarted) {
                let watchers = self.active_positions();
                let mut more = Vec::new();
                self.encounter.rouse(layout, tuning, &watchers, &mut more);
                ev.extend(more);
            }
            self.events.extend(ev.into_iter().map(|e| (None, e)));
        }
        for (id, p) in &mut self.players {
            if p.status.is_downed() && !reviving.contains(id) {
                p.revive = (p.revive - 0.5 * dt).max(0.0);
            }
        }
    }

    /// Cattle, and the burning beacon: noises no player is making right now.
    fn world_noise(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let cows: Vec<Vec2> = layout.district.cows().map(|c| c.center).collect();
        if !cows.is_empty() {
            let herd = cows.iter().copied().sum::<Vec2>() / cows.len() as f32;
            let mut unrest = 0.0;
            for (&id, p) in &self.players {
                if !p.status.is_active() || p.body.crouching || p.pose.pos.distance(herd) > tuning.cattle_radius {
                    continue;
                }
                let moving = p.input.axis != [0.0; 2] && p.input_age <= 0.3;
                if !moving {
                    continue;
                }
                let carried = self.encounter.progress.carried_by(id) as f32;
                unrest += if p.body.sprinting { 1.6 } else { 0.4 } + 0.3 * carried;
            }
            let prog = &mut self.encounter.progress;
            prog.cattle_spook += unrest * dt;
            if prog.cattle_spook >= 1.0 && prog.cattle_cooldown <= 0.0 {
                prog.cattle_spook = 0.0;
                prog.cattle_alarm = tuning.cattle_alarm;
                prog.cattle_cooldown = tuning.cattle_cooldown;
                self.noises.push((herd, tuning.noise_cattle));
                self.events.push((None, Event::CattleSpooked));
            } else if prog.cattle_cooldown > 0.0 {
                prog.cattle_spook = prog.cattle_spook.min(0.9);
            }
        }
        if self.encounter.progress.beacon > 0.0 {
            self.beacon_pulse -= dt;
            if self.beacon_pulse <= 0.0 {
                self.beacon_pulse = 0.5;
                self.noises.push((ground(layout.district.beacon), tuning.noise_beacon));
            }
        }
    }

    fn choose_prey(&mut self, layout: &Layout) -> Option<PlayerId> {
        let th = &self.encounter.threat;
        let locked = matches!(th.state, ThreatState::Warning | ThreatState::Hunting)
            && self
                .target
                .is_some_and(|id| self.players.get(&id).is_some_and(|p| p.status.is_active()));
        if !locked {
            let progress = &self.encounter.progress;
            self.target = self
                .players
                .iter()
                .filter(|(_, p)| p.status.is_active())
                .min_by(|(ia, a), (ib, b)| {
                    let score = |id: &PlayerId, p: &Participant| {
                        p.pose.pos.distance(th.pos)
                            + if layout.line_of_sight(p.pose.pos, th.pos) {
                                0.0
                            } else {
                                100.0
                            }
                            - 8.0 * p.body.fear
                            - 10.0 * progress.carried_by(*id) as f32
                    };
                    score(ia, a).total_cmp(&score(ib, b))
                })
                .map(|(&id, _)| id);
        }
        let id = self.target?;
        if !self.players.get(&id)?.status.is_active() {
            self.target = None;
            return None;
        }
        Some(id)
    }

    fn threat_step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let masking = storm::masking(tuning, self.encounter.elapsed);
        for (pos, radius) in std::mem::take(&mut self.noises) {
            self.encounter.hear(tuning, pos, radius * masking);
        }
        let watchers = self.active_positions();
        let target = self.choose_prey(layout);
        let prey = target.and_then(|id| self.players.get(&id)).map(|p| Prey {
            pos: p.pose.pos,
            sight: (if p.body.crouching { tuning.sight_crouch } else { 1.0 })
                * (if p.light { tuning.sight_light } else { 1.0 }),
            concealed: p.body.crouching && layout.district.tall_grass_at(p.pose.pos),
        });
        let mut ev = Vec::new();
        self.encounter
            .update_threat(layout, tuning, prey, &watchers, dt, &mut ev);
        for e in ev {
            match e {
                Event::Downed => {
                    if let Some(id) = target {
                        self.down(id, tuning);
                    }
                    self.events.push((None, e));
                }
                Event::WarningBegan => {
                    if let Some(id) = target {
                        self.startle(id, tuning, tuning.fear_warn);
                        self.events.push((Some(id), e));
                    }
                }
                Event::HuntBegan | Event::WarningAverted | Event::LostTrack | Event::ThreatReturned => {
                    if let Some(id) = target {
                        self.events.push((Some(id), e));
                    }
                }
                _ => self.events.push((None, e)),
            }
        }
        if matches!(self.encounter.threat.state, ThreatState::Dormant) {
            self.target = None;
        }
    }

    /// The hunted player falls: they drop what they carry and wait for help.
    fn down(&mut self, id: PlayerId, tuning: &Tuning) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let at = Vec3::new(p.pose.pos.x, p.ground_height, p.pose.pos.y);
        p.status = Status::Downed {
            bleed: tuning.bleed_out,
        };
        p.body.sprinting = false;
        p.revive = 0.0;
        p.hold = 0.0;
        p.hold_kind = 0;
        self.encounter.release_all(id, at);
        self.target = None;
    }

    fn startle(&mut self, id: PlayerId, tuning: &Tuning, amount: f32) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if p.status.is_active() && p.body.startle(tuning, amount) {
            self.susto(id, tuning);
        }
    }

    fn susto(&mut self, id: PlayerId, tuning: &Tuning) {
        if let Some(p) = self.players.get(&id) {
            self.noises.push((p.pose.pos, tuning.noise_susto));
        }
        self.events.push((Some(id), Event::Susto));
    }

    fn fear_step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let power = self.encounter.progress.power_on();
        let active: Vec<(PlayerId, Vec2)> = self
            .players
            .iter()
            .filter(|(_, p)| p.status.is_active())
            .map(|(&id, p)| (id, p.pose.pos))
            .collect();
        let th = &self.encounter.threat;
        let hunted = if th.has_sight && matches!(th.state, ThreatState::Warning | ThreatState::Hunting) {
            self.target
        } else {
            None
        };
        let exposure = th.exposure;
        let mut frightened = Vec::new();
        for &(id, pos) in &active {
            let near = active
                .iter()
                .any(|&(other, at)| other != id && at.distance(pos) <= tuning.team_radius);
            let carried = self.encounter.progress.carried_by(id);
            let Some(p) = self.players.get_mut(&id) else {
                continue;
            };
            let input = FearInput {
                lit: layout.is_lit(pos, power),
                teammate_near: near,
                isolated: active.len() > 1 && !near,
                carried,
                praying: p.praying,
                exposure: if hunted == Some(id) { exposure } else { 0.0 },
            };
            if p.body.tick_fear(tuning, input, dt) {
                frightened.push(id);
            }
        }
        for id in frightened {
            self.susto(id, tuning);
        }
    }

    fn outcome_step(&mut self, layout: &Layout, tuning: &Tuning) {
        self.check_failure();
        if self.encounter.outcome.is_over() || !self.encounter.progress.truck_ready(tuning) {
            return;
        }
        let zone = layout.district.truck;
        let downed = self.players.values().any(|p| p.status.is_downed());
        let active: Vec<&Participant> = self.players.values().filter(|p| p.status.is_active()).collect();
        if !downed
            && !active.is_empty()
            && active
                .iter()
                .all(|p| p.pose.pos.distance(zone.zone_center) <= zone.zone_radius)
        {
            self.encounter.outcome = Outcome::Won;
            self.events.push((None, Event::Escaped));
        }
    }

    fn cues(&mut self, tuning: &Tuning, dt: f32) {
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let Some(p) = self.players.get_mut(&id) else {
                continue;
            };
            if matches!(p.status, Status::Dead) {
                continue;
            }
            let Some(cue) = p.cue.tick(dt, &self.encounter, p.pose.pos, tuning) else {
                continue;
            };
            let frightened = p.status.is_active() && p.body.startle(tuning, cue.variant.dread(tuning));
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
            if frightened {
                self.susto(id, tuning);
                self.flush_events();
            }
        }
    }

    // -------------------------------------------------------------- snapshot

    pub fn snapshot(&self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Snapshot {
        let local = &self.players[&id];
        let th = &self.encounter.threat;
        let progress = &self.encounter.progress;
        let towards = th.pos - local.pose.pos;
        // A slightly wider than camera cone accommodates the visible body's
        // extent, but no transforms are sent for enemies behind walls or behind
        // this listener. From a tower deck the whole llano opens up. Host trust
        // and previously seen positions are not hidden.
        let range = if local.ground_height > 3.0 { 130.0 } else { 75.0 };
        let visible = !matches!(local.status, Status::Dead)
            && th.visibility(tuning) > 0.0
            && towards.length() < range
            && layout.line_of_sight(local.pose.pos, th.pos)
            && local.pose.forward2().dot(towards.normalize_or(Vec2::Y)) > 0.55;
        let danger = if th.state == ThreatState::Counting && visible {
            4
        } else if self.target != Some(id) || !local.status.is_active() {
            0
        } else {
            match th.state {
                ThreatState::Warning => 1,
                ThreatState::Hunting if th.has_sight => 2,
                ThreatState::Hunting => 3,
                _ => 0,
            }
        };
        let downed_or_dead = |p: &Participant| p.status.code();
        Snapshot {
            run: self.run,
            tick: self.tick,
            started: self.started,
            players: self
                .players
                .iter()
                .map(|(&pid, p)| PlayerView {
                    id: pid,
                    position: p.pose.pos.to_array(),
                    yaw: p.pose.yaw,
                    pitch: p.pose.pitch,
                    status: downed_or_dead(p),
                    crouch: p.body.crouching,
                    sprint: p.body.sprinting,
                    light: p.light,
                    carrying: progress.carried_by(pid) as u8,
                    revive: p.revive,
                    bleed: match p.status {
                        Status::Downed { bleed } => bleed,
                        _ => 0.0,
                    },
                })
                .collect(),
            relics: progress.relics.iter().map(RelicView::from_relic).collect(),
            aji: progress.aji_taken.clone(),
            outcome: outcome_code(self.encounter.outcome),
            world: WorldView {
                delivered: progress.delivered() as u8,
                total: progress.relics.len() as u8,
                power: progress.power,
                truck: progress.truck,
                warm: (progress.warm / tuning.truck_warmup).clamp(0.0, 1.0),
                beacon: progress.beacon,
                beacon_ready: progress.beacon_ready(),
                cattle: progress.cattle_alarm,
                night: (self.encounter.elapsed / tuning.night_length).clamp(0.0, 1.0),
            },
            elapsed: self.encounter.elapsed,
            stats: [
                self.encounter.stats.warnings,
                self.encounter.stats.hunts,
                self.encounter.stats.recoveries,
                self.encounter.stats.downs,
                self.encounter.stats.revives,
            ],
            threat: visible.then(|| VisibleThreat {
                position: th.pos.to_array(),
                facing: th.facing.to_array(),
                speed: th.speed,
                visibility: th.visibility(tuning),
                state: threat_code(th.state),
            }),
            danger,
            exposure: if matches!(danger, 1..=3) { th.exposure } else { 0.0 },
            me: Vitals {
                fear: local.body.fear,
                stamina: local.body.stamina,
                aji: local.aji,
                stun: local.body.stun,
                hold_kind: local.hold_kind,
                hold: local.hold,
            },
            zones: self
                .encounter
                .zones
                .iter()
                .map(|z| [z.pos.x, z.pos.y, z.life])
                .collect(),
            pings: self
                .pings
                .iter()
                .map(|p| PingView {
                    pos: p.pos.to_array(),
                    left: p.left,
                    by: p.by,
                })
                .collect(),
        }
    }

    /// A short human summary of where the bundles are (logging).
    pub fn relic_summary(&self) -> String {
        self.encounter
            .progress
            .relics
            .iter()
            .map(|r| match r {
                Relic::Ground(_) => 'g',
                Relic::Carried(_) => 'c',
                Relic::Delivered => 'd',
            })
            .collect()
    }

    /// Is he physically standing in the world right now?
    pub fn threat_present(&self) -> bool {
        matches!(
            self.encounter.threat.presence,
            Presence::Present | Presence::Rising { .. }
        )
    }
}
