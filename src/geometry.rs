//! Authored layout of the encounter and the shared spatial queries.
//!
//! This is the single source of truth for WHERE things are. Collision, line of
//! sight, interaction reach, the threat's anchor route, the debug route and
//! every visual builder (house boards, fence posts, ceiba roots, props) read
//! the same numbers from [`Layout`]. Nothing here depends on the ECS.
//!
//! Coordinates: world X east, world Z south, Y up. Ground-plane queries use
//! `Vec2(x, z)`. The ground is flat (y = 0) everywhere the player can walk.

use bevy::math::{Vec2, Vec3};

/// Longest single sub-step of a collision move. Far below the thinnest
/// blocker half-thickness plus the player radius, so nothing can tunnel.
const MAX_SUBSTEP: f32 = 0.05;

// ----------------------------------------------------------------------------
// Primitive shapes
// ----------------------------------------------------------------------------

/// Axis-aligned rectangle on the ground plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect2 {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    pub fn from_center(center: Vec2, half: Vec2) -> Self {
        Self::new(center - half, center + half)
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn half(&self) -> Vec2 {
        (self.max - self.min) * 0.5
    }

    pub fn contains(&self, p: Vec2) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
    }

    /// Push needed to move a circle out of this rectangle, if overlapping.
    fn circle_push(&self, p: Vec2, r: f32) -> Option<Vec2> {
        let q = p.clamp(self.min, self.max);
        let d = p - q;
        let dist_sq = d.length_squared();
        if dist_sq > 1e-12 {
            if dist_sq >= r * r {
                return None;
            }
            let dist = dist_sq.sqrt();
            return Some(d / dist * (r - dist));
        }
        // Centre inside: leave through the nearest side.
        let left = p.x - self.min.x;
        let right = self.max.x - p.x;
        let down = p.y - self.min.y;
        let up = self.max.y - p.y;
        let m = left.min(right).min(down).min(up);
        Some(if m == left {
            Vec2::new(-(left + r), 0.0)
        } else if m == right {
            Vec2::new(right + r, 0.0)
        } else if m == down {
            Vec2::new(0.0, -(down + r))
        } else {
            Vec2::new(0.0, up + r)
        })
    }

    /// Does the segment a→b touch this rectangle? (slab test)
    fn hits_segment(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        let mut t0 = 0.0_f32;
        let mut t1 = 1.0_f32;
        for axis in 0..2 {
            let (o, dir, lo, hi) = if axis == 0 {
                (a.x, d.x, self.min.x, self.max.x)
            } else {
                (a.y, d.y, self.min.y, self.max.y)
            };
            if dir.abs() < 1e-9 {
                if !(lo..=hi).contains(&o) {
                    return false;
                }
            } else {
                let inv = 1.0 / dir;
                let mut ta = (lo - o) * inv;
                let mut tb = (hi - o) * inv;
                if ta > tb {
                    std::mem::swap(&mut ta, &mut tb);
                }
                t0 = t0.max(ta);
                t1 = t1.min(tb);
                if t0 > t1 {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Rect(Rect2),
    Circle { center: Vec2, radius: f32 },
}

impl Shape {
    fn circle_push(&self, p: Vec2, r: f32) -> Option<Vec2> {
        match *self {
            Shape::Rect(rect) => rect.circle_push(p, r),
            Shape::Circle { center, radius } => {
                let d = p - center;
                let min = r + radius;
                let dist_sq = d.length_squared();
                if dist_sq >= min * min {
                    return None;
                }
                let dist = dist_sq.sqrt();
                let dir = if dist > 1e-6 { d / dist } else { Vec2::X };
                Some(dir * (min - dist))
            }
        }
    }

    fn hits_segment(&self, a: Vec2, b: Vec2) -> bool {
        match *self {
            Shape::Rect(rect) => rect.hits_segment(a, b),
            Shape::Circle { center, radius } => segment_point_distance(a, b, center) < radius,
        }
    }
}

/// Distance from point `p` to segment a→b.
pub fn segment_point_distance(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 1e-12 {
        ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// Whether a blocker hides what is behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sight {
    /// Blocks movement AND line of sight (solid wall, ceiba trunk).
    Blocks,
    /// Blocks movement only (fence wire, window sill, furniture, roots).
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockerKind {
    Wall,
    WindowSill,
    Fence,
    Furniture,
    Post,
    Trunk,
    Root,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blocker {
    pub shape: Shape,
    pub sight: Sight,
    pub kind: BlockerKind,
}

// ----------------------------------------------------------------------------
// Authored pieces
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningKind {
    Door,
    Window,
}

/// A hole in a wall, measured along the wall from its start point `a`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
    pub kind: OpeningKind,
    pub from: f32,
    pub to: f32,
    pub bottom: f32,
    pub top: f32,
}

/// Which way a wall's outside faces (used by visuals for trims and shutters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    North,
    South,
    East,
    West,
}

impl Facing {
    /// Outward normal on the ground plane.
    pub fn normal(self) -> Vec2 {
        match self {
            Facing::North => Vec2::new(0.0, -1.0),
            Facing::South => Vec2::new(0.0, 1.0),
            Facing::East => Vec2::new(1.0, 0.0),
            Facing::West => Vec2::new(-1.0, 0.0),
        }
    }
}

/// Axis-aligned wall centred on the segment a→b.
#[derive(Clone, Debug, PartialEq)]
pub struct Wall {
    pub a: Vec2,
    pub b: Vec2,
    pub facing: Facing,
    pub openings: Vec<Opening>,
}

impl Wall {
    pub fn length(&self) -> f32 {
        self.a.distance(self.b)
    }

    pub fn dir(&self) -> Vec2 {
        (self.b - self.a).normalize_or_zero()
    }

    /// Point on the wall centre line `s` metres from `a`.
    pub fn at(&self, s: f32) -> Vec2 {
        self.a + self.dir() * s
    }

    pub fn opening_at(&self, s: f32) -> Option<&Opening> {
        self.openings.iter().find(|o| (o.from..=o.to).contains(&s))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct House {
    /// Outer wall footprint (wall centre lines).
    pub footprint: Rect2,
    pub wall_thickness: f32,
    /// Height of the eaves (north/south walls).
    pub eave: f32,
    /// Height of the ridge, which runs east–west through the footprint centre.
    pub ridge: f32,
    /// Roof overhang beyond the walls.
    pub overhang: f32,
    /// Front porch (corredor) floor area under the lean-to roof.
    pub porch: Rect2,
    /// Height of the porch roof at its outer edge.
    pub porch_low: f32,
    /// Porch roof post positions.
    pub porch_posts: Vec<Vec2>,
    /// Interior post that also anchors the hammock.
    pub inner_post: Vec2,
    pub walls: Vec<Wall>,
}

impl House {
    /// Roof height above a point of the footprint (for gable boards).
    pub fn roof_height_at(&self, z: f32) -> f32 {
        let c = self.footprint.center().y;
        let half = self.footprint.half().y;
        let u = ((z - c).abs() / half).clamp(0.0, 1.0);
        self.ridge + (self.eave - self.ridge) * u
    }
}

/// A straight fence run with gaps (gates), axis-aligned.
#[derive(Clone, Debug, PartialEq)]
pub struct FenceRun {
    pub a: Vec2,
    pub b: Vec2,
    /// Gaps as (from, to) distances along the run.
    pub gaps: Vec<(f32, f32)>,
}

impl FenceRun {
    pub fn length(&self) -> f32 {
        self.a.distance(self.b)
    }

    pub fn at(&self, s: f32) -> Vec2 {
        self.a + (self.b - self.a).normalize_or_zero() * s
    }

    pub fn in_gap(&self, s: f32) -> bool {
        self.gaps.iter().any(|&(f, t)| s > f && s < t)
    }

    /// Solid spans between gaps.
    pub fn spans(&self) -> Vec<(f32, f32)> {
        let mut spans = Vec::new();
        let mut start = 0.0;
        let mut gaps = self.gaps.clone();
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (f, t) in gaps {
            if f > start {
                spans.push((start, f));
            }
            start = t;
        }
        if start < self.length() {
            spans.push((start, self.length()));
        }
        spans
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ceiba {
    pub center: Vec2,
    /// Radius of the trunk proper at chest height (visual and sight blocker).
    pub trunk_radius: f32,
    /// Movement radius around the trunk and buttress roots.
    pub collide_radius: f32,
    /// Angles (radians, ground plane, 0 = +X) of the buttress roots.
    pub root_angles: Vec<f32>,
    /// How far the buttress roots reach from the centre.
    pub root_reach: f32,
    pub height: f32,
    pub canopy_radius: f32,
    /// Where the bones are returned: the hollow between two roots.
    pub offering: Vec3,
    pub offering_radius: f32,
}

/// Furniture and small solids that block movement but not sight.
#[derive(Clone, Debug, PartialEq)]
pub struct Furniture {
    pub name: &'static str,
    pub shape: Shape,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Hard walk limits (the road runs past, fog hides the ends).
    pub bounds: Rect2,
    /// Visible dirt road.
    pub road: Rect2,
    /// Standing at z ≥ this (on the road shoulder) with the bones returned wins.
    pub road_goal_z: f32,
    pub spawn: Vec2,
    pub spawn_yaw: f32,
    pub house: House,
    pub fences: Vec<FenceRun>,
    pub fence_height: f32,
    pub ceiba: Ceiba,
    pub table: Rect2,
    pub table_height: f32,
    pub satchel: Vec3,
    pub satchel_radius: f32,
    pub note: Vec3,
    pub note_radius: f32,
    pub lantern: Vec3,
    pub porch_lamp: Vec3,
    pub furniture: Vec<Furniture>,
    /// Hammock hooks (inside the house).
    pub hammock: (Vec3, Vec3),
    /// Sign by the gate.
    pub sign: Vec2,
    /// The Silbón's authored route: a closed loop of anchors.
    pub ring: Vec<Vec2>,
    /// Worn mud trails (visual; also where the debug route walks).
    pub trails: Vec<Vec<Vec2>>,
    /// Derived collision/sight blockers.
    pub blockers: Vec<Blocker>,
}

impl Default for Layout {
    fn default() -> Self {
        Self::authored()
    }
}

impl Layout {
    /// The one authored encounter space.
    pub fn authored() -> Self {
        let wall_thickness = 0.14;
        let footprint = Rect2::new(Vec2::new(-5.0, -6.0), Vec2::new(5.0, 1.0));
        let win = |from: f32, to: f32| Opening {
            kind: OpeningKind::Window,
            from,
            to,
            bottom: 0.95,
            top: 1.9,
        };
        let door = |from: f32, to: f32| Opening {
            kind: OpeningKind::Door,
            from,
            to,
            bottom: 0.0,
            top: 2.1,
        };
        let walls = vec![
            // Front (south, facing the road): two windows and the main door.
            Wall {
                a: Vec2::new(-5.0, 1.0),
                b: Vec2::new(5.0, 1.0),
                facing: Facing::South,
                openings: vec![win(1.4, 2.6), door(4.4, 5.6), win(7.4, 8.6)],
            },
            // Back (north, toward the ceiba): back door and one window.
            Wall {
                a: Vec2::new(-5.0, -6.0),
                b: Vec2::new(5.0, -6.0),
                facing: Facing::North,
                openings: vec![door(1.5, 2.5), win(6.6, 7.6)],
            },
            Wall {
                a: Vec2::new(-5.0, -6.0),
                b: Vec2::new(-5.0, 1.0),
                facing: Facing::West,
                openings: vec![win(2.2, 3.4)],
            },
            Wall {
                a: Vec2::new(5.0, -6.0),
                b: Vec2::new(5.0, 1.0),
                facing: Facing::East,
                openings: vec![win(2.2, 3.4)],
            },
        ];
        let house = House {
            footprint,
            wall_thickness,
            eave: 2.55,
            ridge: 3.85,
            overhang: 0.45,
            porch: Rect2::new(Vec2::new(-5.4, 1.0), Vec2::new(5.4, 3.3)),
            porch_low: 2.25,
            porch_posts: vec![
                Vec2::new(-5.1, 3.1),
                Vec2::new(-1.7, 3.1),
                Vec2::new(1.7, 3.1),
                Vec2::new(5.1, 3.1),
            ],
            inner_post: Vec2::new(-1.8, -1.0),
            walls,
        };

        let tree_center = Vec2::new(-19.0, -24.0);
        let to_house = (footprint.center() - tree_center).normalize();
        let offering_angle = to_house.y.atan2(to_house.x);
        // Buttress roots fan around the trunk, leaving a hollow facing the house.
        let root_offsets = [0.42_f32, 1.25, 2.05, 2.9, 3.7, 4.55, 5.5];
        let root_angles = root_offsets.iter().map(|o| offering_angle + o).collect();
        let offering_ground = tree_center + to_house * 2.5;
        let ceiba = Ceiba {
            center: tree_center,
            trunk_radius: 1.8,
            collide_radius: 2.7,
            root_angles,
            root_reach: 3.6,
            height: 25.0,
            canopy_radius: 17.0,
            offering: Vec3::new(offering_ground.x, 0.3, offering_ground.y),
            offering_radius: 0.7,
        };

        let table = Rect2::new(Vec2::new(2.2, -5.0), Vec2::new(3.8, -4.1));
        let table_height = 0.78;
        let furniture = vec![
            Furniture {
                name: "table",
                shape: Shape::Rect(table),
                height: table_height,
            },
            Furniture {
                name: "chair",
                shape: Shape::Rect(Rect2::new(Vec2::new(4.05, -4.75), Vec2::new(4.45, -4.35))),
                height: 0.95,
            },
            Furniture {
                name: "shelf",
                shape: Shape::Rect(Rect2::new(Vec2::new(-4.93, -5.8), Vec2::new(-4.55, -4.2))),
                height: 1.8,
            },
            Furniture {
                name: "hammock",
                shape: Shape::Rect(Rect2::new(Vec2::new(-4.8, -1.3), Vec2::new(-2.1, -0.7))),
                height: 0.9,
            },
            Furniture {
                name: "pot_front",
                shape: Shape::Circle {
                    center: Vec2::new(4.45, 0.45),
                    radius: 0.28,
                },
                height: 0.6,
            },
            Furniture {
                name: "pot_back",
                shape: Shape::Circle {
                    center: Vec2::new(4.4, -5.5),
                    radius: 0.25,
                },
                height: 0.55,
            },
            Furniture {
                name: "bench",
                shape: Shape::Rect(Rect2::new(Vec2::new(-4.6, 2.55), Vec2::new(-2.8, 2.95))),
                height: 0.45,
            },
            Furniture {
                name: "firewood",
                shape: Shape::Rect(Rect2::new(Vec2::new(3.4, 1.35), Vec2::new(4.9, 1.95))),
                height: 0.7,
            },
            Furniture {
                name: "barrel",
                shape: Shape::Circle {
                    center: Vec2::new(5.65, -1.0),
                    radius: 0.36,
                },
                height: 0.95,
            },
            Furniture {
                name: "sign",
                shape: Shape::Rect(Rect2::new(Vec2::new(3.0, 23.6), Vec2::new(5.4, 23.9))),
                height: 1.9,
            },
            // Termite mounds out in the paddock.
            Furniture {
                name: "mound",
                shape: Shape::Circle {
                    center: Vec2::new(14.0, -22.0),
                    radius: 0.8,
                },
                height: 1.3,
            },
            Furniture {
                name: "mound",
                shape: Shape::Circle {
                    center: Vec2::new(-26.0, -8.0),
                    radius: 0.7,
                },
                height: 1.0,
            },
            Furniture {
                name: "mound",
                shape: Shape::Circle {
                    center: Vec2::new(26.0, 10.0),
                    radius: 0.9,
                },
                height: 1.5,
            },
            Furniture {
                name: "mound",
                shape: Shape::Circle {
                    center: Vec2::new(-8.0, -38.0),
                    radius: 0.6,
                },
                height: 0.9,
            },
        ];

        let fences = vec![
            // Road side, with the gate.
            FenceRun {
                a: Vec2::new(-40.0, 22.0),
                b: Vec2::new(40.0, 22.0),
                gaps: vec![(38.4, 41.6)],
            },
            FenceRun {
                a: Vec2::new(-40.0, -56.0),
                b: Vec2::new(-40.0, 22.0),
                gaps: vec![],
            },
            FenceRun {
                a: Vec2::new(40.0, -56.0),
                b: Vec2::new(40.0, 22.0),
                gaps: vec![],
            },
            FenceRun {
                a: Vec2::new(-40.0, -56.0),
                b: Vec2::new(40.0, -56.0),
                gaps: vec![],
            },
        ];

        let ring = vec![
            Vec2::new(28.0, -44.0),
            Vec2::new(2.0, -48.0),
            Vec2::new(-24.0, -48.0),
            Vec2::new(-34.0, -30.0),
            Vec2::new(-34.0, -4.0),
            Vec2::new(-24.0, 14.0),
            Vec2::new(14.0, 14.0),
            Vec2::new(30.0, -8.0),
        ];

        let trails = vec![
            vec![
                Vec2::new(0.0, 29.0),
                Vec2::new(0.2, 22.0),
                Vec2::new(-0.3, 14.0),
                Vec2::new(0.4, 7.0),
                Vec2::new(0.0, 3.3),
            ],
            vec![
                Vec2::new(-3.0, -6.2),
                Vec2::new(-5.2, -9.5),
                Vec2::new(-9.5, -14.0),
                Vec2::new(-13.5, -18.2),
                Vec2::new(
                    offering_ground.x + to_house.x * 1.3,
                    offering_ground.y + to_house.y * 1.3,
                ),
            ],
            vec![
                Vec2::new(5.4, 2.0),
                Vec2::new(7.0, -2.0),
                Vec2::new(6.4, -7.0),
                Vec2::new(0.0, -8.4),
                Vec2::new(-3.0, -6.2),
            ],
        ];

        let mut layout = Self {
            bounds: Rect2::new(Vec2::new(-44.0, -60.0), Vec2::new(44.0, 35.2)),
            road: Rect2::new(Vec2::new(-160.0, 27.5), Vec2::new(160.0, 35.0)),
            road_goal_z: 27.0,
            spawn: Vec2::new(0.0, 31.2),
            spawn_yaw: 0.0,
            house,
            fences,
            fence_height: 1.35,
            ceiba,
            table,
            table_height,
            satchel: Vec3::new(3.25, table_height + 0.15, -4.6),
            satchel_radius: 0.32,
            note: Vec3::new(2.75, table_height + 0.01, -4.3),
            note_radius: 0.22,
            lantern: Vec3::new(2.45, table_height, -4.5),
            porch_lamp: Vec3::new(-1.1, 2.2, 2.9),
            furniture,
            hammock: (Vec3::new(-4.86, 1.75, -1.0), Vec3::new(-1.9, 1.75, -1.0)),
            sign: Vec2::new(4.2, 23.75),
            ring,
            trails,
            blockers: Vec::new(),
        };
        layout.blockers = layout.derive_blockers();
        layout
    }

    fn derive_blockers(&self) -> Vec<Blocker> {
        let mut out = Vec::new();
        let half_t = self.house.wall_thickness * 0.5;
        for wall in &self.house.walls {
            let dir = wall.dir();
            let len = wall.length();
            let mut cuts: Vec<(f32, f32, Option<OpeningKind>)> = Vec::new();
            let mut openings = wall.openings.clone();
            openings.sort_by(|a, b| a.from.total_cmp(&b.from));
            let mut s = 0.0;
            for o in &openings {
                if o.from > s {
                    cuts.push((s, o.from, None));
                }
                cuts.push((o.from, o.to, Some(o.kind)));
                s = o.to;
            }
            if s < len {
                cuts.push((s, len, None));
            }
            for (from, to, kind) in cuts {
                // Solid pieces overlap the corners by half a thickness so
                // corners are closed.
                let p0 = wall.a + dir * from - dir * if from == 0.0 { half_t } else { 0.0 };
                let p1 = wall.a + dir * to + dir * if to == len { half_t } else { 0.0 };
                let rect = Rect2::new(
                    p0.min(p1) - Vec2::splat(half_t) * perp_abs(dir),
                    p0.max(p1) + Vec2::splat(half_t) * perp_abs(dir),
                );
                match kind {
                    None => out.push(Blocker {
                        shape: Shape::Rect(rect),
                        sight: Sight::Blocks,
                        kind: BlockerKind::Wall,
                    }),
                    Some(OpeningKind::Window) => out.push(Blocker {
                        shape: Shape::Rect(rect),
                        sight: Sight::Clear,
                        kind: BlockerKind::WindowSill,
                    }),
                    Some(OpeningKind::Door) => {}
                }
            }
        }
        for &post in self
            .house
            .porch_posts
            .iter()
            .chain(std::iter::once(&self.house.inner_post))
        {
            out.push(Blocker {
                shape: Shape::Circle {
                    center: post,
                    radius: 0.12,
                },
                sight: Sight::Clear,
                kind: BlockerKind::Post,
            });
        }
        for f in &self.furniture {
            out.push(Blocker {
                shape: f.shape,
                sight: Sight::Clear,
                kind: BlockerKind::Furniture,
            });
        }
        for run in &self.fences {
            let dir = (run.b - run.a).normalize_or_zero();
            for (from, to) in run.spans() {
                let p0 = run.at(from);
                let p1 = run.at(to);
                let pad = Vec2::splat(0.08) * perp_abs(dir);
                out.push(Blocker {
                    shape: Shape::Rect(Rect2::new(p0.min(p1) - pad, p0.max(p1) + pad)),
                    sight: Sight::Clear,
                    kind: BlockerKind::Fence,
                });
            }
        }
        let c = &self.ceiba;
        out.push(Blocker {
            shape: Shape::Circle {
                center: c.center,
                radius: c.collide_radius,
            },
            sight: Sight::Clear,
            kind: BlockerKind::Root,
        });
        out.push(Blocker {
            shape: Shape::Circle {
                center: c.center,
                radius: c.trunk_radius,
            },
            sight: Sight::Blocks,
            kind: BlockerKind::Trunk,
        });
        for &a in &c.root_angles {
            let d = Vec2::new(a.cos(), a.sin());
            out.push(Blocker {
                shape: Shape::Circle {
                    center: c.center + d * (c.root_reach - 0.6),
                    radius: 0.35,
                },
                sight: Sight::Clear,
                kind: BlockerKind::Root,
            });
        }
        out
    }

    // ------------------------------------------------------------ queries

    /// Move a circle by `delta`, sliding along blockers, never tunnelling.
    pub fn move_circle(&self, from: Vec2, delta: Vec2, radius: f32) -> Vec2 {
        let len = delta.length();
        let steps = ((len / MAX_SUBSTEP).ceil() as usize).clamp(1, 256);
        let step = delta / steps as f32;
        let mut p = from;
        for _ in 0..steps {
            p = self.resolve(p + step, radius);
        }
        p
    }

    /// Push a circle out of every blocker and back inside the bounds.
    pub fn resolve(&self, mut p: Vec2, radius: f32) -> Vec2 {
        for _ in 0..4 {
            let mut moved = false;
            for b in &self.blockers {
                if let Some(push) = b.shape.circle_push(p, radius) {
                    p += push;
                    moved = true;
                }
            }
            p = p.clamp(
                self.bounds.min + Vec2::splat(radius),
                self.bounds.max - Vec2::splat(radius),
            );
            if !moved {
                break;
            }
        }
        p
    }

    /// True if a circle of `radius` at `p` overlaps no blocker.
    pub fn is_free(&self, p: Vec2, radius: f32) -> bool {
        self.blockers.iter().all(|b| b.shape.circle_push(p, radius).is_none())
    }

    /// Ground-plane line of sight: blocked only by sight-blocking shapes
    /// (solid wall pieces and the ceiba trunk). Windows, doors, fences and
    /// furniture do not hide anyone.
    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        !self
            .blockers
            .iter()
            .any(|bl| bl.sight == Sight::Blocks && bl.shape.hits_segment(a, b))
    }

    /// Checks an interaction ray from the eye against a spherical target.
    pub fn aim(&self, eye: Vec3, dir: Vec3, target: Vec3, radius: f32, reach: f32) -> AimStatus {
        if ray_sphere(eye, dir, target, radius).is_none() {
            return AimStatus::NotAimed;
        }
        if !self.line_of_sight(ground(eye), ground(target)) {
            return AimStatus::Occluded;
        }
        let distance = eye.distance(target);
        if distance > reach {
            AimStatus::OutOfReach { distance }
        } else {
            AimStatus::Ready { distance }
        }
    }

    pub fn in_road_goal(&self, p: Vec2) -> bool {
        p.y >= self.road_goal_z
    }

    pub fn nearest_anchor(&self, p: Vec2) -> usize {
        let mut best = 0;
        let mut best_d = f32::MAX;
        for (i, a) in self.ring.iter().enumerate() {
            let d = a.distance_squared(p);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    }

    /// The anchor farthest from `p`, with its distance.
    pub fn farthest_anchor(&self, p: Vec2) -> (usize, f32) {
        let mut best = 0;
        let mut best_d = -1.0;
        for (i, a) in self.ring.iter().enumerate() {
            let d = a.distance(p);
            if d > best_d {
                best_d = d;
                best = i;
            }
        }
        (best, best_d)
    }

    /// Next anchor index when walking from `from` toward `to` the short way.
    pub fn ring_step_toward(&self, from: usize, to: usize) -> usize {
        let n = self.ring.len();
        if from == to {
            return from;
        }
        let forward = (to + n - from) % n;
        if forward <= n / 2 {
            (from + 1) % n
        } else {
            (from + n - 1) % n
        }
    }

    /// Number of ring hops between two anchors the short way.
    pub fn ring_hops(&self, a: usize, b: usize) -> usize {
        let n = self.ring.len();
        let d = (a + n - b) % n;
        d.min(n - d)
    }
}

/// Unit vector perpendicular to an axis-aligned direction, as absolute values.
fn perp_abs(dir: Vec2) -> Vec2 {
    Vec2::new(dir.y.abs(), dir.x.abs())
}

pub fn ground(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// Ray/sphere intersection distance along a normalized `dir`.
pub fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    let t = -b - s;
    if t >= 0.0 {
        Some(t)
    } else if -b + s >= 0.0 {
        Some(0.0)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AimStatus {
    /// The crosshair is not on the target.
    NotAimed,
    /// On target, but a solid wall is between.
    Occluded,
    /// On target and visible, too far to reach.
    OutOfReach { distance: f32 },
    /// On target, visible and within reach.
    Ready { distance: f32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::authored()
    }

    #[test]
    fn walls_stop_the_player_but_the_doorway_lets_them_through() {
        let l = layout();
        let r = 0.3;
        // Walking north into the solid front wall right of the left window.
        let start = Vec2::new(-1.2, 2.5);
        let end = l.move_circle(start, Vec2::new(0.0, -6.0), r);
        assert!(end.y > 1.0, "passed through the front wall: {end:?}");
        // A fast single step must not tunnel either.
        let end_fast = l.move_circle(start, Vec2::new(0.0, -40.0), r);
        assert!(end_fast.y > 1.0, "tunnelled through the wall: {end_fast:?}");
        // The same walk through the doorway ends inside.
        let through = l.move_circle(Vec2::new(0.0, 3.0), Vec2::new(0.0, -5.0), r);
        assert!(through.y < -1.5, "doorway blocked: {through:?}");
        // Window sills block movement too.
        let sill = l.move_circle(Vec2::new(3.0, 3.0), Vec2::new(0.0, -5.0), r);
        assert!(sill.y > 1.0, "climbed through a window: {sill:?}");
        // The fence stops you except at the gate.
        let fence = l.move_circle(Vec2::new(10.0, 25.0), Vec2::new(0.0, -8.0), r);
        assert!(fence.y > 22.0);
        let gate = l.move_circle(Vec2::new(0.0, 25.0), Vec2::new(0.0, -8.0), r);
        assert!(gate.y < 18.0);
    }

    #[test]
    fn solid_walls_and_trunk_block_sight_windows_and_fences_do_not() {
        let l = layout();
        let inside = Vec2::new(-3.0, -4.0);
        // Straight through the solid part of the front wall.
        assert!(!l.line_of_sight(Vec2::new(-1.6, 12.0), Vec2::new(-1.6, -3.0)));
        // Through the front window (x in [-3.6, -2.4]).
        assert!(l.line_of_sight(Vec2::new(-3.0, 12.0), inside));
        // Through the open doorway.
        assert!(l.line_of_sight(Vec2::new(0.0, 12.0), Vec2::new(0.0, -3.0)));
        // Fences never hide anyone.
        assert!(l.line_of_sight(Vec2::new(10.0, 30.0), Vec2::new(10.0, 10.0)));
        // The ceiba trunk does.
        let c = l.ceiba.center;
        assert!(!l.line_of_sight(c + Vec2::new(-8.0, 0.0), c + Vec2::new(8.0, 0.0)));
    }

    #[test]
    fn aiming_respects_reach_and_occlusion() {
        let l = layout();
        let t = l.satchel;
        let eye_near = Vec3::new(2.9, 1.62, -3.3);
        let dir = (t - eye_near).normalize();
        assert!(matches!(
            l.aim(eye_near, dir, t, l.satchel_radius, 2.4),
            AimStatus::Ready { .. }
        ));
        // Looking away.
        assert_eq!(l.aim(eye_near, -dir, t, l.satchel_radius, 2.4), AimStatus::NotAimed);
        // Clearly visible inside the room, but too far to reach.
        let eye_far = Vec3::new(-2.0, 1.62, -1.5);
        let dir_far = (t - eye_far).normalize();
        let far = l.aim(eye_far, dir_far, t, l.satchel_radius, 2.4);
        assert!(matches!(far, AimStatus::OutOfReach { .. }), "{far:?}");
        // Behind the east wall (outside, south of the east window).
        let eye_wall = Vec3::new(5.6, 1.62, -5.2);
        let dir_wall = (t - eye_wall).normalize();
        assert_eq!(l.aim(eye_wall, dir_wall, t, l.satchel_radius, 9.0), AimStatus::Occluded);
    }

    #[test]
    fn every_authored_standing_spot_is_reachable_space() {
        let l = layout();
        assert!(l.is_free(l.spawn, 0.3));
        for anchor in &l.ring {
            assert!(l.is_free(*anchor, 0.5), "anchor inside a blocker: {anchor:?}");
        }
    }
}
