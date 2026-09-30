//! Authored layout of the district and the shared spatial queries.
//!
//! This is the single source of truth for WHERE things are. Collision, line of
//! sight, interaction reach, the Silbón's patrol graph, the debug route and
//! every visual builder (house boards, fences, ceiba roots, props) read the
//! same numbers from [`Layout`]. Nothing here depends on the ECS.
//!
//! Coordinates: world X east, world Z south, Y up. Ground-plane queries use
//! `Vec2(x, z)`. Movement is planar; the terrain is gently rolling and
//! authored decks, ramps and bridges lift the walk surface (`surface_height`).

use bevy::math::{Vec2, Vec3};

pub mod district;
use district::{District, Lamp, Route};

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
    Bank,
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

/// Uniform grid over the blockers so movement queries touch a handful of
/// shapes instead of the whole map.
#[derive(Clone, Debug, PartialEq, Default)]
struct BlockerGrid {
    origin: Vec2,
    w: usize,
    h: usize,
    cells: Vec<Vec<u32>>,
}

impl BlockerGrid {
    const CELL: f32 = 6.0;

    fn build(bounds: Rect2, blockers: &[Blocker]) -> Self {
        let origin = bounds.min - Vec2::splat(8.0);
        let size = bounds.max - bounds.min + Vec2::splat(16.0);
        let w = (size.x / Self::CELL).ceil() as usize + 1;
        let h = (size.y / Self::CELL).ceil() as usize + 1;
        let mut grid = Self {
            origin,
            w,
            h,
            cells: vec![Vec::new(); w * h],
        };
        for (i, b) in blockers.iter().enumerate() {
            let (min, max) = match b.shape {
                Shape::Rect(r) => (r.min, r.max),
                Shape::Circle { center, radius } => (center - Vec2::splat(radius), center + Vec2::splat(radius)),
            };
            let (x0, z0) = grid.cell_of(min);
            let (x1, z1) = grid.cell_of(max);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    grid.cells[z * w + x].push(i as u32);
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: Vec2) -> (usize, usize) {
        let x = ((p.x - self.origin.x) / Self::CELL)
            .floor()
            .clamp(0.0, (self.w - 1) as f32);
        let z = ((p.y - self.origin.y) / Self::CELL)
            .floor()
            .clamp(0.0, (self.h - 1) as f32);
        (x as usize, z as usize)
    }

    /// Indices of every blocker whose cells overlap the box (duplicates are harmless).
    fn near(&self, min: Vec2, max: Vec2) -> impl Iterator<Item = u32> + '_ {
        let (x0, z0) = self.cell_of(min);
        let (x1, z1) = self.cell_of(max);
        (z0..=z1).flat_map(move |z| (x0..=x1).flat_map(move |x| self.cells[z * self.w + x].iter().copied()))
    }
}

/// The Silbón's patrol graph. Nodes are route vertices and edges are the
/// straight route segments between them, so anything he can walk a player
/// can walk too: the patrol never invents a second network.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Patrol {
    pub nodes: Vec<Vec2>,
    pub edges: Vec<(usize, usize)>,
    adjacency: Vec<Vec<usize>>,
    next: Vec<Vec<u8>>,
    dist: Vec<Vec<f32>>,
    hops: Vec<Vec<u8>>,
}

impl Patrol {
    pub fn from_routes(routes: &[Route]) -> Self {
        let mut nodes: Vec<Vec2> = Vec::new();
        let mut edges: Vec<(usize, usize)> = Vec::new();
        let node = |nodes: &mut Vec<Vec2>, p: Vec2| -> usize {
            if let Some(i) = nodes.iter().position(|n| n.distance(p) < 0.75) {
                i
            } else {
                nodes.push(p);
                nodes.len() - 1
            }
        };
        for r in routes.iter().filter(|r| r.patrol) {
            let mut prev = None;
            for &pt in &r.points {
                let i = node(&mut nodes, pt);
                if let Some(j) = prev
                    && i != j
                    && !edges.contains(&(i.min(j), i.max(j)))
                {
                    edges.push((i.min(j), i.max(j)));
                }
                prev = Some(i);
            }
        }
        let n = nodes.len();
        assert!(n < 250, "patrol graph too large for u8 hops");
        let mut adjacency = vec![Vec::new(); n];
        let mut dist = vec![vec![f32::INFINITY; n]; n];
        let mut hops = vec![vec![0u8; n]; n];
        let mut next = vec![vec![u8::MAX; n]; n];
        for i in 0..n {
            dist[i][i] = 0.0;
            next[i][i] = i as u8;
        }
        for &(a, b) in &edges {
            adjacency[a].push(b);
            adjacency[b].push(a);
            let d = nodes[a].distance(nodes[b]);
            dist[a][b] = d;
            dist[b][a] = d;
            hops[a][b] = 1;
            hops[b][a] = 1;
            next[a][b] = b as u8;
            next[b][a] = a as u8;
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    let alt = dist[i][k] + dist[k][j];
                    if alt < dist[i][j] {
                        dist[i][j] = alt;
                        next[i][j] = next[i][k];
                        hops[i][j] = hops[i][k].saturating_add(hops[k][j]);
                    }
                }
            }
        }
        Self {
            nodes,
            edges,
            adjacency,
            next,
            dist,
            hops,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn neighbors(&self, i: usize) -> &[usize] {
        &self.adjacency[i]
    }

    /// Every node can reach every other along authored segments.
    pub fn connected(&self) -> bool {
        self.dist.iter().all(|row| row.iter().all(|d| d.is_finite()))
    }

    pub fn nearest(&self, p: Vec2) -> usize {
        (0..self.len())
            .min_by(|&a, &b| {
                self.nodes[a]
                    .distance_squared(p)
                    .total_cmp(&self.nodes[b].distance_squared(p))
            })
            .unwrap_or(0)
    }

    /// The node farthest from every one of `points` (the largest minimum
    /// distance), with that distance. He rises here, never near anyone.
    pub fn farthest_from(&self, points: &[Vec2]) -> (usize, f32) {
        let clearance = |i: usize| {
            points
                .iter()
                .map(|q| self.nodes[i].distance(*q))
                .fold(f32::INFINITY, f32::min)
        };
        let mut best = (0, -1.0);
        for i in 0..self.len() {
            let c = clearance(i);
            if c > best.1 {
                best = (i, c);
            }
        }
        best
    }

    /// Where he waits while he cannot see anyone: the closest node that is
    /// at least `standoff` from `p` (the farthest one if all are nearer), so
    /// waiting never puts him on top of the player.
    pub fn lurk_node(&self, p: Vec2, standoff: f32) -> usize {
        let mut best: Option<(usize, f32)> = None;
        for i in 0..self.len() {
            let d = self.nodes[i].distance(p);
            if d >= standoff && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map_or_else(|| self.farthest_from(&[p]).0, |(i, _)| i)
    }

    /// Neighbour of `from` on a shortest path to `to`.
    pub fn next_hop(&self, from: usize, to: usize) -> usize {
        match self.next[from][to] {
            u8::MAX => from,
            n => n as usize,
        }
    }

    /// The corners of a walk along the network from the node nearest
    /// `from` to the one nearest `to`, both included.
    pub fn walk(&self, from: Vec2, to: Vec2) -> Vec<Vec2> {
        let (mut at, goal) = (self.nearest(from), self.nearest(to));
        let mut out = vec![self.nodes[at]];
        while at != goal {
            let next = self.next_hop(at, goal);
            if next == at {
                break; // disconnected: stop rather than loop
            }
            at = next;
            out.push(self.nodes[at]);
        }
        out
    }

    pub fn hops(&self, a: usize, b: usize) -> usize {
        self.hops[a][b] as usize
    }

    /// Length of the shortest walk between two nodes.
    pub fn path_len(&self, a: usize, b: usize) -> f32 {
        self.dist[a][b]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub district: District,
    /// Hard walk limits (fences, water and trees make them visible).
    pub bounds: Rect2,
    /// Visible dirt road.
    pub road: Rect2,
    pub spawn: Vec2,
    pub spawn_yaw: f32,
    pub house: House,
    pub ceiba: Ceiba,
    pub table: Rect2,
    pub table_height: f32,
    pub lantern: Vec3,
    pub porch_lamp: Vec3,
    pub furniture: Vec<Furniture>,
    /// Hammock hooks (inside the house).
    pub hammock: (Vec3, Vec3),
    /// The Silbón's patrol graph, derived from the routes.
    pub patrol: Patrol,
    /// Every practical light, house lamps included.
    pub light_sources: Vec<Lamp>,
    /// Derived collision/sight blockers.
    pub blockers: Vec<Blocker>,
    grid: BlockerGrid,
    sight: Vec<u32>,
}

impl Layout {
    /// Build the layout around an authored district: the ranch house, the
    /// ceiba at the shrine, blockers and the patrol graph.
    pub(crate) fn assemble(district: District) -> Self {
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

        let tree_center = district.landmark(district::LandmarkId::Shrine).center;
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
        ];

        let mut layout = Self {
            bounds: Rect2::new(Vec2::new(-76.0, -108.0), Vec2::new(90.0, 35.2)),
            road: Rect2::new(Vec2::new(-160.0, 27.5), Vec2::new(160.0, 35.0)),
            spawn: Vec2::new(0.0, 31.2),
            spawn_yaw: 0.0,
            house,
            ceiba,
            table,
            table_height,
            lantern: Vec3::new(2.45, table_height, -4.5),
            porch_lamp: Vec3::new(-1.1, 2.2, 2.9),
            furniture,
            hammock: (Vec3::new(-4.86, 1.75, -1.0), Vec3::new(-1.9, 1.75, -1.0)),
            patrol: Patrol::from_routes(&district.routes),
            light_sources: Vec::new(),
            blockers: Vec::new(),
            grid: BlockerGrid::default(),
            sight: Vec::new(),
            district,
        };
        layout.blockers = layout.derive_blockers();
        let mut district_blockers = Vec::new();
        layout.district.blockers(&mut district_blockers);
        layout.blockers.extend(district_blockers);
        layout.light_sources = [layout.lantern, layout.porch_lamp]
            .into_iter()
            .map(|pos| Lamp {
                pos,
                radius: 6.0,
                powered: false,
                circuit: 0,
            })
            .chain(layout.district.lamps.iter().copied())
            .collect();
        layout.grid = BlockerGrid::build(layout.bounds, &layout.blockers);
        layout.sight = layout
            .blockers
            .iter()
            .enumerate()
            .filter(|(_, b)| b.sight == Sight::Blocks)
            .map(|(i, _)| i as u32)
            .collect();
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
        // (fences are district rails now)
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
            for i in self.grid.near(p - Vec2::splat(radius), p + Vec2::splat(radius)) {
                if let Some(push) = self.blockers[i as usize].shape.circle_push(p, radius) {
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
        self.grid
            .near(p - Vec2::splat(radius), p + Vec2::splat(radius))
            .all(|i| self.blockers[i as usize].shape.circle_push(p, radius).is_none())
    }

    /// Ground-plane line of sight: blocked only by sight-blocking shapes
    /// (solid walls, trunks, chunky props). Windows, doors, fences, cattle
    /// and low furniture do not hide anyone.
    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        !self
            .sight
            .iter()
            .any(|&i| self.blockers[i as usize].shape.hits_segment(a, b))
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

    /// Where a view ray meets the walkable ground (for marking a spot), or
    /// the ground `max` metres out if it never does.
    pub fn ray_ground(&self, eye: Vec3, dir: Vec3, max: f32) -> Vec3 {
        let dir = dir.normalize_or(Vec3::NEG_Z);
        let mut t = 0.4;
        while t <= max {
            let p = eye + dir * t;
            let g = self.surface_height(Vec2::new(p.x, p.z));
            if p.y <= g + 0.05 {
                return Vec3::new(p.x, g, p.z);
            }
            t += 0.4;
        }
        let flat = Vec2::new(dir.x, dir.z).normalize_or(Vec2::NEG_Y) * max.min(60.0);
        let p = (Vec2::new(eye.x, eye.z) + flat).clamp(self.bounds.min, self.bounds.max);
        Vec3::new(p.x, self.surface_height(p), p.y)
    }

    /// Is `p` inside the glow of a burning lamp? Kerosene lanterns always
    /// burn; a powered lamp only while its line is live (`circuits`, a bit per
    /// `Lamp::circuit`; 0 while the pump has not brought the power up).
    pub fn is_lit(&self, p: Vec2, circuits: u8) -> bool {
        self.light_sources
            .iter()
            .any(|l| (!l.powered || circuits & (1 << l.circuit) != 0) && ground(l.pos).distance(p) <= l.radius)
    }

    /// How deep in water `p` stands. Rect tests come first and the terrain
    /// is sampled only inside a channel, so this is cheap enough for every
    /// path-search cell and every frame's eye. Decks and piers are dry and
    /// authored shallows are shallow. A channel is as deep as the still water
    /// over its bed, so it is dry exactly where the drawn waterline ends; a
    /// ford across it is never more than shallow.
    pub fn wade(&self, p: Vec2) -> Wade {
        let d = &self.district;
        let (shallow, channel) = (d.shallow_at(p), d.channel_at(p));
        if !(shallow || channel) || d.surface_at(p).is_some() {
            return Wade::Dry;
        }
        if !channel {
            return Wade::Shallow;
        }
        let depth = district::WATER_LEVEL - self.terrain(p);
        if depth <= district::WADE_WET {
            Wade::Dry
        } else if shallow || depth <= district::WADE_DEEP {
            Wade::Shallow
        } else {
            Wade::Deep
        }
    }

    /// Walkable water: slow, splashing footsteps.
    pub fn wading(&self, p: Vec2) -> bool {
        self.wade(p) != Wade::Dry
    }

    /// The still water's surface over `p`, if `p` is waded.
    pub fn water_line(&self, p: Vec2) -> Option<f32> {
        self.wading(p).then_some(district::WATER_LEVEL)
    }

    /// Where something lying at `p` rests: the ground, or afloat just under
    /// the water where it is deeper than that. Downed bodies and dropped
    /// bundles lie here, so a friend can find them.
    pub fn rest_height(&self, p: Vec2) -> f32 {
        let ground = self.surface_height(p);
        self.water_line(p).map_or(ground, |w| ground.max(w - 0.2))
    }
}

/// How deep the water is where someone stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wade {
    #[default]
    Dry,
    /// Ankle to knee deep: a ford, a pond's margin, a channel's edge.
    Shallow,
    /// Waist-deep in a channel.
    Deep,
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
        Layout::new()
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
        // The property fence stops you except at the gates.
        let fence = l.move_circle(Vec2::new(10.0, 25.0), Vec2::new(0.0, -8.0), r);
        assert!(fence.y > 22.0);
        let gate = l.move_circle(Vec2::new(0.0, 25.0), Vec2::new(0.0, -8.0), r);
        assert!(gate.y < 18.0);
        let east_gate = l.move_circle(Vec2::new(55.0, 25.0), Vec2::new(0.0, -8.0), r);
        assert!(east_gate.y < 18.0);
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
        // A fence rail stops movement but hides no one.
        assert!(!l.is_free(Vec2::new(10.0, 22.0), 0.3));
        assert!(l.line_of_sight(Vec2::new(10.0, 23.0), Vec2::new(10.0, 21.0)));
        // The ceiba trunk does hide.
        let c = l.ceiba.center;
        assert!(!l.line_of_sight(c + Vec2::new(-8.0, 0.0), c + Vec2::new(8.0, 0.0)));
    }

    #[test]
    fn aiming_respects_reach_and_occlusion() {
        let l = layout();
        let t = l.district.relics[0];
        let radius = district::RELIC_RADIUS;
        let eye_near = Vec3::new(2.9, 1.62, -3.3);
        let dir = (t - eye_near).normalize();
        assert!(matches!(l.aim(eye_near, dir, t, radius, 2.4), AimStatus::Ready { .. }));
        // Looking away.
        assert_eq!(l.aim(eye_near, -dir, t, radius, 2.4), AimStatus::NotAimed);
        // Clearly visible inside the room, but too far to reach.
        let eye_far = Vec3::new(-2.0, 1.62, -1.5);
        let dir_far = (t - eye_far).normalize();
        let far = l.aim(eye_far, dir_far, t, radius, 2.4);
        assert!(matches!(far, AimStatus::OutOfReach { .. }), "{far:?}");
        // Behind the east wall (outside, south of the east window).
        let eye_wall = Vec3::new(5.6, 1.62, -5.2);
        let dir_wall = (t - eye_wall).normalize();
        assert_eq!(l.aim(eye_wall, dir_wall, t, radius, 9.0), AimStatus::Occluded);
    }

    #[test]
    fn every_authored_standing_spot_is_reachable_space() {
        let l = layout();
        assert!(l.is_free(l.spawn, 0.3));
        assert!(l.patrol.connected(), "patrol graph must be one connected network");
        for node in &l.patrol.nodes {
            assert!(l.is_free(*node, 0.45), "patrol node inside a blocker: {node:?}");
        }
    }

    #[test]
    fn lurking_keeps_a_standoff_and_manifestation_keeps_clearance() {
        let l = layout();
        for spot in [
            l.spawn,
            Vec2::new(0.0, -2.5),
            Vec2::new(35.0, -12.0),
            Vec2::new(-46.0, -25.0),
        ] {
            let i = l.patrol.lurk_node(spot, 12.0);
            assert!(l.patrol.nodes[i].distance(spot) >= 12.0 - 1e-3);
            let (far, d) = l.patrol.farthest_from(&[spot]);
            assert!(d >= 30.0, "no manifestation point 30 m from {spot:?}: best {d}");
            assert!(l.patrol.nodes[far].distance(spot) >= 30.0);
        }
    }
}
