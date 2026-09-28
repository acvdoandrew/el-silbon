//! Explicit development automation, using real endpoints and ordinary inputs.
//! No position teleports, objective setters, fake transport, or desktop input.
use super::{protocol::*, session::target, transport::Endpoint};
use crate::{
    app::Launch,
    control::{Intent, Pose, wrap_angle},
    geometry::Layout,
    script::{Observation, RouteScript},
    sim::{Encounter, Objective, Presence, Stats, ThreatState},
    tuning::Tuning,
};
use bevy::math::{Vec2, Vec3};

pub struct Driver {
    run: u64,
    script: RouteScript,
    elapsed: f32,
    outcome_time: f32,
    waypoint: usize,
    saw_win: bool,
    saw_failure: bool,
    saw_capture: bool,
    start_sent: bool,
    take_sent: bool,
    pub done: bool,
    pub error: Option<String>,
}
pub struct Frame {
    pub intent: Intent,
    pub action: Option<Action>,
    pub leave: bool,
}
impl Driver {
    pub fn new(layout: &Layout, tuning: &Tuning) -> Self {
        Self {
            run: 0,
            script: RouteScript::full(layout, tuning),
            elapsed: 0.0,
            outcome_time: 0.0,
            waypoint: 0,
            saw_win: false,
            saw_failure: false,
            saw_capture: false,
            start_sent: false,
            take_sent: false,
            done: false,
            error: None,
        }
    }
    pub fn tick(&mut self, endpoint: &Endpoint, pose: Pose, dt: f32, layout: &Layout, tuning: &Tuning) -> Frame {
        let mut frame = Frame {
            intent: Intent::default(),
            action: None,
            leave: false,
        };
        self.elapsed += dt;
        if self.elapsed > 360.0 {
            self.error = Some(format!(
                "smoke timed out, run {} step {}",
                self.run,
                self.script.step_name()
            ));
        }
        if self.error.is_some() || self.done {
            return frame;
        }
        if endpoint.closed {
            self.error = Some(endpoint.status.clone());
            return frame;
        }
        // Let the endpoint diagnose connection loss; do not misreport stale
        // snapshots as a pathfinding failure when the host has departed.
        if endpoint.snapshot.is_some() && endpoint.snapshot_age() > 0.25 {
            return frame;
        }
        let Some(s) = &endpoint.snapshot else {
            return frame;
        };
        let Some(id) = endpoint.id else {
            return frame;
        };
        if self.run != s.run {
            if self.run > 0 && s.run != self.run + 1 {
                self.error = Some("nonconsecutive restart epoch".into());
            }
            self.run = s.run;
            self.waypoint = 0;
            self.take_sent = false;
            self.outcome_time = 0.0;
            self.script = RouteScript::full(layout, tuning);
            eprintln!("NET SMOKE player={id} observing run={}", s.run);
            if s.players.iter().any(|p| p.caught) || objective(s.objective) != Objective::FindSatchel {
                self.error = Some("restart did not reset players/objective".into());
            }
        }
        if !s.started {
            if id == HOST && s.players.len() == 2 && !self.start_sent {
                frame.action = Some(Action::Start);
                self.start_sent = true;
            }
            return frame;
        }
        let objective = objective(s.objective);
        if objective == Objective::Won {
            self.saw_win = true;
        }
        if objective == Objective::Failed {
            self.saw_failure = true;
        }
        if s.players.iter().any(|p| p.caught) {
            self.saw_capture = true;
        }
        if id == HOST {
            if objective.is_over() {
                self.outcome_time += dt;
                if self.outcome_time > 0.7 && self.outcome_time < 0.7 + dt * 1.5 {
                    frame.action = Some(Action::Restart);
                }
                return frame;
            }
            if s.run == 2 {
                frame.intent = self.walk(pose, &[[0.0, 24.0], [0.0, 18.0], [0.0, 8.0]], dt);
            }
            if s.run == 3 && s.players.len() == 1 {
                if s.satchel == Satchel::Carried(HOST) {
                    if !self.saw_win || !self.saw_failure || !self.saw_capture {
                        self.error = Some("missing win/failure/capture evidence".into());
                    } else {
                        self.done = true;
                        eprintln!(
                            "NET SMOKE PASS host: shared win, shared failure, two restarts, disconnect and item recovery"
                        );
                    }
                } else if let Satchel::Ground(item) = s.satchel {
                    let path = [[0.0, 24.0], [0.0, 8.0], [0.0, 0.0], [0.0, -2.5], [2.15, -3.5]];
                    frame.intent = self.walk(pose, &path, dt);
                    if self.waypoint >= path.len() {
                        frame.intent.look_delta = aim(pose, Vec3::from_array(item), dt, tuning);
                        if !self.take_sent
                            && target(layout, tuning, &pose, s.satchel, objective, id)
                                .is_some_and(|t| t.kind == crate::control::TargetKind::Satchel && t.ready())
                        {
                            frame.action = Some(Action::Take);
                            self.take_sent = true;
                        }
                    }
                } else {
                    self.error = Some("disconnected carrier did not leave a ground satchel".into());
                }
            }
            return frame;
        }
        if objective.is_over() || s.players.iter().any(|p| p.id == id && p.caught) {
            return frame;
        }
        if s.run == 3 && s.satchel == Satchel::Carried(id) {
            if !self.saw_win || !self.saw_failure {
                self.error = Some("client missed shared outcomes".into());
            } else {
                frame.leave = true;
                self.done = true;
                eprintln!("NET SMOKE PASS client: shared win/failure/restarts; leaving while carrying");
            }
            return frame;
        }
        if s.run == 2 && s.satchel == Satchel::Carried(id) {
            frame.intent = self.walk(pose, &[[2.2, -2.2], [0.0, -2.2], [0.0, 0.0], [0.0, 8.0]], dt);
            return frame;
        }
        let mut enc = Encounter::new(layout);
        enc.objective = objective;
        enc.restitution = s.restitution;
        enc.restituting = s.restituting;
        enc.elapsed = s.elapsed;
        enc.stats = Stats {
            warnings: s.stats[0],
            hunts: s.stats[1],
            recoveries: s.stats[2],
        };
        enc.threat.state = match s.danger {
            1 => ThreatState::Warning,
            2 | 3 => ThreatState::Hunting,
            _ => ThreatState::Stalking,
        };
        if let Some(th) = s.threat {
            enc.threat.pos = Vec2::from_array(th.position);
            enc.threat.presence = Presence::Present;
        }
        let result = self.script.tick(&Observation {
            pose,
            encounter: &enc,
            dt,
            target: target(layout, tuning, &pose, s.satchel, objective, id),
        });
        if let Some(log) = result.log {
            eprintln!("NET SMOKE player={id} {log}");
        }
        if let Some(Err(e)) = result.finished {
            self.error = Some(e);
        }
        frame.intent = result.intent;
        if frame.intent.interact_pressed
            && !self.take_sent
            && target(layout, tuning, &pose, s.satchel, objective, id)
                .is_some_and(|t| t.kind == crate::control::TargetKind::Satchel && t.ready())
        {
            frame.action = Some(Action::Take);
            self.take_sent = true;
        }
        if frame.action.is_none() {
            frame.intent.interact_pressed = false;
        }
        frame
    }
    fn walk(&mut self, pose: Pose, path: &[[f32; 2]], dt: f32) -> Intent {
        if self.waypoint >= path.len() {
            return Intent::default();
        }
        let goal = Vec2::from_array(path[self.waypoint]);
        let d = goal - pose.pos;
        if d.length() < 0.22 {
            self.waypoint += 1;
            return Intent::default();
        }
        let dir = d.normalize();
        Intent {
            move_axis: Vec2::new(dir.dot(pose.right2()), dir.dot(pose.forward2())),
            look_delta: Vec2::new(
                wrap_angle(pose.yaw - Pose::yaw_toward(pose.pos, goal)).clamp(-3.0 * dt, 3.0 * dt),
                -pose.pitch * 0.1,
            ),
            ..Default::default()
        }
    }
}
fn aim(pose: Pose, point: Vec3, dt: f32, tuning: &Tuning) -> Vec2 {
    let delta = point - pose.eye(tuning);
    Vec2::new(
        wrap_angle(pose.yaw - Pose::yaw_toward(pose.pos, Vec2::new(point.x, point.z))).clamp(-4.0 * dt, 4.0 * dt),
        (delta.y.atan2(Vec2::new(delta.x, delta.z).length()) - pose.pitch).clamp(-4.0 * dt, 4.0 * dt),
    )
}

/// Two independent processes still use real UDP, handshakes, input packets,
/// host validation and per-listener snapshots. No rendering is claimed here.
pub fn run_headless(launch: Launch) -> Result<(), String> {
    let layout = Layout::authored();
    let tuning = Tuning::with_seed(launch.seed);
    let mut endpoint = Endpoint::new(
        launch.network.ok_or("headless requires --host/--join")?,
        &layout,
        &tuning,
    )?;
    let mut driver = Driver::new(&layout, &tuning);
    let mut pose = Pose::spawn(&layout);
    let mut last_run = 0;
    eprintln!("NET SMOKE headless {}", endpoint.status);
    loop {
        endpoint.update(STEP, &layout, &tuning);
        if let Some(s) = &endpoint.snapshot
            && let Some(local) = s.players.iter().find(|p| Some(p.id) == endpoint.id)
        {
            pose.pos = Vec2::from_array(local.position);
            if s.run != last_run {
                pose.yaw = local.yaw;
                pose.pitch = local.pitch;
                last_run = s.run;
            }
        }
        let frame = driver.tick(&endpoint, pose, STEP, &layout, &tuning);
        if let Some(e) = driver.error {
            return Err(e);
        }
        pose.look(frame.intent.look_delta, &tuning);
        endpoint.input(
            frame.intent.move_axis.to_array(),
            pose.yaw,
            pose.pitch,
            frame.intent.interact_held,
        );
        if let Some(action) = frame.action {
            endpoint.command(action, &layout, &tuning);
        }
        if frame.leave {
            endpoint.close("Client left smoke session carrying the satchel.");
        }
        endpoint.notices.clear();
        if driver.done {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}
