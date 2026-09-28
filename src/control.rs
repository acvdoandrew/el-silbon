//! Player-side rules shared by the real controller, the debug route and the
//! headless tests: what an input frame means, how the body moves through the
//! authored geometry and what the crosshair is allowed to act on.

use bevy::math::{Vec2, Vec3};

use crate::geometry::{AimStatus, Layout};
use crate::sim::{Encounter, Objective, TickInput};
use crate::tuning::Tuning;

/// One frame of player intent, from devices or from the debug route.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intent {
    /// x = strafe right, y = forward; length ≤ 1.
    pub move_axis: Vec2,
    /// Radians: x = yaw to the right, y = pitch up.
    pub look_delta: Vec2,
    /// Interact went down this frame.
    pub interact_pressed: bool,
    /// Interact is held.
    pub interact_held: bool,
    pub toggle_flashlight: bool,
}

/// Where the player stands and looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub pos: Vec2,
    /// 0 looks north (−Z); positive turns left (counter-clockwise from above).
    pub yaw: f32,
    pub pitch: f32,
}

impl Pose {
    pub fn spawn(layout: &Layout) -> Self {
        Self {
            pos: layout.spawn,
            yaw: layout.spawn_yaw,
            pitch: 0.0,
        }
    }

    pub fn eye(&self, tuning: &Tuning) -> Vec3 {
        Vec3::new(self.pos.x, tuning.eye_height, self.pos.y)
    }

    pub fn forward2(&self) -> Vec2 {
        Vec2::new(-self.yaw.sin(), -self.yaw.cos())
    }

    pub fn right2(&self) -> Vec2 {
        Vec2::new(self.yaw.cos(), -self.yaw.sin())
    }

    pub fn look_dir(&self) -> Vec3 {
        let f = self.forward2() * self.pitch.cos();
        Vec3::new(f.x, self.pitch.sin(), f.y)
    }

    /// Apply a look delta (x = turn right, y = look up).
    pub fn look(&mut self, delta: Vec2, tuning: &Tuning) {
        self.yaw = wrap_angle(self.yaw - delta.x);
        self.pitch = (self.pitch + delta.y).clamp(-tuning.max_pitch, tuning.max_pitch);
    }

    /// Walk with collision. `axis` is clamped to unit length.
    pub fn walk(&mut self, layout: &Layout, tuning: &Tuning, axis: Vec2, speed: f32, dt: f32) {
        let axis = axis.clamp_length_max(1.0);
        if axis == Vec2::ZERO {
            return;
        }
        let wish = self.right2() * axis.x + self.forward2() * axis.y;
        self.pos = layout.move_circle(self.pos, wish * speed * dt, tuning.player_radius);
    }

    /// Yaw that faces a ground point.
    pub fn yaw_toward(from: Vec2, to: Vec2) -> f32 {
        let d = to - from;
        (-d.x).atan2(-d.y)
    }
}

pub fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut a = a % tau;
    if a > std::f32::consts::PI {
        a -= tau;
    } else if a < -std::f32::consts::PI {
        a += tau;
    }
    a
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetKind {
    Satchel,
    Note,
    Offering,
}

/// What the crosshair is on this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub kind: TargetKind,
    pub status: AimStatus,
}

impl Target {
    pub fn ready(&self) -> bool {
        matches!(self.status, AimStatus::Ready { .. })
    }
}

/// Evaluate every interactable the current objective allows. Occluded
/// targets produce no prompt at all; the nearest reachable one wins.
pub fn evaluate_target(layout: &Layout, tuning: &Tuning, pose: &Pose, enc: &Encounter) -> Option<Target> {
    if enc.objective.is_over() {
        return None;
    }
    let eye = pose.eye(tuning);
    let dir = pose.look_dir();
    let mut candidates: [Option<(TargetKind, Vec3, f32, f32)>; 3] = [None; 3];
    if enc.objective == Objective::FindSatchel {
        candidates[0] = Some((
            TargetKind::Satchel,
            layout.satchel,
            layout.satchel_radius,
            tuning.satchel_reach,
        ));
    }
    if enc.objective == Objective::ReturnBones {
        candidates[1] = Some((
            TargetKind::Offering,
            layout.ceiba.offering,
            layout.ceiba.offering_radius,
            tuning.offering_reach,
        ));
    }
    candidates[2] = Some((TargetKind::Note, layout.note, layout.note_radius, tuning.note_reach));

    let mut best: Option<(Target, f32, bool)> = None;
    for (kind, center, radius, reach) in candidates.into_iter().flatten() {
        let status = layout.aim(eye, dir, center, radius, reach);
        let (distance, ready) = match status {
            AimStatus::Ready { distance } => (distance, true),
            AimStatus::OutOfReach { distance } => (distance, false),
            AimStatus::NotAimed | AimStatus::Occluded => continue,
        };
        let better = match best {
            None => true,
            Some((_, d, r)) => (ready && !r) || (ready == r && distance < d),
        };
        if better {
            best = Some((Target { kind, status }, distance, ready));
        }
    }
    best.map(|(t, _, _)| t)
}

/// Turn an intent plus the current target into the truth layer's claims.
pub fn tick_input(dt: f32, pose: &Pose, intent: &Intent, target: Option<Target>) -> TickInput {
    let ready = |k: TargetKind| target.is_some_and(|t| t.kind == k && t.ready());
    TickInput {
        dt,
        player: pose.pos,
        take_satchel: intent.interact_pressed && ready(TargetKind::Satchel),
        hold_offering: intent.interact_held && ready(TargetKind::Offering),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_only_for_reachable_visible_targets_of_the_current_objective() {
        let layout = Layout::authored();
        let tuning = Tuning::default();
        let mut enc = Encounter::new(&layout);
        let mut pose = Pose {
            pos: Vec2::new(2.9, -3.3),
            yaw: 0.0,
            pitch: 0.0,
        };
        let aim_at = |pose: &mut Pose, p: Vec3| {
            let eye = pose.eye(&tuning);
            pose.yaw = Pose::yaw_toward(pose.pos, Vec2::new(p.x, p.z));
            let h = Vec2::new(p.x - eye.x, p.z - eye.z).length();
            pose.pitch = (p.y - eye.y).atan2(h);
        };
        aim_at(&mut pose, layout.satchel);
        let t = evaluate_target(&layout, &tuning, &pose, &enc).expect("satchel in view");
        assert_eq!(t.kind, TargetKind::Satchel);
        assert!(t.ready());

        // Pressing takes it; merely holding does not.
        let held_only = Intent {
            interact_held: true,
            ..Default::default()
        };
        assert!(!tick_input(0.016, &pose, &held_only, Some(t)).take_satchel);
        let press = Intent {
            interact_pressed: true,
            interact_held: true,
            ..Default::default()
        };
        assert!(tick_input(0.016, &pose, &press, Some(t)).take_satchel);

        // Once carried, the satchel is no longer a target.
        enc.objective = Objective::ReturnBones;
        let after = evaluate_target(&layout, &tuning, &pose, &enc);
        assert!(after.is_none_or(|t| t.kind != TargetKind::Satchel));

        // The hollow from across the paddock: aimed, visible, out of reach.
        pose.pos = Vec2::new(-10.0, -14.0);
        aim_at(&mut pose, layout.ceiba.offering);
        let far = evaluate_target(&layout, &tuning, &pose, &enc).expect("hollow in view");
        assert_eq!(far.kind, TargetKind::Offering);
        assert!(!far.ready());
        assert!(!tick_input(0.016, &pose, &press, Some(far)).hold_offering);
    }

    #[test]
    fn walking_uses_yaw_and_collides() {
        let layout = Layout::authored();
        let tuning = Tuning::default();
        let mut pose = Pose::spawn(&layout);
        // Yaw 0 walks north (−Z).
        pose.walk(&layout, &tuning, Vec2::new(0.0, 1.0), 3.0, 1.0);
        assert!((pose.pos - Vec2::new(0.0, 28.2)).length() < 1e-3);
        // Turning right by 90° then walking forward goes east (+X).
        pose.look(Vec2::new(std::f32::consts::FRAC_PI_2, 0.0), &tuning);
        pose.walk(&layout, &tuning, Vec2::new(0.0, 1.0), 2.0, 1.0);
        assert!((pose.pos - Vec2::new(2.0, 28.2)).length() < 1e-3);
        assert!((Pose::yaw_toward(Vec2::ZERO, Vec2::new(1.0, 0.0)) - pose.yaw).abs() < 1e-4);
    }
}
