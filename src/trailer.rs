//! `--trailer` (DEBUG): renders the teaser's shots, frame by frame, into
//! `<shots>/trailer/<NN_shot>/00000.png…` at 1920×1080 and 30 fps, from an
//! off-screen target (so the window's size never matters). Every shot is a
//! camera path with staged presentation: him walking where the shot wants
//! him, lightning when it wants it, friends' torches, an omen, the catch.
//! Nothing here touches the rules; the staging is a presentation mirror,
//! rebuilt every frame, like `--photos`. `TRAILER_STILLS=1` renders one
//! frame per shot instead (for framing); `TRAILER_ONLY=name` renders only
//! the shots whose names contain it (or any of a comma list). The edit
//! (titles, sound) is made from these frames by `trailer/`.

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

/// The frame size: 1920×1080, or 1080×1920 with `TRAILER_VERTICAL=1` (the
/// same shots for a phone, written to `trailer_vertical/`).
fn size() -> (u32, u32) {
    if vertical() { (HEIGHT, WIDTH) } else { (WIDTH, HEIGHT) }
}

fn vertical() -> bool {
    std::env::var("TRAILER_VERTICAL").is_ok()
}
/// Exposure added over the night's grade: a touch brighter than play, for
/// video. The +0.55 the old exposure brightness gave it (0.7 stops a unit),
/// so its look is unchanged; the player's brightness is a gamma now.
pub const EXPOSURE_LIFT: f32 = 0.7 * 0.55;
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
    /// El Velo: from this many seconds into the shot nobody sees him.
    pub vanish_at: Option<f32>,
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

/// Tureco loose at `at`, facing `face`, growling; barking from `bark_at`.
#[derive(Clone, Copy)]
pub struct Dog {
    pub at: Vec2,
    pub face: Vec2,
    pub bark_at: Option<f32>,
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
    /// La Rabia: all but one bundle already lie at the ceiba, the last is
    /// in hand and is laid this many seconds in (and the llano answers).
    pub lay: Option<f32>,
    /// All five bundles lie in the ceiba's arc, nobody carrying one.
    pub bones_home: bool,
    /// He rises out of the grass this many seconds in (unseen before).
    pub rise_at: Option<f32>,
    /// Pepper wards burning on the ground.
    pub wards: Vec<Vec2>,
    /// The lookout's fire is lit.
    pub beacon: bool,
    /// Tureco, loose.
    pub dog: Option<Dog>,
    /// The camera passes its keys at an even pace, eased only at the ends
    /// (for a turn traced by many keys).
    pub glide: bool,
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
            lay: None,
            bones_home: false,
            rise_at: None,
            wards: Vec::new(),
            beacon: false,
            dog: None,
            glide: false,
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
    if shot.glide {
        let (t0, t1) = (keys[0].t, keys[keys.len() - 1].t);
        let u = t0 + smooth((t - t0) / (t1 - t0).max(1e-3)) * (t1 - t0);
        for w in keys.windows(2) {
            if u <= w[1].t {
                let k = (u - w[0].t) / (w[1].t - w[0].t).max(1e-3);
                return (w[0].pos.lerp(w[1].pos, k), w[0].look.lerp(w[1].look, k));
            }
        }
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
        vanish_at: None,
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
            // across the rope, so it reads from the post to his collar
            key(0.0, p3 + Vec3::new(3.2, 1.0, 2.6), p3 + Vec3::new(-0.3, 0.55, 0.0)),
            key(4.0, p3 + Vec3::new(2.5, 0.8, 2.0), p3 + Vec3::new(-0.3, 0.5, 0.0)),
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
    // Friends walk in to it at a walking pace (the game's stride is sized to
    // the speed over the ground: a short path is a shuffle).
    s.mates = vec![
        Mate {
            steady: true,
            ..Mate::walk(truck + Vec2::new(-1.1, -7.8), truck + Vec2::new(5.0, -1.6))
        },
        Mate {
            steady: true,
            ..Mate::walk(truck + Vec2::new(-2.3, -7.6), truck + Vec2::new(2.2, -2.1))
        },
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
        vanish_at: None,
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
        vanish_at: None,
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

    // 12. Hiding in the tall grass: he passes right in front, slowly, his
    //     right side to us (the sack rides on his left).
    let hide = Vec2::new(fields.x + 2.0, fields.y + 2.0);
    let mut s = Shot::new(
        "12_hide",
        4.5,
        vec![
            key(0.0, at(hide.x, hide.y, 0.55), at(hide.x - 1.0, hide.y - 3.0, 2.0)),
            key(4.5, at(hide.x, hide.y, 0.5), at(hide.x + 1.5, hide.y - 3.0, 2.3)),
        ],
    );
    s.fov = 62.0;
    s.handheld = 0.45;
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: hide + Vec2::new(-3.6, -1.9),
            },
            HimKey {
                t: 4.5,
                at: hide + Vec2::new(2.8, -1.7),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
        vanish_at: None,
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
        vanish_at: None,
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
        vanish_at: None,
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
            // three-quarters on, so the stride reads
            key(0.0, from_spawn(2.6, 3.0, 1.2), from_spawn(9.0, 0.0, 1.3)),
            key(3.5, from_spawn(1.2, 2.7, 1.15), from_spawn(6.5, 0.0, 1.3)),
        ],
    );
    s.fov = 45.0;
    s.handheld = 0.3;
    s.key_light = Some((from_spawn(4.0, 1.5, 3.2), 26_000.0));
    s.still = 0.8;
    s.mates = [(-1.2, 13.0), (-0.4, 13.9), (0.45, 13.4), (1.25, 14.3)]
        .iter()
        .map(|&(r, a)| Mate {
            steady: true,
            ..Mate::walk(
                layout.spawn + ahead * a + right * r,
                layout.spawn + ahead * (a - 8.5) + right * r * 0.9,
            )
        })
        .collect();
    out.push(s);

    // 17. La Rabia: first person, the last bundle laid in the ceiba's arc
    //     beside the other four; every lamp on the llano stutters.
    let out_dir = (Vec2::new(offering.x, offering.z) - ceiba).normalize_or(Vec2::X);
    let altar = Vec2::new(offering.x, offering.z);
    let roots = Vec3::new(altar.x, g(altar) + 0.2, altar.y);
    let mut s = Shot::new(
        "17_lay",
        4.5,
        vec![
            key(
                0.0,
                at(altar.x + out_dir.x * 5.0, altar.y + out_dir.y * 5.0, 1.62),
                roots + Vec3::Y * 0.9,
            ),
            key(
                2.2,
                at(altar.x + out_dir.x * 2.0, altar.y + out_dir.y * 2.0, 1.45),
                roots,
            ),
            key(
                4.5,
                at(altar.x + out_dir.x * 2.3, altar.y + out_dir.y * 2.3, 1.55),
                roots + Vec3::Y * 1.4,
            ),
        ],
    );
    s.pov = true;
    s.torch = true;
    s.fov = 64.0;
    s.lay = Some(2.3);
    s.still = 2.5 / 4.5;
    s.key_light = Some((Vec3::new(offering.x, offering.y + 1.5, offering.z), 12_000.0));
    out.push(s);

    // 18. The shelf radio in the dark house: a slow push in on its dial.
    let radio = d.radio;
    let side = Vec3::X;
    let mut s = Shot::new(
        "18_radio",
        3.5,
        vec![
            key(
                0.0,
                radio + side * 1.6 + Vec3::new(0.0, 0.25, 0.35),
                radio + Vec3::Y * 0.1,
            ),
            key(
                3.5,
                radio + side * 0.85 + Vec3::new(0.0, 0.12, 0.15),
                radio + Vec3::Y * 0.1,
            ),
        ],
    );
    s.fov = 40.0;
    s.handheld = 0.1;
    s.key_light = Some((radio + side * 0.9 + Vec3::Y * 0.8, 1_500.0));
    out.push(s);

    // 19. Wading the caño past the moored boat, torches low over the water.
    let boat = d.boat.center();
    let wade = Vec2::new(boat.x + 2.0, -64.5);
    let mut s = Shot::new(
        "19_cano",
        4.0,
        vec![
            key(0.0, at(18.6, -63.0, 2.5), at(wade.x + 5.0, wade.y, 0.2)),
            key(4.0, at(18.8, -63.4, 2.4), at(wade.x + 2.5, wade.y, 0.3)),
        ],
    );
    s.fov = 38.0;
    s.handheld = 0.25;
    s.key_light = Some((at(wade.x + 4.0, -62.5, 3.5), 26_000.0));
    s.mates = vec![
        Mate {
            steady: true,
            ..Mate::walk(wade + Vec2::new(10.0, 0.6), wade + Vec2::new(2.5, 0.2))
        },
        Mate {
            steady: true,
            carrying: true,
            ..Mate::walk(wade + Vec2::new(12.5, 2.0), wade + Vec2::new(5.0, 1.5))
        },
    ];
    out.push(s);

    // 20. A friend down in the tall grass, crawling, the torch dropped beside
    //     them; another creeps in, crouched, to reach them.
    let fallen = Vec2::new(fields.x - 7.0, fields.y + 9.0);
    let mut s = Shot::new(
        "20_downed",
        4.0,
        vec![
            key(
                0.0,
                at(fallen.x - 4.0, fallen.y + 3.5, 1.9),
                at(fallen.x, fallen.y, 0.2),
            ),
            key(
                4.0,
                at(fallen.x - 3.2, fallen.y + 2.8, 1.7),
                at(fallen.x + 0.3, fallen.y, 0.2),
            ),
        ],
    );
    s.fov = 50.0;
    s.handheld = 0.3;
    s.key_light = Some((at(fallen.x - 1.5, fallen.y + 1.5, 3.0), 14_000.0));
    s.mates = vec![
        Mate {
            down: true,
            steady: true,
            ..Mate::walk(fallen + Vec2::new(0.6, -0.4), fallen + Vec2::new(-0.2, 0.1))
        },
        Mate {
            crouch: true,
            steady: true,
            ..Mate::walk(fallen + Vec2::new(5.5, -3.8), fallen + Vec2::new(1.2, -0.6))
        },
    ];
    out.push(s);

    // 21. El Velo: he walks the grass under a cold light, and then he is not
    //     there. Only the whistle (in the edit) goes on.
    let veil = Vec2::new(fields.x - 8.0, fields.y + 1.0);
    let mut s = Shot::new(
        "21_velo",
        5.0,
        vec![
            key(0.0, at(fields.x - 13.5, fields.y + 5.0, 1.9), at(veil.x, veil.y, 1.6)),
            key(
                5.0,
                at(fields.x - 13.0, fields.y + 4.6, 1.9),
                at(veil.x + 2.0, veil.y - 1.0, 1.6),
            ),
        ],
    );
    s.fov = 30.0;
    s.handheld = 0.2;
    s.him = Some(Him {
        keys: vec![
            HimKey { t: 0.0, at: veil },
            HimKey {
                t: 5.0,
                at: veil + Vec2::new(4.0, -2.0),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
        vanish_at: Some(2.6),
    });
    s.him_light = 34_000.0;
    s.still = 0.3;
    out.push(s);

    // 22. His face: a slow push in under the brim until the eyes catch the
    //     torch; he stands quite still and looks back.
    let still_at = Vec2::new(fields.x - 10.0, fields.y + 2.0);
    let toward = Vec2::new(-0.8, 0.6).normalize();
    let eye_at = |back: f32, up: f32| {
        let p = still_at + toward * back;
        Vec3::new(p.x, g(still_at) + up, p.y)
    };
    let eyes = eye_at(0.154, 2.405);
    let mut s = Shot::new(
        "22_face",
        5.0,
        vec![
            key(0.0, eye_at(3.4, 2.2), eyes),
            key(5.0, eye_at(1.35, 2.33), eyes + Vec3::Y * 0.01),
        ],
    );
    s.fov = 36.0;
    s.handheld = 0.12;
    s.torch = true;
    s.key_light = Some((eye_at(1.0, 1.7), 2_500.0));
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: still_at }],
        state: ThreatState::Stalking,
        face: Some(toward),
        vanish_at: None,
    });
    s.still = 0.9;
    out.push(s);

    // 23. The baba in the caño: low over the water, its eyes and snout
    //     awash in the torchlight.
    if let Some(c) = d
        .fauna
        .iter()
        .find(|f| f.kind == crate::geometry::district::FaunaKind::Caiman)
    {
        let fwd = Vec2::new(-c.yaw.sin(), -c.yaw.cos());
        let side = Vec2::new(-fwd.y, fwd.x);
        let water = crate::geometry::district::WATER_LEVEL;
        let head = c.at + fwd * 1.0;
        let low = |p: Vec2, up: f32| Vec3::new(p.x, water + up, p.y);
        let mut s = Shot::new(
            "23_caiman",
            4.0,
            vec![
                key(0.0, low(c.at + fwd * 3.6 + side * 1.2, 0.32), low(head, 0.02)),
                key(4.0, low(c.at + fwd * 2.6 + side * 0.7, 0.26), low(head, 0.0)),
            ],
        );
        s.fov = 40.0;
        s.handheld = 0.2;
        s.torch = true;
        s.key_light = Some((low(c.at + fwd * 1.8 + side * 1.4, 1.4), 22_000.0));
        out.push(s);
    }

    // 24. His sack of bones swinging on his back, followed from behind his
    //     shoulder as he walks the road outside the gate, backlit by its
    //     lamps; a strike shows him whole.
    let lane_z = gate.y + 5.5;
    let mut s = Shot::new(
        "24_sack",
        4.0,
        vec![
            key(0.0, at(gate.x + 7.2, lane_z + 2.6, 1.8), at(gate.x + 3.6, lane_z, 1.95)),
            key(4.0, at(gate.x - 0.9, lane_z + 2.6, 1.8), at(gate.x - 4.6, lane_z, 1.95)),
        ],
    );
    s.fov = 52.0;
    s.handheld = 0.25;
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: Vec2::new(gate.x + 4.5, lane_z),
            },
            HimKey {
                t: 4.0,
                at: Vec2::new(gate.x - 4.0, lane_z),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
        vanish_at: None,
    });
    s.him_light = 14_000.0;
    s.storm = Storm::FlashAt(2.6);
    out.push(s);

    // 25. He stoops over the bones laid at the ceiba, counting them, turned
    //     three-quarters to us across the arc of bundles.
    let side = Vec2::new(-out_dir.y, out_dir.x);
    let at_altar = altar + out_dir * 1.55 + side * 0.4;
    let towards = (-out_dir * 0.5 + side * 0.86).normalize();
    let cam_at = at_altar + side * 3.9 + out_dir * 2.2;
    let mut s = Shot::new(
        "25_count",
        4.0,
        vec![
            key(0.0, at(cam_at.x, cam_at.y, 1.3), at(at_altar.x, at_altar.y, 1.25)),
            key(
                4.0,
                at(cam_at.x - side.x * 0.7, cam_at.y - side.y * 0.7, 1.2),
                at(at_altar.x, at_altar.y, 1.15),
            ),
        ],
    );
    s.fov = 45.0;
    s.handheld = 0.15;
    s.bones_home = true;
    s.key_light = Some((Vec3::new(offering.x, offering.y + 1.2, offering.z), 9_000.0));
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: at_altar }],
        state: ThreatState::Counting,
        face: Some(towards),
        vanish_at: None,
    });
    out.push(s);

    // 26. He rises out of the tall grass under a cold light, facing us.
    let rise = Vec2::new(fields.x - 8.0, fields.y + 1.0);
    let toward_cam = Vec2::new(-0.78, 0.62).normalize();
    let mut s = Shot::new(
        "26_rise",
        5.5,
        vec![
            key(
                0.0,
                at(rise.x + toward_cam.x * 7.5, rise.y + toward_cam.y * 7.5, 1.2),
                at(rise.x, rise.y, 1.5),
            ),
            key(
                5.5,
                at(rise.x + toward_cam.x * 6.2, rise.y + toward_cam.y * 6.2, 1.1),
                at(rise.x, rise.y, 1.9),
            ),
        ],
    );
    s.fov = 40.0;
    s.handheld = 0.2;
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: rise }],
        state: ThreatState::Stalking,
        face: Some(toward_cam),
        vanish_at: None,
    });
    s.rise_at = Some(1.0);
    s.him_light = 40_000.0;
    s.still = 0.85;
    out.push(s);

    // 27. A friend down on the lit road, crawling toward us; another comes
    //     crouched to reach them; a strike shows him standing behind them.
    let mut s = Shot::new(
        "27_crawl",
        5.5,
        vec![
            key(0.0, from_spawn(1.6, 0.7, 0.5), from_spawn(9.0, 0.0, 0.6)),
            key(5.5, from_spawn(1.1, 0.5, 0.45), from_spawn(9.0, 0.2, 0.8)),
        ],
    );
    s.fov = 46.0;
    s.handheld = 0.2;
    s.key_light = Some((from_spawn(4.5, 1.2, 3.0), 26_000.0));
    s.mates = vec![
        Mate {
            down: true,
            steady: true,
            ..Mate::walk(
                layout.spawn + ahead * 7.6 + right * 0.3,
                layout.spawn + ahead * 3.4 + right * 0.1,
            )
        },
        Mate {
            crouch: true,
            steady: true,
            ..Mate::walk(
                layout.spawn + ahead * 6.5 - right * 4.5,
                layout.spawn + ahead * 4.0 - right * 0.9,
            )
        },
    ];
    let behind = layout.spawn + ahead * 15.0 + right * 1.2;
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: behind }],
        state: ThreatState::Stalking,
        face: Some(-ahead),
        vanish_at: None,
    });
    s.rise_at = Some(1.5);
    s.storm = Storm::FlashAt(4.0);
    s.still = 4.1 / 5.5;
    out.push(s);

    // 28. A pepper ward burns on the track; he squats at its edge counting
    //     his bones while two friends creep past behind him.
    let ward = Vec2::new(16.0, 8.0);
    let edge = ward + Vec2::new(0.95, 0.15).normalize() * 3.6;
    let mut s = Shot::new(
        "28_ward",
        5.0,
        vec![
            key(0.0, at(ward.x - 2.0, ward.y + 8.0, 1.7), at(ward.x + 1.8, ward.y, 0.7)),
            key(5.0, at(ward.x - 1.0, ward.y + 7.0, 1.5), at(ward.x + 2.2, ward.y, 0.8)),
        ],
    );
    s.fov = 50.0;
    s.handheld = 0.2;
    s.wards = vec![ward];
    s.key_light = Some((at(edge.x - 0.5, edge.y + 2.5, 3.5), 12_000.0));
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: edge }],
        state: ThreatState::Counting,
        face: Some((ward - edge).normalize()),
        vanish_at: None,
    });
    s.mates = vec![
        Mate {
            crouch: true,
            steady: true,
            ..Mate::walk(ward + Vec2::new(-6.5, -1.6), ward + Vec2::new(1.5, -2.2))
        },
        Mate {
            crouch: true,
            steady: true,
            ..Mate::walk(ward + Vec2::new(-8.0, -1.0), ward + Vec2::new(0.0, -1.7))
        },
    ];
    out.push(s);

    // 29. Tureco loose in the yard, hackles up, barking at the dark; a
    //     strike shows what he smelled.
    let dog = Vec2::new(8.0, 14.0);
    let sniff = Vec2::new(1.0, -0.12).normalize();
    let mut s = Shot::new(
        "29_bark",
        4.0,
        vec![
            key(
                0.0,
                at(dog.x - 1.9, dog.y + 1.4, 0.65),
                at(dog.x + sniff.x * 8.0, dog.y + sniff.y * 8.0, 0.9),
            ),
            key(
                4.0,
                at(dog.x - 1.6, dog.y + 1.2, 0.6),
                at(dog.x + sniff.x * 8.0, dog.y + sniff.y * 8.0, 1.0),
            ),
        ],
    );
    s.fov = 50.0;
    s.handheld = 0.25;
    s.dog = Some(Dog {
        at: dog,
        face: sniff,
        bark_at: Some(1.0),
    });
    s.key_light = Some((at(dog.x - 0.5, dog.y + 1.5, 2.5), 7_000.0));
    s.him = Some(Him {
        keys: vec![HimKey {
            t: 0.0,
            at: dog + sniff * 20.0,
        }],
        state: ThreatState::Stalking,
        face: Some(-sniff),
        vanish_at: None,
    });
    s.storm = Storm::FlashAt(2.5);
    s.still = 2.6 / 4.0;
    out.push(s);

    // 30. The lookout's fire burns; he walks to it through the grass.
    let fire = d.beacon;
    let fire2 = Vec2::new(fire.x, fire.z);
    let mut s = Shot::new(
        "30_beacon",
        5.0,
        vec![
            key(0.0, at(fire2.x - 9.0, fire2.y + 17.0, 1.2), fire),
            key(5.0, at(fire2.x - 8.0, fire2.y + 15.5, 1.1), fire - Vec3::Y * 1.0),
        ],
    );
    s.fov = 50.0;
    s.handheld = 0.15;
    s.beacon = true;
    s.him = Some(Him {
        keys: vec![
            HimKey {
                t: 0.0,
                at: fire2 + Vec2::new(-7.0, 14.0),
            },
            HimKey {
                t: 5.0,
                at: fire2 + Vec2::new(-3.8, 8.5),
            },
        ],
        state: ThreatState::Stalking,
        face: None,
        vanish_at: None,
    });
    out.push(s);

    // 31. From the lookout: a strike lights the whole llano and he is a
    //     speck far out on the open ground.
    let speck = Vec2::new(38.5, -49.0);
    let view = Vec2::new(32.0, -30.0);
    let deck = Vec3::new(43.0, d.watch_height + 1.9, -86.0);
    let mut s = Shot::new(
        "31_wide",
        6.0,
        vec![
            key(0.0, deck, at(view.x, view.y, 0.0)),
            key(6.0, deck + Vec3::new(-0.3, 0.0, 0.4), at(view.x, view.y, 0.3)),
        ],
    );
    s.fov = 46.0;
    s.handheld = 0.05;
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: speck }],
        state: ThreatState::Stalking,
        face: Some((Vec2::new(deck.x, deck.z) - speck).normalize()),
        vanish_at: None,
    });
    s.storm = Storm::FlashAt(1.5);
    s.still = 1.6 / 6.0;
    out.push(s);

    // 32. First person in the grass, torch on: the eye turns slowly round…
    //     and he is standing right behind.
    let eye_at = Vec2::new(fields.x - 4.0, fields.y + 6.0);
    let a0 = std::f32::consts::PI * 0.95;
    let eye3 = at(eye_at.x, eye_at.y, 1.58);
    let n = 12;
    let mut cam = vec![key(0.0, eye3, eye3 + Vec3::new(a0.cos() * 6.0, -0.15, a0.sin() * 6.0))];
    for i in 0..=n {
        let k = i as f32 / n as f32;
        let a = a0 + std::f32::consts::PI * k;
        let up = -0.15 + 0.85 * smooth((k - 0.6) / 0.4);
        cam.push(key(
            0.9 + 4.8 * k,
            eye3,
            eye3 + Vec3::new(a.cos() * 6.0, up, a.sin() * 6.0),
        ));
    }
    let mut s = Shot::new("32_turn", 7.5, cam);
    s.glide = true;
    s.pov = true;
    s.torch = true;
    s.fov = 64.0;
    s.handheld = 0.35;
    let behind = eye_at + Vec2::new((a0 + std::f32::consts::PI).cos(), (a0 + std::f32::consts::PI).sin()) * 2.5;
    s.him = Some(Him {
        keys: vec![HimKey { t: 0.0, at: behind }],
        state: ThreatState::Warning,
        face: Some((eye_at - behind).normalize()),
        vanish_at: None,
    });
    s.him_light = 4_000.0;
    s.still = 0.92;
    out.push(s);

    // 33. A friend walks toward us down the road, torch in hand, looking
    //     about; the camera backs away before them.
    let mut s = Shot::new(
        "33_survivor",
        4.5,
        vec![
            key(0.0, from_spawn(4.6, 0.35, 1.5), from_spawn(9.0, 0.0, 1.45)),
            key(4.5, from_spawn(2.0, 0.3, 1.5), from_spawn(5.4, 0.0, 1.45)),
        ],
    );
    s.fov = 28.0;
    s.handheld = 0.3;
    s.key_light = Some((from_spawn(5.0, 1.0, 3.0), 22_000.0));
    s.mates = vec![Mate {
        steady: true,
        ..Mate::walk(layout.spawn + ahead * 9.6, layout.spawn + ahead * 5.6)
    }];
    s.still = 0.7;
    out.push(s);

    // 34. The dark house: a long slow push toward the shelf radio, a strike
    //     in the window.
    let radio = d.radio;
    let mut s = Shot::new(
        "34_radio_long",
        9.0,
        vec![
            key(0.0, radio + Vec3::new(3.2, 0.35, 0.6), radio + Vec3::Y * 0.05),
            key(9.0, radio + Vec3::new(0.8, 0.12, 0.12), radio + Vec3::Y * 0.08),
        ],
    );
    s.fov = 42.0;
    s.handheld = 0.06;
    s.key_light = Some((radio + Vec3::new(0.9, 0.8, 0.0), 1_200.0));
    s.storm = Storm::FlashAt(6.5);
    s.still = 0.2;
    out.push(s);

    // 35. The chigüires on the caño's bank, low across the water.
    let herd: Vec<Vec2> = d
        .fauna
        .iter()
        .filter(|f| f.kind == crate::geometry::district::FaunaKind::Capybara)
        .map(|f| f.at)
        .collect();
    if !herd.is_empty() {
        let c = herd.iter().copied().sum::<Vec2>() / herd.len() as f32;
        let mut s = Shot::new(
            "35_capybara",
            4.0,
            vec![
                key(0.0, at(c.x - 3.6, c.y + 2.6, 1.35), at(c.x, c.y, 0.3)),
                key(4.0, at(c.x - 3.0, c.y + 2.0, 1.25), at(c.x + 0.3, c.y, 0.3)),
            ],
        );
        s.fov = 42.0;
        s.handheld = 0.2;
        s.key_light = Some((at(c.x - 1.2, c.y + 1.4, 2.6), 24_000.0));
        out.push(s);
    }

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
    /// The laying's telegraph went out this shot.
    lay_sent: bool,
    exit_in: Option<u32>,
}

/// The shots render into this image, never the window.
pub(crate) fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    layout: Res<LayoutRes>,
    camera: Single<Entity, With<Player>>,
) {
    let (width, height) = size();
    let image = Image::new_target_texture(
        width,
        height,
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
        shots.retain(|s| only.split(',').any(|o| s.name.contains(o)));
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
        lay_sent: false,
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
    let seen = shot.him.as_ref().filter(|him| him.vanish_at.is_none_or(|v| t < v));
    match seen {
        Some(him) => {
            let (at, face, speed) = him_at(him, t);
            th.pos = at;
            th.facing = face;
            th.speed = speed;
            th.presence = match shot.rise_at {
                Some(r) if t < r => Presence::Hidden,
                Some(r) => Presence::Rising { t: t - r },
                None => Presence::Present,
            };
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
            s.zones = shot.wards.iter().map(|w| [w.x, w.y, 30.0]).collect();
            s.world.beacon = if shot.beacon { 30.0 } else { 0.0 };
            if let Some(dog) = shot.dog {
                s.dog.pos = dog.at.to_array();
                s.dog.facing = dog.face.normalize_or(Vec2::Y).to_array();
                s.dog.mood = if dog.bark_at.is_some_and(|b| t >= b) { 3 } else { 2 };
            }
            s.players.retain(|p| p.id < crate::photos::PHOTO_PLAYER_BASE);
            let laid = shot.lay.is_some_and(|at| t >= at);
            if let Some(p) = s.players.iter_mut().find(|p| Some(p.id) == me) {
                p.carrying = u8::from(shot.carrying || (shot.lay.is_some() && !laid));
                p.status = 0;
            }
            if shot.bones_home {
                for r in s.relics.iter_mut() {
                    r.state = 2;
                }
                s.world.delivered = s.relics.len() as u8;
            }
            if shot.lay.is_some() {
                let last = s.relics.len().saturating_sub(1);
                for (i, r) in s.relics.iter_mut().enumerate() {
                    r.state = if i < last || laid { 2 } else { 1 };
                }
                s.world.delivered = (last + usize::from(laid)) as u8;
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
                    watching: 0,
                });
            }
        }
    }
    let _ = &launch;
    if capturing && !run.lay_sent && shot.lay.is_some_and(|at| t >= at) {
        omens.write(EncounterMsg(Event::RelicDelivered));
        run.lay_sent = true;
    }

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
    let dir = launch.shots_dir.join(match (run.stills, vertical()) {
        (true, false) => "trailer_stills",
        (true, true) => "trailer_vertical_stills",
        (false, false) => "trailer",
        (false, true) => "trailer_vertical",
    });
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
        run.lay_sent = false;
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
