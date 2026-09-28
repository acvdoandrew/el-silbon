//! DEBUG ONLY: the deterministic smoke route.
//!
//! A scripted player that plays the real encounter through the same intent
//! path as a human: it turns and walks with look/move deltas, relies on the
//! real crosshair targeting, presses/holds interact, hides behind walls and
//! requests restarts through the same restart path as the menu button. It
//! never writes truth. The only thing it reads that a player could not is the
//! Silbón's position, and only to point the camera at him for screenshots.
//!
//! The full route: roadside → house → satchel → exposed yard (warning, hunt)
//! → hide inside (he loses track) → ceiba restitution → road (win) →
//! restart → satchel → stand exposed (caught) → restart.

use bevy::math::{Vec2, Vec3};

use crate::control::{Intent, Pose, Target, TargetKind, wrap_angle};
use crate::geometry::Layout;
use crate::sim::{Encounter, Objective, ThreatState};
use crate::tuning::Tuning;

const TURN_RATE: f32 = 4.0;
const ARRIVE: f32 = 0.3;

const ROAD_TO_TABLE: &[[f32; 2]] = &[
    [0.0, 28.0],
    [0.0, 22.0],
    [0.0, 12.0],
    [0.0, 4.0],
    [0.0, 1.0],
    [0.6, -1.5],
    [2.9, -3.3],
];
const TABLE_TO_YARD: &[[f32; 2]] = &[[0.3, -1.2], [0.0, 1.0], [0.0, 4.0], [0.0, 9.0]];
/// Back into the house and against the front wall right of the door: hidden
/// from the whole south-west and west, away from door and window lines.
const YARD_TO_COVER: &[[f32; 2]] = &[[0.0, 4.0], [0.0, 1.0], [0.0, 0.3], [2.2, 0.35]];
const COVER_TO_TREE: &[[f32; 2]] = &[
    [0.4, -1.5],
    [-3.0, -5.0],
    [-3.0, -7.2],
    [-6.5, -10.5],
    [-11.0, -16.0],
    [-16.2, -20.9],
];
const TREE_TO_ROAD: &[[f32; 2]] = &[
    [-9.0, -12.0],
    [-7.5, -4.0],
    [-7.5, 6.0],
    [-2.0, 12.0],
    [0.0, 18.0],
    [0.0, 22.0],
    [0.0, 28.5],
];

#[derive(Clone, Debug, PartialEq)]
enum Step {
    Wait(f32),
    /// Turn the view toward a world point (screenshot framing).
    Face([f32; 3]),
    Capture(&'static str),
    Walk(&'static [[f32; 2]]),
    Take {
        capture: Option<&'static str>,
    },
    Restitute {
        capture: &'static str,
    },
    WaitThreat {
        state: ThreatState,
        timeout: f32,
    },
    WaitRecovery {
        timeout: f32,
    },
    WaitObjective {
        objective: Objective,
        timeout: f32,
    },
    Restart,
    ExpectReset,
    Log(&'static str),
}

/// What the script sees each frame (the same data the HUD shows, plus the
/// Silbón's position for camera framing).
pub struct Observation<'a> {
    pub pose: Pose,
    pub encounter: &'a Encounter,
    pub target: Option<Target>,
    pub dt: f32,
}

/// What the script asks for this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptFrame {
    pub intent: Intent,
    pub capture: Option<&'static str>,
    pub restart: bool,
    pub log: Option<String>,
    /// Some(Ok) = route complete, Some(Err) = the route failed.
    pub finished: Option<Result<(), String>>,
}

#[derive(Clone, Debug)]
pub struct RouteScript {
    steps: Vec<Step>,
    index: usize,
    t: f32,
    waypoint: usize,
    best: f32,
    stall: f32,
    baseline: u32,
    captured: bool,
    spawn: Vec2,
    satchel: Vec3,
    offering: Vec3,
    eye_height: f32,
    total: f32,
}

impl RouteScript {
    /// Win loop, restart, fail loop, restart.
    pub fn full(layout: &Layout, tuning: &Tuning) -> Self {
        use Step::*;
        let steps = vec![
            Log("route: roadside"),
            Wait(1.5),
            // Frame the house and the ceiba together, clear of the HUD.
            Face([-6.0, 6.0, -10.0]),
            Capture("01_roadside"),
            Walk(&ROAD_TO_TABLE[..3]),
            Face([-6.0, 6.0, -10.0]),
            Capture("02_approach"),
            Walk(&ROAD_TO_TABLE[3..]),
            Log("route: satchel"),
            Take {
                capture: Some("03_interior"),
            },
            Walk(TABLE_TO_YARD),
            Log("route: waiting exposed in the yard"),
            WaitThreat {
                state: ThreatState::Warning,
                timeout: 90.0,
            },
            Capture("04_warning"),
            WaitThreat {
                state: ThreatState::Hunting,
                timeout: 10.0,
            },
            Capture("05_hunting"),
            Log("route: breaking line of sight inside the house"),
            Walk(YARD_TO_COVER),
            WaitRecovery { timeout: 20.0 },
            Log("route: he lost track; carrying the bones to the ceiba"),
            Walk(COVER_TO_TREE),
            Restitute {
                capture: "06_ceiba_hollow",
            },
            Log("route: bones returned; back to the road"),
            Walk(&TREE_TO_ROAD[..2]),
            // Look back at the whole tree: the landmark, bones at its roots.
            Face([-19.0, 10.0, -24.0]),
            Capture("07_ceiba_landmark"),
            Walk(&TREE_TO_ROAD[2..]),
            WaitObjective {
                objective: Objective::Won,
                timeout: 5.0,
            },
            Wait(0.6),
            Capture("08_win"),
            Wait(0.6),
            Restart,
            ExpectReset,
            Log("route: restart ok; second run stands exposed until caught"),
            Walk(ROAD_TO_TABLE),
            Take { capture: None },
            Walk(TABLE_TO_YARD),
            WaitThreat {
                state: ThreatState::Hunting,
                timeout: 100.0,
            },
            WaitObjective {
                objective: Objective::Failed,
                timeout: 30.0,
            },
            Wait(0.6),
            Capture("09_caught"),
            Wait(0.6),
            Restart,
            ExpectReset,
            Log("route: restart ok after failure"),
        ];
        Self::new(steps, layout, tuning)
    }

    fn new(steps: Vec<Step>, layout: &Layout, tuning: &Tuning) -> Self {
        Self {
            steps,
            index: 0,
            t: 0.0,
            waypoint: 0,
            best: f32::MAX,
            stall: 0.0,
            baseline: 0,
            captured: false,
            spawn: layout.spawn,
            satchel: layout.satchel,
            offering: layout.ceiba.offering,
            eye_height: tuning.eye_height,
            total: 0.0,
        }
    }

    /// Simulated seconds the route has been running.
    pub fn elapsed(&self) -> f32 {
        self.total
    }

    /// Name of the current step, for logs.
    pub fn step_name(&self) -> String {
        self.steps
            .get(self.index)
            .map(|s| format!("{s:?}"))
            .unwrap_or_else(|| "done".to_string())
    }

    fn next(&mut self) {
        self.index += 1;
        self.t = 0.0;
        self.waypoint = 0;
        self.best = f32::MAX;
        self.stall = 0.0;
        self.captured = false;
    }

    fn steer(&self, pose: &Pose, yaw: f32, pitch: f32, dt: f32) -> Vec2 {
        let max = TURN_RATE * dt;
        Vec2::new(
            wrap_angle(pose.yaw - yaw).clamp(-max, max),
            (pitch - pose.pitch).clamp(-max, max),
        )
    }

    /// Yaw and pitch that put the crosshair on a world point.
    fn aim_angles(&self, pose: &Pose, p: Vec3) -> (f32, f32) {
        let eye = Vec3::new(pose.pos.x, self.eye_height, pose.pos.y);
        let yaw = Pose::yaw_toward(pose.pos, Vec2::new(p.x, p.z));
        let h = Vec2::new(p.x - eye.x, p.z - eye.z).length();
        (yaw, (p.y - eye.y).atan2(h))
    }

    fn aim_at(&self, pose: &Pose, p: Vec3, dt: f32) -> Vec2 {
        let (yaw, pitch) = self.aim_angles(pose, p);
        self.steer(pose, yaw, pitch, dt)
    }

    fn face_threat(&self, obs: &Observation) -> Vec2 {
        let th = obs.encounter.threat.pos;
        self.aim_at(&obs.pose, Vec3::new(th.x, 2.2, th.y), obs.dt)
    }

    pub fn tick(&mut self, obs: &Observation) -> ScriptFrame {
        let mut out = ScriptFrame::default();
        let dt = obs.dt;
        self.total += dt;
        self.t += dt;
        let enc = obs.encounter;
        let Some(step) = self.steps.get(self.index).cloned() else {
            out.finished = Some(Ok(()));
            return out;
        };
        let fail = |msg: String| Some(Err(msg));
        match step {
            Step::Log(msg) => {
                out.log = Some(msg.to_string());
                self.next();
            }
            Step::Wait(secs) => {
                if self.t >= secs {
                    self.next();
                }
            }
            Step::Face([x, y, z]) => {
                let (yaw, pitch) = self.aim_angles(&obs.pose, Vec3::new(x, y, z));
                out.intent.look_delta = self.steer(&obs.pose, yaw, pitch, dt);
                let err = wrap_angle(obs.pose.yaw - yaw).abs() + (obs.pose.pitch - pitch).abs();
                if err < 0.01 || self.t > 3.0 {
                    self.next();
                }
            }
            Step::Capture(name) => {
                if !self.captured {
                    out.capture = Some(name);
                    self.captured = true;
                }
                if self.t >= 0.15 {
                    self.next();
                }
            }
            Step::Walk(path) => {
                // Reaching the road (or being caught) freezes the player; the
                // walk is over either way.
                if enc.objective.is_over() {
                    self.next();
                    return out;
                }
                let Some(&[x, z]) = path.get(self.waypoint) else {
                    self.next();
                    return out;
                };
                let goal = Vec2::new(x, z);
                let dist = obs.pose.pos.distance(goal);
                if dist < ARRIVE {
                    self.waypoint += 1;
                    self.best = f32::MAX;
                    self.stall = 0.0;
                    if self.waypoint >= path.len() {
                        self.next();
                    }
                    return out;
                }
                let yaw = Pose::yaw_toward(obs.pose.pos, goal);
                out.intent.look_delta = self.steer(&obs.pose, yaw, 0.0, dt);
                if wrap_angle(obs.pose.yaw - yaw).abs() < 0.6 {
                    out.intent.move_axis = Vec2::new(0.0, 1.0);
                }
                if dist < self.best - 0.05 {
                    self.best = dist;
                    self.stall = 0.0;
                } else {
                    self.stall += dt;
                    if self.stall > 4.0 {
                        out.finished = fail(format!("stuck walking to {goal:?}, at {:?}", obs.pose.pos));
                    }
                }
            }
            Step::Take { capture } => {
                out.intent.look_delta = self.aim_at(&obs.pose, self.satchel, dt);
                if enc.objective == Objective::ReturnBones {
                    self.next();
                } else if obs.target.is_some_and(|t| t.kind == TargetKind::Satchel && t.ready()) {
                    match capture {
                        Some(name) if !self.captured => {
                            out.capture = Some(name);
                            self.captured = true;
                        }
                        _ => {
                            out.intent.interact_pressed = true;
                            out.intent.interact_held = true;
                        }
                    }
                } else if self.t > 6.0 {
                    out.finished = fail(format!("satchel never became reachable (target {:?})", obs.target));
                }
            }
            Step::Restitute { capture } => {
                out.intent.look_delta = self.aim_at(&obs.pose, self.offering, dt);
                if enc.objective == Objective::Escape {
                    self.next();
                    return out;
                }
                if obs.target.is_some_and(|t| t.kind == TargetKind::Offering && t.ready()) {
                    out.intent.interact_held = true;
                    out.intent.interact_pressed = !enc.restituting;
                }
                if enc.restitution >= 0.5 && !self.captured {
                    out.capture = Some(capture);
                    self.captured = true;
                }
                if self.t > 15.0 {
                    out.finished = fail(format!(
                        "restitution did not complete (progress {:.2}, target {:?})",
                        enc.restitution, obs.target
                    ));
                }
            }
            Step::WaitThreat { state, timeout } => {
                out.intent.look_delta = self.face_threat(obs);
                if enc.threat.state == state {
                    // Recoveries after this point count for WaitRecovery.
                    self.baseline = enc.stats.recoveries;
                    self.next();
                } else if enc.objective.is_over() || self.t > timeout {
                    out.finished = fail(format!(
                        "waited {:.1}s for {state:?}; threat is {:?}, objective {:?}",
                        self.t, enc.threat.state, enc.objective
                    ));
                }
            }
            Step::WaitRecovery { timeout } => {
                if enc.stats.recoveries > self.baseline {
                    self.next();
                } else if enc.objective.is_over() || self.t > timeout {
                    out.finished = fail(format!(
                        "no recovery after {:.1}s; threat {:?}, exposure {:.2}, objective {:?}",
                        self.t, enc.threat.state, enc.threat.exposure, enc.objective
                    ));
                }
            }
            Step::WaitObjective { objective, timeout } => {
                if enc.threat.state != ThreatState::Resolved && enc.threat.state != ThreatState::Dormant {
                    out.intent.look_delta = self.face_threat(obs);
                }
                if enc.objective == objective {
                    self.next();
                } else if self.t > timeout {
                    out.finished = fail(format!(
                        "waited {:.1}s for {objective:?}; objective is {:?}",
                        self.t, enc.objective
                    ));
                }
            }
            Step::Restart => {
                out.restart = true;
                self.next();
            }
            Step::ExpectReset => {
                // Give the restart one frame to land.
                if self.t < 0.05 {
                    return out;
                }
                let fresh = enc.objective == Objective::FindSatchel
                    && enc.threat.state == ThreatState::Dormant
                    && enc.restitution == 0.0
                    && enc.elapsed < 0.5
                    && obs.pose.pos.distance(self.spawn) < 0.5;
                if fresh {
                    self.next();
                } else {
                    out.finished = fail(format!(
                        "restart did not reset: objective {:?}, threat {:?}, elapsed {:.2}, pos {:?}",
                        enc.objective, enc.threat.state, enc.elapsed, obs.pose.pos
                    ));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{evaluate_target, tick_input};
    use crate::sim::Event;

    /// Headless replay of the smoke route through the same rules the game
    /// uses: intent → look/walk with collision → crosshair targeting → truth.
    #[test]
    fn debug_route_wins_then_gets_caught_then_restarts_cleanly() {
        let layout = Layout::authored();
        let tuning = Tuning::default();
        let mut script = RouteScript::full(&layout, &tuning);
        let mut enc = Encounter::new(&layout);
        let mut pose = Pose::spawn(&layout);
        let mut events = Vec::new();
        let mut seen = Vec::new();
        let dt = 1.0 / 60.0;
        let mut target = evaluate_target(&layout, &tuning, &pose, &enc);
        for _ in 0..(60 * 60 * 15) {
            let frame = script.tick(&Observation {
                pose,
                encounter: &enc,
                target,
                dt,
            });
            if let Some(done) = frame.finished {
                done.unwrap_or_else(|e| panic!("route failed at {}: {e}", script.step_name()));
                break;
            }
            if frame.restart {
                enc.reset(&layout);
                pose = Pose::spawn(&layout);
                target = evaluate_target(&layout, &tuning, &pose, &enc);
                continue;
            }
            if enc.objective.is_over() {
                continue;
            }
            pose.look(frame.intent.look_delta, &tuning);
            pose.walk(&layout, &tuning, frame.intent.move_axis, enc.player_speed(&tuning), dt);
            target = evaluate_target(&layout, &tuning, &pose, &enc);
            events.clear();
            enc.step(
                &layout,
                &tuning,
                tick_input(dt, &pose, &frame.intent, target),
                &mut events,
            );
            seen.extend(events.iter().copied());
        }
        for e in [
            Event::SatchelTaken,
            Event::ThreatManifested,
            Event::WarningBegan,
            Event::HuntBegan,
            Event::LostTrack,
            Event::RestitutionComplete,
            Event::Escaped,
            Event::Caught,
        ] {
            assert!(seen.contains(&e), "route never produced {e:?}; saw {seen:?}");
        }
        assert_eq!(enc.stats, crate::sim::Stats::default());
    }
}
