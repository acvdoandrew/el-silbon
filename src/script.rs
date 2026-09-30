//! DEBUG ONLY: the deterministic smoke routes.
//!
//! A scripted player that plays the real run through the same intent path as
//! a human: it turns and walks with look/move deltas, relies on the real
//! crosshair targeting, presses and holds interact, marks places and asks
//! for restarts through the same commands as the menu. It never writes
//! truth: no teleports, no objective setters, no muted threat. What it reads
//! is what a listener's snapshot carries (the HUD's data), plus the map.
//!
//! One engine serves the solo smoke (`--smoke`, `--tour`), the headless and
//! rendered two-process network smokes (`net::smoke`) and the session tests:
//! [`RouteScript::tick`] turns an [`Observation`] into a [`ScriptFrame`].
//!
//! Routes (see `RouteScript::full`): bundles from the table, the corral, the
//! fields, the caño and the lookout deck to the ceiba altar; the windmill
//! pump; the truck's ignition and warm-up; every survivor standing at the
//! truck for the escape (WIN); restart; then a second night in which he finds
//! us, we break line of sight until he loses us (recovery), stand in the open
//! until he catches us (FAILURE); restart. `tour` first walks a ground-level
//! circuit of every landmark.
//!
//! While the route is busy, a reflex plays the survival the game asks of a
//! player: the moment he warns or hunts, it turns to face him (a flick, the
//! horizon level) and finds the nearest place he cannot see (deep in the
//! shadow of a palm trunk, a wall or the truck, or crouched in tall grass
//! beyond his grass sight), straight there or round a corner. With such a
//! place a step or two away it stays in his view until he commits to the
//! hunt, then vanishes into it: a hunt that loses its prey sends him far
//! away, where an averted warning leaves him close by. Otherwise it runs for
//! cover at once, and scatters pepper when he closes in with a hunt in
//! sight. Once he has lost us the route carries on, except in lamplight,
//! where fear eases and waiting for him to wander off is worth it. The
//! snapshot is its judge: a hunt in sight says he sees us, a hunt out of
//! sight says he does not, and a cover that leaves the warning standing is
//! given up. Its paths come from a grid search over the real collision
//! blockers, allocated once per step.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use bevy::math::{Vec2, Vec3};

use crate::control::{Intent, Pose, Target, TargetKind, evaluate_target, wrap_angle};
use crate::geometry::district::LandmarkId;
use crate::geometry::{Layout, Shape, Sight, Wade, ground};
use crate::net::protocol::{Action, PlayerId, Snapshot};
use crate::sim::{Encounter, Outcome};
use crate::tuning::Tuning;

const TURN_RATE: f32 = 4.0;
const ARRIVE: f32 = 0.3;
/// Grid pitch of the local path search, metres.
const CELL: f32 = 0.25;
/// Seconds without a warning or hunt before the reflex may let go.
const CALM_FOR: f32 = 2.0;
/// Lying low: seconds he may stay out of view (while we sweep the horizon
/// for him from a place we can see out of) before we believe he has gone.
const GONE: f32 = 6.0;
/// Lying low in a shadow we cannot see out of: by now he has lost patience
/// waiting where he last had us and roams.
const SHADOW_WAIT: f32 = 20.0;
/// Lying low never lasts longer than this.
const LOW_MAX: f32 = 30.0;
/// Turn rate (radians per second) of the sweep for him while lying low.
const SWEEP: f32 = 3.0;
/// Turn rate of the survival reflex: a flick of the mouse, not a pan.
const SPIN: f32 = 8.0;
/// A sighting this recent (seconds) still says where he stands when a
/// warning begins.
const GLIMPSE: f32 = 1.2;
/// A sighting this recent (seconds) is a hint where to look first.
const HINT: f32 = 45.0;
/// Seconds after a susto lets go of us in which being seen is the scream's
/// doing, not the hiding place's: time to crouch back into it.
const SHAKEN: f32 = 0.8;
/// How far ahead of a corner (metres) the walk turns on: the body it sees
/// lags the authority by a frame or two at walking speed.
const LEAD: f32 = 0.35;
/// Warned with a hiding place this close (metres of walk), stay in his view
/// until he commits to the hunt, then vanish: a hunt that loses us sends
/// him far away, where an averted warning leaves him close by.
const BAIT_RANGE: f32 = 2.5;
/// Never bait him from closer than this (metres).
const BAIT_MIN: f32 = 9.0;

/// How far (metres) beyond the truck's boarding zone hiding may stray while
/// the engine warms.
const TRUCK_STRAY: f32 = 12.0;
/// Farthest (metres, straight) a hiding place round a corner is sought.
const DETOUR_RANGE: f32 = 18.0;
/// Most hiding places round a corner planned per choice.
const MAX_DETOURS: usize = 6;
/// Closest he may come (metres) before the reflex scatters pepper.
const PEPPER_RANGE: f32 = 9.5;
/// Closest (metres) he may come before pepper is scattered when there is no
/// cover to run to: close in this far first, so the ward reaches him at once.
const CHARGE_RANGE: f32 = 6.5;

/// The road to the doorstep, then in through the front door to the table.
const DOOR_APPROACH: &[[f32; 2]] = &[[0.0, 27.0], [0.0, 18.0], [0.0, 4.0]];
const DOOR_INSIDE: &[[f32; 2]] = &[[0.0, 4.0], [0.0, 0.0], [0.6, -1.5], [2.9, -3.3]];
/// Out the back door toward the western track.
const BACK_DOOR: &[[f32; 2]] = &[[-3.0, -5.0], [-3.0, -7.4], [-3.0, -8.0], [-10.0, -23.0]];
/// In by the back door from the western track.
const BACK_DOOR_IN: &[[f32; 2]] = &[[-10.0, -23.0], [-3.0, -8.0], [-3.0, -7.4], [-3.0, -5.0]];
/// Back out the front door to the porch.
const DOOR_OUT: &[[f32; 2]] = &[[0.6, -1.5], [0.0, 0.0], [0.0, 4.0]];

/// Yaw/pitch deltas that turn the view toward (yaw, pitch), rate-limited.
pub fn steer(pose: &Pose, yaw: f32, pitch: f32, dt: f32) -> Vec2 {
    steer_at(pose, yaw, pitch, TURN_RATE, dt)
}

/// [`steer`] at `rate` radians per second.
fn steer_at(pose: &Pose, yaw: f32, pitch: f32, rate: f32, dt: f32) -> Vec2 {
    let max = rate * dt;
    Vec2::new(
        wrap_angle(pose.yaw - yaw).clamp(-max, max),
        (pitch - pose.pitch).clamp(-max, max),
    )
}

/// The pose once this frame's look has been applied. The host moves the body
/// in the yaw the input packet carries, i.e. after the look, so every move
/// axis of a frame is worked out in this basis.
fn after_look(pose: &Pose, look: Vec2) -> Pose {
    Pose {
        yaw: wrap_angle(pose.yaw - look.x),
        ..*pose
    }
}

/// The move axis that walks along the world direction `dir` for a body
/// facing as `after` does.
fn axis_along(after: &Pose, dir: Vec2) -> Vec2 {
    Vec2::new(dir.dot(after.right2()), dir.dot(after.forward2()))
}

/// Yaw and pitch that put the crosshair on a world point.
pub fn aim_angles(layout: &Layout, tuning: &Tuning, pose: &Pose, p: Vec3) -> (f32, f32) {
    let eye = pose.eye(tuning, layout);
    let yaw = Pose::yaw_toward(pose.pos, Vec2::new(p.x, p.z));
    let flat = Vec2::new(p.x - eye.x, p.z - eye.z).length();
    (yaw, (p.y - eye.y).atan2(flat))
}

pub fn aim_at(layout: &Layout, tuning: &Tuning, pose: &Pose, p: Vec3, dt: f32) -> Vec2 {
    let (yaw, pitch) = aim_angles(layout, tuning, pose, p);
    steer(pose, yaw, pitch, dt)
}

/// Turn toward `goal` on the ground and walk straight at it once roughly
/// facing it (the axis is worked out for the yaw after this frame's look).
pub fn toward(pose: &Pose, goal: Vec2, dt: f32) -> Intent {
    let yaw = Pose::yaw_toward(pose.pos, goal);
    let look_delta = steer(pose, yaw, 0.0, dt);
    let after = after_look(pose, look_delta);
    let mut intent = Intent {
        look_delta,
        ..Default::default()
    };
    if wrap_angle(after.yaw - yaw).abs() < 0.6 {
        intent.move_axis = axis_along(&after, (goal - pose.pos).normalize_or_zero());
    }
    intent
}

/// What the crosshair is on for `me`, from a listener's snapshot alone (the
/// same evaluation the client HUD makes).
pub fn crosshair(layout: &Layout, tuning: &Tuning, snap: &Snapshot, me: PlayerId, pose: &Pose) -> Option<Target> {
    let data = snap.scene_data(me, snap.me.stun > 0.0);
    evaluate_target(layout, tuning, pose, &data.scene())
}

// ---------------------------------------------------------------- path search

const UNKNOWN: u8 = 0;
const OPEN: u8 = 1;
const MID: u8 = 2;
const TIGHT: u8 = 3;
/// Waist-deep water, whatever the room.
const DEEP: u8 = 4;
const BLOCKED: u8 = 5;
/// Path cost multiplier by clearance class: prefer open ground, squeeze
/// through doors when nothing else exists. Wading waist-deep takes about
/// three times as long as walking and everything hears it, so it costs
/// twice that where it is allowed (see [`Water`]).
const COST: [f32; 6] = [0.0, 1.0, 1.5, 3.0, 6.0, f32::INFINITY];

/// Which water a path search may cross.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Water {
    /// Waist-deep too, at `COST[DEEP]`, where that saves a long way.
    Wade,
    /// Never waist-deep; shallows (the ford, a channel's edge) are fine.
    Dry,
}

/// How much room a walker of `radius` has at `p`, or waist-deep water.
fn classify(layout: &Layout, radius: f32, p: Vec2) -> u8 {
    let b = layout.bounds;
    let class = if p.x < b.min.x + radius || p.x > b.max.x - radius || p.y < b.min.y + radius || p.y > b.max.y - radius
    {
        BLOCKED
    } else if layout.is_free(p, radius + 0.25) {
        OPEN
    } else if layout.is_free(p, radius + 0.12) {
        MID
    } else if layout.is_free(p, radius + 0.02) {
        TIGHT
    } else {
        BLOCKED
    };
    if class != BLOCKED && layout.wade(p) == Wade::Deep {
        DEEP
    } else {
        class
    }
}

/// Every sampled circle along a → b is free of blockers.
fn segment_free(layout: &Layout, a: Vec2, b: Vec2, clearance: f32) -> bool {
    let steps = ((a.distance(b) / 0.2).ceil() as usize).max(1);
    (0..=steps).all(|k| layout.is_free(a.lerp(b, k as f32 / steps as f32), clearance))
}

/// Somewhere along a → b is waist-deep water.
fn segment_deep(layout: &Layout, a: Vec2, b: Vec2) -> bool {
    let steps = ((a.distance(b) / 0.2).ceil() as usize).max(1);
    (0..=steps).any(|k| layout.wade(a.lerp(b, k as f32 / steps as f32)) == Wade::Deep)
}

fn poly_len(from: Vec2, path: &[Vec2]) -> f32 {
    let mut at = from;
    let mut len = 0.0;
    for &p in path {
        len += at.distance(p);
        at = p;
    }
    len
}

#[derive(Clone, Copy)]
struct Open {
    f: f32,
    i: u32,
}
impl PartialEq for Open {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Open {}
impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Open {
    /// Reversed: the binary heap pops the cheapest first.
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.total_cmp(&self.f).then_with(|| other.i.cmp(&self.i))
    }
}

/// Clearance of one search cell, computed on first touch.
fn cell_class(
    class: &mut [u8],
    layout: &Layout,
    radius: f32,
    lo: Vec2,
    w: usize,
    x: usize,
    y: usize,
    forced: [usize; 2],
) -> u8 {
    let i = y * w + x;
    if class[i] == UNKNOWN {
        let mut c = classify(layout, radius, lo + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * CELL);
        if c == BLOCKED && forced.contains(&i) {
            c = TIGHT;
        }
        class[i] = c;
    }
    class[i]
}

/// Reused scratch space for path searches over the collision layout.
#[derive(Default)]
struct Nav {
    g: Vec<f32>,
    parent: Vec<u32>,
    class: Vec<u8>,
    closed: Vec<bool>,
    open: BinaryHeap<Open>,
}

impl Nav {
    /// A* on a fine grid inside the padded box around both ends, then
    /// string-pulled. `out` receives the corners after `from`, ending exactly
    /// at `to`. False when no collision-free way (a dry one, for
    /// [`Water::Dry`]) exists inside the box.
    fn local(&mut self, layout: &Layout, radius: f32, from: Vec2, to: Vec2, water: Water, out: &mut Vec<Vec2>) -> bool {
        out.clear();
        let lo = from.min(to) - Vec2::splat(8.0);
        let hi = from.max(to) + Vec2::splat(8.0);
        let w = ((hi.x - lo.x) / CELL).ceil() as usize + 1;
        let h = ((hi.y - lo.y) / CELL).ceil() as usize + 1;
        let n = w * h;
        if n > 400_000 {
            return false;
        }
        self.g.clear();
        self.g.resize(n, f32::INFINITY);
        self.parent.clear();
        self.parent.resize(n, u32::MAX);
        self.class.clear();
        self.class.resize(n, UNKNOWN);
        self.closed.clear();
        self.closed.resize(n, false);
        self.open.clear();
        let cell = |p: Vec2| {
            let c = (p - lo) / CELL;
            ((c.x.max(0.0) as usize).min(w - 1), (c.y.max(0.0) as usize).min(h - 1))
        };
        let ((sx, sy), (gx, gy)) = (cell(from), cell(to));
        let (start, goal) = (sy * w + sx, gy * w + gx);
        let forced = [start, goal];
        let heuristic = |x: usize, y: usize| {
            let (dx, dy) = (x.abs_diff(gx) as f32, y.abs_diff(gy) as f32);
            (dx.max(dy) + 0.414 * dx.min(dy)) * CELL
        };
        self.g[start] = 0.0;
        self.open.push(Open {
            f: heuristic(sx, sy),
            i: start as u32,
        });
        let passable = |class: u8| class != BLOCKED && (class != DEEP || water == Water::Wade);
        let mut found = false;
        let mut expanded = 0usize;
        while let Some(Open { i, .. }) = self.open.pop() {
            let i = i as usize;
            if self.closed[i] {
                continue;
            }
            self.closed[i] = true;
            if i == goal {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > 300_000 {
                break;
            }
            let (x, y) = (i % w, i / w);
            for (dx, dy) in [
                (1i32, 0i32),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                let ni = ny * w + nx;
                if self.closed[ni] {
                    continue;
                }
                let class = cell_class(&mut self.class, layout, radius, lo, w, nx, ny, forced);
                if !passable(class) {
                    continue;
                }
                let diagonal = dx != 0 && dy != 0;
                if diagonal
                    && (!passable(cell_class(&mut self.class, layout, radius, lo, w, nx, y, forced))
                        || !passable(cell_class(&mut self.class, layout, radius, lo, w, x, ny, forced)))
                {
                    continue;
                }
                let step = if diagonal { 1.414 } else { 1.0 } * CELL * COST[class as usize];
                let g = self.g[i] + step;
                if g < self.g[ni] {
                    self.g[ni] = g;
                    self.parent[ni] = i as u32;
                    self.open.push(Open {
                        f: g + heuristic(nx, ny),
                        i: ni as u32,
                    });
                }
            }
        }
        if !found {
            return false;
        }
        let mut points = vec![to];
        let mut at = self.parent[goal];
        while at != u32::MAX && at as usize != start {
            let (x, y) = (at as usize % w, at as usize / w);
            points.push(lo + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * CELL);
            at = self.parent[at as usize];
        }
        points.push(from);
        points.reverse();
        // String-pull: the farthest corner still reachable in a straight line,
        // never cutting through deep water the search kept out of (deep[k]:
        // how many of the first k cells were waist-deep).
        let clearance = radius + 0.06;
        let mut deep = vec![0u32; points.len() + 1];
        for (k, &p) in points.iter().enumerate() {
            deep[k + 1] = deep[k] + u32::from(layout.wade(p) == Wade::Deep);
        }
        let straight = |i: usize, j: usize| {
            segment_free(layout, points[i], points[j], clearance)
                && (deep[j + 1] > deep[i] || !segment_deep(layout, points[i], points[j]))
        };
        let mut i = 0;
        while i + 1 < points.len() {
            let mut j = (i + 60).min(points.len() - 1);
            while j > i + 1 && !straight(i, j) {
                j -= 1;
            }
            out.push(points[j]);
            i = j;
        }
        true
    }

    /// Corners from `from` to `to`. The lookout deck is only reachable up its
    /// ramp, so a walk on or off it goes through the ramp's foot.
    fn plan(&mut self, layout: &Layout, tuning: &Tuning, from: Vec2, to: Vec2, out: &mut Vec<Vec2>) -> bool {
        let d = &layout.district;
        let on_lookout = |p: Vec2| d.watch_ramp.contains(p) || d.watch_deck.contains(p);
        if on_lookout(from) == on_lookout(to) {
            return self.route(layout, tuning, from, to, out);
        }
        let foot = d.landmark(LandmarkId::Watchtower).approach;
        let r = tuning.player_radius;
        let mut first = Vec::new();
        let mut second = Vec::new();
        let ok = if on_lookout(to) {
            self.route(layout, tuning, from, foot, &mut first)
                && self.local(layout, r, foot, to, Water::Wade, &mut second)
        } else {
            self.local(layout, r, from, foot, Water::Wade, &mut first)
                && self.route(layout, tuning, foot, to, &mut second)
        };
        out.clear();
        if ok {
            out.extend(first);
            out.extend(second);
        }
        ok
    }

    /// A way on the ground: dry if there is one anywhere, and only otherwise
    /// through waist-deep water. The box around a short walk can hold no dry
    /// way where the far bank is near (from the lookout's ramp foot to the
    /// base shed it holds neither the bridge nor the bank road), and then the
    /// trail network goes round by them, as before the caño could be waded.
    fn route(&mut self, layout: &Layout, tuning: &Tuning, from: Vec2, to: Vec2, out: &mut Vec<Vec2>) -> bool {
        self.route_by(layout, tuning, from, to, Water::Dry, out)
            || self.route_by(layout, tuning, from, to, Water::Wade, out)
    }

    /// Straight grid search when close, otherwise onto the trail network,
    /// along it, and off again, crossing only the `water` given.
    fn route_by(
        &mut self,
        layout: &Layout,
        tuning: &Tuning,
        from: Vec2,
        to: Vec2,
        water: Water,
        out: &mut Vec<Vec2>,
    ) -> bool {
        let r = tuning.player_radius;
        if from.distance(to) <= 40.0 && self.local(layout, r, from, to, water, out) {
            return true;
        }
        out.clear();
        let patrol = &layout.patrol;
        // Enough nodes that the nearest one across water or a fence cannot
        // crowd out the one a short dry walk away.
        let nearest = |p: Vec2| {
            let mut ids: Vec<usize> = (0..patrol.len()).collect();
            ids.sort_by(|&a, &b| {
                patrol.nodes[a]
                    .distance_squared(p)
                    .total_cmp(&patrol.nodes[b].distance_squared(p))
            });
            ids.truncate(7);
            ids
        };
        let (starts, ends) = (nearest(from), nearest(to));
        let mut onto: Vec<Option<Vec<Vec2>>> = Vec::new();
        for &a in &starts {
            let mut leg = Vec::new();
            let ok =
                from.distance(patrol.nodes[a]) < 60.0 && self.local(layout, r, from, patrol.nodes[a], water, &mut leg);
            onto.push(ok.then_some(leg));
        }
        let mut off: Vec<Option<Vec<Vec2>>> = Vec::new();
        for &b in &ends {
            let mut leg = Vec::new();
            let ok = to.distance(patrol.nodes[b]) < 60.0 && self.local(layout, r, patrol.nodes[b], to, water, &mut leg);
            off.push(ok.then_some(leg));
        }
        let mut best: Option<(f32, usize, usize)> = None;
        for (i, &a) in starts.iter().enumerate() {
            let Some(first) = &onto[i] else { continue };
            for (j, &b) in ends.iter().enumerate() {
                let Some(last) = &off[j] else { continue };
                let along = patrol.path_len(a, b);
                if !along.is_finite() {
                    continue;
                }
                let cost = poly_len(from, first) + along + poly_len(patrol.nodes[b], last);
                if best.is_none_or(|(c, ..)| cost < c) {
                    best = Some((cost, i, j));
                }
            }
        }
        let Some((_, i, j)) = best else {
            return false;
        };
        out.extend(onto[i].iter().flatten());
        let (mut at, goal) = (starts[i], ends[j]);
        while at != goal {
            let next = patrol.next_hop(at, goal);
            if next == at {
                out.clear();
                return false;
            }
            at = next;
            out.push(patrol.nodes[at]);
        }
        out.extend(off[j].iter().flatten());
        true
    }
}

/// A spot to stand and act on `target`: free ground, in sight of it, close
/// enough to reach it from the eye. Nearest to `from` first, but where he
/// was lately seen (`wary`), spots in the shadow of something between him
/// and us come before spots in his sight; `skip` walks down the list of
/// distinct spots when earlier ones proved unreachable.
fn stand_spot(
    layout: &Layout,
    tuning: &Tuning,
    target: Vec3,
    reach: f32,
    from: Vec2,
    wary: Option<Vec2>,
    skip: usize,
) -> Option<Vec2> {
    let at = ground(target);
    let mut spots: Vec<(f32, Vec2)> = Vec::new();
    for radius in [0.7_f32, 1.0, 1.3, 1.6, 1.9] {
        for k in 0..24 {
            let a = k as f32 * std::f32::consts::TAU / 24.0;
            let p = at + Vec2::new(a.cos(), a.sin()) * radius;
            if !layout.bounds.contains(p)
                || !layout.is_free(p, tuning.player_radius + 0.15)
                || !layout.line_of_sight(p, at)
            {
                continue;
            }
            let eye = Vec3::new(p.x, layout.surface_height(p) + tuning.eye_height, p.y);
            if eye.distance(target) > reach * 0.8 {
                continue;
            }
            let seen = wary.is_some_and(|th| cover_at(layout, tuning, p, th, GRASS_MARGIN) == Cover::Open);
            spots.push((from.distance(p) + 0.5 * radius + if seen { 20.0 } else { 0.0 }, p));
        }
    }
    spots.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut distinct: Vec<Vec2> = Vec::new();
    for (_, p) in spots {
        if distinct.iter().all(|q| q.distance(p) > 0.7) {
            distinct.push(p);
        }
    }
    distinct.get(skip).copied()
}

/// How well a place hides us from him, weakest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cover {
    /// In plain sight of him.
    Open,
    /// Behind something, but on the edge of its shadow: a step aside shows us.
    Edge,
    /// Crouched in tall grass, beyond the range he sees into it from.
    Grass,
    /// Deep in the shadow of a trunk, a wall or a prop.
    Shadow,
}

impl Cover {
    fn hides(self) -> bool {
        self != Cover::Open
    }
}

/// A body's width around `p` lies inside tall grass.
fn in_grass(layout: &Layout, p: Vec2, margin: f32) -> bool {
    [Vec2::ZERO, Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
        .into_iter()
        .all(|o| layout.district.tall_grass_at(p + o * margin))
}

/// Grass margin (metres) a place to run to must have all round: arriving a
/// little off the mark still leaves the body inside the grass.
const GRASS_MARGIN: f32 = 0.6;
/// Grass margin where we stand: the rules conceal a crouching body whose
/// centre is in the grass; this allows for the body we see trailing its truth.
const GRASS_HERE: f32 = 0.15;

/// How well `p` hides us from `threat`. The shadow test also looks a body's
/// width to either side, so a place on the very edge of a shadow is only
/// [`Cover::Edge`]; the grass hides a crouching body beyond `grass_sight`
/// when its centre is `margin` inside it.
fn cover_at(layout: &Layout, tuning: &Tuning, p: Vec2, threat: Vec2, margin: f32) -> Cover {
    let blocked = !layout.line_of_sight(p, threat);
    if blocked {
        let d = threat - p;
        let side = Vec2::new(-d.y, d.x).normalize_or_zero() * 0.4;
        if !layout.line_of_sight(p + side, threat) && !layout.line_of_sight(p - side, threat) {
            return Cover::Shadow;
        }
    }
    if in_grass(layout, p, margin) && p.distance(threat) > tuning.grass_sight + 1.0 {
        Cover::Grass
    } else if blocked {
        Cover::Edge
    } else {
        Cover::Open
    }
}

/// Where he stands and where we are, for judging places to hide.
struct Peril<'a> {
    layout: &'a Layout,
    tuning: &'a Tuning,
    pos: Vec2,
    threat: Vec2,
    leash: Option<(Vec2, f32)>,
    /// Places that hid us on paper but not from him.
    failed: &'a [Vec2],
}

/// A place to run to: where, how well it hides us, what reaching it costs
/// (lower is better) and the corners on the way there (none: straight).
#[derive(Clone, Debug, Default)]
struct Refuge {
    at: Vec2,
    cover: Option<Cover>,
    score: f32,
    path: Vec<Vec2>,
}

impl Refuge {
    fn hides(&self) -> bool {
        self.cover.is_some_and(Cover::hides)
    }
}

impl Peril<'_> {
    /// How well `p` hides us and what walking `walk` metres to it costs
    /// (lower is better): `None` outside the leash, on a place that failed,
    /// or on top of him.
    fn rate(&self, p: Vec2, walk: f32) -> Option<(Cover, f32)> {
        let (layout, tuning) = (self.layout, self.tuning);
        let r = tuning.player_radius;
        if !layout.bounds.contains(p)
            || !layout.is_free(p, r + 0.2)
            || self.leash.is_some_and(|(c, radius)| p.distance(c) > radius)
            || self.failed.iter().any(|f| f.distance(p) < 1.5)
        {
            return None;
        }
        let there = p.distance(self.threat);
        if there < 3.0 {
            return None;
        }
        let closer = 0.8 * (self.pos.distance(self.threat) - there).max(0.0);
        let wet = if layout.wading(p) { 3.0 } else { 0.0 };
        let cover = cover_at(layout, tuning, p, self.threat, GRASS_MARGIN);
        let score = match cover {
            Cover::Open => 0.1 * walk - there,
            Cover::Edge => walk + closer + wet + 4.0,
            Cover::Grass => walk + closer + wet + 1.0,
            Cover::Shadow => walk + closer + wet,
        };
        Some((cover, score))
    }

    /// The way from here to `p`: straight when nothing is in between,
    /// otherwise round what is, if that is a short walk; with its length.
    fn way(&self, nav: &mut Nav, p: Vec2) -> Option<(f32, Vec<Vec2>)> {
        let r = self.tuning.player_radius;
        let straight = self.pos.distance(p);
        if segment_free(self.layout, self.pos, p, r + 0.05) {
            return Some((straight, Vec::new()));
        }
        if straight > DETOUR_RANGE {
            return None;
        }
        let mut path = Vec::new();
        if !nav.local(self.layout, r, self.pos, p, Water::Wade, &mut path) {
            return None;
        }
        let len = poly_len(self.pos, &path);
        (len <= 2.0 * straight + 4.0).then_some((len, path))
    }

    /// The nearest place he cannot see us: candidates on rings around us, in
    /// the shadow of every sight-blocking trunk, wall and prop nearby, and at
    /// the near edge of every tall-grass patch, reached straight or round a
    /// corner. With nothing hiding us in reach, the place farthest from him.
    fn pick_hide(&self, cands: &mut Vec<Vec2>, nav: &mut Nav) -> Option<Refuge> {
        let (layout, pos, threat) = (self.layout, self.pos, self.threat);
        cands.clear();
        for ring in [2.5_f32, 4.0, 6.0, 8.0, 11.0, 14.0, 18.0, 23.0] {
            for k in 0..24 {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                cands.push(pos + Vec2::new(a.cos(), a.sin()) * ring);
            }
        }
        for b in &layout.blockers {
            if b.sight != Sight::Blocks {
                continue;
            }
            let center = match b.shape {
                Shape::Circle { center, .. } => center,
                Shape::Rect(rect) => rect.center(),
            };
            if center.distance(pos) > 30.0 {
                continue;
            }
            let away = (center - threat).normalize_or_zero();
            if away == Vec2::ZERO {
                continue;
            }
            // Out of the far side of the shape along the line from him.
            let extent = match b.shape {
                Shape::Circle { radius, .. } => radius,
                Shape::Rect(rect) => {
                    let h = rect.half();
                    (h.x / away.x.abs().max(1e-3)).min(h.y / away.y.abs().max(1e-3))
                }
            };
            cands.push(center + away * (extent + 0.9));
        }
        for g in &layout.district.grass {
            let entry = pos.max(g.min + Vec2::splat(1.0)).min(g.max - Vec2::splat(1.0));
            if entry.distance(pos) < 40.0 {
                cands.push(entry);
                cands.push(entry + (g.center() - entry).normalize_or_zero() * 1.5);
            }
        }
        // Rated first as if straight there (the least the walk can be),
        // hiding places before open ground; then settled in that order by
        // the way actually walked, until nothing left can beat the best.
        let mut rated: Vec<(bool, f32, Vec2)> = cands
            .iter()
            .filter_map(|&p| {
                self.rate(p, pos.distance(p))
                    .map(|(cover, score)| (cover.hides(), score, p))
            })
            .collect();
        rated.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        let mut best: Option<Refuge> = None;
        let mut detours = 0;
        for (hides, bound, p) in rated {
            if let Some(b) = &best
                && (b.hides() && !hides || (b.hides() == hides && bound >= b.score))
            {
                break;
            }
            if !segment_free(layout, pos, p, self.tuning.player_radius + 0.05) {
                if detours >= MAX_DETOURS {
                    continue;
                }
                detours += 1;
            }
            let Some((walk, path)) = self.way(nav, p) else {
                continue;
            };
            let Some((cover, score)) = self.rate(p, walk) else {
                continue;
            };
            let better = best
                .as_ref()
                .is_none_or(|b| (cover.hides() && !b.hides()) || (cover.hides() == b.hides() && score < b.score));
            if better {
                best = Some(Refuge {
                    at: p,
                    cover: Some(cover),
                    score,
                    path,
                });
            }
        }
        best
    }
}

// -------------------------------------------------------------------- reflex

enum Turn {
    /// He is no concern: the step runs.
    Clear,
    /// He just lost us: the step resumes, and must re-plan from here.
    Resumed,
    /// The reflex has the controls this frame.
    Busy,
}

/// Survival: face him, get out of his sight (beside cover, once he commits to
/// the hunt), lie low where it is lit, pepper when he charges.
#[derive(Default)]
struct Reflex {
    /// Where he was last in view, and how many seconds ago.
    seen: Option<(Vec2, f32)>,
    /// `seen` is this warning's (or hunt's) sighting, not an older one.
    located: bool,
    /// Not located yet: look where he was last seen before sweeping.
    hint: bool,
    /// The place we are running to, and how well it hides us.
    hide: Option<(Vec2, Cover)>,
    /// The corners on the way to `hide` still ahead.
    way: Vec<Vec2>,
    nav: Nav,
    /// Places that hid us on paper but not from him, this encounter.
    failed: Vec<Vec2>,
    evading: bool,
    /// A warning or hunt stood last frame.
    alarmed: bool,
    /// Seconds since the danger passed (lying low).
    calm: f32,
    retarget: f32,
    /// Seconds a warning has outlasted the cover we believe in.
    covered: f32,
    /// Seconds the hunt has been in sight while we are in grass.
    sighted: f32,
    /// Closest we have come to the hiding place, and for how long we have not
    /// come closer.
    closest: f32,
    stalled: f32,
    aji_wait: f32,
    /// Seconds left in which a susto (ours: frozen, standing, screaming),
    /// not the place, is what showed us to him.
    shaken: f32,
    cands: Vec<Vec2>,
}

impl Reflex {
    /// A warning or hunt begins: nothing decided before still holds.
    fn begin(&mut self, snap: &Snapshot) {
        self.evading = true;
        self.drop_hide();
        self.retarget = 0.0;
        self.failed.clear();
        self.covered = 0.0;
        self.sighted = 0.0;
        self.closest = f32::MAX;
        self.stalled = 0.0;
        self.locate(snap);
    }

    /// A warning or hunt begins (perhaps anew while we lie low): a glimpse
    /// just before it still says where he stands; an older one (perhaps an
    /// earlier encounter, and he has walked since) only says where to look
    /// first.
    fn locate(&mut self, snap: &Snapshot) {
        self.located = snap.threat.is_some() || self.seen.is_some_and(|s| s.1 <= GLIMPSE);
        self.hint = !self.located && self.seen.is_some_and(|s| s.1 < HINT);
    }

    /// Lying low, may we walk on? Not before the danger has passed for a
    /// moment. In the dark, then. In lamplight, once he shows far enough
    /// away that he could not notice even a standing, lit player at the
    /// height of the night, or has stayed out of view long enough while we
    /// sweep for him, or, hidden in a shadow we cannot see out of, once he
    /// has had time to lose patience; never longer than `LOW_MAX`.
    fn clear_to_go(&self, layout: &Layout, tuning: &Tuning, snap: &Snapshot, pos: Vec2, cover: Cover) -> bool {
        if self.calm < CALM_FOR {
            return false;
        }
        // Fear grows in the dark: lying low there only waits for a susto
        // (frozen, standing up and screaming). Lamplight calms it, so only a
        // lit hiding place is worth waiting in until he gives up.
        if !layout.is_lit(pos, snap.world.circuits) || self.calm >= LOW_MAX {
            return true;
        }
        let Some((at, age)) = self.seen else {
            return true;
        };
        if age < 0.25 {
            let far = tuning.at_pressure(1.0).warn_distance * tuning.sight_light + 2.0;
            return at.distance(pos) > far;
        }
        match cover {
            Cover::Shadow | Cover::Edge => self.calm >= SHADOW_WAIT,
            Cover::Grass | Cover::Open => age >= GONE,
        }
    }

    fn drop_hide(&mut self) {
        self.hide = None;
        self.way.clear();
    }

    fn stand_down(&mut self) {
        self.evading = false;
        self.alarmed = false;
        self.located = false;
        self.hint = false;
        self.drop_hide();
        self.failed.clear();
        self.calm = 0.0;
        self.covered = 0.0;
        self.sighted = 0.0;
    }

    /// Remember a place that did not hide us from him.
    fn fail(&mut self, at: Vec2) {
        if self.failed.iter().any(|f| f.distance(at) < 1.0) {
            return;
        }
        if self.failed.len() >= 12 {
            self.failed.remove(0);
        }
        self.failed.push(at);
    }

    /// The reflex's state, for failure reports.
    fn note(&self) -> String {
        format!(
            "reflex: evading {}, located {}, seen {:?}, hide {:?}, {} failed places",
            self.evading,
            self.located,
            self.seen,
            self.hide,
            self.failed.len()
        )
    }

    /// Warned or hunted without having seen him: he is outside our view.
    /// Look where he was last seen, then sweep the horizon until he shows,
    /// the horizon level.
    fn search(&mut self, obs: &Observation, snap: &Snapshot, out: &mut ScriptFrame) -> Turn {
        let (pose, dt) = (&obs.pose, obs.dt);
        let max = SPIN * dt;
        let mut look = Vec2::new(max, (-pose.pitch).clamp(-max, max));
        if !matches!(snap.danger, 1..=3) {
            look.x = 0.0;
        } else if self.hint {
            match self.seen.filter(|s| s.1 < HINT) {
                Some((at, _)) => {
                    let yaw = Pose::yaw_toward(pose.pos, at);
                    if wrap_angle(pose.yaw - yaw).abs() > 0.3 {
                        look = steer_at(pose, yaw, 0.0, SPIN, dt);
                    } else {
                        self.hint = false;
                    }
                }
                None => self.hint = false,
            }
        }
        out.intent.look_delta = look;
        // Out of his sight already (a hunt he cannot see): keep low.
        out.intent.crouch = snap.danger == 3;
        Turn::Busy
    }

    /// Every frame, whoever has the controls: remember where he was last in
    /// view and how long ago (the memory ages while he is out of view).
    fn watch(&mut self, snap: &Snapshot, dt: f32) {
        match snap.threat {
            Some(t) => self.seen = Some((Vec2::from_array(t.position), 0.0)),
            None => {
                if let Some(s) = &mut self.seen {
                    s.1 += dt;
                }
            }
        }
    }

    fn tick(&mut self, obs: &Observation, snap: &Snapshot, leash: Option<(Vec2, f32)>, out: &mut ScriptFrame) -> Turn {
        // Judge the grass by his anger: the bones line says how angry he is.
        let raged = obs.tuning.at_rage(snap.world.delivered);
        let (layout, tuning, dt) = (obs.layout, &raged, obs.dt);
        let pos = obs.pose.pos;
        self.aji_wait = (self.aji_wait - dt).max(0.0);
        let alarm = matches!(snap.danger, 1..=3);
        if alarm {
            self.calm = 0.0;
            if !self.evading {
                self.begin(snap);
                out.log = Some("he has seen us: breaking line of sight".into());
            } else if !self.alarmed {
                // Found again while lying low: where he stands now matters.
                self.locate(snap);
                self.covered = 0.0;
                self.sighted = 0.0;
                out.log = Some("he has found us again".into());
            }
        } else if self.evading {
            self.calm += dt;
        }
        self.alarmed = alarm;
        if !self.evading {
            return Turn::Clear;
        }
        // Hiding: the beam off, as anyone would.
        out.dark = true;
        if snap.threat.is_some() {
            self.located = true;
        }
        let threat = match self.seen {
            Some((at, _)) if self.located => at,
            _ if alarm => return self.search(obs, snap, out),
            // The danger passed before he ever showed: nothing to hide from.
            _ => {
                self.stand_down();
                out.log = Some("out of his sight; carrying on".into());
                return Turn::Resumed;
            }
        };

        // The snapshot judges our cover: a hunt in sight says he sees us, a
        // hunt out of sight that he does not, and him showing in our view
        // along a clear line that he sees us back (grass hides us from him
        // however clear the line). Cover that fails the judge is given up.
        let dist = pos.distance(threat);
        let cover = cover_at(layout, tuning, pos, threat, GRASS_HERE);
        // Seconds a hunt has had us in sight while we crouch in grass: the
        // grass needs a moment to take (we crouch as we arrive).
        self.sighted = if snap.danger == 2 && cover == Cover::Grass {
            self.sighted + dt
        } else {
            0.0
        };
        // A susto stands us up screaming wherever we hide: being seen through
        // it (and for a moment after, while we crouch again) says nothing of
        // the place, so we stay in it.
        self.shaken = if snap.me.stun > 0.0 {
            SHAKEN
        } else {
            (self.shaken - dt).max(0.0)
        };
        let shaken = self.shaken > 0.0;
        if shaken {
            self.sighted = 0.0;
        }
        let exposed = match snap.danger {
            2 => cover != Cover::Grass || self.sighted >= 0.4,
            3 => false,
            _ => snap.threat.is_some() && cover != Cover::Grass,
        };
        if exposed && cover.hides() && !shaken {
            self.fail(pos);
        }
        let arrived = self.hide.is_some_and(|(at, _)| at.distance(pos) <= 0.4);
        let settled = matches!(cover, Cover::Shadow | Cover::Grass) || (cover == Cover::Edge && arrived);
        let mut hidden = snap.danger == 3 || (settled && (!exposed || shaken));
        if hidden && snap.danger == 1 {
            // He unsees us within `warn_break_time`; a warning that lasts
            // much longer means we are not hidden from him.
            self.covered += dt;
            if self.covered > 1.6 {
                self.fail(pos);
                self.covered = 0.0;
                self.drop_hide();
                hidden = false;
            }
        } else {
            self.covered = 0.0;
        }
        if !alarm && self.clear_to_go(layout, tuning, snap, pos, cover) {
            self.stand_down();
            out.log = Some("he has moved on; carrying on".into());
            return Turn::Resumed;
        }
        if arrived
            && !settled
            && let Some((at, meant)) = self.hide
            && meant.hides()
        {
            // Standing on the place we ran to for cover, and it does not
            // hide us: another.
            self.fail(at);
            self.drop_hide();
        }

        // Face him: yaw at once, the horizon level; lying low with him out
        // of view, sweep the horizon for him. The walk below is worked out
        // for the yaw after this look.
        let yaw = Pose::yaw_toward(pos, threat);
        let look = if alarm || self.seen.is_some_and(|s| s.1 < 1.0) {
            steer_at(&obs.pose, yaw, 0.0, SPIN, dt)
        } else {
            let max = SWEEP * dt;
            Vec2::new(max, (-obs.pose.pitch).clamp(-max, max))
        };
        let after = after_look(&obs.pose, look);
        let facing = wrap_angle(after.yaw - yaw).abs();
        out.intent.look_delta = look;
        if hidden {
            // Wait it out, low: crouched in grass we are hidden as well.
            out.intent.crouch = true;
            return Turn::Busy;
        }

        self.retarget -= dt;
        if self.hide.is_none() || self.retarget <= 0.0 {
            self.retarget = 0.4;
            // Keep the place we run to unless another is clearly better.
            let peril = Peril {
                layout,
                tuning,
                pos,
                threat,
                leash,
                failed: &self.failed,
            };
            let best = peril.pick_hide(&mut self.cands, &mut self.nav);
            let keep = self.hide.and_then(|(at, _)| {
                let walk = poly_len(pos, &self.way) + self.way.last().map_or(pos, |&c| c).distance(at);
                peril.rate(at, walk).map(|(cover, score)| Refuge {
                    at,
                    cover: Some(cover),
                    score,
                    path: std::mem::take(&mut self.way),
                })
            });
            let pick = match (best, keep) {
                (Some(b), Some(k)) => {
                    if (k.hides() && !b.hides()) || (k.hides() == b.hides() && b.score + 1.5 >= k.score) {
                        Some(k)
                    } else {
                        Some(b)
                    }
                }
                (best, keep) => best.or(keep),
            };
            match pick {
                Some(Refuge { at, cover, path, .. }) => {
                    if self.hide.is_none_or(|(old, _)| old != at) {
                        self.closest = f32::MAX;
                        self.stalled = 0.0;
                    }
                    self.hide = cover.map(|c| (at, c));
                    self.way = path;
                }
                None => self.drop_hide(),
            }
        }
        // Corners passed (or already in a straight line past them) are done.
        while let Some(&corner) = self.way.first() {
            let next = self.way.get(1).copied().or(self.hide.map(|(at, _)| at));
            let passed = pos.distance(corner) < ARRIVE + LEAD
                || next.is_some_and(|n| {
                    pos.distance(corner) < 1.5 && segment_free(layout, pos, n, tuning.player_radius + 0.05)
                });
            if !passed {
                break;
            }
            self.way.remove(0);
        }
        // Not getting anywhere (something in the way): another place. A
        // susto freezes us; that is no reason to give the place up.
        if let Some((at, _)) = self.hide
            && snap.me.stun <= 0.0
        {
            let to_go = poly_len(pos, &self.way) + self.way.last().map_or(pos, |&c| c).distance(at);
            if to_go < self.closest - 0.05 {
                self.closest = to_go;
                self.stalled = 0.0;
            } else {
                self.stalled += dt;
                if self.stalled > 1.0 && to_go > 0.5 {
                    self.fail(at);
                    self.drop_hide();
                    self.closest = f32::MAX;
                    self.stalled = 0.0;
                    self.retarget = 0.0;
                }
            }
        }

        let mut dir: Option<Vec2> = None;
        let mut run = false;
        // Metres still to walk to the hiding place, round its corners.
        let remaining = self
            .hide
            .map(|(at, _)| poly_len(pos, &self.way) + self.way.last().map_or(pos, |&c| c).distance(at));
        if let (Some((at, _)), Some(remaining)) = (self.hide, remaining) {
            let to = self.way.first().copied().unwrap_or(at) - pos;
            let len = to.length();
            if len > 0.05 {
                dir = Some(to / len);
                // Running is loud: only while he is on to us.
                run = alarm && remaining > 1.0;
            }
        }
        // Warned, in his view, with cover a step or two away: hold still
        // until the hunt begins, then step into it.
        let bait = snap.danger == 1
            && dist > BAIT_MIN
            && self.hide.is_some_and(|(_, cover)| cover.hides())
            && remaining.is_some_and(|walk| walk <= BAIT_RANGE);
        if bait {
            dir = None;
            run = false;
        }
        // A hunt in sight with pepper in hand. Exposure fills in seconds:
        // where cover is out of reach in time, close in and scatter it (a
        // ward stops him where it touches him); where cover is in reach,
        // scatter it only if the margin is thin. One ward at a time: a
        // standing ward is a wall he will not cross.
        if snap.danger == 2 && snap.me.aji > 0 {
            let ward = snap
                .zones
                .iter()
                .any(|z| z[2] > 0.5 && Vec2::new(z[0], z[1]).distance(pos) < 5.5);
            if !ward {
                let near = 1.0 - (dist / tuning.warn_distance).clamp(0.0, 1.0);
                let pace = (1.0 + tuning.exposure_near_boost * near) / (tuning.exposure_time * 0.7);
                let left = (1.0 - snap.exposure).max(0.0) / pace;
                let speed = tuning.walk_speed
                    * if snap.me.stamina > 0.1 {
                        tuning.sprint_factor
                    } else {
                        1.0
                    };
                let can_hide = self
                    .hide
                    .zip(remaining)
                    .is_some_and(|((_, cover), walk)| cover.hides() && walk / speed + 0.5 < left);
                let range = if can_hide { PEPPER_RANGE } else { CHARGE_RANGE };
                if dist < range {
                    if facing < 0.25 && self.aji_wait <= 0.0 && (!can_hide || left < 2.5) {
                        out.intent.use_aji = true;
                        self.aji_wait = 1.5;
                    }
                    if !can_hide {
                        dir = None;
                        run = false;
                    }
                } else if !can_hide && leash.is_none() {
                    dir = Some((threat - pos).normalize_or_zero());
                    run = true;
                }
            }
        }
        if let Some(d) = dir {
            out.intent.move_axis = axis_along(&after, d);
            out.intent.sprint = run && snap.me.stamina > 0.05;
        }
        Turn::Busy
    }
}

// --------------------------------------------------------------------- steps

/// What the script asks for of the crosshair target.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    Bundle(usize),
    Aji(usize),
    Batteries(usize),
    Altar,
    Pump,
    Ignition,
    /// The key box: tried with the code from the three pages that hold it.
    Lockbox,
    /// The dynamo's line panel.
    Panel,
    /// The shelf radio: tuned to the numbers station on the padlock's tag,
    /// then listened to until every digit has been counted.
    Radio,
}

impl Want {
    fn matches(self, kind: TargetKind) -> bool {
        match (self, kind) {
            (Want::Bundle(i), TargetKind::Relic(j)) => i == j as usize,
            (Want::Aji(i), TargetKind::Aji(j)) => i == j as usize,
            (Want::Batteries(i), TargetKind::Batteries(j)) => i == j as usize,
            (Want::Altar, TargetKind::Altar)
            | (Want::Pump, TargetKind::Pump)
            | (Want::Ignition, TargetKind::Ignition)
            | (Want::Lockbox, TargetKind::Lockbox)
            | (Want::Panel, TargetKind::Panel)
            | (Want::Radio, TargetKind::Radio) => true,
            _ => false,
        }
    }

    fn reach(self, tuning: &Tuning) -> f32 {
        match self {
            Want::Bundle(_) | Want::Aji(_) | Want::Batteries(_) => tuning.relic_reach,
            Want::Altar => tuning.altar_reach,
            Want::Pump | Want::Ignition | Want::Lockbox | Want::Panel => tuning.site_reach,
            Want::Radio => tuning.note_reach,
        }
    }

    /// Where the thing is now: a dropped bundle lies where it was dropped.
    fn point(self, layout: &Layout, snap: &Snapshot) -> Vec3 {
        let d = &layout.district;
        match self {
            Want::Bundle(i) => snap
                .relics
                .get(i)
                .filter(|r| r.state == 0)
                .map_or(d.relics[i], |r| Vec3::from_array(r.pos)),
            Want::Aji(i) => d.aji[i],
            Want::Batteries(i) => d.batteries[i],
            Want::Altar => layout.ceiba.offering,
            Want::Pump => d.pump,
            Want::Ignition => d.ignition,
            Want::Lockbox => d.lockbox,
            Want::Panel => d.panel,
            Want::Radio => d.radio,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cond {
    /// This player carries bundle `i`.
    Holding(usize),
    /// Bundle `i` lies on the ground.
    Ground(usize),
    Delivered(usize),
    /// Pepper `i` has been picked up.
    AjiTaken(usize),
    /// Spare batteries `i` have been picked up.
    BatteriesTaken(usize),
    Power,
    /// The truck key is out of its box.
    Key,
    /// Lamp line `c` is live.
    Line(u8),
    /// The engine runs.
    Truck,
    /// He has warned at least once this run.
    Warned,
    /// A warning was averted, or he lost track, or he finished counting.
    Recovered,
    Outcome(Outcome),
    Started,
    /// The host restarted: the epoch is one ahead of the route's.
    NextRun,
    Party(usize),
    /// Marks on the map by this player and by someone else.
    Marks {
        mine: bool,
        other: bool,
    },
    /// A partner crouches (the route's acknowledgement signal).
    Crouched,
    /// Someone else carries bundle `i` (the host moved on: an acknowledgement).
    Carried(usize),
    /// Every digit of the padlock has been counted off the radio.
    Digits,
}

/// An interaction the route waits out.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Job {
    want: Want,
    until: Cond,
    /// Take a screenshot when the hold passes this fraction.
    shot: Option<(f32, &'static str)>,
    timeout: f32,
    /// Skip instead of failing the route.
    soft: bool,
}

impl Job {
    fn new(want: Want, until: Cond) -> Self {
        Self {
            want,
            until,
            shot: None,
            timeout: 300.0,
            soft: false,
        }
    }

    fn shot(mut self, at: f32, name: &'static str) -> Self {
        self.shot = Some((at, name));
        self
    }

    fn soft(mut self) -> Self {
        self.soft = true;
        self.timeout = 60.0;
        self
    }
}

/// Where the reflex may hide.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Leash {
    Free,
    /// At the truck while its engine warms: everyone standing must be in its
    /// boarding zone once it is warm, so hiding strays no farther than
    /// `TRUCK_STRAY` beyond the zone and, calm again, walks back into it.
    Truck,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Log(&'static str),
    Wait(f32),
    /// Turn the view toward a world point (screenshot framing).
    Face([f32; 3]),
    Capture(&'static str),
    /// Walk fixed corners (the doorway legs).
    Via(&'static [[f32; 2]]),
    /// Walk anywhere. Unguarded, being caught ends the step, not the route.
    Go {
        to: Vec2,
        guard: bool,
    },
    /// Walk to a spot, aim, and use the crosshair target until the condition
    /// holds.
    Do(Job),
    /// Stand and watch (facing him while he is visible). Guarded, the reflex
    /// plays; unguarded, being caught is fine.
    Await {
        cond: Cond,
        timeout: f32,
        guard: bool,
        leash: Leash,
    },
    /// Mark the altar until the mark shows in the snapshot.
    Ping,
    /// Hold crouch until the condition holds (an acknowledgement a partner can see).
    Crouch(Cond),
    /// The host starts the run.
    Start,
    Restart,
    /// The restart landed: a new epoch, and everything back at the start.
    ExpectReset,
    /// Disconnect (the network smoke).
    Leave,
}

impl Step {
    /// The reflex is on, and a fall or an early ending is the route's failure.
    fn guarded(&self) -> bool {
        match self {
            Step::Go { guard, .. } | Step::Await { guard, .. } => *guard,
            Step::Via(_) | Step::Do(_) | Step::Ping | Step::Crouch(_) => true,
            _ => false,
        }
    }

    fn awaits_outcome(&self) -> bool {
        matches!(
            self,
            Step::Await {
                cond: Cond::Outcome(_),
                ..
            }
        )
    }

    /// Steps during which the host may already have restarted.
    fn allows_next_run(&self) -> bool {
        matches!(
            self,
            Step::Restart
                | Step::ExpectReset
                | Step::Await {
                    cond: Cond::NextRun,
                    ..
                }
        )
    }
}

/// What the script sees each frame (the same data the HUD shows).
pub struct Observation<'a> {
    pub layout: &'a Layout,
    pub tuning: &'a Tuning,
    pub me: PlayerId,
    pub pose: Pose,
    /// Diagnostics only: the route decides from the snapshot.
    pub encounter: &'a Encounter,
    pub snapshot: Option<&'a Snapshot>,
    pub target: Option<Target>,
    pub dt: f32,
}

/// What the script asks for this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptFrame {
    pub intent: Intent,
    pub capture: Option<&'static str>,
    pub restart: bool,
    /// The host starts the run.
    pub start: bool,
    /// Mark this place for the party.
    pub ping: Option<[f32; 3]>,
    /// Disconnect now.
    pub leave: bool,
    /// Keep the torch switched off: a lit torch he can see draws him.
    pub dark: bool,
    pub log: Option<String>,
    /// Some(Ok) = route complete, Some(Err) = the route failed.
    pub finished: Option<Result<(), String>>,
}

impl ScriptFrame {
    /// The explicit session command of this frame (start, restart, mark).
    pub fn command(&self) -> Option<Action> {
        if self.restart {
            Some(Action::Restart)
        } else if self.start {
            Some(Action::Start)
        } else {
            self.ping.map(|at| Action::Ping { at })
        }
    }
}

enum Follow {
    Moving,
    Done,
    Stuck,
}

pub struct RouteScript {
    steps: Vec<Step>,
    index: usize,
    /// The run epoch the route expects to be playing.
    run: u64,
    /// Seconds the current step has run (reflex time excluded).
    t: f32,
    total: f32,
    nav: Nav,
    /// The walk in progress, planned once per step and reused.
    path: Vec<Vec2>,
    scratch: Vec<Vec2>,
    waypoint: usize,
    planned: bool,
    /// The reflex ended: re-plan from where it left us.
    stale: bool,
    attempt: usize,
    replans: u32,
    best: f32,
    stall: f32,
    stare: f32,
    press: f32,
    sent: f32,
    captured: bool,
    reflex: Reflex,
    spawn: Vec2,
    /// The skill check we are watching: its id, the needle in the snapshot
    /// we last saw, seconds since that snapshot, and whether we pressed.
    check: Option<(u32, f32, f32, bool)>,
    /// Counting the numbers station's pips, and the run time heard up to.
    ear: crate::radio::Listener,
    heard_to: Option<f64>,
}

// ------------------------------------------------------------------ routes

fn to_table(s: &mut Vec<Step>, layout: &Layout) {
    use Step::*;
    let d = &layout.district;
    s.extend([
        Via(DOOR_APPROACH),
        Via(DOOR_INSIDE),
        Log("route: the bundle on the table"),
        Face(d.relics[0].to_array()),
        Capture("02_table"),
        Do(Job::new(Want::Bundle(0), Cond::Holding(0))),
    ]);
}

fn bundle(i: usize) -> Step {
    Step::Do(Job::new(Want::Bundle(i), Cond::Holding(i)))
}

/// A pepper lying beside the route: taken if the pouch has room and it can
/// be reached quickly, otherwise left where it is.
fn pepper(i: usize) -> Step {
    Step::Do(Job {
        timeout: 25.0,
        ..Job::new(Want::Aji(i), Cond::AjiTaken(i)).soft()
    })
}

fn deliver(n: usize) -> Job {
    Job::new(Want::Altar, Cond::Delivered(n))
}

/// Spare batteries beside the route: taken when the torch has run low enough
/// to want them and they are quick to reach, otherwise left.
fn batteries(i: usize) -> Step {
    Step::Do(Job {
        timeout: 25.0,
        ..Job::new(Want::Batteries(i), Cond::BatteriesTaken(i)).soft()
    })
}

/// Run one: every bundle, the pump, the truck, the escape; then restart.
fn win_run(s: &mut Vec<Step>, layout: &Layout) {
    use Step::*;
    win_to_altar(s, layout, true);
    win_rest(s, layout);
    s.extend([
        Wait(1.0),
        Capture("10_escaped"),
        Wait(0.6),
        Restart,
        ExpectReset,
        Log("route: the escape ends, restart ok"),
    ]);
}

/// The opening of run one: the road in, the peppers, the table bundle, and
/// its delivery to the altar. Alone (`radio`), the numbers are counted off
/// the shelf radio first, while he still sleeps; in company a friend does it.
fn win_to_altar(s: &mut Vec<Step>, layout: &Layout, radio: bool) {
    use Step::*;
    let d = &layout.district;
    let ranch = d.landmark(LandmarkId::Ranch);
    s.extend([
        Log("route: the road in"),
        Wait(1.5),
        Face(ranch.look.to_array()),
        Capture("01_roadside"),
        Log("route: peppers on the way"),
        Do(Job::new(Want::Aji(0), Cond::AjiTaken(0)).soft()),
        Via(DOOR_APPROACH),
        Do(Job::new(Want::Aji(1), Cond::AjiTaken(1)).soft()),
        Via(DOOR_INSIDE),
    ]);
    if radio {
        s.extend([
            Log("route: the radio's numbers hour, while he still sleeps"),
            Do(Job::new(Want::Radio, Cond::Digits)),
        ]);
    }
    s.extend([
        Log("route: the bundle on the table"),
        Face(d.relics[0].to_array()),
        Capture("02_table"),
        bundle(0),
        Log("route: carrying the bones to the ceiba"),
        Via(BACK_DOOR),
        Do(deliver(1).shot(0.55, "03_altar")),
    ]);
}

/// The rest of run one, from the first delivery to the win: the other four
/// bundles, the pump, the truck, and the wait at the truck until the engine
/// is warm and every survivor stands in its zone.
fn win_rest(s: &mut Vec<Step>, layout: &Layout) {
    use Step::*;
    let d = &layout.district;
    let tower = d.landmark(LandmarkId::Watchtower);
    let ramp_foot = Vec2::new(tower.approach.x, tower.approach.y - 5.0);
    s.extend([
        Face([layout.ceiba.center.x, 9.0, layout.ceiba.center.y]),
        Capture("04_ceiba"),
        Log("route: the corral and the fields"),
        pepper(5),
        batteries(2),
        bundle(1),
        bundle(2),
        Do(deliver(3)),
        Log("route: the caño and the lookout"),
        pepper(6),
        batteries(3),
        bundle(3),
        Go {
            to: tower.approach,
            guard: true,
        },
        Go {
            to: ramp_foot,
            guard: true,
        },
        bundle(4),
        Go {
            to: ramp_foot,
            guard: true,
        },
        Go {
            to: tower.approach,
            guard: true,
        },
        Do(deliver(5)),
        Log("route: the windmill"),
        pepper(4),
        batteries(1),
        Do(Job::new(Want::Pump, Cond::Power).shot(0.5, "05_pump")),
        Face([d.pump.x, d.pump.y + 3.0, d.pump.z]),
        Capture("06_power"),
        Log("route: the key box at the windmill, with the numbers from the radio (or a friend's)"),
        Do(Job::new(Want::Lockbox, Cond::Key)),
        Log("route: the bridge line on at the panel, for the wait at the truck"),
        Do(Job::new(Want::Panel, Cond::Line(2))),
        Log("route: the truck"),
        Do(Job::new(Want::Ignition, Cond::Truck).shot(0.5, "09_ignition")),
        Face([d.truck.center.x, 1.5, d.truck.center.y]),
        Capture("09_truck"),
        Log("route: the engine warms; everyone stands at the truck"),
        Await {
            cond: Cond::Outcome(Outcome::Won),
            timeout: 240.0,
            guard: true,
            leash: Leash::Truck,
        },
    ]);
}

/// Run two: he finds us, we break sight until he loses us, then stand in
/// the open until he catches us; then restart.
fn fail_run(s: &mut Vec<Step>, layout: &Layout) {
    use Step::*;
    let exposed = layout.district.landmark(LandmarkId::Corral).approach;
    to_table(s, layout);
    s.push(Via(DOOR_OUT));
    s.extend([
        Log("route: the bundle wakes him; into the open"),
        Go {
            to: exposed,
            guard: true,
        },
        Await {
            cond: Cond::Warned,
            timeout: 600.0,
            guard: false,
            leash: Leash::Free,
        },
        Capture("07_warning"),
        Log("route: he has seen us; break line of sight until he loses us"),
        Await {
            cond: Cond::Recovered,
            timeout: 180.0,
            guard: true,
            leash: Leash::Free,
        },
        Log("route: recovered; standing in the open until he catches us"),
        Go {
            to: exposed,
            guard: false,
        },
        Await {
            cond: Cond::Outcome(Outcome::Failed),
            timeout: 1500.0,
            guard: false,
            leash: Leash::Free,
        },
        // The catch: a second of silence and his whistle, then him; capture
        // him over the caught player, and let the black lift before the
        // restart.
        Wait(1.9),
        Capture("08_caught"),
        Wait(2.5),
        Restart,
        ExpectReset,
        Log("route: restart ok"),
    ]);
}

impl RouteScript {
    fn new(steps: Vec<Step>, layout: &Layout) -> Self {
        Self {
            steps,
            index: 0,
            run: 1,
            t: 0.0,
            total: 0.0,
            nav: Nav::default(),
            path: Vec::new(),
            scratch: Vec::new(),
            waypoint: 0,
            planned: false,
            stale: false,
            attempt: 0,
            replans: 0,
            best: f32::MAX,
            stall: 0.0,
            stare: 0.0,
            press: 0.0,
            sent: 0.0,
            captured: false,
            reflex: Reflex::default(),
            spawn: layout.spawn,
            check: None,
            ear: crate::radio::Listener::default(),
            heard_to: None,
        }
    }

    /// The solo smoke: escape, restart, get caught, restart.
    pub fn full(layout: &Layout, _tuning: &Tuning) -> Self {
        let mut s = Vec::new();
        win_run(&mut s, layout);
        fail_run(&mut s, layout);
        Self::new(s, layout)
    }

    /// Ground-level circuit of every landmark while he still sleeps, then
    /// the full route from a fresh start.
    pub fn tour(layout: &Layout, _tuning: &Tuning) -> Self {
        use LandmarkId::*;
        use Step::*;
        let d = &layout.district;
        let mut s = vec![Wait(1.0)];
        let visit = |s: &mut Vec<Step>, id: LandmarkId| {
            let mark = d.landmark(id);
            s.extend([
                Log(mark.name),
                Go {
                    to: mark.approach,
                    guard: true,
                },
                Face(mark.look.to_array()),
                Capture(mark.shot),
            ]);
        };
        visit(&mut s, Entry);
        visit(&mut s, Extraction);
        visit(&mut s, Fields);
        // Up the tower ramp for the view, and back down the same way.
        let tower = d.landmark(Watchtower);
        let ramp_foot = Vec2::new(tower.approach.x, tower.approach.y - 5.0);
        let ramp_top = d.watch_deck.center();
        s.extend([
            Log(tower.name),
            Go {
                to: tower.approach,
                guard: true,
            },
            Face(tower.look.to_array()),
            Capture(tower.shot),
            Go {
                to: ramp_foot,
                guard: true,
            },
            Go {
                to: Vec2::new(ramp_top.x, ramp_top.y + 4.0),
                guard: true,
            },
            Face([0.0, 2.0, -20.0]),
            Capture("20_lookout_vantage"),
            Go {
                to: ramp_foot,
                guard: true,
            },
            Go {
                to: tower.approach,
                guard: true,
            },
        ]);
        for id in [Corral, Cano, Shrine, Marsh, WaterTower, Ranch] {
            visit(&mut s, id);
        }
        s.extend([Capture("00_overview"), Restart, ExpectReset]);
        win_run(&mut s, layout);
        fail_run(&mut s, layout);
        Self::new(s, layout)
    }

    /// The network smoke, host side. See `net::smoke` for the shared story.
    pub fn net_host(layout: &Layout, _tuning: &Tuning) -> Self {
        use Step::*;
        let exposed = layout.district.landmark(LandmarkId::Corral).approach;
        let mut s = vec![
            Log("waiting for the partner"),
            Await {
                cond: Cond::Party(2),
                timeout: 1500.0,
                guard: false,
                leash: Leash::Free,
            },
            Start,
            Await {
                cond: Cond::Started,
                timeout: 20.0,
                guard: false,
                leash: Leash::Free,
            },
            // Run 1: the host brings the table bundle to the altar, the
            // partner watches it land and marks the altar; the host marks
            // back and waits for the partner's acknowledgement. Then the
            // whole win route: every bundle, the pump, the truck, and both
            // standing in its zone (a shared victory both must see).
            Log("run 1: the host takes the table bundle to the altar"),
        ];
        win_to_altar(&mut s, layout, false);
        s.extend([
            Log("run 1: delivered; waiting for the partner's mark"),
            Await {
                cond: Cond::Marks {
                    mine: false,
                    other: true,
                },
                timeout: 120.0,
                guard: true,
                leash: Leash::Free,
            },
            Ping,
            Await {
                cond: Cond::Crouched,
                timeout: 120.0,
                guard: true,
                leash: Leash::Free,
            },
            Log("run 1: both marks acknowledged; on to the rest of the bones"),
        ]);
        win_rest(&mut s, layout);
        s.extend([
            Log("run 1: shared victory; letting the partner see it"),
            Wait(2.5),
            Restart,
            ExpectReset,
            // Run 2: the bundle wakes him; both stand in the open until he
            // has caught them both (a shared failure both must see).
            Log("run 2: the host wakes him and joins the partner in the open"),
        ]);
        to_table(&mut s, layout);
        s.push(Via(DOOR_OUT));
        s.extend([
            Go {
                to: exposed,
                guard: false,
            },
            Await {
                cond: Cond::Outcome(Outcome::Failed),
                timeout: 1500.0,
                guard: false,
                leash: Leash::Free,
            },
            Log("run 2: shared failure; letting the partner see it"),
            Wait(2.5),
            Restart,
            ExpectReset,
            // Run 3: the partner takes a bundle and disconnects; the host
            // sees the load fall, goes and recovers it, and lays it down.
            Log("run 3: waiting for the carrier to leave"),
            Await {
                cond: Cond::Party(1),
                timeout: 600.0,
                guard: true,
                leash: Leash::Free,
            },
            Await {
                cond: Cond::Ground(0),
                timeout: 5.0,
                guard: true,
                leash: Leash::Free,
            },
            Log("run 3: the carrier's bundle is on the ground; recovering it"),
        ]);
        to_table(&mut s, layout);
        s.extend([
            Log("run 3: recovered; carrying it to the altar"),
            Via(BACK_DOOR),
            Do(deliver(1)),
            Log("run 3: the recovered bundle is delivered"),
        ]);
        Self::new(s, layout)
    }

    /// The network smoke, partner side.
    pub fn net_client(layout: &Layout, _tuning: &Tuning) -> Self {
        use Step::*;
        let exposed = layout.district.landmark(LandmarkId::Corral).approach;
        let altar = ground(layout.ceiba.offering);
        let stand = altar + (altar - layout.ceiba.center).normalize() * 1.3;
        let mut s = vec![
            Await {
                cond: Cond::Started,
                timeout: 1500.0,
                guard: false,
                leash: Leash::Free,
            },
            // Run 1.
            Log("run 1: walking to the altar to watch the host's delivery"),
            Go { to: stand, guard: true },
            Face(layout.ceiba.offering.to_array()),
            Await {
                cond: Cond::Delivered(1),
                timeout: 400.0,
                guard: true,
                leash: Leash::Free,
            },
            Log("run 1: the host's delivery is in my snapshot; marking the altar"),
            Ping,
            Await {
                cond: Cond::Marks {
                    mine: true,
                    other: true,
                },
                timeout: 60.0,
                guard: true,
                leash: Leash::Free,
            },
            Log("run 1: my mark and the host's mark are both in one snapshot; acknowledging"),
            Crouch(Cond::Carried(1)),
            // The relay: the partner counts the numbers off the shelf radio
            // and opens the key box with them, so the host finds it open.
            Log("run 1: the host took the next bundle; the radio's numbers, then the key box"),
            Go {
                to: Vec2::from_array(BACK_DOOR[3]),
                guard: true,
            },
            Via(BACK_DOOR_IN),
            Do(Job::new(Want::Radio, Cond::Digits)),
            Via(BACK_DOOR),
            Do(Job::new(Want::Lockbox, Cond::Key)),
            Log("run 1: the key is out; back to the lit porch to wait for power"),
            Go {
                to: Vec2::from_array(DOOR_APPROACH[2]),
                guard: true,
            },
            Await {
                cond: Cond::Power,
                timeout: 1500.0,
                guard: true,
                leash: Leash::Free,
            },
            Log("run 1: the lamps are on; walking to the truck's boarding zone"),
            Go {
                to: layout.district.landmark(LandmarkId::Extraction).approach,
                guard: true,
            },
            Await {
                cond: Cond::Outcome(Outcome::Won),
                timeout: 1500.0,
                guard: true,
                leash: Leash::Truck,
            },
            Log("run 1: shared victory seen; waiting for the host to restart"),
            Await {
                cond: Cond::NextRun,
                timeout: 60.0,
                guard: false,
                leash: Leash::Free,
            },
            ExpectReset,
            // Run 2.
            Log("run 2: standing in the open until he catches us"),
            Go {
                to: exposed,
                guard: false,
            },
            Await {
                cond: Cond::Outcome(Outcome::Failed),
                timeout: 1500.0,
                guard: false,
                leash: Leash::Free,
            },
            Log("run 2: shared failure seen"),
            Await {
                cond: Cond::NextRun,
                timeout: 60.0,
                guard: false,
                leash: Leash::Free,
            },
            ExpectReset,
            // Run 3: take the bundle and leave carrying it.
            Log("run 3: taking the table bundle"),
        ];
        to_table(&mut s, layout);
        s.extend([Log("run 3: leaving while carrying"), Leave]);
        Self::new(s, layout)
    }

    /// Simulated seconds the route has been running.
    pub fn elapsed(&self) -> f32 {
        self.total
    }

    /// The run epoch the route currently expects.
    pub fn run(&self) -> u64 {
        self.run
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
        self.reset_step();
    }

    fn reset_step(&mut self) {
        self.t = 0.0;
        self.path.clear();
        self.waypoint = 0;
        self.planned = false;
        self.stale = false;
        self.attempt = 0;
        self.replans = 0;
        self.best = f32::MAX;
        self.stall = 0.0;
        self.stare = 0.0;
        self.press = 0.0;
        self.sent = 0.0;
        self.captured = false;
    }

    fn holds(&self, cond: Cond, snap: &Snapshot, me: PlayerId) -> bool {
        match cond {
            Cond::Holding(i) => snap.relics.get(i).is_some_and(|r| r.state == 1 && r.owner == me),
            Cond::Ground(i) => snap.relics.get(i).is_some_and(|r| r.state == 0),
            Cond::Carried(i) => snap.relics.get(i).is_some_and(|r| r.state == 1 && r.owner != me),
            Cond::Delivered(n) => snap.world.delivered as usize >= n,
            Cond::AjiTaken(i) => snap.aji.get(i).copied().unwrap_or(false),
            Cond::BatteriesTaken(i) => snap.batteries.get(i).copied().unwrap_or(false),
            Cond::Power => snap.world.power >= 1.0,
            Cond::Key => snap.world.key,
            Cond::Line(c) => snap.world.circuits & (1 << c) != 0,
            Cond::Truck => snap.world.truck >= 1.0,
            Cond::Warned => snap.stats[0] >= 1,
            Cond::Recovered => snap.stats[2] >= 1,
            Cond::Outcome(o) => snap.outcome() == o,
            Cond::Started => snap.started,
            Cond::NextRun => snap.run > self.run,
            Cond::Party(n) => snap.players.len() == n,
            Cond::Marks { mine, other } => {
                (!mine || snap.pings.iter().any(|p| p.by == me)) && (!other || snap.pings.iter().any(|p| p.by != me))
            }
            Cond::Crouched => snap.players.iter().any(|p| p.id != me && p.crouch && p.status == 0),
            Cond::Digits => self.ear.code().is_some(),
        }
    }

    /// A failure message with everything needed to see where and why.
    fn failure(&self, obs: &Observation, snap: &Snapshot, why: String) -> Result<(), String> {
        let me = snap.player(obs.me);
        Err(format!(
            "{why} [step {}; run {} (route expects {}), route {:.0}s, night {:.0}s, at ({:.1}, {:.1}), \
             delivered {}/{}, power {:.2}, truck {:.2}, warm {:.2}, danger {}, outcome {:?}, \
             warnings/hunts/recoveries/downs/revives {:?}, carrying {}, aji {}, status {}, \
             fear {:.2}, stun {:.1}, stamina {:.2}, exposure {:.2}, wards {}, threat {:?}/{:?}; {}]",
            self.step_name(),
            snap.run,
            self.run,
            self.total,
            snap.elapsed,
            obs.pose.pos.x,
            obs.pose.pos.y,
            snap.world.delivered,
            snap.world.total,
            snap.world.power,
            snap.world.truck,
            snap.world.warm,
            snap.danger,
            snap.outcome(),
            snap.stats,
            me.map_or(0, |p| p.carrying),
            snap.me.aji,
            me.map_or(9, |p| p.status),
            snap.me.fear,
            snap.me.stun,
            snap.me.stamina,
            snap.exposure,
            snap.zones.len(),
            obs.encounter.threat.state,
            obs.encounter.threat.movement,
            self.reflex.note(),
        ))
    }

    /// Watch the skill check in flight and press just inside the start of
    /// its zone (the great part). The needle runs on between snapshots.
    fn skill_press(&mut self, snap: &Snapshot, tuning: &Tuning, dt: f32) -> Option<(u32, f32)> {
        let Some(c) = snap.me.check else {
            self.check = None;
            return None;
        };
        let (id, base, since, pressed) = match self.check {
            Some((id, base, since, pressed)) if id == c.id && base == c.needle => (id, base, since + dt, pressed),
            Some((id, _, _, pressed)) if id == c.id => (id, c.needle, 0.0, pressed),
            _ => (c.id, c.needle, 0.0, false),
        };
        let at = base + since / tuning.check_sweep;
        let press = !pressed && at >= c.zone + tuning.check_great * 0.5;
        self.check = Some((id, base, since, pressed || press));
        press.then_some((id, at))
    }

    /// Walk the planned corners.
    fn follow(&mut self, obs: &Observation, intent: &mut Intent) -> Follow {
        let pos = obs.pose.pos;
        while let Some(&goal) = self.path.get(self.waypoint) {
            let dist = pos.distance(goal);
            // The body we see lags the authority by a frame or two: a corner
            // is turned a step early when the way on from here is clear.
            let early = dist < ARRIVE + LEAD
                && self
                    .path
                    .get(self.waypoint + 1)
                    .is_some_and(|&next| segment_free(obs.layout, pos, next, obs.tuning.player_radius + 0.05));
            if dist < ARRIVE || early {
                self.waypoint += 1;
                self.best = f32::MAX;
                self.stall = 0.0;
                continue;
            }
            *intent = toward(&obs.pose, goal, obs.dt);
            if dist < self.best - 0.05 {
                self.best = dist;
                self.stall = 0.0;
            } else {
                self.stall += obs.dt;
                if self.stall > 4.0 {
                    return Follow::Stuck;
                }
            }
            return Follow::Moving;
        }
        Follow::Done
    }

    /// Rejoin the fixed corners from wherever the reflex left us.
    fn rejoin(&mut self, obs: &Observation) -> bool {
        let Some(&next) = self.path.get(self.waypoint) else {
            return true;
        };
        if obs.pose.pos.distance(next) < 1.0 {
            return true;
        }
        if !self
            .nav
            .plan(obs.layout, obs.tuning, obs.pose.pos, next, &mut self.scratch)
        {
            return false;
        }
        self.scratch.extend_from_slice(&self.path[self.waypoint + 1..]);
        std::mem::swap(&mut self.path, &mut self.scratch);
        self.waypoint = 0;
        self.best = f32::MAX;
        self.stall = 0.0;
        true
    }

    /// Plan the walk to the `attempt`-th stand spot for `job`.
    fn plan_spot(&mut self, obs: &Observation, job: &Job, aim: Vec3) -> bool {
        let reach = job.want.reach(obs.tuning);
        for k in self.attempt..self.attempt + 6 {
            let wary = self.reflex.seen.filter(|s| s.1 < HINT).map(|s| s.0);
            let Some(spot) = stand_spot(obs.layout, obs.tuning, aim, reach, obs.pose.pos, wary, k) else {
                return false;
            };
            if self
                .nav
                .plan(obs.layout, obs.tuning, obs.pose.pos, spot, &mut self.path)
            {
                self.attempt = k;
                self.waypoint = 0;
                self.planned = true;
                self.best = f32::MAX;
                self.stall = 0.0;
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self, obs: &Observation) -> ScriptFrame {
        let mut out = ScriptFrame::default();
        let dt = obs.dt;
        self.total += dt;
        let Some(snap) = obs.snapshot else {
            return out;
        };
        let Some(step) = self.steps.get(self.index).copied() else {
            out.finished = Some(Ok(()));
            return out;
        };
        let (layout, tuning) = (obs.layout, obs.tuning);
        let fail = |script: &Self, why: String| Some(script.failure(obs, snap, why));

        // The run epoch only moves where the route expects it to.
        if snap.run < self.run || snap.run > self.run + 1 || (snap.run == self.run + 1 && !step.allows_next_run()) {
            out.finished = fail(
                self,
                format!("unexpected run epoch {} (route expects {})", snap.run, self.run),
            );
            return out;
        }
        let me = snap.player(obs.me);
        let down = me.is_none_or(|p| p.status != 0);
        let over = snap.outcome().is_over();
        if step.guarded() && !step.awaits_outcome() && (down || over) {
            let why = if down {
                "the player went down".to_string()
            } else {
                format!("the run ended {:?}", snap.outcome())
            };
            out.finished = fail(self, format!("{why} before the route meant it to"));
            return out;
        }
        self.reflex.watch(snap, dt);
        // Terminal snapshots freeze threat state; never let a stale warning
        // keep the survival reflex from observing the outcome.
        if step.guarded() && !over {
            let leash = match step {
                Step::Await {
                    leash: Leash::Truck, ..
                } => {
                    let zone = layout.district.truck;
                    Some((zone.zone_center, zone.zone_radius + TRUCK_STRAY))
                }
                _ => None,
            };
            match self.reflex.tick(obs, snap, leash, &mut out) {
                Turn::Busy => return out,
                Turn::Resumed => self.stale = true,
                Turn::Clear => {}
            }
        } else {
            self.reflex.stand_down();
        }
        // Seen lately, he is about: the torch stays off (its beam draws him).
        out.dark |= self.reflex.seen.is_some_and(|s| s.1 < HINT);
        if self.stale {
            self.stale = false;
            if matches!(step, Step::Via(_)) {
                if !self.rejoin(obs) {
                    out.finished = fail(self, "no way back to the route after hiding".into());
                    return out;
                }
            } else {
                self.planned = false;
            }
        }
        self.t += dt;
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
                let (yaw, pitch) = aim_angles(layout, tuning, &obs.pose, Vec3::new(x, y, z));
                out.intent.look_delta = steer(&obs.pose, yaw, pitch, dt);
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
            Step::Via(corners) => {
                if !self.planned {
                    self.path.clear();
                    self.path.extend(corners.iter().copied().map(Vec2::from_array));
                    self.waypoint = 0;
                    self.planned = true;
                }
                match self.follow(obs, &mut out.intent) {
                    Follow::Moving => {}
                    Follow::Done => self.next(),
                    Follow::Stuck => {
                        let goal = self.path.get(self.waypoint).copied();
                        out.finished = fail(self, format!("stuck walking to {goal:?}"));
                    }
                }
            }
            Step::Go { to, guard } => {
                if !guard && (down || over) {
                    self.next();
                    return out;
                }
                if !self.planned {
                    if !self.nav.plan(layout, tuning, obs.pose.pos, to, &mut self.path) {
                        out.finished = fail(self, format!("no collision-free way from {:?} to {to:?}", obs.pose.pos));
                        return out;
                    }
                    self.waypoint = 0;
                    self.planned = true;
                    self.best = f32::MAX;
                    self.stall = 0.0;
                }
                match self.follow(obs, &mut out.intent) {
                    Follow::Moving => {}
                    Follow::Done => self.next(),
                    Follow::Stuck => {
                        self.replans += 1;
                        if self.replans > 4 {
                            out.finished = fail(self, format!("stuck on the way to {to:?}"));
                        } else {
                            self.planned = false;
                        }
                    }
                }
            }
            Step::Do(job) => {
                if self.holds(job.until, snap, obs.me) {
                    self.next();
                    return out;
                }
                if matches!(job.want, Want::Aji(_)) && snap.me.aji >= tuning.aji_max {
                    out.log = Some("pepper pouch full: leaving that one".into());
                    self.next();
                    return out;
                }
                if matches!(job.want, Want::Batteries(_)) && snap.me.battery >= tuning.battery_full {
                    out.log = Some("torch still fresh: leaving the batteries".into());
                    self.next();
                    return out;
                }
                let aim = job.want.point(layout, snap);
                let usable = obs.target.is_some_and(|t| t.usable() && job.want.matches(t.kind));
                let mut gave_up: Option<String> = None;
                if usable && job.want == Want::Lockbox {
                    // The combination counted off the radio, tried now and
                    // then until the box opens.
                    self.stare = 0.0;
                    out.intent.look_delta = aim_at(layout, tuning, &obs.pose, aim, dt);
                    self.press -= dt;
                    if self.press <= 0.0 {
                        out.intent.code = self.ear.code();
                        self.press = 0.6;
                    }
                } else if usable && job.want == Want::Radio {
                    // Turn the dial to the frequency on the padlock's tag,
                    // one click at a time, then count the pips.
                    self.stare = 0.0;
                    out.intent.look_delta = aim_at(layout, tuning, &obs.pose, aim, dt);
                    let now = snap.elapsed as f64;
                    let stop = crate::radio::numbers_stop(tuning.seed) as u8 + 1;
                    if snap.world.radio == stop {
                        if let Some(from) = self.heard_to.filter(|&from| from < now && now - from < 1.0) {
                            let code = crate::sim::lock_code(tuning.seed);
                            let beats = crate::radio::beats_between(tuning.seed, code, from, now);
                            self.ear.hear(from, now, &beats);
                        }
                    } else {
                        self.press -= dt;
                        if self.press <= 0.0 {
                            out.intent.interact_pressed = true;
                            // Long enough for the dial to show the click.
                            self.press = 0.5;
                        }
                    }
                    self.heard_to = Some(now);
                } else if usable {
                    self.stare = 0.0;
                    out.intent.look_delta = aim_at(layout, tuning, &obs.pose, aim, dt);
                    out.intent.interact_held = true;
                    // Pick-ups are presses: repeat until the host agrees.
                    self.press -= dt;
                    if self.press <= 0.0 {
                        out.intent.interact_pressed = true;
                        self.press = 0.3;
                    }
                    out.intent.skill = self.skill_press(snap, tuning, dt);
                    if let Some((frac, name)) = job.shot
                        && !self.captured
                        && snap.me.hold >= frac
                    {
                        out.capture = Some(name);
                        self.captured = true;
                    }
                } else {
                    if !self.planned && !self.plan_spot(obs, &job, aim) {
                        gave_up = Some(format!("no reachable place to stand for {:?}", job.want));
                    }
                    if gave_up.is_none() {
                        if self.waypoint < self.path.len() {
                            if let Follow::Stuck = self.follow(obs, &mut out.intent) {
                                self.replans += 1;
                                self.attempt += 1;
                                self.planned = false;
                                if self.replans > 4 {
                                    gave_up = Some(format!("stuck on the way to {:?}", job.want));
                                }
                            }
                        } else {
                            out.intent.look_delta = aim_at(layout, tuning, &obs.pose, aim, dt);
                            self.stare += dt;
                            if self.stare > 1.5 {
                                // Not aimed or not in reach from here: another spot.
                                self.stare = 0.0;
                                self.attempt += 1;
                                self.planned = false;
                                self.replans += 1;
                                if self.replans > 6 {
                                    gave_up =
                                        Some(format!("{:?} never became usable (target {:?})", job.want, obs.target));
                                }
                            }
                        }
                    }
                }
                if gave_up.is_none() && self.t > job.timeout {
                    gave_up = Some(format!(
                        "{:?} never completed within {}s (target {:?})",
                        job.want, job.timeout, obs.target
                    ));
                }
                if let Some(why) = gave_up {
                    if job.soft {
                        out.log = Some(format!("skipping optional step: {why}"));
                        self.next();
                    } else {
                        out.finished = fail(self, why);
                    }
                }
            }
            Step::Await {
                cond, timeout, leash, ..
            } => {
                if let Cond::Outcome(wanted) = cond
                    && over
                    && snap.outcome() != wanted
                {
                    out.finished = fail(self, format!("the run ended {:?}, not {wanted:?}", snap.outcome()));
                    return out;
                }
                // Waiting for both marks at once: mark the altar again if
                // mine faded while the other was busy hiding.
                if let Cond::Marks { mine: true, .. } = cond
                    && !snap.pings.iter().any(|p| p.by == obs.me)
                {
                    self.sent -= dt;
                    if self.sent <= 0.0 {
                        out.ping = Some(layout.ceiba.offering.to_array());
                        self.sent = 2.5;
                    }
                }
                // Back into the truck's boarding zone after hiding outside it.
                let zone = layout.district.truck;
                if leash == Leash::Truck && !over && obs.pose.pos.distance(zone.zone_center) > zone.zone_radius - 1.5 {
                    if !self.planned {
                        if !self
                            .nav
                            .plan(layout, tuning, obs.pose.pos, zone.zone_center, &mut self.path)
                        {
                            out.finished = fail(self, "no way back into the truck's boarding zone".into());
                            return out;
                        }
                        self.waypoint = 0;
                        self.planned = true;
                        self.best = f32::MAX;
                        self.stall = 0.0;
                    }
                    if let Follow::Stuck = self.follow(obs, &mut out.intent) {
                        self.planned = false;
                    }
                } else {
                    self.planned = false;
                }
                if out.intent.move_axis == Vec2::ZERO
                    && let Some(t) = snap.threat
                {
                    out.intent.look_delta = aim_at(
                        layout,
                        tuning,
                        &obs.pose,
                        Vec3::new(t.position[0], 2.2, t.position[1]),
                        dt,
                    );
                }
                if self.holds(cond, snap, obs.me) {
                    self.next();
                } else if self.t > timeout {
                    out.finished = fail(self, format!("waited {:.0}s for {cond:?}", self.t));
                }
            }
            Step::Ping => {
                if snap.pings.iter().any(|p| p.by == obs.me) {
                    self.next();
                } else {
                    let altar = layout.ceiba.offering;
                    out.intent.look_delta = aim_at(layout, tuning, &obs.pose, altar, dt);
                    self.sent -= dt;
                    if self.sent <= 0.0 {
                        // A mark refused (cooldown, a stale epoch) is simply sent again.
                        out.ping = Some(altar.to_array());
                        self.sent = 2.5;
                    }
                    if self.t > 30.0 {
                        out.finished = fail(self, "my mark never appeared in the snapshot".into());
                    }
                }
            }
            Step::Crouch(cond) => {
                out.intent.crouch = true;
                if self.holds(cond, snap, obs.me) {
                    self.next();
                } else if self.t > 300.0 {
                    out.finished = fail(self, format!("waited {:.0}s for {cond:?} while crouching", self.t));
                }
            }
            Step::Start => {
                if snap.started {
                    self.next();
                } else {
                    if !self.captured {
                        out.start = true;
                        self.captured = true;
                    }
                    if self.t > 20.0 {
                        out.finished = fail(self, "the run never started".into());
                    }
                }
            }
            Step::Restart => {
                out.restart = true;
                self.next();
            }
            Step::ExpectReset => {
                if snap.run == self.run + 1 {
                    let fresh = !over
                        && snap.elapsed < 3.0
                        && snap.world.delivered == 0
                        && snap.relics.iter().all(|r| r.state == 0)
                        && snap.players.iter().all(|p| p.status == 0)
                        && snap.stats == [0; 5]
                        && obs.pose.pos.distance(self.spawn) < 3.6;
                    if fresh {
                        self.run += 1;
                        self.next();
                    } else if self.t > 5.0 {
                        out.finished = fail(self, "restart did not reset the run".into());
                    }
                } else if self.t > 5.0 {
                    out.finished = fail(self, "the restart never arrived".into());
                }
            }
            Step::Leave => {
                out.leave = true;
                self.next();
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Somewhere along the walk from `from` through `path` is waist-deep.
    fn wades(layout: &Layout, from: Vec2, path: &[Vec2]) -> bool {
        let mut at = from;
        path.iter().any(|&p| {
            let deep = segment_deep(layout, at, p);
            at = p;
            deep
        })
    }

    /// From the lookout's ramp foot the base shed lies straight across the
    /// caño, and the search box around the two holds neither the bridge nor
    /// the bank road. The walk still keeps out of waist-deep water, there and
    /// back: over the bridge and along the bank road, as before the caño could
    /// be waded. Where no dry way exists at all (a bundle afloat
    /// mid-channel), it wades.
    #[test]
    fn the_walk_keeps_out_of_the_cano_while_a_dry_way_exists() {
        let (l, t) = (Layout::new(), Tuning::default());
        let foot = l.district.landmark(LandmarkId::Watchtower).approach;
        // Open ground nearest the bundle's hiding place by the base shed.
        let bundle = Vec2::new(52.5, -87.5);
        let shed = (-12..=12)
            .flat_map(|x| (-12..=12).map(move |y| bundle + Vec2::new(x as f32, y as f32) * 0.5))
            .filter(|&p| l.is_free(p, t.player_radius + 0.25))
            .min_by(|a, b| a.distance(bundle).total_cmp(&b.distance(bundle)))
            .expect("open ground by the base shed");
        let mut nav = Nav::default();
        let mut path = Vec::new();
        for (from, to) in [(foot, shed), (shed, foot)] {
            assert!(segment_deep(&l, from, to), "the straight way from {from} wades");
            assert!(nav.plan(&l, &t, from, to, &mut path), "no way from {from} to {to}");
            assert_eq!(path.last(), Some(&to));
            assert!(!wades(&l, from, &path), "waded from {from} to {to}: {path:?}");
        }
        let afloat = Vec2::new(34.0, -65.5);
        assert_eq!(l.wade(afloat), Wade::Deep);
        assert!(nav.plan(&l, &t, foot, afloat, &mut path), "no way to {afloat}");
        assert_eq!(path.last(), Some(&afloat));
    }
}
