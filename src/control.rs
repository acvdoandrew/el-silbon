//! Player-side rules shared by the host session, the client HUD, the debug
//! route and the headless tests: what an input frame means, how the body
//! moves through the authored geometry and what the crosshair may act on.

use bevy::math::{Vec2, Vec3};

use crate::geometry::{AimStatus, Layout, ground};
use crate::sim::{PlayerId, Relic};
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
    /// Crouch is held.
    pub crouch: bool,
    /// Sprint is held.
    pub sprint: bool,
    /// Put a bundle down this frame.
    pub drop: bool,
    /// Scatter a pepper this frame.
    pub use_aji: bool,
    /// Mark where you look this frame.
    pub ping: bool,
    /// Press for a skill check this frame: the check's id and where the
    /// needle stood as far as this player could tell.
    pub skill: Option<(u32, f32)>,
    /// Try this combination on the key box's padlock this frame.
    pub code: Option<[u8; 3]>,
    /// Name which of him walks tonight (a `sim::Variant` code) this frame.
    pub name: Option<u8>,
    /// Drive off in the ready truck now, without whoever is not aboard.
    pub drive_off: bool,
}

/// Where the player stands and looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub pos: Vec2,
    /// 0 looks north (−Z); positive turns left (counter-clockwise from above).
    pub yaw: f32,
    pub pitch: f32,
    /// How far the eye is lowered (crouching, lying downed).
    pub lower: f32,
}

impl Pose {
    pub fn spawn(layout: &Layout) -> Self {
        Self::at(layout.spawn, layout.spawn_yaw)
    }

    pub fn at(pos: Vec2, yaw: f32) -> Self {
        Self {
            pos,
            yaw,
            pitch: 0.0,
            lower: 0.0,
        }
    }

    /// The eye: standing, crouched or lying on the ground, and never under
    /// the water (someone downed in a channel floats, face up).
    pub fn eye(&self, tuning: &Tuning, layout: &Layout) -> Vec3 {
        let y = tuning.eye_height - self.lower + layout.surface_height(self.pos);
        let y = layout.water_line(self.pos).map_or(y, |w| y.max(w + 0.15));
        Vec3::new(self.pos.x, y, self.pos.y)
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

    /// Walk with collision at `speed`; `axis` is clamped to unit length.
    /// Returns the ground actually covered.
    pub fn walk(&mut self, layout: &Layout, tuning: &Tuning, axis: Vec2, speed: f32, dt: f32) -> f32 {
        let axis = axis.clamp_length_max(1.0);
        if axis == Vec2::ZERO || speed <= 0.0 {
            return 0.0;
        }
        let wish = self.right2() * axis.x + self.forward2() * axis.y;
        let before = self.pos;
        self.pos = layout.move_circle(self.pos, wish * speed * dt, tuning.player_radius);
        before.distance(self.pos)
    }

    /// Yaw that faces a ground point.
    pub fn yaw_toward(from: Vec2, to: Vec2) -> f32 {
        let d = to - from;
        (-d.x).atan2(-d.y)
    }

    /// Look straight at a world point from the current position.
    pub fn look_at(&mut self, tuning: &Tuning, layout: &Layout, point: Vec3) {
        let eye = self.eye(tuning, layout);
        self.yaw = Self::yaw_toward(self.pos, ground(point));
        let flat = Vec2::new(point.x - eye.x, point.z - eye.z).length();
        self.pitch = (point.y - eye.y).atan2(flat).clamp(-tuning.max_pitch, tuning.max_pitch);
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
    /// Bone bundle by index.
    Relic(u8),
    /// Pepper by index.
    Aji(u8),
    /// Spare torch batteries by index.
    Batteries(u8),
    /// Readable note by id.
    Note(u8),
    /// The altar in the ceiba's roots: lay bones down, or pray.
    Altar,
    Pump,
    Ignition,
    Beacon,
    /// The padlocked box with the truck key.
    Lockbox,
    /// A downed teammate.
    Body(PlayerId),
    /// Tureco, tied behind the house: hold to untie him.
    Dog,
    /// The dynamo's line panel at the windmill: press to switch lines.
    Panel,
}

impl TargetKind {
    /// Held rather than pressed.
    pub fn is_hold(self) -> bool {
        matches!(
            self,
            TargetKind::Altar
                | TargetKind::Pump
                | TargetKind::Ignition
                | TargetKind::Beacon
                | TargetKind::Body(_)
                | TargetKind::Dog
        )
    }
}

/// Why an interactable refuses to work yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    NeedBones,
    NeedPower,
    NeedKey,
}

/// What the crosshair is on this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub kind: TargetKind,
    pub status: AimStatus,
    pub blocked: Option<Blocked>,
    /// Where the aimed thing is (for marking it).
    pub center: Vec3,
}

impl Target {
    pub fn ready(&self) -> bool {
        matches!(self.status, AimStatus::Ready { .. })
    }
    /// In reach, in view and not refused.
    pub fn usable(&self) -> bool {
        self.ready() && self.blocked.is_none()
    }
}

/// Everything the crosshair needs to know about the shared world and the
/// player, built identically by the host (from truth) and the client (from
/// the latest snapshot).
#[derive(Clone, Debug)]
pub struct Scene<'a> {
    pub relics: &'a [Relic],
    pub aji_taken: &'a [bool],
    pub batteries_taken: &'a [bool],
    pub power_on: bool,
    pub truck_running: bool,
    pub bones_home: bool,
    pub beacon_ready: bool,
    /// The truck key is out of its box.
    pub key: bool,
    /// Tureco, while still tied: where he is.
    pub dog_tied: Option<Vec2>,
    /// Bundles this player carries, and peppers held.
    pub carrying: usize,
    pub aji_held: u8,
    /// This player's torch charge, 0..1.
    pub battery: f32,
    /// Downed teammates: id and position.
    pub bodies: &'a [(PlayerId, Vec2)],
    pub me: PlayerId,
    /// On their feet and not frozen by a susto.
    pub acting: bool,
}

/// Owned backing data for a [`Scene`]; the host builds it from truth, the
/// client from the latest snapshot.
#[derive(Clone, Debug, Default)]
pub struct SceneData {
    pub relics: Vec<Relic>,
    pub aji_taken: Vec<bool>,
    pub batteries_taken: Vec<bool>,
    pub power_on: bool,
    pub truck_running: bool,
    pub bones_home: bool,
    pub beacon_ready: bool,
    pub key: bool,
    pub dog_tied: Option<Vec2>,
    pub carrying: usize,
    pub aji_held: u8,
    pub battery: f32,
    pub bodies: Vec<(PlayerId, Vec2)>,
    pub me: PlayerId,
    pub acting: bool,
}

impl SceneData {
    pub fn scene(&self) -> Scene<'_> {
        Scene {
            relics: &self.relics,
            aji_taken: &self.aji_taken,
            batteries_taken: &self.batteries_taken,
            power_on: self.power_on,
            truck_running: self.truck_running,
            bones_home: self.bones_home,
            beacon_ready: self.beacon_ready,
            key: self.key,
            dog_tied: self.dog_tied,
            carrying: self.carrying,
            aji_held: self.aji_held,
            battery: self.battery,
            bodies: &self.bodies,
            me: self.me,
            acting: self.acting,
        }
    }
}

/// Evaluate every interactable the world currently allows. Occluded targets
/// produce no prompt at all; the nearest reachable one wins.
pub fn evaluate_target(layout: &Layout, tuning: &Tuning, pose: &Pose, scene: &Scene) -> Option<Target> {
    if !scene.acting {
        return None;
    }
    let d = &layout.district;
    let eye = pose.eye(tuning, layout);
    let dir = pose.look_dir();
    let mut candidates: Vec<(TargetKind, Vec3, f32, f32, Option<Blocked>)> = Vec::with_capacity(24);
    for (i, r) in scene.relics.iter().enumerate() {
        if let Relic::Ground(p) = r {
            candidates.push((
                TargetKind::Relic(i as u8),
                *p,
                crate::geometry::district::RELIC_RADIUS,
                tuning.relic_reach,
                None,
            ));
        }
    }
    if scene.aji_held < tuning.aji_max {
        for (i, p) in d.aji.iter().enumerate() {
            if !scene.aji_taken.get(i).copied().unwrap_or(true) {
                candidates.push((TargetKind::Aji(i as u8), *p, 0.22, tuning.relic_reach, None));
            }
        }
    }
    if scene.battery < tuning.battery_full {
        for (i, p) in d.batteries.iter().enumerate() {
            if !scene.batteries_taken.get(i).copied().unwrap_or(true) {
                candidates.push((TargetKind::Batteries(i as u8), *p, 0.2, tuning.relic_reach, None));
            }
        }
    }
    for n in &d.notes {
        candidates.push((TargetKind::Note(n.id), n.pos, 0.24, tuning.note_reach, None));
    }
    candidates.push((
        TargetKind::Altar,
        layout.ceiba.offering,
        layout.ceiba.offering_radius,
        tuning.altar_reach,
        None,
    ));
    if !scene.power_on {
        candidates.push((TargetKind::Pump, d.pump, 0.65, tuning.site_reach, None));
    }
    if !scene.truck_running {
        let blocked = if !scene.bones_home {
            Some(Blocked::NeedBones)
        } else if !scene.power_on {
            Some(Blocked::NeedPower)
        } else if !scene.key {
            Some(Blocked::NeedKey)
        } else {
            None
        };
        candidates.push((TargetKind::Ignition, d.ignition, 0.6, tuning.site_reach, blocked));
    }
    if !scene.key {
        candidates.push((TargetKind::Lockbox, d.lockbox, 0.2, tuning.site_reach, None));
    }
    if scene.power_on {
        candidates.push((TargetKind::Panel, d.panel, 0.25, tuning.site_reach, None));
    }
    if let Some(at) = scene.dog_tied {
        let y = layout.surface_height(at) + 0.45;
        candidates.push((TargetKind::Dog, Vec3::new(at.x, y, at.y), 0.45, tuning.body_reach, None));
    }
    if scene.beacon_ready {
        candidates.push((TargetKind::Beacon, d.beacon, 0.6, tuning.site_reach, None));
    }
    for &(id, at) in scene.bodies {
        if id != scene.me {
            // Where the body lies: afloat in deep water.
            let y = layout.rest_height(at) + 0.3;
            candidates.push((
                TargetKind::Body(id),
                Vec3::new(at.x, y, at.y),
                0.75,
                tuning.body_reach,
                None,
            ));
        }
    }

    // A downed teammate outranks anything lying beside them (usually the
    // bundle they dropped when they fell); otherwise the nearest wins.
    let mut best: Option<(Target, f32, bool, u8)> = None;
    for (kind, center, radius, reach, blocked) in candidates {
        let status = layout.aim(eye, dir, center, radius, reach);
        let (distance, ready) = match status {
            AimStatus::Ready { distance } => (distance, true),
            AimStatus::OutOfReach { distance } => (distance, false),
            AimStatus::NotAimed | AimStatus::Occluded => continue,
        };
        let priority = u8::from(matches!(kind, TargetKind::Body(_)));
        let better = match best {
            None => true,
            Some((_, d, r, pr)) => (ready && !r) || (ready == r && (priority > pr || (priority == pr && distance < d))),
        };
        if better {
            best = Some((
                Target {
                    kind,
                    status,
                    blocked,
                    center,
                },
                distance,
                ready,
                priority,
            ));
        }
    }
    best.map(|(t, ..)| t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene<'a>(relics: &'a [Relic], aji: &'a [bool], bodies: &'a [(PlayerId, Vec2)]) -> Scene<'a> {
        Scene {
            relics,
            aji_taken: aji,
            batteries_taken: &[],
            power_on: false,
            truck_running: false,
            bones_home: false,
            beacon_ready: true,
            key: false,
            dog_tied: None,
            carrying: 0,
            aji_held: 0,
            battery: 1.0,
            bodies,
            me: 1,
            acting: true,
        }
    }

    fn fixture() -> (Layout, Tuning, Vec<Relic>, Vec<bool>) {
        let l = Layout::new();
        let relics = l.district.relics.iter().map(|&p| Relic::Ground(p)).collect();
        let aji = vec![false; l.district.aji.len()];
        (l, Tuning::default(), relics, aji)
    }

    #[test]
    fn prompt_only_for_reachable_visible_targets() {
        let (l, t, relics, aji) = fixture();
        let s = scene(&relics, &aji, &[]);
        let mut pose = Pose::at(Vec2::new(2.9, -3.3), 0.0);
        pose.look_at(&t, &l, l.district.relics[0]);
        let hit = evaluate_target(&l, &t, &pose, &s).expect("bundle in view");
        assert_eq!(hit.kind, TargetKind::Relic(0));
        assert!(hit.usable());
        // Once carried it is no longer a target.
        let mut carried = relics.clone();
        carried[0] = Relic::Carried(1);
        let after = evaluate_target(&l, &t, &pose, &scene(&carried, &aji, &[]));
        assert!(after.is_none_or(|x| x.kind != TargetKind::Relic(0)));
        // The altar from across the clearing: aimed, visible, out of reach.
        let altar = l.ceiba.offering;
        let back = (Vec2::new(altar.x, altar.z) - l.ceiba.center).normalize();
        let mut far = Pose::at(Vec2::new(altar.x, altar.z) + back * 9.0, 0.0);
        far.look_at(&t, &l, altar);
        let seen = evaluate_target(&l, &t, &far, &s).expect("altar in view");
        assert_eq!(seen.kind, TargetKind::Altar);
        assert!(!seen.ready());
        // A frozen or downed player targets nothing.
        let mut frozen = scene(&relics, &aji, &[]);
        frozen.acting = false;
        assert!(evaluate_target(&l, &t, &pose, &frozen).is_none());
    }

    #[test]
    fn shrine_note_leaves_the_offering_target_clear_for_returning_carriers() {
        let (l, t, relics, aji) = fixture();
        let mut s = scene(&relics, &aji, &[]);
        s.carrying = 2;
        for at in [Vec2::new(-18.8, -46.9), Vec2::new(-18.8, -45.7)] {
            let mut pose = Pose::at(at, 0.0);
            pose.look_at(&t, &l, l.ceiba.offering);
            let hit = evaluate_target(&l, &t, &pose, &s).expect("offering is reachable");
            assert_eq!(hit.kind, TargetKind::Altar);
            assert!(hit.usable());
        }
    }

    #[test]
    fn a_wall_hides_the_prompt_and_crouching_lowers_the_eye() {
        let (l, t, relics, aji) = fixture();
        let s = scene(&relics, &aji, &[]);
        // Outside the east wall, looking through it at the table bundle.
        let mut pose = Pose::at(Vec2::new(5.6, -5.2), 0.0);
        pose.look_at(&t, &l, l.district.relics[0]);
        assert!(evaluate_target(&l, &t, &pose, &s).is_none_or(|x| x.kind != TargetKind::Relic(0)));
        let standing = Pose::at(Vec2::new(0.0, 12.0), 0.0);
        let mut low = standing;
        low.lower = t.crouch_lower;
        assert!(low.eye(&t, &l).y < standing.eye(&t, &l).y - 0.5);
    }

    #[test]
    fn the_ignition_says_why_it_refuses_and_only_downed_teammates_are_bodies() {
        let (l, t, relics, aji) = fixture();
        let mut pose = Pose::at(Vec2::new(55.7, 29.4), 0.0);
        pose.look_at(&t, &l, l.district.ignition);
        let mut s = scene(&relics, &aji, &[]);
        let hit = evaluate_target(&l, &t, &pose, &s).expect("ignition in view");
        assert_eq!(hit.kind, TargetKind::Ignition);
        assert!(hit.ready() && !hit.usable());
        assert_eq!(hit.blocked, Some(Blocked::NeedBones));
        s.bones_home = true;
        assert_eq!(
            evaluate_target(&l, &t, &pose, &s).unwrap().blocked,
            Some(Blocked::NeedPower)
        );
        s.power_on = true;
        assert_eq!(
            evaluate_target(&l, &t, &pose, &s).unwrap().blocked,
            Some(Blocked::NeedKey)
        );
        s.key = true;
        assert!(evaluate_target(&l, &t, &pose, &s).unwrap().usable());
        // A downed teammate lying in the road is a body target; you are not.
        let body_at = Vec2::new(40.0, 30.0);
        let bodies = [(2, body_at), (1, Vec2::new(0.0, 0.0))];
        let s2 = scene(&relics, &aji, &bodies);
        let mut medic = Pose::at(body_at + Vec2::new(0.0, -1.4), 0.0);
        medic.look_at(&t, &l, Vec3::new(body_at.x, l.surface_height(body_at) + 0.3, body_at.y));
        let found = evaluate_target(&l, &t, &medic, &s2).expect("teammate in view");
        assert_eq!(found.kind, TargetKind::Body(2));
        assert!(found.kind.is_hold() && found.usable());
    }

    #[test]
    fn walking_uses_yaw_collides_and_reports_ground_covered() {
        let (l, t, _, _) = fixture();
        let mut pose = Pose::spawn(&l);
        // Yaw 0 walks north (−Z).
        let moved = pose.walk(&l, &t, Vec2::new(0.0, 1.0), 3.0, 1.0);
        assert!((pose.pos - Vec2::new(0.0, 28.2)).length() < 1e-3);
        assert!((moved - 3.0).abs() < 1e-3);
        // Turning right by 90° then walking forward goes east (+X).
        pose.look(Vec2::new(std::f32::consts::FRAC_PI_2, 0.0), &t);
        pose.walk(&l, &t, Vec2::new(0.0, 1.0), 2.0, 1.0);
        assert!((pose.pos - Vec2::new(2.0, 28.2)).length() < 1e-3);
        assert!((Pose::yaw_toward(Vec2::ZERO, Vec2::new(1.0, 0.0)) - pose.yaw).abs() < 1e-4);
        // Walking into the fence covers no ground.
        let mut blocked = Pose::at(Vec2::new(10.0, 22.6), 0.0);
        assert!(blocked.walk(&l, &t, Vec2::new(0.0, 1.0), 3.0, 1.0) < 0.6);
        // No speed, no motion.
        assert_eq!(pose.walk(&l, &t, Vec2::new(0.0, 1.0), 0.0, 1.0), 0.0);
    }
}
