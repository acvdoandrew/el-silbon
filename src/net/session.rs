//! Host-only run authority for one to four players. Solo play is a session
//! of one, hosted play a session of several: the rules are the same.
//!
//! The session owns player poses and bodies, bone-bundle ownership, every
//! accepted interaction, noise, the one threat and the outcome. Clients send
//! inputs and actions; they never write truth.
use super::protocol::*;
use crate::awards::Deeds;
use crate::survivor::Survivor;
use crate::{
    body::{Body, BodyInput, FearInput, Ground, Status},
    control::{Pose, SceneData, TargetKind, evaluate_target},
    director::{Director, Mood, Omen},
    geometry::{Layout, district::SurfaceKind, ground},
    pacing::{self, Inputs, Menace, Respiro},
    perception::{CueDirector, DogSense, dog_senses},
    rng::Rng,
    sim::{Encounter, Event, Outcome, Presence, Prey, Relic, ThreatState, Variant},
    skill::{LAST_TURN, Pulse, Rhythm, Verdict},
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
    /// The torch is shining: switched on, with charge left.
    pub light: bool,
    /// Torch charge, 0..1.
    pub battery: f32,
    /// Revive progress while downed, 0..1.
    pub revive: f32,
    /// Progress of the current hold, and what it is (see `Vitals`).
    pub hold: f32,
    pub hold_kind: u8,
    /// Skill checks while working a long task.
    pub rhythm: Rhythm,
    /// The night's omens for this player.
    director: Director,
    input: Input,
    input_age: f32,
    action_sequence: u64,
    cue: CueDirector,
    ground_height: f32,
    ping_wait: f32,
    /// Seconds before this voice can call again.
    call_wait: f32,
    prayed: bool,
    praying: bool,
    /// Pulse timers: pump, ignition, prayer and revive noises, and the torch
    /// catching his eye.
    pulse: [f32; 5],
    /// What they did tonight, for the awards.
    pub deeds: Deeds,
    /// Gone for the night: the friend on their feet whose eyes and ears they
    /// share (see `Session::watch`).
    pub watch: Option<PlayerId>,
}

impl Participant {
    fn new(pose: Pose, seed: u64, layout: &Layout) -> Self {
        Self {
            ground_height: layout.rest_height(pose.pos),
            pose,
            status: Status::Active,
            body: Body::default(),
            aji: 0,
            light: true,
            battery: 1.0,
            revive: 0.0,
            hold: 0.0,
            hold_kind: 0,
            rhythm: Rhythm::new(seed ^ 0x5C11),
            director: Director::new(seed ^ 0x0DE7),
            input: Input::default(),
            input_age: 1.0,
            action_sequence: 0,
            cue: CueDirector::new(seed),
            ping_wait: 0.0,
            call_wait: 0.0,
            prayed: false,
            praying: false,
            pulse: [0.0; 5],
            deeds: Deeds::default(),
            watch: None,
        }
    }

    /// Down or dead: not on their feet.
    pub fn is_down(&self) -> bool {
        !self.status.is_active()
    }

    /// Gone for the night (bled out, or taken in his sack). The body never
    /// moves again, so nothing that only thaws as it moves is left on it.
    fn die(&mut self) {
        self.status = Status::Dead;
        self.revive = 0.0;
        self.body.stun = 0.0;
        self.body.sprinting = false;
    }

    /// Their own body as they feel it: fear, breath, hands, torch.
    fn vitals(&self, tuning: &Tuning) -> Vitals {
        Vitals {
            fear: self.body.fear,
            stamina: self.body.stamina,
            aji: self.aji,
            battery: self.battery,
            stun: self.body.stun,
            hold_kind: self.hold_kind,
            hold: self.hold,
            check: self.rhythm.check.map(|c| CheckView {
                id: c.id,
                needle: c.needle(tuning),
                zone: c.zone,
            }),
            stall: self.rhythm.stall_left(),
        }
    }
}

/// Tureco: tied behind the house until someone unties him, then at the heels
/// of whoever did (or the nearest friend). He growls when he is near and,
/// when he has the courage, barks him off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dog {
    pub pos: Vec2,
    pub facing: Vec2,
    pub free: bool,
    pub owner: Option<PlayerId>,
    /// Seconds before he dares bark again.
    pub courage: f32,
    pub growling: bool,
    /// Seconds the bark still shows.
    pub barking: f32,
}

impl Dog {
    fn tied(layout: &Layout) -> Self {
        Self {
            pos: layout.district.dog_post,
            facing: Vec2::new(0.0, 1.0),
            free: false,
            owner: None,
            courage: 0.0,
            growling: false,
            barking: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ping {
    pub pos: Vec3,
    pub left: f32,
    pub by: PlayerId,
}

/// A call out loud, where the caller is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Call {
    pub kind: CallKind,
    pub pos: Vec3,
    pub left: f32,
    pub by: PlayerId,
}

/// What of a friend's own night reaches the fallen who watch them: his eye
/// on them and the hunt, their fright, their omens (placed, like theirs,
/// from the eye the watcher shares) and Tureco's growl at their heels. Never
/// what their hands do (pickups, prayers, skill checks): those stay theirs.
pub fn shared_with_watcher(e: Event) -> bool {
    matches!(
        e,
        Event::WarningBegan
            | Event::HuntBegan
            | Event::Susto
            | Event::DogGrowl
            | Event::OmenLampsDie
            | Event::OmenSilence
            | Event::OmenBones
            | Event::OmenDrag
            | Event::OmenHat
            | Event::OmenPhantom
            | Event::OmenStolenLight
            | Event::OmenFootsteps
            | Event::OmenFalseMark
    )
}

pub struct Session {
    pub encounter: Encounter,
    pub players: BTreeMap<PlayerId, Participant>,
    pub pings: Vec<Ping>,
    pub calls: Vec<Call>,
    pub run: u64,
    pub tick: u64,
    pub started: bool,
    target: Option<PlayerId>,
    /// The fallen player in his sack, if any.
    captive: Option<PlayerId>,
    /// Who the truck left behind when someone drove off without them.
    left_behind: Vec<PlayerId>,
    pub dog: Dog,
    serial: u64,
    beacon_pulse: f32,
    /// Seconds until the next sign of which of him walks, and its stream.
    tell: f32,
    tell_rng: Rng,
    noises: Vec<(Vec2, f32)>,
    events: Vec<(Option<PlayerId>, Event)>,
    pub outbox: Vec<(PlayerId, ServerMessage)>,
    /// Who each player is (kept across restarts, given out in the lobby).
    survivors: BTreeMap<PlayerId, Survivor>,
    /// El Respiro: when he may press and when the night breathes. Hidden AI
    /// state, never on the wire.
    pub pacing: Respiro,
}

impl Session {
    pub fn new(layout: &Layout, tuning: &Tuning) -> Self {
        let mut s = Self {
            encounter: Encounter::new(layout),
            players: BTreeMap::new(),
            pings: Vec::new(),
            calls: Vec::new(),
            run: 1,
            tick: 0,
            started: false,
            target: None,
            captive: None,
            left_behind: Vec::new(),
            dog: Dog::tied(layout),
            serial: 0,
            beacon_pulse: 0.0,
            tell: tuning.tell_every.0,
            tell_rng: Rng::fork(tuning.seed, 0x7E11),
            noises: Vec::new(),
            events: Vec::new(),
            outbox: Vec::new(),
            survivors: BTreeMap::new(),
            pacing: Respiro::new(tuning.night, tuning.seed),
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
        let free = Survivor::assign(Survivor::default(), &self.taken_survivors(id));
        self.survivors.insert(id, free);
        Ok(())
    }

    fn taken_survivors(&self, except: PlayerId) -> Vec<Survivor> {
        self.survivors
            .iter()
            .filter(|(id, _)| **id != except)
            .map(|(_, s)| *s)
            .collect()
    }

    /// A player asks to be someone: granted unless another player already
    /// is, then the first one free. Only in the lobby.
    pub fn choose_survivor(&mut self, id: PlayerId, wanted: Survivor) -> Result<Survivor, String> {
        if self.started {
            return Err("Choose who you are before the night begins.".into());
        }
        if !self.players.contains_key(&id) {
            return Err("Unknown player.".into());
        }
        let given = Survivor::assign(wanted, &self.taken_survivors(id));
        self.survivors.insert(id, given);
        Ok(given)
    }

    pub fn survivor(&self, id: PlayerId) -> Survivor {
        self.survivors.get(&id).copied().unwrap_or_default()
    }

    pub fn remove_player(&mut self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            let at = Vec3::new(p.pose.pos.x, p.ground_height, p.pose.pos.y);
            self.encounter.release_all(id, at);
        }
        self.players.remove(&id);
        self.survivors.remove(&id);
        self.outbox.retain(|(recipient, _)| *recipient != id);
        self.pings.retain(|p| p.by != id);
        self.calls.retain(|c| c.by != id);
        if self.target == Some(id) || self.captive == Some(id) {
            self.target = None;
            self.captive = None;
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
            Action::Become { survivor } => self.choose_survivor(id, Survivor::from_code(survivor)).map(|_| ()),
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
            Action::Call { kind } => self.call(id, kind, tuning),
            Action::Watch { id: friend } => self.watch(id, friend),
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
                    Action::Skill { id: check, needle } => self.skill(id, check, needle, layout, tuning),
                    Action::TryCode { code } => self.try_code(id, code, layout, tuning),
                    Action::Name { variant } => self.name(id, variant, layout, tuning),
                    Action::DriveOff => self.drive_off(id, layout, tuning),
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

    /// A call out loud. It comes from the caller's own body, whatever the
    /// client claims, and it is a noise he hears. The cry for help is the
    /// fallen's alone: never from his sack, never from the dead.
    fn call(&mut self, id: PlayerId, kind: CallKind, tuning: &Tuning) -> Result<(), String> {
        if !self.started || self.encounter.outcome.is_over() {
            return Err("The run is not active.".into());
        }
        let captive = self.captive == Some(id);
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        if captive {
            return Err("You are in his sack.".into());
        }
        match (kind, p.status) {
            (_, Status::Dead) => return Err("You are dead.".into()),
            (CallKind::Help, Status::Active) => return Err("You are on your feet.".into()),
            (CallKind::Help, Status::Downed { .. }) => {}
        }
        if p.call_wait > 0.0 {
            return Err("Your voice is spent; wait a moment.".into());
        }
        p.call_wait = tuning.call_cooldown;
        let at = p.pose.pos;
        let pos = Vec3::new(at.x, p.ground_height, at.y);
        self.noises.push((at, tuning.noise_call));
        self.calls.retain(|c| c.by != id);
        self.calls.push(Call {
            kind,
            pos,
            left: tuning.call_life,
            by: id,
        });
        Ok(())
    }

    /// Whoever was in his sack falls out, and gets time to be found.
    fn release_captive(&mut self, tuning: &Tuning) {
        if let Some(id) = self.captive.take()
            && let Some(p) = self.players.get_mut(&id)
            && let Status::Downed { bleed } = &mut p.status
        {
            *bleed = bleed.max(tuning.bleed_after_sack);
        }
    }

    /// A friend the fallen may watch: in the night and on their feet. Not
    /// the downed, and not the one in his sack (there their eyes are his), so
    /// a watch moves on when its friend falls.
    fn watchable(&self, id: PlayerId) -> bool {
        self.players.get(&id).is_some_and(|p| p.status.is_active())
    }

    /// Gone for the night, a player watches a friend on their feet. The
    /// fallen see only what that friend's eyes see and hear only the whistle
    /// that friend hears (see `snapshot` and `cues`), never more.
    fn watch(&mut self, id: PlayerId, friend: PlayerId) -> Result<(), String> {
        if !self.started || self.encounter.outcome.is_over() {
            return Err("The run is not active.".into());
        }
        if !self.players.get(&id).is_some_and(|p| matches!(p.status, Status::Dead)) {
            return Err("Only the fallen watch over their friends.".into());
        }
        if friend == id || !self.watchable(friend) {
            return Err("They are not on their feet.".into());
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.watch = Some(friend);
        }
        Ok(())
    }

    /// The fallen always watch a friend on their feet: when theirs falls or
    /// leaves, the watch moves to the one standing nearest where they lie
    /// (ties to the lowest id).
    fn watch_step(&mut self) {
        let standing: Vec<(PlayerId, Vec2)> = self
            .players
            .iter()
            .filter(|(_, p)| p.status.is_active())
            .map(|(&id, p)| (id, p.pose.pos))
            .collect();
        for p in self.players.values_mut() {
            if !matches!(p.status, Status::Dead) {
                p.watch = None;
                continue;
            }
            if p.watch.is_some_and(|w| standing.iter().any(|&(id, _)| id == w)) {
                continue;
            }
            let at = p.pose.pos;
            p.watch = standing
                .iter()
                .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
                .map(|&(id, _)| id);
        }
    }

    /// What the crosshair may act on for `id`, built from truth.
    pub fn scene_data(&self, id: PlayerId) -> SceneData {
        let p = &self.players[&id];
        let progress = &self.encounter.progress;
        SceneData {
            relics: progress.relics.clone(),
            aji_taken: progress.aji_taken.clone(),
            batteries_taken: progress.batteries_taken.clone(),
            power_on: progress.power_on(),
            truck_running: progress.truck_running(),
            bones_home: progress.bones_home(),
            beacon_ready: progress.beacon_ready(),
            key: progress.key,
            dog_tied: (!self.dog.free).then_some(self.dog.pos),
            carrying: progress.carried_by(id),
            aji_held: p.aji,
            battery: p.battery,
            bodies: self
                .players
                .iter()
                .filter(|(pid, q)| q.status.is_downed() && self.captive != Some(**pid))
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
                if self.encounter.progress.carried_by(id) >= pacing::carry_max(tuning.night) {
                    return Err("Your arms are full: lay a bundle down first.".into());
                }
                if !self.encounter.take_relic(i as usize, id, &mut ev) {
                    return Err("That bundle is already taken.".into());
                }
                if self.encounter.threat.state == ThreatState::Dormant {
                    let watchers = self.active_positions();
                    self.encounter.manifest(layout, tuning, &watchers, &mut ev);
                    // He has risen: the grace begins.
                    self.pacing.begin();
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
            TargetKind::Batteries(i) => {
                if !self.encounter.take_batteries(i as usize, &mut ev) {
                    return Err("Those batteries are already taken.".into());
                }
                if let Some(p) = self.players.get_mut(&id) {
                    p.battery = (p.battery + tuning.battery_pickup).min(1.0);
                }
                self.events.extend(ev.drain(..).map(|e| (Some(id), e)));
            }
            TargetKind::Panel => {
                if !self.encounter.switch_lines(&mut ev) {
                    return Err("There is no power to switch yet.".into());
                }
                self.noises.push((ground(layout.district.panel), tuning.noise_unlock));
            }
            TargetKind::Note(_) => {}
            TargetKind::Radio => {
                // Anyone can turn it, it changes for everyone, and it squeals.
                self.encounter.turn_dial(&mut ev);
                self.noises.push((ground(layout.district.radio), tuning.noise_dial));
            }
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
        let pos = p.pose.pos;
        if let Some(p) = self.players.get_mut(&id) {
            p.deeds.drops += 1;
        }
        self.noises.push((pos, tuning.noise_drop));
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
        p.deeds.aji += 1;
        let at = p.pose.pos + p.pose.forward2() * 1.6;
        let pos = p.pose.pos;
        let mut ev = Vec::new();
        self.encounter.place_aji(tuning, at, &mut ev);
        self.noises.push((pos, tuning.noise_aji));
        self.events.extend(ev.into_iter().map(|e| (None, e)));
        self.flush_events();
        Ok(())
    }

    /// Naming him at the ceiba: only in reach of the altar.
    fn name(&mut self, id: PlayerId, code: u8, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        let guess = Variant::from_code(code).ok_or("No such name.")?;
        let data = self.scene_data(id);
        let pose = self.players[&id].pose;
        evaluate_target(layout, tuning, &pose, &data.scene())
            .filter(|t| t.ready() && t.kind == TargetKind::Altar)
            .ok_or("Name him at the ceiba's roots.")?;
        let watchers = self.active_positions();
        let mut ev = Vec::new();
        self.encounter.name_him(layout, tuning, guess, &watchers, &mut ev)?;
        self.target = None;
        self.events.extend(ev.into_iter().map(|e| (None, e)));
        self.flush_events();
        Ok(())
    }

    /// A combination tried on the key box: only in reach of it; a wrong one
    /// rattles the padlock where he may hear it.
    fn try_code(&mut self, id: PlayerId, code: [u8; 3], layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        if code.iter().any(|&d| d > 9) {
            return Err("Invalid combination.".into());
        }
        let data = self.scene_data(id);
        let pose = self.players[&id].pose;
        evaluate_target(layout, tuning, &pose, &data.scene())
            .filter(|t| t.usable() && t.kind == TargetKind::Lockbox)
            .ok_or("Aim at the key box, within reach.")?;
        let mut ev = Vec::new();
        let opened = self.encounter.try_code(tuning, code, &mut ev);
        let at = ground(layout.district.lockbox);
        if opened {
            self.noises.push((at, tuning.noise_unlock));
            self.events.extend(ev.into_iter().map(|e| (None, e)));
        } else {
            self.noises.push((at, tuning.noise_rattle));
            self.events.extend(ev.into_iter().map(|e| (Some(id), e)));
        }
        self.flush_events();
        Ok(())
    }

    /// A press for a skill check: the host judges the claimed needle.
    fn skill(&mut self, id: PlayerId, check: u32, needle: f32, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        let p = self.players.get_mut(&id).ok_or("Player is disconnected.")?;
        let kind = p.rhythm.kind();
        let verdict = p.rhythm.press(check, needle, tuning)?;
        self.check_result(id, kind, verdict, layout, tuning);
        self.flush_events();
        Ok(())
    }

    /// What a check's verdict does to the task in hand (`kind`, a
    /// `Vitals::hold_kind`): a great press speeds it, a miss screeches (a
    /// noise he hears over the work itself, and a fright for the worker) and
    /// throws the work back further than the check let it run. The stall
    /// that follows lives in the worker's `Rhythm`.
    fn check_result(&mut self, id: PlayerId, kind: u8, verdict: Verdict, layout: &Layout, tuning: &Tuning) {
        let d = &layout.district;
        let (site, work) = match kind {
            1 => (ground(layout.ceiba.offering), tuning.noise_altar),
            3 => (ground(d.pump), tuning.noise_pump),
            4 => (ground(d.ignition), tuning.noise_truck),
            _ => return,
        };
        let mut ev = Vec::new();
        match verdict {
            Verdict::Good => {}
            Verdict::Great => {
                match kind {
                    1 => {
                        if let Some(p) = self.players.get_mut(&id) {
                            p.hold = (p.hold + tuning.check_bonus * 3.0).min(LAST_TURN);
                        }
                    }
                    3 => self
                        .encounter
                        .work_pump(tuning, tuning.check_bonus * tuning.pump_hold, &mut ev),
                    _ => {
                        self.encounter
                            .work_truck(tuning, tuning.check_bonus * tuning.truck_hold, &mut ev);
                    }
                }
                self.events.push((Some(id), Event::SkillGreat));
            }
            Verdict::Miss => {
                let progress = &mut self.encounter.progress;
                match kind {
                    1 => {
                        if let Some(p) = self.players.get_mut(&id) {
                            p.hold = 0.0;
                        }
                    }
                    3 if !progress.power_on() => {
                        progress.power = (progress.power - tuning.check_setback / tuning.pump_hold).max(0.0);
                    }
                    4 if !progress.truck_running() => {
                        progress.truck = (progress.truck - tuning.check_setback / tuning.truck_hold).max(0.0);
                    }
                    _ => {}
                }
                self.noises
                    .push((site, tuning.noise_miss.max(work * tuning.noise_miss_over)));
                self.events.push((None, Event::SkillMissed));
                self.startle(id, tuning, tuning.fear_miss);
            }
        }
        if ev.contains(&Event::TruckStarted) {
            let watchers = self.active_positions();
            self.encounter.rouse(layout, tuning, &watchers, &mut ev);
        }
        self.events.extend(ev.into_iter().map(|e| (None, e)));
    }

    pub fn restart(&mut self, layout: &Layout, tuning: &Tuning) {
        self.run += 1;
        self.tick = 0;
        self.encounter.reset(layout);
        self.target = None;
        self.captive = None;
        self.left_behind.clear();
        self.dog = Dog::tied(layout);
        self.outbox.clear();
        self.pings.clear();
        self.calls.clear();
        self.events.clear();
        self.noises.clear();
        self.beacon_pulse = 0.0;
        self.tell = tuning.tell_every.0;
        self.tell_rng = Rng::fork(tuning.seed, 0x7E11);
        self.pacing = Respiro::new(tuning.night, tuning.seed);
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
            // Gone for the night, the friend they watch: what that friend
            // lives through of him reaches them too (see `shared_with_watcher`).
            let friend = self
                .players
                .get(&id)
                .filter(|p| matches!(p.status, Status::Dead))
                .and_then(|p| p.watch);
            let mine: Vec<Event> = events
                .iter()
                .filter(|(to, e)| {
                    to.is_none_or(|t| t == id) || (friend.is_some() && *to == friend && shared_with_watcher(*e))
                })
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
        for c in &mut self.calls {
            c.left -= dt;
        }
        self.calls.retain(|c| c.left > 0.0);
        self.move_players(layout, tuning, dt);
        self.resolve_holds(layout, tuning, dt);
        self.world_noise(layout, tuning, dt);
        self.tells(tuning, dt);
        self.threat_step(layout, tuning, dt);
        self.dog_step(layout, tuning, dt);
        self.fear_step(layout, tuning, dt);
        self.omens(tuning, dt);
        self.outcome_step(layout, tuning);
        // Before the whistles go out, so the fallen hear their friend's.
        self.watch_step();
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
            p.call_wait = (p.call_wait - dt).max(0.0);
            for t in &mut p.pulse {
                *t = (*t - dt).max(0.0);
            }
            if let Status::Downed { bleed } = &mut p.status {
                *bleed -= dt;
                if *bleed <= 0.0 {
                    p.die();
                    self.events.push((None, Event::Died));
                }
            }
            if matches!(p.status, Status::Dead) || self.captive == Some(id) {
                p.pose.lower = tuning.downed_lower;
                continue;
            }
            let fresh = p.input_age <= 0.3;
            let downed = p.status.is_downed();
            let pos = p.pose.pos;
            let ground_kind = Ground {
                wade: layout.wade(pos),
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
            if p.input.light && p.battery > 0.0 {
                p.battery = (p.battery - dt / tuning.battery_life).max(0.0);
            }
            p.light = p.input.light && p.battery > 0.0;
            p.ground_height = layout.rest_height(p.pose.pos);
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
                        p.hold += p.rhythm.allow(p.hold, dt / tuning.deliver_hold);
                        if p.hold >= 1.0 {
                            p.hold = 0.0;
                            // The next bundle is a rite of its own.
                            p.rhythm.rest();
                            self.encounter.deliver_relic(id, &mut ev);
                            // The Son weeps for every bone of his father laid down.
                            if Variant::of(tuning.seed) == Variant::Hijo {
                                ev.push(Event::Weeping);
                            }
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
                        let step = p.rhythm.allow(self.encounter.progress.power, dt / tuning.pump_hold);
                        self.encounter.work_pump(tuning, step * tuning.pump_hold, &mut ev);
                        if p.pulse[0] <= 0.0 {
                            p.pulse[0] = 1.5;
                            self.noises.push((ground(layout.district.pump), tuning.noise_pump));
                        }
                        progress = Some(self.encounter.progress.power);
                    }
                    TargetKind::Ignition => {
                        kind = 4;
                        let step = p.rhythm.allow(self.encounter.progress.truck, dt / tuning.truck_hold);
                        self.encounter.work_truck(tuning, step * tuning.truck_hold, &mut ev);
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
                    TargetKind::Dog if !self.dog.free => {
                        kind = 7;
                        if p.hold_kind != kind {
                            p.hold = 0.0;
                        }
                        p.hold += dt / tuning.untie_hold;
                        if p.hold >= 1.0 {
                            p.hold = 0.0;
                            self.dog.free = true;
                            self.dog.owner = Some(id);
                            ev.push(Event::DogFreed);
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
            // Long tasks ask for skill checks as they are worked.
            let pulse = match self.players.get_mut(&id) {
                Some(p) if matches!(kind, 1 | 3 | 4) => {
                    let first = (kind == 1).then_some(tuning.check_first_rite);
                    p.rhythm.work(kind, first, tuning, dt)
                }
                Some(p) => {
                    p.rhythm.rest();
                    Pulse::Quiet
                }
                None => Pulse::Quiet,
            };
            match pulse {
                Pulse::Started => self.events.push((Some(id), Event::SkillCheck)),
                Pulse::Missed => self.check_result(id, kind, Verdict::Miss, layout, tuning),
                Pulse::Quiet | Pulse::Stalled => {}
            }
            // Working the rite, the pump, the engine or the beacon pushes
            // the night on; lifting and carrying bones does not.
            if matches!(kind, 3 | 4) || ev.iter().any(|e| matches!(e, Event::RelicDelivered | Event::BeaconLit)) {
                self.pacing.push_forward(self.encounter.progress.delivered());
            }
            if ev.contains(&Event::TruckStarted) {
                let watchers = self.active_positions();
                let mut more = Vec::new();
                self.encounter.rouse(layout, tuning, &watchers, &mut more);
                if more.contains(&Event::ThreatManifested) {
                    self.pacing.begin();
                }
                ev.extend(more);
            }
            if let Some(p) = self.players.get_mut(&id) {
                for e in &ev {
                    match e {
                        Event::RelicDelivered => p.deeds.delivered += 1,
                        Event::Revived => p.deeds.revives += 1,
                        _ => {}
                    }
                }
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
            let mut stirred: Option<(PlayerId, f32)> = None;
            for (&id, p) in &self.players {
                if !p.status.is_active() || p.body.crouching || p.pose.pos.distance(herd) > tuning.cattle_radius {
                    continue;
                }
                let moving = p.input.axis != [0.0; 2] && p.input_age <= 0.3;
                if !moving {
                    continue;
                }
                let carried = self.encounter.progress.carried_by(id) as f32;
                let stir = if p.body.sprinting { 1.6 } else { 0.4 } + 0.3 * carried;
                unrest += stir;
                if stirred.is_none_or(|(_, most)| stir > most) {
                    stirred = Some((id, stir));
                }
            }
            // The Drover's return: the herd knows him and bellows as he passes.
            let th = &self.encounter.threat;
            if Variant::of(tuning.seed) == Variant::Arriero
                && matches!(th.presence, Presence::Present)
                && th.state != ThreatState::Dormant
                && th.pos.distance(herd) < 18.0
            {
                unrest += 3.0;
            }
            let prog = &mut self.encounter.progress;
            prog.cattle_spook += unrest * dt;
            if prog.cattle_spook >= 1.0 && prog.cattle_cooldown <= 0.0 {
                prog.cattle_spook = 0.0;
                prog.cattle_alarm = tuning.cattle_alarm;
                prog.cattle_cooldown = tuning.cattle_cooldown;
                self.noises.push((herd, tuning.noise_cattle));
                self.events.push((None, Event::CattleSpooked));
                if let Some((id, _)) = stirred
                    && let Some(p) = self.players.get_mut(&id)
                {
                    p.deeds.cattle += 1;
                }
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

    /// While he walks the llano, now and then a sign of which of him it is:
    /// a whip cracking for the Drover, glass clinking for the Drunkard. (The
    /// Son's sign is his weeping at the rite.)
    fn tells(&mut self, tuning: &Tuning, dt: f32) {
        let present = matches!(self.encounter.threat.presence, Presence::Present)
            && self.encounter.threat.state != ThreatState::Dormant;
        if !present {
            return;
        }
        self.tell -= dt;
        if self.tell > 0.0 {
            return;
        }
        let (lo, hi) = tuning.tell_every;
        self.tell = self.tell_rng.range(lo, hi);
        match Variant::of(tuning.seed) {
            Variant::Arriero => self.events.push((None, Event::WhipCrack)),
            Variant::Borracho => self.events.push((None, Event::BottleClink)),
            Variant::Hijo => {}
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

    /// A lit torch he can see draws him to look, from farther than he would
    /// notice the one holding it: the beam is the brightest thing on the llano.
    fn light_lure(&mut self, layout: &Layout, tuning: &Tuning) {
        let th = &self.encounter.threat;
        if th.state != ThreatState::Stalking || !matches!(th.presence, Presence::Present) {
            return;
        }
        let at = th.pos;
        // His anger draws him to a beam from farther (La Rabia).
        let range = tuning.at_rage(self.encounter.progress.rage()).light_lure_range;
        for p in self.players.values_mut() {
            if !p.status.is_active() || !p.light || p.pulse[4] > 0.0 {
                continue;
            }
            let dist = p.pose.pos.distance(at);
            if dist > range || !layout.line_of_sight(p.pose.pos, at) {
                continue;
            }
            p.pulse[4] = tuning.light_lure_pulse;
            // Heard at exactly the reach it needs: light is not masked by rain.
            self.encounter.hear(tuning, p.pose.pos, dist + 1.0);
        }
    }

    /// El Respiro reads the night's truth and gives its orders before he
    /// moves: a floor under his warning, a leash, a fade.
    fn pace(&mut self, target: Option<PlayerId>, dt: f32) {
        if self.encounter.dawn {
            self.encounter.threat.leash = None;
            return;
        }
        let th = &self.encounter.threat;
        let present = th.state != ThreatState::Dormant && matches!(th.presence, Presence::Present);
        let menace: Vec<Menace> = self
            .players
            .iter()
            .filter(|(_, p)| p.status.is_active())
            .map(|(&id, p)| Menace {
                id,
                seen: present && th.has_sight && target == Some(id),
                near: present && p.pose.pos.distance(th.pos) < pacing::NEAR,
                lure: p.light && p.pulse[4] > 0.0,
                growl: self.dog.growling && self.dog.owner == Some(id),
                afraid: p.body.fear >= pacing::AFRAID,
            })
            .collect();
        let nearest = if present {
            self.active_positions()
                .iter()
                .map(|p| p.distance(th.pos))
                .fold(f32::INFINITY, f32::min)
        } else {
            f32::INFINITY
        };
        let input = Inputs {
            players: &menace,
            nearest,
            calm: th.state == ThreatState::Stalking,
            present: matches!(th.presence, Presence::Present | Presence::Rising { .. }),
            laid: self.encounter.progress.delivered(),
        };
        let orders = self.pacing.tick(&input, dt);
        let th = &mut self.encounter.threat;
        if let Some(floor) = orders.floor {
            th.cooldown = th.cooldown.max(floor);
        }
        th.leash = orders.leash;
        if orders.withdraw {
            self.encounter.withdraw();
        }
    }

    fn threat_step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        self.light_lure(layout, tuning);
        let masking = storm::masking(tuning, self.encounter.elapsed);
        for (pos, radius) in std::mem::take(&mut self.noises) {
            if radius >= pacing::PUSH_NOISE {
                self.pacing.push_forward(self.encounter.progress.delivered());
            }
            self.encounter.hear(tuning, pos, radius * masking);
        }
        let watchers = self.active_positions();
        let target = self.choose_prey(layout);
        self.pace(target, dt);
        let prey = target.and_then(|id| self.players.get(&id)).map(|p| Prey {
            pos: p.pose.pos,
            sight: (if p.body.crouching { tuning.sight_crouch } else { 1.0 })
                * (if p.light { tuning.sight_light } else { 1.0 }),
            concealed: p.body.crouching && layout.district.tall_grass_at(p.pose.pos),
        });
        let mut ev = Vec::new();
        self.encounter
            .update_threat(layout, tuning, prey, &watchers, dt, &mut ev);
        let mut hauled = Vec::new();
        for e in ev {
            match e {
                Event::Downed => {
                    if let Some(id) = target {
                        self.down(id, tuning);
                        // With others still standing, he takes the fallen
                        // with him in his sack rather than leaving them.
                        let standing = self.active_positions();
                        if !standing.is_empty() && self.captive.is_none() {
                            self.captive = Some(id);
                            if let Some(p) = self.players.get_mut(&id) {
                                p.deeds.sacked += 1;
                            }
                            self.encounter.haul(layout, &standing, &mut hauled);
                        }
                    }
                    self.events.push((None, e));
                }
                Event::SackDropped => {
                    self.release_captive(tuning);
                    self.events.push((None, e));
                }
                Event::Taken => {
                    if let Some(id) = self.captive.take()
                        && let Some(p) = self.players.get_mut(&id)
                    {
                        p.die();
                    }
                    self.events.push((None, e));
                }
                Event::WarningBegan => {
                    if let Some(id) = target {
                        if let Some(p) = self.players.get_mut(&id) {
                            p.deeds.warned += 1;
                        }
                        self.startle(id, tuning, tuning.fear_warn);
                        self.events.push((Some(id), e));
                    }
                }
                Event::HuntBegan | Event::WarningAverted | Event::LostTrack | Event::ThreatReturned => {
                    match e {
                        Event::HuntBegan => self.pacing.hunt_began(),
                        Event::LostTrack => self.pacing.hunt_resolved(),
                        _ => {}
                    }
                    if let Some(id) = target {
                        self.events.push((Some(id), e));
                    }
                }
                _ => self.events.push((None, e)),
            }
        }
        self.events.extend(hauled.into_iter().map(|e| (None, e)));
        // The one in the sack goes where he goes.
        if let Some(id) = self.captive {
            let (at, facing) = (self.encounter.threat.pos, self.encounter.threat.facing);
            if let Some(p) = self.players.get_mut(&id) {
                p.pose.pos = layout.move_circle(at, -facing * 0.5, 0.2);
                p.ground_height = layout.rest_height(p.pose.pos);
                p.revive = 0.0;
            }
        }
        if matches!(self.encounter.threat.state, ThreatState::Dormant) {
            self.target = None;
        }
    }

    /// Tureco follows his friend, growls at the dark when he is near, and,
    /// when he has the courage, barks him off.
    fn dog_step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let dog = &mut self.dog;
        dog.courage = (dog.courage - dt).max(0.0);
        dog.barking = (dog.barking - dt).max(0.0);
        if dog.free {
            // His friend, or the nearest one standing; if his friend is in
            // the sack, he runs after the sack.
            let alive = |id: &PlayerId| self.players.get(id).is_some_and(|p| p.status.is_active());
            if dog.owner.is_some_and(|o| !alive(&o) && self.captive != Some(o)) {
                dog.owner = None;
            }
            if dog.owner.is_none() {
                dog.owner = self
                    .players
                    .iter()
                    .filter(|(_, p)| p.status.is_active() && p.pose.pos.distance(dog.pos) < 12.0)
                    .min_by(|a, b| {
                        a.1.pose
                            .pos
                            .distance(dog.pos)
                            .total_cmp(&b.1.pose.pos.distance(dog.pos))
                    })
                    .map(|(&id, _)| id);
            }
            let goal = match dog.owner {
                Some(o) if self.captive == Some(o) => Some(self.encounter.threat.pos),
                Some(o) => self
                    .players
                    .get(&o)
                    .map(|p| p.pose.pos - p.pose.forward2() * tuning.dog_trail),
                None => None,
            };
            if let Some(goal) = goal {
                let to = goal - dog.pos;
                let dist = to.length();
                if dist > 0.4 {
                    let step = (tuning.dog_speed * dt).min(dist * 2.5 * dt).min(dist);
                    dog.pos = layout.move_circle(dog.pos, to / dist * step, 0.3);
                    dog.facing = to / dist;
                }
            }
        }
        // What he senses of him: the truth, but only up close.
        let th = &self.encounter.threat;
        let near = th.state != ThreatState::Dormant && matches!(th.presence, Presence::Present);
        let sense = if near {
            dog_senses(th.pos.distance(dog.pos), tuning)
        } else {
            DogSense::Calm
        };
        let toward = (th.pos - dog.pos).normalize_or(dog.facing);
        let friend = dog.owner;
        match sense {
            DogSense::Calm => dog.growling = false,
            DogSense::Growl | DogSense::Bark => {
                dog.facing = toward;
                if !dog.growling {
                    dog.growling = true;
                    if let Some(f) = friend {
                        self.events.push((Some(f), Event::DogGrowl));
                    }
                }
            }
        }
        if sense == DogSense::Bark && self.dog.courage <= 0.0 {
            let mut ev = Vec::new();
            if self.encounter.flinch(&mut ev) {
                self.dog.courage = tuning.dog_courage;
                self.dog.barking = 1.2;
                self.noises.push((self.dog.pos, tuning.noise_bark));
                if ev.contains(&Event::SackDropped) {
                    self.release_captive(tuning);
                }
                self.target = None;
                self.events.extend(ev.into_iter().map(|e| (None, e)));
            }
        }
    }

    /// The hunted player falls: they drop what they carry and wait for help.
    fn down(&mut self, id: PlayerId, tuning: &Tuning) {
        let carried = self.encounter.progress.carried_by(id) as u16;
        let elapsed = self.encounter.elapsed;
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        p.deeds.downs += 1;
        p.deeds.drops += carried;
        p.deeds.first_down.get_or_insert(elapsed);
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
        if let Some(p) = self.players.get_mut(&id) {
            p.deeds.sustos += 1;
            self.noises.push((p.pose.pos, tuning.noise_susto));
        }
        self.events.push((Some(id), Event::Susto));
    }

    fn fear_step(&mut self, layout: &Layout, tuning: &Tuning, dt: f32) {
        let power = self.encounter.progress.live_circuits();
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

    /// The night works on each player left alone for a while.
    fn omens(&mut self, tuning: &Tuning, dt: f32) {
        let th = &self.encounter.threat;
        let awake = th.state != ThreatState::Dormant;
        let pursued_id = matches!(th.state, ThreatState::Warning | ThreatState::Hunting)
            .then_some(self.target)
            .flatten();
        let pressure = self.encounter.pressure;
        let rage = self.encounter.progress.rage();
        let company = self.players.len() > 1;
        let mut sent = Vec::new();
        for (&id, p) in &mut self.players {
            let mood = Mood {
                awake,
                pursued: pursued_id == Some(id),
                able: p.status.is_active(),
                fear: p.body.fear,
                pressure,
                company,
                rage,
            };
            if let Some(omen) = p.director.tick(mood, tuning, dt) {
                sent.push((
                    id,
                    match omen {
                        Omen::LampsDie => Event::OmenLampsDie,
                        Omen::Silence => Event::OmenSilence,
                        Omen::Bones => Event::OmenBones,
                        Omen::Drag => Event::OmenDrag,
                        Omen::Hat => Event::OmenHat,
                        Omen::Phantom => Event::OmenPhantom,
                        Omen::StolenLight => Event::OmenStolenLight,
                        Omen::Footsteps => Event::OmenFootsteps,
                        Omen::FalseMark => Event::OmenFalseMark,
                    },
                ));
            }
        }
        self.events.extend(sent.into_iter().map(|(id, e)| (Some(id), e)));
    }

    /// Someone aboard the ready truck drives off without waiting: those
    /// aboard escape, everyone else (down, in his sack, or simply not there)
    /// is left behind. Only when there is someone to leave.
    fn drive_off(&mut self, id: PlayerId, layout: &Layout, tuning: &Tuning) -> Result<(), String> {
        if self.players.len() < 2 {
            return Err("There is nobody to leave behind.".into());
        }
        if !self.encounter.progress.truck_ready(tuning) {
            return Err("The truck is not ready to go.".into());
        }
        let zone = layout.district.truck;
        let aboard =
            |p: &Participant| p.status.is_active() && p.pose.pos.distance(zone.zone_center) <= zone.zone_radius;
        if !aboard(&self.players[&id]) {
            return Err("Get to the truck first.".into());
        }
        self.left_behind = self
            .players
            .iter()
            .filter(|(_, p)| !aboard(p))
            .map(|(&pid, _)| pid)
            .collect();
        self.encounter.outcome = Outcome::Won;
        self.events.push((None, Event::Escaped));
        // The run is over, so no step will send it: send it now.
        self.flush_events();
        Ok(())
    }

    fn outcome_step(&mut self, layout: &Layout, tuning: &Tuning) {
        self.check_failure();
        // The rooster crows: he sinks for the night. With bones still out
        // the night ends there; with them all home, the way out stays open.
        if !self.encounter.outcome.is_over()
            && !self.encounter.dawn
            && self.encounter.elapsed >= pacing::dawn(tuning.night_length)
        {
            let mut ev = Vec::new();
            self.encounter.dawn(&mut ev);
            if ev.contains(&Event::SackDropped) {
                self.release_captive(tuning);
            }
            if !self.encounter.progress.bones_home() {
                self.encounter.outcome = Outcome::Dawn;
            }
            self.events.extend(ev.into_iter().map(|e| (None, e)));
        }
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
        // Veiled, his whistle keeps everyone closer company.
        let veiled = self.veiled();
        // (the fallen, the friend they watch): the fallen hear no whistle of
        // their own, only the one their friend hears.
        let watchers: Vec<(PlayerId, PlayerId)> = self
            .players
            .iter()
            .filter(|(_, p)| matches!(p.status, Status::Dead))
            .filter_map(|(&id, p)| p.watch.map(|friend| (id, friend)))
            .collect();
        for id in ids {
            let Some(p) = self.players.get_mut(&id) else {
                continue;
            };
            if matches!(p.status, Status::Dead) {
                continue;
            }
            p.cue.veil(veiled, tuning);
            let heard = p.cue.tick(dt, &self.encounter, p.pose.pos, tuning);
            let imagined = if p.status.is_active() && self.encounter.threat.state != ThreatState::Dormant {
                p.cue.phantom(p.body.fear, tuning, dt)
            } else {
                None
            };
            let Some(cue) = heard.or(imagined) else {
                continue;
            };
            // A whistle that was never there does not add to the fright
            // that conjured it: dread must not feed on itself.
            let frightened = p.status.is_active() && !cue.phantom && p.body.startle(tuning, cue.variant.dread(tuning));
            self.serial += 1;
            let message = ServerMessage::Cue {
                run: self.run,
                serial: self.serial,
                variant: cue.variant as u8,
                speed: cue.speed,
                phantom: cue.phantom,
                take: cue.take,
            };
            // The same whistle under the same serial to each of the fallen
            // watching this friend: still categorical, still never placed.
            for &(watcher, _) in watchers.iter().filter(|&&(_, friend)| friend == id) {
                self.outbox.push((watcher, message.clone()));
            }
            self.outbox.push((id, message));
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
        // Whose eyes this listener has: their own, or, gone for the night,
        // those of the friend on their feet they watch (with `anima_sight`).
        // Never the dead body's own eye, and never all-seeing.
        let viewer: Option<(PlayerId, &Participant)> = if matches!(local.status, Status::Dead) {
            local
                .watch
                .filter(|&friend| tuning.anima_sight && self.watchable(friend))
                .and_then(|friend| self.players.get(&friend).map(|p| (friend, p)))
        } else {
            Some((id, local))
        };
        // A slightly wider than camera cone accommodates the visible body's
        // extent, but no transforms are sent for enemies behind walls or behind
        // this viewer. From a tower deck the whole llano opens up. Host trust
        // and previously seen positions are not hidden. Veiled (El Velo) he
        // is seen by nobody, from anywhere: no position goes out at all.
        let visible = viewer.is_some_and(|(_, v)| {
            let towards = th.pos - v.pose.pos;
            let range = if v.ground_height > 3.0 { 130.0 } else { 75.0 };
            !self.veiled()
                && th.visibility(tuning) > 0.0
                && towards.length() < range
                && layout.line_of_sight(v.pose.pos, th.pos)
                && v.pose.forward2().dot(towards.normalize_or(Vec2::Y)) > 0.55
        });
        let pursued = viewer.is_some_and(|(vid, v)| self.target == Some(vid) && v.status.is_active());
        let danger = if th.state == ThreatState::Counting && visible {
            4
        } else if !pursued {
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
                    hauled: self.captive == Some(pid),
                    survivor: self.survivor(pid).code(),
                    watching: p.watch.unwrap_or(0),
                })
                .collect(),
            relics: progress.relics.iter().map(RelicView::from_relic).collect(),
            aji: progress.aji_taken.clone(),
            batteries: progress.batteries_taken.clone(),
            outcome: outcome_code(self.encounter.outcome),
            world: WorldView {
                delivered: progress.delivered() as u8,
                total: progress.relics.len() as u8,
                power: progress.power,
                truck: progress.truck,
                warm: (progress.warm / tuning.truck_warmup).clamp(0.0, 1.0),
                beacon: progress.beacon,
                beacon_ready: progress.beacon_ready(),
                key: progress.key,
                circuits: progress.live_circuits(),
                banished: progress.banished,
                naming: progress.naming_cooldown,
                cattle: progress.cattle_alarm,
                night: (self.encounter.elapsed / tuning.night_length).clamp(0.0, 1.0),
                radio: progress.radio,
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
            me: local.vitals(tuning),
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
            dog: DogView {
                pos: self.dog.pos.to_array(),
                facing: self.dog.facing.to_array(),
                mood: if !self.dog.free {
                    0
                } else if self.dog.barking > 0.0 {
                    3
                } else if self.dog.growling {
                    2
                } else {
                    1
                },
                owner: self.dog.owner.unwrap_or(0),
            },
            left_behind: self.left_behind.clone(),
            // What everyone did, once the night is over (for the awards).
            deeds: if self.encounter.outcome.is_over() {
                self.players.iter().map(|(&pid, p)| (pid, p.deeds)).collect()
            } else {
                Vec::new()
            },
            calls: self
                .calls
                .iter()
                .map(|c| CallView {
                    kind: c.kind,
                    pos: c.pos.to_array(),
                    left: c.left,
                    by: c.by,
                })
                .collect(),
            // The friend's own body, not a view of him: their fear, breath
            // and hands follow the watcher whatever `anima_sight` says, for
            // as long as the night lasts (an ending, Dawn included, ends it).
            watched: if matches!(local.status, Status::Dead) && !self.encounter.outcome.is_over() {
                local
                    .watch
                    .filter(|&friend| self.watchable(friend))
                    .and_then(|friend| self.players.get(&friend))
                    .map(|p| p.vitals(tuning))
            } else {
                None
            },
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

    /// El Velo: nobody sees him now, though he walks, sees, hears and
    /// whistles. Only while the director veils him and he only stalks, so a
    /// warning unveils him on the very tick it begins; never after dawn.
    pub fn veiled(&self) -> bool {
        self.pacing.veiled() && self.encounter.threat.state == ThreatState::Stalking && !self.encounter.dawn
    }

    /// Is he physically standing in the world right now?
    pub fn threat_present(&self) -> bool {
        matches!(
            self.encounter.threat.presence,
            Presence::Present | Presence::Rising { .. }
        )
    }
}
