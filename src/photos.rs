//! DEBUG ONLY: the authored viewpoints for `--photos`. No gameplay runs; the
//! camera is placed at each viewpoint and the frame is saved, so every
//! landmark can be reviewed in seconds.
//!
//! Every view is *searched* from the [`Layout`] rather than hard-coded: a
//! camera may only stand on open ground (off water, clear of walls, props,
//! fences, roofs, trunks and tree crowns) with an unobstructed sight line to
//! what it frames, so moving a landmark or a tree moves its photo with it.
//! Pure data; nothing here touches the ECS.
//!
//! These frames are presentation review, never gameplay proof. Each [`Shot`]
//! says in `label` exactly what the driver alters relative to play (camera
//! teleport, pinned storm, mirrored power, placed avatars, UI state); the
//! driver prints it on the image and in the manifest.

use bevy::math::{Vec2, Vec3};

use crate::geometry::district::{LandmarkId, NoteSite};
use crate::geometry::{Facing, Layout, OpeningKind, Rect2, segment_point_distance};
use crate::tuning::Tuning;

/// Shots are framed for this aspect ratio; on a narrower window the driver
/// widens the vertical field of view so the same horizontal frame is kept.
pub const REF_ASPECT: f32 = 16.0 / 9.0;
/// Simulated seconds between the two frames of a time-separated pair.
pub const PAIR_GAP_SECS: f32 = 3.0;
/// Party ids in the photo mirror start here (real ids are small).
pub const PHOTO_PLAYER_BASE: u64 = 100;

/// Which surface the frame is about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Surface {
    /// The world, HUD hidden.
    World,
    /// The map overlay over the real HUD.
    Map,
    /// A note page (`lore::note`) over the real HUD.
    Note(u8),
    /// The downed panel over the real HUD. Mirror only: no outcome is set.
    Downed,
    /// The catch held at this instant of its clock (world only, HUD hidden).
    Lunge(f32),
}

impl Surface {
    pub fn is_ui(self) -> bool {
        !matches!(self, Surface::World | Surface::Lunge(_))
    }
}

/// A teammate placed for a photo (mirrored into the snapshot, never sent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Companion {
    pub pos: Vec2,
    pub yaw: f32,
    pub carrying: bool,
    pub crouch: bool,
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub name: String,
    pub pos: Vec3,
    pub target: Vec3,
    /// Vertical field of view at [`REF_ASPECT`].
    pub fov_deg: f32,
    /// Show the world as it looks late in a run: power on, the truck running,
    /// the beacon burning.
    pub powered: bool,
    /// Place him here (ground position and the way he faces) for the frame.
    pub silbon: Option<(Vec2, Vec2)>,
    /// Photo-only teammates.
    pub party: Vec<Companion>,
    pub surface: Surface,
    /// Capture twice, [`PAIR_GAP_SECS`] of simulated time apart, same view:
    /// rain and grass motion show up as an image difference.
    pub pair: bool,
    /// What differs from play in this frame.
    pub label: String,
    /// Every search found open ground and a clear line; false means the
    /// authored fallback was used and the frame may be obstructed.
    pub clear: bool,
}

/// What a landmark photo frames.
struct Subject {
    /// Half width and height (above the ground at the centre) of the mass.
    half: f32,
    top: f32,
    /// Height at the centre the camera looks at.
    look: f32,
    /// Where the camera would like to stand, as an offset from the centre.
    prefer: Vec2,
    /// Height of the overview camera above ground.
    cam_h: f32,
}

fn subject(layout: &Layout, id: LandmarkId) -> Subject {
    use LandmarkId::*;
    let d = &layout.district;
    let s = |half, top, look, prefer: (f32, f32), cam_h| Subject {
        half,
        top,
        look,
        prefer: Vec2::new(prefer.0, prefer.1),
        cam_h,
    };
    match id {
        Entry => s(5.0, 4.0, 2.4, (-11.0, 5.0), 3.0),
        Ranch => s(7.0, 4.2, 1.8, (-17.0, 15.0), 4.5),
        WaterTower => {
            let top = d.tower_height + 1.5;
            s(d.tower_half.max_element() + 1.5, top, top * 0.55, (14.0, 12.0), 3.5)
        }
        Corral => s(10.0, 2.5, 1.2, (-16.0, 12.0), 4.5),
        Shrine => s(6.0, 10.0, 5.0, (10.0, 6.0), 3.0),
        Fields => s(8.0, 2.5, 1.0, (-14.0, 9.0), 3.0),
        Cano => s(8.0, 3.0, 0.8, (12.0, 6.0), 3.5),
        Watchtower => s(4.5, d.watch_height + 2.5, 4.6, (-14.0, 16.0), 2.5),
        Marsh => s(8.0, 2.5, 0.6, (13.0, 6.0), 2.5),
        Extraction => s(5.0, 3.0, 1.4, (-5.5, -6.0), 3.5),
    }
}

/// Searches for cameras that see their subject.
struct Site<'a> {
    layout: &'a Layout,
    tuning: &'a Tuning,
}

impl Site<'_> {
    fn ground(&self, p: Vec2, h: f32) -> Vec3 {
        Vec3::new(p.x, self.layout.terrain(p) + h, p.y)
    }

    /// A camera (at `height`) or a body may stand here: inside the map, off
    /// the water, clear of walls, props, fences, roofs and trunks, and under
    /// no tree crown.
    fn open(&self, p: Vec2, height: f32, radius: f32) -> bool {
        let l = self.layout;
        let d = &l.district;
        let margin = Vec2::splat(2.0);
        if !Rect2::new(l.bounds.min + margin, l.bounds.max - margin).contains(p) {
            return false;
        }
        if !l.is_free(p, radius) || d.water_at(p) {
            return false;
        }
        let fp = l.house.footprint;
        if Rect2::new(fp.min - Vec2::splat(0.8), fp.max + Vec2::splat(0.8)).contains(p) {
            return false;
        }
        if d.sheds
            .iter()
            .any(|s| Rect2::from_center(s.center, s.half + Vec2::splat(0.8)).contains(p))
        {
            return false;
        }
        // Palm fronds sit high above the trunk; grove crowns start at the
        // height of a raised camera.
        !d.palms
            .iter()
            .any(|&c| c.distance(p) < 1.4 || (height > 7.0 && c.distance(p) < 4.5))
            && !d
                .groves
                .iter()
                .any(|&c| c.distance(p) < 1.4 || (height > 3.3 && c.distance(p) < 5.4))
    }

    /// The camera sees `subject` from `cam`: no solid wall or trunk, no tree
    /// crown, no roof and no rise of the ground in between. The line stops
    /// `standoff` metres short of the subject so its own walls do not count.
    fn sees(&self, cam: Vec3, subject: Vec3, standoff: f32) -> bool {
        let l = self.layout;
        let d = &l.district;
        let a = Vec2::new(cam.x, cam.z);
        let s = Vec2::new(subject.x, subject.z);
        let len = a.distance(s);
        if len < 0.5 {
            return false;
        }
        let end = a + (s - a) * ((len - standoff.min(len - 0.5)) / len);
        if !l.line_of_sight(a, end) {
            return false;
        }
        let seg = end - a;
        let seg_len = seg.length();
        // Fraction of the way from the camera to the subject, and the height
        // of the sight line there.
        let k = seg_len / len;
        let y_at = |f: f32| cam.y + (subject.y - cam.y) * f * k;
        let closest = |c: Vec2| -> (f32, f32) {
            let f = if seg_len > 1e-3 {
                ((c - a).dot(seg) / (seg_len * seg_len)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (segment_point_distance(a, end, c), y_at(f))
        };
        for &c in d.palms.iter().chain(&d.groves) {
            if closest(c).0 < 1.0 {
                return false;
            }
        }
        for &c in &d.groves {
            let (dist, y) = closest(c);
            if dist < 4.2 && (3.3..7.6).contains(&y) {
                return false;
            }
        }
        // Sample the line for roofs and hills.
        let steps = ((seg_len / 0.75).ceil() as usize).clamp(4, 200);
        for i in 1..steps {
            let f = i as f32 / steps as f32;
            let p = a + seg * f;
            let y = y_at(f);
            if y < l.terrain(p) + 0.35 {
                return false;
            }
            for shed in &d.sheds {
                let own = Rect2::from_center(shed.center, shed.half + Vec2::splat(standoff)).contains(s);
                if !own && Rect2::from_center(shed.center, shed.half).contains(p) && y < shed.eave + 0.4 {
                    return false;
                }
            }
        }
        true
    }

    /// Third-person overview: the best camera on rings around the subject.
    fn overview(&self, center: Vec2, sub: &Subject, fov_deg: f32) -> Option<Vec3> {
        let look = self.ground(center, sub.look);
        let tan = (fov_deg.to_radians() * 0.5).tan();
        let fit_w = sub.half * 1.25 / (tan * REF_ASPECT);
        let fit_h = (sub.top - sub.look).max(sub.look) * 1.25 / tan;
        let base = fit_w.max(fit_h).max(6.0) * 1.4;
        let pref = sub.prefer.y.atan2(sub.prefer.x);
        const BEARINGS: usize = 24;
        let mut best: Option<(f32, Vec3)> = None;
        for i in 0..BEARINGS {
            let off = i.min(BEARINGS - i) as f32 * std::f32::consts::TAU / BEARINGS as f32;
            let angle = pref + i as f32 * std::f32::consts::TAU / BEARINGS as f32;
            for (k, scale) in [1.0_f32, 1.2, 1.45, 1.75, 2.1].into_iter().enumerate() {
                let p = center + Vec2::new(angle.cos(), angle.sin()) * base * scale;
                let cam = self.ground(p, sub.cam_h);
                if !self.open(p, sub.cam_h, 1.0) || !self.sees(cam, look, sub.half * 0.6) {
                    continue;
                }
                let score = off * 4.0 + k as f32 * 0.6;
                if best.is_none_or(|(s, _)| score < s) {
                    best = Some((score, cam));
                }
            }
        }
        best.map(|(_, cam)| cam)
    }

    /// Eye-height view as a player would have it: the authored approach, or
    /// the nearest open point to it.
    fn eye_view(&self, approach: Vec2, look: Vec3, standoff: f32) -> Option<Vec3> {
        let eye = self.tuning.eye_height;
        let look2 = Vec2::new(look.x, look.z);
        for ring in 0..6 {
            let r = [0.0_f32, 1.5, 3.0, 4.5, 6.0, 8.0][ring];
            let n = if ring == 0 { 1 } else { 12 };
            for i in 0..n {
                let a = i as f32 * std::f32::consts::TAU / 12.0;
                let p = approach + Vec2::new(a.cos(), a.sin()) * r;
                let cam = self.ground(p, eye);
                if p.distance(look2) < 3.0 || !self.open(p, eye, 0.9) || !self.sees(cam, look, standoff) {
                    continue;
                }
                let pitch = ((look.y - cam.y) / p.distance(look2)).atan().abs();
                if pitch < 40.0_f32.to_radians() {
                    return Some(cam);
                }
            }
        }
        None
    }

    /// Move a monster spot and its cameras rigidly until the monster, each
    /// camera offset and each companion stand on open, roughly level ground
    /// with the cameras seeing the monster. Returns the monster's spot.
    fn stage(&self, want: Vec2, cameras: &[Vec2], cam_h: f32, look_h: f32, party: &[Vec2]) -> Option<Vec2> {
        let eye = self.tuning.eye_height;
        let ok = |m: Vec2| {
            let floor = self.layout.terrain(m);
            self.open(m, 2.0, 0.8)
                && party.iter().all(|&o| self.open(m + o, 2.0, 0.5))
                && cameras.iter().all(|&o| {
                    let c = m + o;
                    let cam = self.ground(c, cam_h.max(eye));
                    (self.layout.terrain(c) - floor).abs() < 1.2
                        && self.open(c, cam_h, 0.9)
                        && self.sees(cam, self.ground(m, look_h), 0.0)
                })
        };
        if ok(want) {
            return Some(want);
        }
        for r in [2.0_f32, 4.0, 6.0, 8.0, 11.0] {
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::TAU / 12.0;
                let m = want + Vec2::new(a.cos(), a.sin()) * r;
                if ok(m) {
                    return Some(m);
                }
            }
        }
        None
    }

    /// Eye-level camera beside a note, looking down at the page.
    fn note_view(&self, note: &NoteSite) -> Option<(Vec3, Vec3)> {
        let l = self.layout;
        let at = Vec2::new(note.pos.x, note.pos.z);
        for r in [1.2_f32, 1.5, 1.9, 2.4] {
            for i in 0..16 {
                let a = i as f32 * std::f32::consts::TAU / 16.0;
                let p = at + Vec2::new(a.cos(), a.sin()) * r;
                if !l.bounds.contains(p) || !l.is_free(p, 0.3) || !l.line_of_sight(p, at) {
                    continue;
                }
                let cam = Vec3::new(p.x, l.surface_height(p) + self.tuning.eye_height, p.y);
                return Some((cam, note.pos));
            }
        }
        None
    }
}

/// The frame's `label`: everything that differs from play.
fn label(shot: &Shot, moved_camera: bool) -> String {
    let mut parts = vec![if moved_camera {
        "camera teleported"
    } else {
        "camera at the real spawn pose"
    }];
    parts.push("storm clock pinned to a calm moment");
    if shot.powered {
        parts.push("power/truck/beacon mirrored ON");
    }
    if shot.silbon.is_some() {
        parts.push("Silbon placed by the debug driver");
    }
    if !shot.party.is_empty() {
        parts.push("teammates are photo-only avatars");
    }
    match shot.surface {
        Surface::World => parts.push("HUD hidden"),
        Surface::Map => parts.push("map opened by the debug driver"),
        Surface::Note(_) => parts.push("note page opened by the debug driver"),
        Surface::Downed => {
            parts.push("DOWNED panel mirrored, camera lowered as when down (no outcome set; not a real down)")
        }
        Surface::Lunge(_) => parts.push("caught jump scare held by the debug driver (not a real catch); HUD hidden"),
    }
    if !shot.clear {
        parts.push("NO CLEAR VIEW FOUND: fallback camera, may be obstructed");
    }
    parts.join("; ")
}

pub fn shots(layout: &Layout, tuning: &Tuning) -> Vec<Shot> {
    let site = Site { layout, tuning };
    let d = &layout.district;
    let eye = tuning.eye_height;
    let mut out: Vec<Shot> = Vec::new();
    let mut push = |mut s: Shot, moved: bool| {
        s.label = label(&s, moved);
        out.push(s);
    };
    let base = |name: String, pos: Vec3, target: Vec3, fov_deg: f32| Shot {
        name,
        pos,
        target,
        fov_deg,
        powered: false,
        silbon: None,
        party: Vec::new(),
        surface: Surface::World,
        pair: false,
        label: String::new(),
        clear: true,
    };

    for (name, powered) in [("00_overview", false), ("00_overview_late", true)] {
        push(
            Shot {
                powered,
                ..base(
                    name.into(),
                    Vec3::new(10.0, 150.0, 92.0),
                    Vec3::new(8.0, 0.0, -34.0),
                    52.0,
                )
            },
            true,
        );
    }

    // Per landmark: an eye-height view like a player's (a) and a raised
    // overview (b). `_a` frames are captured twice for rain/grass motion.
    for m in &d.landmarks {
        let sub = subject(layout, m.id);
        let standoff = sub.half * 0.6;
        let a = site.eye_view(m.approach, m.look, standoff);
        push(
            Shot {
                pair: true,
                clear: a.is_some(),
                ..base(
                    format!("{}_a", m.shot),
                    a.unwrap_or_else(|| site.ground(m.approach, eye)),
                    m.look,
                    68.0,
                )
            },
            true,
        );
        let fov = 62.0;
        let b = site.overview(m.center, &sub, fov);
        let (dx, dz) = (sub.prefer.x, sub.prefer.y);
        push(
            Shot {
                powered: matches!(m.id, LandmarkId::Extraction | LandmarkId::Watchtower),
                clear: b.is_some(),
                ..base(
                    format!("{}_b", m.shot),
                    b.unwrap_or_else(|| site.ground(m.center + Vec2::new(dx, dz), sub.cam_h)),
                    site.ground(m.center, sub.look),
                    fov,
                )
            },
            true,
        );
    }

    // Rain under a roof: the porch and the room behind the front door, both
    // looking out at the storm.
    if let Some(wall) = layout
        .house
        .walls
        .iter()
        .find(|w| w.facing == Facing::South && w.openings.iter().any(|o| o.kind == OpeningKind::Door))
        && let Some(door) = wall.openings.iter().find(|o| o.kind == OpeningKind::Door)
    {
        let mid = wall.at((door.from + door.to) * 0.5);
        let out_dir = wall.facing.normal();
        let porch = layout.house.porch;
        let porch_cam = Vec2::new(mid.x, (porch.min.y + porch.max.y) * 0.5);
        let inner = mid - out_dir * 3.5;
        for (name, p, clear) in [
            ("20_porch_rain", porch_cam, layout.is_free(porch_cam, 0.3)),
            (
                "21_interior_rain",
                inner,
                layout.is_free(inner, 0.3) && layout.line_of_sight(inner, mid + out_dir * 12.0),
            ),
        ] {
            let cam = Vec3::new(p.x, layout.surface_height(p) + eye, p.y);
            let look = mid + out_dir * 14.0;
            push(
                Shot {
                    clear,
                    pair: true,
                    ..base(
                        name.into(),
                        cam,
                        Vec3::new(look.x, layout.terrain(look) + 1.3, look.y),
                        70.0,
                    )
                },
                true,
            );
        }
    }

    // A few frames with him in them. The monster faces its camera.
    let ranch = d.landmark(LandmarkId::Ranch);
    let fields = d.landmark(LandmarkId::Fields);
    let shrine = d.landmark(LandmarkId::Shrine);
    let entry = d.landmark(LandmarkId::Entry);
    let staged = |name: &str,
                  want: Vec2,
                  cam_off: Vec2,
                  facing: Vec2,
                  look_h: f32,
                  fov: f32,
                  powered: bool,
                  push: &mut dyn FnMut(Shot, bool)| {
        let spot = site.stage(want, &[cam_off], eye, look_h, &[]);
        let m = spot.unwrap_or(want);
        push(
            Shot {
                powered,
                silbon: Some((m, facing)),
                clear: spot.is_some(),
                ..base(name.into(), site.ground(m + cam_off, eye), site.ground(m, look_h), fov)
            },
            true,
        );
    };
    let portrait = ranch.center + Vec2::new(-3.0, 11.0);
    staged(
        "40_silbon_portrait",
        portrait,
        Vec2::new(0.0, 7.0),
        Vec2::new(0.0, 1.0),
        1.7,
        44.0,
        false,
        &mut push,
    );
    let far = fields.center + Vec2::new(-14.0, 9.0);
    let far_cam = fields.approach + Vec2::new(-4.0, 6.0);
    staged(
        "41_silbon_distance",
        far,
        far_cam - far,
        Vec2::new(0.4, 0.9).normalize(),
        1.6,
        60.0,
        false,
        &mut push,
    );
    let behind = shrine.center + Vec2::new(6.5, 5.0);
    staged(
        "42_silbon_ceiba",
        behind,
        shrine.approach - behind,
        (shrine.approach - behind).normalize(),
        1.8,
        56.0,
        false,
        &mut push,
    );
    let gate = entry.center + Vec2::new(1.0, -4.0);
    staged(
        "43_silbon_gate",
        gate,
        entry.center + Vec2::new(0.0, 10.0) - gate,
        Vec2::new(0.0, 1.0),
        1.9,
        58.0,
        true,
        &mut push,
    );

    // Model review and teammates share one yard: front, side and back four
    // metres away in open ground, then two avatars beside where he stood.
    let front = Vec2::new(0.0, 4.6);
    let side = Vec2::new(4.6, 0.0);
    let back = Vec2::new(0.0, -4.6);
    let crowd = Vec2::new(0.0, 5.0);
    let mates = [Vec2::new(-1.6, -0.4), Vec2::new(1.6, -0.8)];
    let yard_spot = site.stage(Vec2::new(12.0, 9.0), &[front, side, back, crowd], eye, 1.55, &mates);
    let yard = yard_spot.unwrap_or(Vec2::new(12.0, 9.0));
    let clear = yard_spot.is_some();
    let facing = Vec2::new(0.0, 1.0);
    for (name, offset) in [
        ("44_silbon_front", front),
        ("45_silbon_side", side),
        ("46_silbon_back", back),
    ] {
        push(
            Shot {
                silbon: Some((yard, facing)),
                clear,
                ..base(
                    name.into(),
                    site.ground(yard + offset, eye),
                    site.ground(yard, 1.55),
                    52.0,
                )
            },
            true,
        );
    }
    // Teammates: two avatars, one carrying bones, one crouching with a torch.
    let party = vec![
        Companion {
            pos: yard + mates[0],
            yaw: std::f32::consts::PI,
            carrying: true,
            crouch: false,
        },
        Companion {
            pos: yard + mates[1],
            yaw: std::f32::consts::PI - 0.4,
            carrying: false,
            crouch: true,
        },
    ];
    push(
        Shot {
            party,
            clear,
            ..base(
                "47_avatars".into(),
                site.ground(yard + crowd, eye),
                site.ground(yard, 1.2),
                52.0,
            )
        },
        true,
    );

    // The real UI, over what a player would see. The camera stays at the
    // spawn pose (lowered to the downed height for the downed panel), as the
    // pose is in play; only the UI state is set by the driver.
    let yaw = layout.spawn_yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let spawn_eye = |lower: f32| {
        Vec3::new(
            layout.spawn.x,
            layout.surface_height(layout.spawn) + eye - lower,
            layout.spawn.y,
        )
    };
    let standing = spawn_eye(0.0);
    let two_mates = |a: Vec2, b: Vec2| {
        vec![
            Companion {
                pos: a,
                yaw: 0.0,
                carrying: true,
                crouch: false,
            },
            Companion {
                pos: b,
                yaw: 0.0,
                carrying: false,
                crouch: false,
            },
        ]
    };
    push(
        Shot {
            surface: Surface::Map,
            party: two_mates(
                ranch.approach + Vec2::new(1.5, 0.0),
                d.landmark(LandmarkId::Corral).approach,
            ),
            ..base("50_ui_map".into(), standing, standing + forward * 10.0, 68.0)
        },
        false,
    );
    // The table note, and two of the older papers' styles: the shelf radio's
    // dark card and a copla.
    for (id, name) in [(0u8, "51_ui_note"), (10, "56_ui_page_radio"), (13, "57_ui_page_copla")] {
        let Some(note) = d.notes.iter().find(|n| n.id == id) else {
            continue;
        };
        let view = site.note_view(note);
        let (cam, target) = view.unwrap_or((standing, standing + forward * 10.0));
        push(
            Shot {
                surface: Surface::Note(note.id),
                clear: view.is_some(),
                ..base(name.into(), cam, target, 68.0)
            },
            id == 0,
        );
    }
    let lowered = spawn_eye(tuning.downed_lower);
    push(
        Shot {
            surface: Surface::Downed,
            ..base("52_ui_downed".into(), lowered, lowered + forward * 10.0, 68.0)
        },
        false,
    );
    // Tureco, tied at his post behind the house, seen from the back door.
    let post = d.dog_post;
    let dog_ground = layout.surface_height(post);
    push(
        base(
            "55_tureco".into(),
            Vec3::new(post.x + 1.6, dog_ground + 1.3, post.y + 1.9),
            Vec3::new(post.x - 0.2, dog_ground + 0.45, post.y),
            55.0,
        ),
        false,
    );
    // The catch at four instants: the first flash far off, the third close,
    // on the ground under him, the grab just before the black.
    for (name, at) in [
        ("53_caught_lunge_first_flash", 0.03),
        ("54_caught_lunge_third_flash", 0.44),
        ("58_caught_lunge_over_you", 1.1),
        ("59_caught_lunge_grab", 1.72),
    ] {
        let eye = standing;
        push(
            Shot {
                surface: Surface::Lunge(at),
                ..base(name.into(), eye, eye + forward * 10.0, 68.0)
            },
            false,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A photo is only useful if the camera is not inside a tree, wall or
    /// off the map. Every search must have found a clear view on the real
    /// layout, so moving a landmark into a palm fails here, not in review.
    #[test]
    fn every_photo_camera_is_on_the_map_with_a_clear_view() {
        let layout = Layout::new();
        for s in shots(&layout, &Tuning::default()) {
            assert!(s.clear, "{}: no clear view was found", s.name);
            if s.pos.y < 20.0 {
                let p = Vec2::new(s.pos.x, s.pos.z);
                assert!(layout.bounds.contains(p), "{} is outside the map at {p:?}", s.name);
            }
        }
    }
}
