//! `--trailer` (DEBUG): renders the teaser's shots, frame by frame, into
//! `<shots>/trailer/<NN_shot>/00000.png…` at 1920×1080 and 30 fps, from an
//! off-screen target (so the window's size never matters). Every shot is a
//! camera path with staged presentation: him walking where the shot wants
//! him, lightning when it wants it, friends' torches, an omen, the catch.
//! Nothing here touches the rules; the staging is a presentation mirror,
//! rebuilt every frame, like `--photos`. `TRAILER_STILLS=1` renders one
//! frame per shot instead (for framing); `TRAILER_ONLY=name` renders only
//! the shots whose names contain it. The edit (titles, sound) is made from
//! these frames by `tools/trailer/`.

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::app::{EncounterMsg, Launch, LayoutRes, Settings, StormClock, Truth, TuningRes};
use crate::geometry::Layout;
use crate::geometry::district::LandmarkId;
use crate::net::Network;
use crate::player::{Flashlight, LightOn, Player, TorchHand};
use crate::sim::{Event, Presence, ThreatState};
use crate::world::models::ModelsPending;
use crate::world::omen::{Catch, Fright, LUNGE_GAP};

pub const FPS: f32 = 30.0;
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
/// Frames written but not yet on disk before the clock waits for them.
const IN_FLIGHT: usize = 4;

/// A camera keyframe: at `t` seconds into the shot the eye is at `pos`
/// looking at `look`. Between keys the camera eases in and out.
#[derive(Clone, Copy)]
pub struct Key {
    pub t: f32,
    pub pos: Vec3,
    pub look: Vec3,
}

/// Where he is at `t` seconds into the shot (ground position).
#[derive(Clone, Copy)]
pub struct HimKey {
    pub t: f32,
    pub at: Vec2,
}

#[derive(Clone)]
pub struct Him {
    pub keys: Vec<HimKey>,
    /// Stalking walks, hunting lifts the arms, counting stoops.
    pub state: ThreatState,
    /// When he has no motion of his own, which way he faces.
    pub face: Option<Vec2>,
}

/// A friend walking from `from` to `to` over the shot, torch lit.
#[derive(Clone, Copy)]
pub struct Mate {
    pub from: Vec2,
    pub to: Vec2,
    pub crouch: bool,
    pub sprint: bool,
    /// Down, crawling.
    pub down: bool,
    pub carrying: bool,
    /// An even pace from start to end (otherwise eased in and out).
    pub steady: bool,
}

impl Mate {
    fn walk(from: Vec2, to: Vec2) -> Self {
        Self {
            from,
            to,
            crouch: false,
            sprint: false,
            down: false,
            carrying: false,
            steady: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Storm {
    Calm,
    /// A strike flashes this many seconds into the shot.
    FlashAt(f32),
}

#[derive(Clone)]
pub struct Shot {
    pub name: &'static str,
    pub dur: f32,
    pub cam: Vec<Key>,
    pub fov: f32,
    /// First-person: the torch in hand, head bob.
    pub pov: bool,
    pub torch: bool,
    pub carrying: bool,
    pub powered: bool,
    pub him: Option<Him>,
    pub mates: Vec<Mate>,
    pub storm: Storm,
    /// An omen raised as the shot starts (placed near the eye).
    pub omen: Option<Event>,
    /// The catch: its clock starts (at the silence) this many seconds in.
    pub catch_at: Option<f32>,
    /// A soft moon-coloured light placed for the shot: where, how strong.
    pub key_light: Option<(Vec3, f32)>,
    /// Handheld drift of the camera (0 still .. 1 shaky).
    pub handheld: f32,
    /// Where the still is taken (fraction of the shot) in stills mode.
    pub still: f32,
    /// His hat lying on the ground here.
    pub hat: Option<Vec2>,
    /// A cold light that follows him (between him and the camera, above):
    /// how strong. Overrides `key_light`.
    pub him_light: f32,
}

impl Shot {
    fn new(name: &'static str, dur: f32, cam: Vec<Key>) -> Self {
        Self {
            name,
            dur,
            cam,
            fov: 48.0,
            pov: false,
            torch: false,
            carrying: false,
            powered: false,
            him: None,
            mates: Vec::new(),
            storm: Storm::Calm,
            omen: None,
            catch_at: None,
            key_light: None,
            handheld: 0.15,
            still: 0.5,
            hat: None,
            him_light: 0.0,
        }
    }
}

fn smooth(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// The camera at `t` seconds into a shot.
pub fn camera_at(shot: &Shot, t: f32) -> (Vec3, Vec3) {
    let keys = &shot.cam;
    if keys.len() == 1 || t <= keys[0].t {
        return (keys[0].pos, keys[0].look);
    }
    for w in keys.windows(2) {
        if t <= w[1].t {
            let k = smooth((t - w[0].t) / (w[1].t - w[0].t).max(1e-3));
            return (w[0].pos.lerp(w[1].pos, k), w[0].look.lerp(w[1].look, k));
        }
    }
    let last = keys[keys.len() - 1];
    (last.pos, last.look)
}

/// Where he stands at `t`, and how fast he is going.
fn him_at(him: &Him, t: f32) -> (Vec2, Vec2, f32) {
    let keys = &him.keys;
    let at = |t: f32| {
        if keys.len() == 1 || t <= keys[0].t {
            return keys[0].at;
        }
        for w in keys.windows(2) {
            if t <= w[1].t {
                let k = (t - w[0].t) / (w[1].t - w[0].t).max(1e-3);
                return w[0].at.lerp(w[1].at, k);
            }
        }
        keys[keys.len() - 1].at
    };
    let p = at(t);
    let ahead = at(t + 0.1);
    let v = (ahead - p) / 0.1;
    let face = him.face.unwrap_or_else(|| v.normalize_or(Vec2::NEG_Y));
    (p, face, v.length())
}

/// The shot list. Every place comes from the layout.
pub fn shots(layout: &Layout) -> Vec<Shot> {
    let d = &layout.district;
    let g = |p: Vec2| layout.surface_height(p);
    let at = |x: f32, z: f32, up: f32| Vec3::new(x, g(Vec2::new(x, z)) + up, z);
    let key = |t: f32, pos: Vec3, look: Vec3| Key { t, pos, look };
    let gate = d.landmark(LandmarkId::Entry).center;
    let ranch = d.landmark(LandmarkId::Ranch).center;
    let fields = d.landmark(LandmarkId::Fields).center;
    let ceiba = layout.ceiba.center;
    let offering = layout.ceiba.offering;
    let truck = d.truck.center;
    let post = d.dog_post;
    let pump = d.pump;
    let mut out = Vec::new();

    // 1. Over the llano toward the hacienda gate, in the rain.
    let mut s = Shot::new(
        "01_gate",
        5.0,
        vec![
            key(0.0, at(gate.x - 3.0, gate.y + 16.0, 1.9), at(gate.x, gate.y, 2.2)),
            key(5.0, at(gate.x - 1.0, gate.y + 9.0, 1.7), at(gate.x, gate.y - 4.0, 2.0)),
        ],
    );
    s.fov = 42.0;
    out.push(s);

    // 2. The ceiba, its candles and offerings: a slow low orbit.
    let c3 = Vec3::new(ceiba.x, g(ceiba) + 2.5, ceiba.y);
    let orbit = |a: f32, r: f32, h: f32| Vec3::new(ceiba.x + a.cos() * r, g(ceiba) + h, ceiba.y + a.sin() * r);
    let face = (Vec2::new(offering.x, offering.z) - ceiba).to_angle();
    let mut s = Shot::new(
        "02_ceiba",
        4.5,
        vec![
            key(0.0, orbit(face - 0.35, 9.0, 1.2), c3),
            key(4.5, orbit(face + 0.25, 7.5, 1.0), c3 - Vec3::Y * 0.6),
        ],
    );
    s.key_light = Some((Vec3::new(offering.x, offering.y + 1.5, offering.z), 18_000.0));
    out.push(s);

    // 3. The corral in the rain, the herd uneasy, low behind the rails.
    let herd: Vec<Vec2> = d.cows().map(|c| c.center).collect();
    let hc = herd.iter().copied().sum::<Vec2>() / herd.len().max(1) as f32;
    let mut s = Shot::new(
        "03_corral",
        4.0,
        vec![
            key(0.0, at(hc.x - 15.0, hc.y + 13.0, 2.4), at(hc.x, hc.y, 0.9)),
            key(4.0, at(hc.x - 13.0, hc.y + 11.5, 2.1), at(hc.x + 1.0, hc.y - 1.0, 0.9)),
        ],
    );
    s.key_light = Some((at(hc.x - 3.0, hc.y + 3.0, 4.0), 25_000.0));
    out.push(s);

    // 4. Low through the tall grass of the fields; a strike shows him at the
    //    far edge, standing still.
    // In open grass, clear of the fields' shelter.
    let far = Vec2::new(fields.x - 4.0, fields.y - 8.0);
    let mut s = Shot::new(
        "04_fields_flash",
        4.5,
        vec![
            key(0.0, at(fields.x - 14.0, fields.y + 8.0, 0.8), at(far.x, far.y, 2.0)),
            key(4.5, at(fields.x - 11.0, fields.y + 6.0, 0.8), at(far.x, far.y, 2.2)),
        ],
    );
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: far }],
        state: ThreatState::Stalking,
        face: Some((Vec2::new(fields.x - 14.0, fields.y + 8.0) - far).normalize()),
    });
    s.storm = Storm::FlashAt(2.6);
    s.still = 2.66 / 4.5;
    s.him_light = 5_000.0;
    out.push(s);

    // 5. Tureco by his post at the back of the house, the lit window.
    let p3 = Vec3::new(post.x, g(post), post.y);
    let mut s = Shot::new(
        "05_tureco",
        4.0,
        vec![
            key(0.0, p3 + Vec3::new(-3.6, 1.0, -3.2), p3 + Vec3::new(0.3, 0.6, 0.4)),
            key(4.0, p3 + Vec3::new(-2.8, 0.8, -2.5), p3 + Vec3::new(0.2, 0.5, 0.3)),
        ],
    );
    s.omen = Some(Event::OmenLampsDie);
    s.key_light = Some((p3 + Vec3::new(-1.5, 2.5, -1.5), 9_000.0));
    out.push(s);

    // 6. First person: carrying a bundle toward the ceiba, torch on.
    let from = Vec2::new(ceiba.x + 14.0, ceiba.y + 16.0);
    let to = Vec2::new(ceiba.x + 6.0, ceiba.y + 7.0);
    let mut s = Shot::new(
        "06_carry",
        4.0,
        vec![
            key(0.0, at(from.x, from.y, 1.62), c3 - Vec3::Y * 1.0),
            key(4.0, at(to.x, to.y, 1.62), c3 - Vec3::Y * 1.2),
        ],
    );
    s.pov = true;
    s.torch = true;
    s.carrying = true;
    s.fov = 68.0;
    out.push(s);

    // 7. The windmill's pump: the lamps come back across the yard.
    let mut s = Shot::new(
        "07_power",
        3.5,
        vec![
            key(0.0, pump + Vec3::new(4.5, 0.3, 5.5), pump + Vec3::new(0.0, 1.5, 0.0)),
            key(3.5, pump + Vec3::new(5.5, 1.0, 7.0), pump + Vec3::new(0.0, 3.0, 0.0)),
        ],
    );
    s.powered = true;
    s.key_light = Some((pump + Vec3::new(2.0, 3.0, 2.0), 12_000.0));
    out.push(s);

    // 8. The truck: lamps burning, friends' torches around it.
    let t3 = Vec3::new(truck.x, 0.0, truck.y);
    let mut s = Shot::new(
        "08_truck",
        4.0,
        vec![
            key(0.0, t3 + Vec3::new(9.5, 1.6, -6.5), t3 + Vec3::new(0.0, 1.2, 0.0)),
            key(4.0, t3 + Vec3::new(7.5, 1.4, -5.0), t3 + Vec3::new(-0.5, 1.2, 0.2)),
        ],
    );
    s.powered = true;
    s.mates = vec![
        Mate::walk(truck + Vec2::new(3.5, -3.0), truck + Vec2::new(2.5, -2.0)),
        Mate::walk(truck + Vec2::new(-1.0, -4.5), truck + Vec2::new(0.0, -3.0)),
    ];
    out.push(s);

    // 9. Co-op: following two friends through the tall grass, and between
    //    them, for a flash, a third figure that is not a friend.
    let a = Vec2::new(fields.x - 8.0, fields.y + 10.0);
    let mut s = Shot::new(
        "09_friends",
        5.0,
        vec![
            key(0.0, at(a.x - 2.0, a.y + 5.0, 1.62), at(a.x + 6.0, a.y - 6.0, 1.4)),
            key(5.0, at(a.x + 1.0, a.y + 1.5, 1.62), at(a.x + 8.0, a.y - 8.0, 1.4)),
        ],
    );
    s.pov = true;
    s.torch = true;
    s.fov = 68.0;
    s.mates = vec![
        Mate::walk(a + Vec2::new(1.5, 0.0), a + Vec2::new(4.5, -4.0)),
        Mate::walk(a + Vec2::new(4.0, 2.0), a + Vec2::new(7.0, -2.0)),
    ];
    s.him = Some(Him {
        keys: vec![HimKey {
            t: 0.0,
            at: a + Vec2::new(8.0, -9.0),
        }],
        state: ThreatState::Stalking,
        face: Some(Vec2::new(-0.6, 0.8)),
    });
    s.storm = Storm::FlashAt(3.6);
    out.push(s);

    // 10. He crosses the road beyond the gate, backlit by its lamp.
    let mut s = Shot::new(
        "10_crossing",
        4.5,
        vec![
            key(0.0, at(gate.x + 4.0, gate.y - 12.0, 1.4), at(gate.x, gate.y + 4.0, 2.2)),
            key(
                4.5,
                at(gate.x + 3.4, gate.y - 11.0, 1.4),
                at(gate.x - 1.0, gate.y + 4.0, 2.2),
            ),
        ],
    );
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: Vec2::new(gate.x + 7.0, gate.y + 5.0),
            },
            HimKey {
                t: 4.5,
                at: Vec2::new(gate.x - 6.0, gate.y + 5.5),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
    });
    s.key_light = Some((at(gate.x, gate.y + 8.0, 3.0), 14_000.0));
    out.push(s);

    // 11. His hat on the trail in the torchlight.
    let trail = Vec2::new(ranch.x + 6.0, ranch.y + 20.0);
    let mut s = Shot::new(
        "11_hat",
        3.0,
        vec![
            key(0.0, at(trail.x, trail.y, 1.55), at(trail.x - 1.2, trail.y - 7.5, 0.1)),
            key(
                3.0,
                at(trail.x - 0.3, trail.y - 1.8, 1.45),
                at(trail.x - 1.2, trail.y - 7.5, 0.0),
            ),
        ],
    );
    s.pov = true;
    s.torch = true;
    s.fov = 60.0;
    s.hat = Some(Vec2::new(trail.x - 1.2, trail.y - 7.5));
    out.push(s);

    // 12. Hiding in the tall grass: he passes right in front, slowly.
    let hide = Vec2::new(fields.x + 2.0, fields.y + 2.0);
    let mut s = Shot::new(
        "12_hide",
        4.5,
        vec![
            key(0.0, at(hide.x, hide.y, 0.55), at(hide.x + 1.0, hide.y - 3.0, 2.0)),
            key(4.5, at(hide.x, hide.y, 0.5), at(hide.x - 1.5, hide.y - 3.0, 2.3)),
        ],
    );
    s.fov = 62.0;
    s.handheld = 0.45;
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: hide + Vec2::new(3.6, -1.9),
            },
            HimKey {
                t: 4.5,
                at: hide + Vec2::new(-2.8, -1.7),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
    });
    s.him_light = 12_000.0;
    out.push(s);

    // 13. Running through the grass, looking back: he is coming.
    let run0 = Vec2::new(fields.x - 2.0, fields.y + 3.0);
    let run1 = run0 + Vec2::new(-8.0, -6.0);
    let mut s = Shot::new(
        "13_chase",
        3.5,
        vec![
            key(0.0, at(run0.x, run0.y, 1.55), at(run0.x + 8.0, run0.y + 7.0, 1.9)),
            key(3.5, at(run1.x, run1.y, 1.5), at(run1.x + 6.0, run1.y + 5.5, 1.9)),
        ],
    );
    s.pov = true;
    s.torch = true;
    s.fov = 72.0;
    s.handheld = 1.0;
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: run0 + Vec2::new(8.0, 7.0),
            },
            HimKey {
                t: 3.5,
                at: run1 + Vec2::new(6.0, 5.5),
            },
        ],
        state: ThreatState::Hunting,
        face: None,
    });
    s.him_light = 22_000.0;
    out.push(s);

    // 14. A strike: he is much closer than you thought.
    let near = Vec2::new(fields.x - 16.0, fields.y + 12.0);
    let mut s = Shot::new(
        "14_reveal",
        2.2,
        vec![key(0.0, at(near.x, near.y, 1.6), at(near.x + 5.0, near.y - 4.0, 2.2))],
    );
    s.fov = 60.0;
    s.handheld = 0.6;
    s.pov = true;
    s.him = Some(Him {
        keys: vec![HimKey {
            t: 0.0,
            at: near + Vec2::new(3.0, -2.4),
        }],
        state: ThreatState::Warning,
        face: Some(Vec2::new(-0.78, 0.62)),
    });
    s.storm = Storm::FlashAt(0.35);
    s.still = 0.4 / 2.2;
    out.push(s);

    // 15. The catch.
    let eye = Vec2::new(gate.x - 2.0, gate.y - 6.0);
    let mut s = Shot::new(
        "15_catch",
        3.1,
        vec![key(0.0, at(eye.x, eye.y, 1.62), at(eye.x + 0.5, eye.y - 10.0, 1.6))],
    );
    s.pov = true;
    s.torch = true;
    s.fov = 68.0;
    s.catch_at = Some(0.0);
    s.still = 0.6;
    out.push(s);

    // 16. The party: all four survivors walk out of the dark toward us under
    // the gate's lamps, torches on, as the camera backs away before them.
    let ahead = Vec2::new(-layout.spawn_yaw.sin(), -layout.spawn_yaw.cos());
    let right = Vec2::new(-ahead.y, ahead.x);
    let from_spawn = |a: f32, r: f32, up: f32| {
        let p = layout.spawn + ahead * a + right * r;
        at(p.x, p.y, up)
    };
    let mut s = Shot::new(
        "16_party",
        3.5,
        vec![
            key(0.0, from_spawn(1.6, 0.3, 1.15), from_spawn(8.0, 0.0, 1.3)),
            key(3.5, from_spawn(0.2, -0.2, 1.1), from_spawn(8.0, 0.0, 1.35)),
        ],
    );
    s.fov = 45.0;
    s.handheld = 0.3;
    s.key_light = Some((from_spawn(4.0, 1.5, 3.2), 26_000.0));
    s.still = 0.8;
    s.mates = [(-1.2, 8.5), (-0.4, 9.4), (0.45, 8.9), (1.25, 9.8)]
        .iter()
        .map(|&(r, a)| Mate {
            steady: true,
            ..Mate::walk(
                layout.spawn + ahead * a + right * r,
                layout.spawn + ahead * (a - 4.5) + right * r * 0.9,
            )
        })
        .collect();
    out.push(s);

    // Not in the cut, for review (TRAILER_ONLY=90): the four survivors going
    // past on the road outside the gate, side on — one sprinting, one
    // creeping crouched, one carrying bones, one walking — then one down,
    // crawling, as another crouches to them.
    // In front of the spawn, where the gate lamps light the road.
    let lane = layout.spawn + ahead * 6.0;
    let cam = |side: f32, up: f32| {
        let p = layout.spawn + right * side;
        at(p.x, p.y, up)
    };
    let walkers = [
        (-9.0, 9.0, 1.6, true, false, false),
        (-3.2, 3.2, -1.2, false, true, false),
        (-6.0, 6.0, 0.8, false, false, true),
        (-6.0, 6.0, -0.2, false, false, false),
    ];
    let mut s = Shot::new(
        "90_survivors_walk",
        4.0,
        vec![
            key(0.0, cam(-0.8, 1.3), at(lane.x, lane.y, 1.0)),
            key(4.0, cam(0.8, 1.3), at(lane.x, lane.y, 1.0)),
        ],
    );
    s.fov = 60.0;
    s.handheld = 0.0;
    s.key_light = Some((
        at(layout.spawn.x + ahead.x * 3.0, layout.spawn.y + ahead.y * 3.0, 4.0),
        40_000.0,
    ));
    s.mates = walkers
        .iter()
        .map(|&(a, b, dz, sprint, crouch, carrying)| Mate {
            sprint,
            crouch,
            carrying,
            steady: true,
            ..Mate::walk(lane + right * a + ahead * dz, lane + right * b + ahead * dz)
        })
        .collect();
    out.push(s);
    let mut s = Shot::new(
        "91_survivors_down",
        3.0,
        vec![
            key(
                0.0,
                at(
                    lane.x - ahead.x * 3.5 + right.x * 1.5,
                    lane.y - ahead.y * 3.5 + right.y * 1.5,
                    1.3,
                ),
                at(lane.x, lane.y, 0.4),
            ),
            key(
                3.0,
                at(
                    lane.x - ahead.x * 3.5 + right.x * 0.5,
                    lane.y - ahead.y * 3.5 + right.y * 0.5,
                    1.2,
                ),
                at(lane.x, lane.y, 0.4),
            ),
        ],
    );
    s.fov = 50.0;
    s.handheld = 0.0;
    s.key_light = Some((at(lane.x - ahead.x * 2.0, lane.y - ahead.y * 2.0, 3.5), 30_000.0));
    s.mates = vec![
        Mate {
            down: true,
            steady: true,
            ..Mate::walk(lane - right * 1.2, lane + right * 1.2)
        },
        Mate {
            crouch: true,
            ..Mate::walk(lane + right * 1.6 + ahead * 0.6, lane + right * 1.7 + ahead * 0.5)
        },
    ];
    out.push(s);
    out
}

/// A strike's run time to hang a flash on, far from the start of the night.
fn strike_time(seed: u64) -> f32 {
    (4..400)
        .filter_map(|slot| crate::storm::strike(seed, slot))
        .find(|s| s.power > 0.8 && !s.double)
        .map_or(200.0, |s| s.at)
}

#[derive(Resource)]
pub struct TrailerRun {
    shots: Vec<Shot>,
    index: usize,
    frame: u32,
    warm: u32,
    idle: u32,
    target: Handle<Image>,
    stills: bool,
    written: Vec<PathBuf>,
    omen_sent: bool,
    exit_in: Option<u32>,
}

/// The shots render into this image, never the window.
pub(crate) fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    layout: Res<LayoutRes>,
    camera: Single<Entity, With<Player>>,
) {
    let image = Image::new_target_texture(
        WIDTH,
        HEIGHT,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    let target = images.add(image);
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(target.clone().into()));
    commands.spawn((
        Name::new("trailer key light"),
        TrailerKey,
        PointLight {
            color: Color::srgb(0.62, 0.72, 0.95),
            intensity: 0.0,
            range: 30.0,
            radius: 0.5,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
    ));
    let mut shots = shots(&layout.0);
    if let Ok(only) = std::env::var("TRAILER_ONLY") {
        shots.retain(|s| s.name.contains(&only));
    }
    info!(
        "trailer: {} shots, {:.1} s",
        shots.len(),
        shots.iter().map(|s| s.dur).sum::<f32>()
    );
    commands.insert_resource(TrailerRun {
        shots,
        index: 0,
        frame: 0,
        warm: 0,
        idle: 0,
        target,
        stills: std::env::var("TRAILER_STILLS").is_ok(),
        written: Vec::new(),
        omen_sent: false,
        exit_in: None,
    });
}

#[derive(Component)]
pub struct TrailerKey;

#[allow(clippy::too_many_arguments)]
pub(crate) fn drive(
    mut commands: Commands,
    mut run: ResMut<TrailerRun>,
    launch: Res<Launch>,
    probe: Res<crate::debug::PipelineProbe>,
    models: Res<ModelsPending>,
    screenshots: Query<(), With<Screenshot>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    (layout, tuning): (Res<LayoutRes>, Res<TuningRes>),
    mut camera: Single<(&mut Transform, &mut Projection), With<Player>>,
    (mut clock, mut net, mut truth, mut fright): (ResMut<StormClock>, ResMut<Network>, ResMut<Truth>, ResMut<Fright>),
    (mut settings, mut hand, mut light_on): (ResMut<Settings>, ResMut<TorchHand>, ResMut<LightOn>),
    mut torch: Single<&mut Flashlight>,
    mut key_light: Single<(&mut PointLight, &mut Transform), (With<TrailerKey>, Without<Player>)>,
    mut hat: Single<
        (&mut crate::world::omen::OmenHat, &mut Transform, &mut Visibility),
        (Without<TrailerKey>, Without<Player>, Without<crate::ui::HudRoot>),
    >,
    mut omens: MessageWriter<EncounterMsg>,
    (mut exit, mut hud): (
        MessageWriter<AppExit>,
        Query<&mut Visibility, (With<crate::ui::HudRoot>, Without<crate::world::omen::OmenHat>)>,
    ),
) {
    for mut v in &mut hud {
        if *v != Visibility::Hidden {
            *v = Visibility::Hidden;
        }
    }
    let run = &mut *run;
    if let Some(n) = run.exit_in {
        if n == 0 && screenshots.is_empty() {
            let missing = run.written.iter().filter(|p| !p.exists()).count();
            if missing == 0 {
                info!("TRAILER DONE: {} frames", run.written.len());
                exit.write(AppExit::Success);
            } else {
                error!("TRAILER FAIL: {missing} frames not written");
                exit.write(AppExit::error());
            }
            run.exit_in = Some(u32::MAX);
        } else if n != u32::MAX {
            run.exit_in = Some(n.saturating_sub(1));
        }
        return;
    }
    let Some(shot) = run.shots.get(run.index).cloned() else {
        run.exit_in = Some(30);
        return;
    };
    let total = (shot.dur * FPS).round() as u32;
    let capturing = run.warm >= 12 && run.idle >= 8;
    let t = if run.stills {
        shot.dur * shot.still
    } else if capturing {
        run.frame as f32 / FPS
    } else {
        0.0
    };

    // Stage the frame.
    let (pos, look) = camera_at(&shot, t);
    let sway = |k: f32| (t * k).sin() * (t * k * 0.41 + 0.7).sin();
    let drift = Vec3::new(sway(1.3), sway(1.7) * 0.6, sway(1.1)) * 0.05 * shot.handheld;
    let (tf, projection) = &mut *camera;
    **tf = Transform::from_translation(pos + drift).looking_at(look + drift * 2.0, Vec3::Y);
    if let Projection::Perspective(p) = &mut **projection {
        p.fov = shot.fov.to_radians();
    }
    settings.head_bob = shot.pov;
    // A touch brighter than play, for video.
    settings.brightness = 0.55;
    hand.0 = shot.pov;
    torch.on = shot.torch;
    light_on.0 = shot.torch;
    let seed = tuning.0.seed;
    let storm_t = match shot.storm {
        Storm::Calm => crate::debug::calm_time(seed) + t,
        Storm::FlashAt(at) => strike_time(seed) - at + t,
    };
    clock.t = storm_t;
    clock.prev = storm_t - 1.0 / FPS;
    let (kl, ktf) = &mut *key_light;
    let follow = shot.him.as_ref().filter(|_| shot.him_light > 0.0).map(|him| {
        let (at, _, _) = him_at(him, t);
        let toward = (Vec2::new(pos.x, pos.z) - at).normalize_or(Vec2::Y);
        let spot = at + toward * 1.8;
        (
            Vec3::new(spot.x, layout.0.surface_height(spot) + 3.6, spot.y),
            shot.him_light,
        )
    });
    match follow.or(shot.key_light) {
        Some((at, power)) => {
            kl.intensity = power;
            ktf.translation = at;
        }
        None => kl.intensity = 0.0,
    }
    // His hat, where the shot wants it.
    let (hat_life, hat_tf, hat_vis) = &mut *hat;
    if let Some(p) = shot.hat {
        hat_life.0 = 60.0;
        hat_tf.translation = Vec3::new(p.x, layout.0.surface_height(p) + 0.03, p.y);
        hat_tf.rotation = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.12);
        **hat_vis = Visibility::Inherited;
    }
    // Him.
    let th = &mut truth.encounter.threat;
    match &shot.him {
        Some(him) => {
            let (at, face, speed) = him_at(him, t);
            th.pos = at;
            th.facing = face;
            th.speed = speed;
            th.presence = Presence::Present;
            th.state = him.state;
        }
        None => {
            th.presence = Presence::Hidden;
            th.state = ThreatState::Dormant;
        }
    }
    // The catch.
    match shot.catch_at {
        Some(start) => {
            let eye = shot.cam[0].pos;
            let ahead = (shot.cam[0].look - eye).truncate_y();
            fright.catch = Some(Catch {
                eye,
                ground: layout.0.surface_height(Vec2::new(eye.x, eye.z)),
                dir: ahead,
                forward: ahead,
                variation: 0,
            });
            fright.lunge = Some(t - start - LUNGE_GAP);
        }
        None => {
            fright.catch = None;
            fright.lunge = None;
        }
    }
    // The mirror: power, friends, the bundle in hand.
    if let Some(endpoint) = net.endpoint.as_mut() {
        let me = endpoint.id;
        if let Some(s) = endpoint.snapshot.as_mut() {
            if shot.powered {
                s.world.power = 1.0;
                s.world.truck = 1.0;
                s.world.warm = 1.0;
            } else {
                s.world.power = 0.0;
                s.world.truck = 0.0;
            }
            s.danger = 0;
            s.players.retain(|p| p.id < crate::photos::PHOTO_PLAYER_BASE);
            if let Some(p) = s.players.iter_mut().find(|p| Some(p.id) == me) {
                p.carrying = u8::from(shot.carrying);
                p.status = 0;
            }
            for (i, m) in shot.mates.iter().enumerate() {
                let k = if m.steady {
                    (t / shot.dur).clamp(0.0, 1.0)
                } else {
                    smooth(t / shot.dur)
                };
                let at = m.from.lerp(m.to, k);
                let dir = (m.to - m.from).normalize_or(Vec2::NEG_Y);
                s.players.push(crate::net::protocol::PlayerView {
                    id: crate::photos::PHOTO_PLAYER_BASE + i as u64,
                    position: at.to_array(),
                    yaw: (-dir.x).atan2(-dir.y),
                    pitch: -0.15,
                    status: u8::from(m.down),
                    crouch: m.crouch,
                    sprint: m.sprint,
                    light: true,
                    carrying: u8::from(m.carrying),
                    revive: 0.0,
                    bleed: 0.0,
                    hauled: false,
                    // Each staged teammate someone else.
                    survivor: ((i + 1) % 4) as u8,
                });
            }
        }
    }
    let _ = &launch;

    // Warm up each shot until the renderer has everything it needs.
    if !capturing {
        run.warm += 1;
        if probe.0.load(Ordering::Relaxed) == 0 && models.settled() && run.warm > 6 {
            run.idle += 1;
        } else {
            run.idle = 0;
        }
        if !run.omen_sent
            && let Some(e) = shot.omen
        {
            omens.write(EncounterMsg(e));
            run.omen_sent = true;
        }
        return;
    }
    // Never more than a few frames waiting for the disk.
    if screenshots.iter().count() >= IN_FLIGHT {
        virtual_time.pause();
        return;
    }
    virtual_time.unpause();
    let dir = launch
        .shots_dir
        .join(if run.stills { "trailer_stills" } else { "trailer" });
    let path = if run.stills {
        dir.join(format!("{}.png", shot.name))
    } else {
        dir.join(shot.name).join(format!("{:05}.png", run.frame))
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    commands
        .spawn(Screenshot::image(run.target.clone()))
        .observe(save_to_disk(path.clone()));
    run.written.push(path);
    run.frame += 1;
    if run.stills || run.frame >= total {
        info!("trailer: {} done", shot.name);
        run.index += 1;
        run.frame = 0;
        run.warm = 0;
        run.idle = 0;
        run.omen_sent = false;
    }
}

trait TruncateY {
    fn truncate_y(self) -> Vec2;
}

impl TruncateY for Vec3 {
    fn truncate_y(self) -> Vec2 {
        Vec2::new(self.x, self.z).normalize_or(Vec2::NEG_Y)
    }
}
