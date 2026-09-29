//! Fixed Llanos district ("Hacienda Santa Rosa"): the one authored map.
//!
//! Every gameplay-relevant placement lives here: landmarks, structures,
//! fences, solid props, water, walkable surfaces, terrain pads, the route
//! network (which is also the Silbón's patrol graph), interaction sites,
//! lamps and vegetation footprints. Presentation may vary colour, boards and
//! grass blades, never footprints or walk surfaces.
use super::{Blocker, BlockerKind, Layout, Rect2, Shape, Sight, segment_point_distance};
use crate::noise::fbm2;
use crate::rng::Rng;
use bevy::math::{Vec2, Vec3};

/// Still-water level the flooded marsh posts are measured from.
pub const MARSH_WATER: f32 = -0.13;

/// The altar table in the ceiba's hollow: top above the ground, its centre
/// out from the offering point, and half extents (across, outward).
pub const ALTAR_TABLE_TOP: f32 = 0.465;
pub const ALTAR_TABLE_OUT: f32 = 0.12;
pub const ALTAR_TABLE_HALF: Vec2 = Vec2::new(0.95, 0.3);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LandmarkId {
    Entry,
    Ranch,
    WaterTower,
    Corral,
    Shrine,
    Fields,
    Cano,
    Watchtower,
    Marsh,
    Extraction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Landmark {
    pub id: LandmarkId,
    pub name: &'static str,
    pub shot: &'static str,
    pub center: Vec2,
    pub approach: Vec2,
    pub look: Vec3,
    /// Radius kept free of vegetation.
    pub clearing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub name: &'static str,
    pub points: Vec<Vec2>,
    pub width: f32,
    /// Worn mud is painted along the route.
    pub trail: bool,
    /// Its vertices and segments are part of the Silbón's patrol graph.
    pub patrol: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShedStyle {
    Casita,
    Deposito,
    Cocina,
    PumpHouse,
    Shelter,
    LeanTo,
    StiltHut,
    BaseShed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shed {
    pub center: Vec2,
    pub half: Vec2,
    pub floor: f32,
    pub eave: f32,
    pub ridge: f32,
    pub enclosed: bool,
    pub style: ShedStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailStyle {
    /// Chunky three-rail timber (yards, pens, gates).
    Timber,
    /// Weathered posts strung with barbed wire (property lines).
    Wire,
    /// Leaning, half-collapsed posts (marsh remnants).
    Broken,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rail {
    pub a: Vec2,
    pub b: Vec2,
    pub height: f32,
    pub base: f32,
    pub style: RailStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    Crates,
    Barrel,
    Trough,
    Hay,
    Well,
    Cart,
    Truck,
    Pickup,
    TankTower,
    Coop,
    Cow,
    Pole,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub kind: PropKind,
    pub center: Vec2,
    pub half: Vec2,
    pub height: f32,
}

impl Prop {
    /// Chunky solids hide a tall enemy behind them; thin posts, cows and
    /// low clutter do not.
    pub fn blocks_sight(&self) -> bool {
        self.height >= 1.6 && self.half.min_element() >= 0.45 && !matches!(self.kind, PropKind::Cow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// Sound planks that creak underfoot.
    Boardwalk,
    /// The watchtower ramp and deck: solid timber.
    Timber,
}

/// North/south ramp, or level platform. Guard rails share these dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub rect: Rect2,
    pub north: f32,
    pub south: f32,
    pub kind: SurfaceKind,
}

impl Surface {
    pub fn height(&self, p: Vec2) -> f32 {
        let t = ((p.y - self.rect.min.y) / (self.rect.max.y - self.rect.min.y)).clamp(0.0, 1.0);
        self.north + (self.south - self.north) * t
    }
}

/// A practical light. Powered lamps stay dark until the pump is restored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    pub pos: Vec3,
    pub radius: f32,
    pub powered: bool,
    /// Which line feeds a powered lamp: 0 the hacienda and the western road,
    /// 1 the middle road and the corral, 2 the eastern road and the bridge.
    pub circuit: u8,
}

/// Every line switched on (bit per `Lamp::circuit`).
pub const ALL_CIRCUITS: u8 = 0b111;
/// The lines the old dynamo feeds when the pump first brings it up: the
/// bridge stays dark until someone switches it on at the panel.
pub const FIRST_CIRCUITS: u8 = 0b011;
/// The panel's switch settings, in the order a press steps through them.
pub const CIRCUIT_SETTINGS: [u8; 3] = [0b011, 0b110, 0b101];

/// A painted board; `text` indexes `lore::sign`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignSite {
    pub pos: Vec2,
    /// Rotation about the vertical: the board's front faces (sin yaw, cos yaw).
    pub yaw: f32,
    /// Board width and height in metres.
    pub size: Vec2,
    pub text: u8,
}

/// A readable note; its text lives in `lore`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteSite {
    pub pos: Vec3,
    pub id: u8,
}

/// The extraction truck: a solid body, its ignition and the boarding zone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TruckSite {
    pub center: Vec2,
    pub half: Vec2,
    pub zone_center: Vec2,
    pub zone_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PadShape {
    Circle(Vec2, f32),
    Rect(Rect2),
}

/// Ground flattened to `height` inside the shape, blending into the rolling
/// terrain over `blend` metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    shape: PadShape,
    blend: f32,
    height: f32,
}

impl Pad {
    fn distance(&self, p: Vec2) -> f32 {
        match self.shape {
            PadShape::Circle(c, r) => (p.distance(c) - r).max(0.0),
            PadShape::Rect(r) => (p - p.clamp(r.min, r.max)).length(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct District {
    pub landmarks: [Landmark; 10],
    pub routes: Vec<Route>,
    pub sheds: Vec<Shed>,
    pub rails: Vec<Rail>,
    /// Decorative wire lines standing off the trails and out in the flood.
    /// Never solid and never on a route; `base` is an absolute post-top
    /// datum where nonzero (the waterline), otherwise the terrain.
    pub ruins: Vec<Rail>,
    pub props: Vec<Prop>,
    /// Deep water: never walkable except across authored surfaces.
    pub water: Vec<Rect2>,
    /// Walkable water: slow and noisy.
    pub shallows: Vec<Rect2>,
    pub surfaces: Vec<Surface>,
    /// Tall grass that hides a crouching player.
    pub grass: Vec<Rect2>,
    pub pads: Vec<Pad>,
    pub palms: Vec<Vec2>,
    pub groves: Vec<Vec2>,
    pub lamps: Vec<Lamp>,
    /// Bone bundle spawn points; the first lies on the house table.
    pub relics: Vec<Vec3>,
    /// Where each bundle may have been hidden (the first of each is the
    /// authored spot): one is picked per night (`Layout::with_seed`).
    pub relic_sites: Vec<Vec<Vec3>>,
    pub aji: Vec<Vec3>,
    /// Spare torch batteries: one pair at each, beside the landmarks' clutter.
    pub batteries: Vec<Vec3>,
    pub notes: Vec<NoteSite>,
    /// Painted boards; `text` indexes `lore::sign`.
    pub signs: Vec<SignSite>,
    pub pump: Vec3,
    pub beacon: Vec3,
    pub ignition: Vec3,
    /// The padlocked box with the truck key, on the crates by the windmill.
    pub lockbox: Vec3,
    /// Where Tureco is tied, behind the house by the back door.
    pub dog_post: Vec2,
    /// The dynamo's line panel, on its post beside the pump.
    pub panel: Vec3,
    pub truck: TruckSite,
    pub tower_half: Vec2,
    pub tower_height: f32,
    pub tank_radius: f32,
    /// Ground height of the water tower plateau.
    pub tower_ground: f32,
    pub watch_deck: Rect2,
    pub watch_ramp: Rect2,
    pub watch_height: f32,
    pub watch_side: Vec2,
}

fn p(x: f32, z: f32) -> Vec2 {
    Vec2::new(x, z)
}
fn rect(x0: f32, z0: f32, x1: f32, z1: f32) -> Rect2 {
    Rect2::new(p(x0, z0), p(x1, z1))
}
fn route(name: &'static str, points: &[[f32; 2]], width: f32) -> Route {
    Route {
        name,
        points: points.iter().copied().map(Vec2::from_array).collect(),
        width,
        trail: true,
        patrol: true,
    }
}

/// Height of the flattened plateau under the water tower.
const TOWER_GROUND: f32 = 2.4;
/// The relic radius, shared by the pickup rule and the mesh.
pub const RELIC_RADIUS: f32 = 0.32;

impl District {
    pub fn landmark(&self, id: LandmarkId) -> &Landmark {
        self.landmarks.iter().find(|l| l.id == id).expect("authored landmark")
    }

    /// The lookout cabin's roof: the one shed the deck's builder and the rain
    /// shelter mask both read. It sits over the deck footprint at deck height.
    pub fn watch_shelter(&self) -> Shed {
        Shed {
            center: self.watch_deck.center(),
            half: self.watch_deck.half(),
            floor: self.watch_height,
            eave: 2.7,
            ridge: 3.9,
            enclosed: false,
            style: ShedStyle::LeanTo,
        }
    }

    pub(super) fn authored() -> Self {
        use LandmarkId::*;
        let mark = |id, name, shot, center: Vec2, approach: Vec2, y, clearing| Landmark {
            id,
            name,
            shot,
            center,
            approach,
            look: Vec3::new(center.x, y, center.y),
            clearing,
        };
        let landmarks = [
            mark(Entry, "ENTRADA", "10_entry", p(0.0, 22.0), p(0.0, 27.5), 2.4, 6.0),
            mark(Ranch, "EL RANCHO", "11_ranch", p(0.0, -2.5), p(0.0, 10.0), 2.0, 13.0),
            mark(
                WaterTower,
                "EL MOLINO",
                "12_water_tower",
                p(-46.0, -25.0),
                p(-39.0, -14.0),
                TOWER_GROUND + 6.0,
                12.0,
            ),
            mark(
                Corral,
                "EL CORRAL",
                "13_corral",
                p(35.0, -12.0),
                p(35.0, 2.0),
                1.6,
                15.0,
            ),
            mark(
                Shrine,
                "LA CEIBA",
                "14_shrine",
                p(-21.0, -49.0),
                p(-18.0, -40.0),
                2.4,
                9.0,
            ),
            mark(Fields, "EL PAJONAL", "15_fields", p(58.0, 7.0), p(53.0, 13.0), 1.6, 4.0),
            mark(Cano, "EL CANO", "16_cano", p(18.0, -65.0), p(18.0, -54.0), 1.4, 5.0),
            mark(
                Watchtower,
                "EL MIRADOR",
                "17_watchtower",
                p(43.0, -88.0),
                p(43.0, -54.0),
                7.0,
                5.0,
            ),
            mark(
                Marsh,
                "LA CIENAGA",
                "18_marsh",
                p(-43.0, -83.0),
                p(-43.0, -78.0),
                0.8,
                4.0,
            ),
            mark(
                Extraction,
                "LA SALIDA",
                "19_extraction",
                p(56.0, 31.0),
                p(44.0, 30.0),
                1.6,
                6.0,
            ),
        ];
        let mut routes = vec![
            route("Entry to ranch", &[[0., 31.], [0., 27.], [0., 18.], [0., 10.]], 3.2),
            route("Ranch to corral / exposed", &[[0., 10.], [16., 8.], [35., 2.]], 3.0),
            route(
                "Corral to entry / covered",
                &[[35., 2.], [27., 16.], [10., 18.], [0., 18.]],
                2.2,
            ),
            route(
                "Ranch to shrine / western track",
                &[[-3., -8.], [-10., -23.], [-18., -40.]],
                2.4,
            ),
            route(
                "Corral to shrine / exposed",
                &[
                    [35., 2.],
                    [35., -12.],
                    [35., -18.],
                    [17., -36.],
                    [-5., -36.],
                    [-18., -40.],
                ],
                2.6,
            ),
            route(
                "Ranch to water tower",
                &[[0., 10.], [-15., 9.], [-24., 8.], [-30., 0.], [-39., -14.]],
                3.0,
            ),
            route(
                "Water tower to marsh",
                &[[-39., -14.], [-59., -18.], [-60., -40.], [-53., -59.], [-43., -78.]],
                2.4,
            ),
            route(
                "Marsh to lookout / bank road",
                &[
                    [-43., -78.],
                    [-25., -82.],
                    [-2., -80.],
                    [18., -78.],
                    [33., -77.],
                    [33., -97.],
                    [55., -97.],
                    [79., -86.],
                    [80., -56.],
                    [43., -54.],
                ],
                2.6,
            ),
            route("Cano bridge", &[[18., -54.], [18., -65.], [18., -78.]], 2.2),
            route(
                "Shrine to cano",
                &[[-18., -40.], [-8., -51.], [5., -54.], [18., -54.]],
                2.1,
            ),
            route(
                "Shrine western escape",
                &[[-18., -40.], [-30., -40.], [-33., -52.], [-25., -65.], [-2., -80.]],
                1.8,
            ),
            route("Corral to fields", &[[35., 2.], [43., 9.], [53., 13.]], 2.1),
            route(
                "Fields to entry",
                &[[53., 13.], [42., 18.], [27., 16.], [10., 18.], [0., 18.]],
                2.3,
            ),
            route(
                "Fields to lookout / reed shortcut",
                &[
                    [53., 13.],
                    [68., 0.],
                    [72., -23.],
                    [64., -43.],
                    [55., -51.],
                    [43., -54.],
                ],
                1.5,
            ),
            route("Corral to cano", &[[35., -18.], [17., -36.], [18., -54.]], 2.0),
            route(
                "Lookout ramp",
                &[[43., -54.], [43., -59.], [43., -83.], [43., -88.]],
                2.4,
            ),
            route(
                "Ford across the cano",
                &[[5., -54.], [0., -60.], [0., -72.], [-2., -80.]],
                1.8,
            ),
            route("East gate", &[[53., 13.], [55., 21.], [55., 27.], [55., 30.]], 3.0),
            route("Road to extraction", &[[0., 31.], [28., 31.], [55., 30.]], 5.0),
            route("Road to the west", &[[-40., 31.], [0., 31.]], 5.0),
        ];
        // The tower ramp is a raised walkway, not a patrol route; the two
        // roads are drawn as the road itself, not as worn trails.
        for r in &mut routes {
            match r.name {
                "Lookout ramp" => r.patrol = false,
                "Road to extraction" | "Road to the west" => r.trail = false,
                _ => {}
            }
        }

        let mut sheds = Vec::new();
        let mut shed = |style, id, offset: Vec2, half: Vec2, floor, eave, enclosed| {
            let c = landmarks.iter().find(|l| l.id == id).unwrap().center + offset;
            sheds.push(Shed {
                center: c,
                half,
                floor,
                eave,
                ridge: eave + 1.05,
                enclosed,
                style,
            });
        };
        use ShedStyle as S;
        shed(S::Casita, Ranch, p(-13., -1.), p(3.6, 2.8), 0., 2.35, true);
        shed(S::Deposito, Ranch, p(13., -9.), p(3.2, 2.6), 0., 2.2, false);
        shed(S::Cocina, Ranch, p(-15., -11.), p(2.8, 2.2), 0., 2.3, true);
        shed(
            S::PumpHouse,
            WaterTower,
            p(7., -4.),
            p(2.8, 2.5),
            TOWER_GROUND,
            2.5,
            true,
        );
        shed(S::Shelter, Corral, p(0., -13.), p(10., 4.), 0., 2.8, false);
        shed(S::LeanTo, Fields, p(4., -4.), p(3.3, 2.6), 0., 2.2, false);
        shed(S::StiltHut, Cano, p(8., -9.), p(3.1, 2.8), 0.35, 2.65, true);
        shed(S::BaseShed, Watchtower, p(9., 0.), p(2.5, 2.6), 0., 2.3, true);

        let mut rails = Vec::new();
        let mut rail = |a, b, height, style| {
            rails.push(Rail {
                a,
                b,
                height,
                base: 0.,
                style,
            })
        };
        use RailStyle::*;
        // Property lines: weathered posts and barbed wire. The road entry and
        // the east gate are left open.
        rail(p(-72., 22.), p(-4., 22.), 1.3, Wire);
        rail(p(4., 22.), p(51., 22.), 1.3, Wire);
        rail(p(59., 22.), p(86., 22.), 1.3, Wire);
        rail(p(-72., 22.), p(-72., -88.), 1.3, Wire);
        rail(p(86., 22.), p(86., -104.), 1.3, Wire);
        rail(p(86., -104.), p(10., -104.), 1.3, Wire);
        // Road-end barriers are visible, rather than fog hiding walk limits.
        rail(p(-72., 22.), p(-72., 35.2), 1.0, Timber);
        rail(p(86., 22.), p(86., 35.2), 1.0, Timber);
        rail(p(-72., 35.2), p(86., 35.2), 1.0, Timber);
        // Ranch yard: generous ends and a central opening.
        rail(p(-20., 5.), p(-20., -11.), 1.15, Timber);
        rail(p(9., 5.), p(20., 5.), 1.15, Timber);
        rail(p(20., 5.), p(20., -17.), 1.15, Timber);
        // Two pens flank a six-metre aisle. Gates open at both north and south.
        for (left, right) in [(24., 32.), (38., 46.)] {
            rail(p(left, -7.), p(left, -21.), 1.45, Timber);
            rail(p(right, -7.), p(right, -21.), 1.45, Timber);
            rail(p(left, -21.), p(right, -21.), 1.45, Timber);
            rail(p(left, -7.), p(left + 2., -7.), 1.45, Timber);
            rail(p(right - 2., -7.), p(right, -7.), 1.45, Timber);
        }
        // Field remnants do not wall off the track.
        rail(p(48., -3.), p(48., -15.), 1.1, Wire);
        rail(p(72., 6.), p(79., 6.), 0.8, Wire);
        // Broken marsh fence: the route remains on the firm, southern bank.
        rail(p(-51., -86.), p(-46., -86.), 0.75, Broken);
        rail(p(-40., -87.), p(-35., -87.), 0.9, Broken);
        rail(p(-25., -87.), p(-21., -87.), 0.6, Broken);
        // Fence remnants that lead the eye into the caño.
        rail(p(-8., -68.), p(-8., -60.), 0.8, Broken);
        rail(p(76., -68.), p(76., -62.), 0.8, Broken);

        // Old wire lines through the pajonal, clear of the trails.
        let mut ruins = Vec::new();
        for (a, b, height, base) in [
            (p(57.5, 15.), p(70., 4.5), 1.15, 0.),
            (p(71.5, -4.), p(75.5, -17.), 1.15, 0.),
            // Posts standing out of the marsh flood, tops measured from the waterline.
            (p(-46.5, -88.6), p(-53., -102.), 0.95, MARSH_WATER),
            (p(-35., -88.6), p(-31.5, -101.), 0.95, MARSH_WATER),
            (p(-21., -89.), p(-6., -93.), 0.95, MARSH_WATER),
        ] {
            ruins.push(Rail {
                a,
                b,
                height,
                base,
                style: Broken,
            });
        }

        let mut props = Vec::new();
        let mut prop = |kind, center: Vec2, half: Vec2, height| {
            props.push(Prop {
                kind,
                center,
                half,
                height,
            });
        };
        use PropKind::*;
        let at = |id: LandmarkId, off: Vec2| landmarks.iter().find(|l| l.id == id).unwrap().center + off;
        // Main entry: tool crates and a rusted drum inside the fence line.
        prop(Crates, at(Entry, p(-9.0, 2.5)), p(1.6, 0.85), 1.4);
        prop(Barrel, at(Entry, p(-6.3, 2.2)), p(0.5, 0.5), 1.0);
        // Ranch cluster.
        prop(Well, at(Ranch, p(9., 8.)), p(1.15, 1.15), 1.0);
        prop(Cart, at(Ranch, p(-11., 9.)), p(1.4, 0.8), 1.25);
        prop(Crates, at(Ranch, p(10., -5.)), p(1.3, 0.65), 1.2);
        prop(Barrel, at(Ranch, p(-8., 0.)), p(0.45, 0.45), 1.0);
        prop(Pickup, at(Ranch, p(14., 4.)), p(2.5, 1.1), 1.6);
        prop(Coop, at(Ranch, p(-17., -5.)), p(0.8, 0.6), 1.1);
        prop(TankTower, at(Ranch, p(17., -14.)), p(1.2, 1.2), 6.0);
        // Water tower.
        prop(Barrel, at(WaterTower, p(4., 1.)), p(0.6, 0.6), 1.4);
        prop(Crates, at(WaterTower, p(9., 0.)), p(1.0, 0.6), 0.9);
        prop(Trough, at(WaterTower, p(8., 4.)), p(2.4, 0.6), 0.8);
        prop(Barrel, at(WaterTower, p(10.6, -5.)), p(0.9, 0.9), 1.8);
        // Corral: troughs and hay under the pens, a water trough outside.
        prop(Trough, at(Corral, p(-7., -3.)), p(2.5, 0.65), 0.85);
        prop(Trough, at(Corral, p(7., -3.)), p(2.5, 0.65), 0.85);
        prop(Trough, at(Corral, p(-7., 8.)), p(1.6, 0.5), 0.8);
        prop(Hay, at(Corral, p(-6., -14.)), p(1.5, 1.2), 1.6);
        prop(Hay, at(Corral, p(6., -14.)), p(1.5, 1.2), 1.25);
        prop(Barrel, at(Corral, p(-2.6, 5.)), p(0.45, 0.45), 1.0);
        // Six head of cattle: fixed spots, idle animation only.
        for c in [
            p(27.5, -11.0),
            p(29.5, -17.0),
            p(41.0, -10.0),
            p(43.5, -16.0),
            p(31.0, -25.5),
            p(39.5, -26.5),
        ] {
            prop(Cow, c, p(0.65, 0.65), 1.3);
        }
        // Fields.
        prop(Cart, at(Fields, p(4., -3.)), p(1.5, 0.7), 1.0);
        prop(Barrel, at(Fields, p(6., -6.)), p(0.45, 0.45), 0.9);
        // Caño and watchtower base.
        prop(Crates, at(Cano, p(11., -6.)), p(0.8, 0.6), 0.95);
        prop(Barrel, at(Watchtower, p(6., 4.)), p(0.5, 0.5), 1.0);
        prop(Crates, at(Watchtower, p(11., 4.)), p(1.0, 0.6), 0.8);
        // Extraction: the truck, its cargo and the power poles.
        let truck = TruckSite {
            center: p(54.5, 32.2),
            half: p(2.7, 1.05),
            zone_center: p(55.0, 30.0),
            zone_radius: 7.0,
        };
        prop(PropKind::Truck, truck.center, truck.half, 2.0);
        prop(Crates, p(50.0, 34.0), p(0.9, 0.6), 0.9);
        prop(Barrel, p(51.3, 33.8), p(0.35, 0.35), 0.9);
        let pole_xs = [-48., -36., -24., -12., 12., 24., 36., 48.];
        for x in pole_xs {
            prop(Pole, p(x, 26.6), p(0.15, 0.15), 6.0);
        }
        for c in [p(-6., 12.), p(8., 12.), p(24., -3.), p(46., -3.), p(66., 26.6)] {
            prop(Pole, c, p(0.15, 0.15), 6.0);
        }

        let water = vec![
            rect(-8., -70., 76., -61.),
            rect(-74., -107., 8., -88.),
            rect(-66., -88., -57., -70.),
            rect(-42., 11., -31., 20.),
            rect(61.5, 22.5, 67.5, 36.),
        ];
        let shallows = vec![
            // The ford: a slow, noisy shortcut across the caño.
            rect(-3., -70., 3., -61.),
            // Pond margins.
            rect(-44., 9., -29., 11.),
            rect(-44., 20., -29., 22.),
            // The reed shortcut wades through a flooded dip.
            rect(66., -30., 76., -22.),
        ];
        let grass = vec![
            rect(44., -8., 84., 21.),
            rect(-56., -86., 10., -71.),
            rect(-38., -62., -26., -44.),
            rect(4., -44., 24., -30.),
            rect(-64., -40., -52., -20.),
        ];
        let watch_deck = rect(39.5, -91.5, 46.5, -84.5);
        let watch_ramp = rect(41.4, -84.5, 44.6, -58.5);
        let watch_height = 6.4;
        let surfaces = vec![
            Surface {
                rect: rect(15.8, -72., 20.2, -59.),
                north: 0.18,
                south: 0.18,
                kind: SurfaceKind::Boardwalk,
            },
            Surface {
                rect: rect(22.5, -77.5, 29.5, -70.),
                north: 0.35,
                south: 0.35,
                kind: SurfaceKind::Boardwalk,
            },
            Surface {
                rect: watch_deck,
                north: watch_height,
                south: watch_height,
                kind: SurfaceKind::Timber,
            },
            Surface {
                rect: watch_ramp,
                north: watch_height,
                south: 0.,
                kind: SurfaceKind::Timber,
            },
            // The extraction bridge spans the whole road.
            Surface {
                rect: rect(60.5, 27.2, 68.5, 35.2),
                north: 0.22,
                south: 0.22,
                kind: SurfaceKind::Boardwalk,
            },
        ];
        let pads = vec![
            // The road corridor and its verges are dead flat.
            Pad {
                shape: PadShape::Rect(rect(-160., 20., 160., 40.)),
                blend: 3.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Circle(p(0., -2.5), 22.),
                blend: 6.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Rect(rect(20., -31., 50., -3.)),
                blend: 4.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Circle(p(-21., -49.), 9.),
                blend: 6.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Circle(p(62., 3.), 5.),
                blend: 3.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Rect(rect(12., -79., 32., -57.)),
                blend: 3.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Circle(p(43., -88.), 9.),
                blend: 4.,
                height: 0.,
            },
            Pad {
                shape: PadShape::Circle(p(43., -56.), 5.),
                blend: 3.,
                height: 0.,
            },
            // The windmill stands on a small rise: high ground.
            Pad {
                shape: PadShape::Circle(p(-46., -25.), 11.),
                blend: 9.,
                height: TOWER_GROUND,
            },
        ];

        let table_relic = Vec3::new(3.25, 0.93, -4.6);
        // Each bundle's hiding places within its landmark. The table bundle
        // is where the story starts; the others move from night to night.
        let relic_sites = vec![
            vec![table_relic],
            vec![
                Vec3::new(35.0, 0.78, -26.5),
                Vec3::new(27.0, 0.78, -6.5),
                Vec3::new(41.0, 0.78, -3.0),
            ],
            vec![
                Vec3::new(62.0, 1.05, 4.0),
                Vec3::new(56.0, 0.73, 9.0),
                Vec3::new(66.0, 0.73, -2.5),
            ],
            vec![Vec3::new(26.0, 1.2, -75.5), Vec3::new(24.5, 1.13, -73.0)],
            // Not the deck's open front: seen from the whole llano, with
            // nowhere up there to break his line of sight.
            vec![
                Vec3::new(43.0, watch_height + 0.72, -89.6),
                Vec3::new(52.5, 0.78, -87.5),
            ],
        ];
        let relics: Vec<Vec3> = relic_sites.iter().map(|s| s[0]).collect();
        let aji = vec![
            Vec3::new(-9.0, 1.55, 24.5),
            Vec3::new(-3.7, 0.62, 2.75),
            Vec3::new(10.0, 1.32, -7.5),
            Vec3::new(-13.0, 1.02, -5.2),
            Vec3::new(-38.6, TOWER_GROUND + 1.02, -30.4),
            Vec3::new(32.4, 0.72, -28.2),
            Vec3::new(24.6, 1.22, -75.4),
        ];
        // Spare batteries lie beside other finds, where a hand would have set
        // them down: the porch, the windmill shed, the corral, the stilt hut,
        // the fields' shelter and the lookout deck.
        let batteries = vec![
            Vec3::new(-3.2, 0.62, 3.0),
            Vec3::new(-38.1, TOWER_GROUND + 1.02, -30.9),
            Vec3::new(32.9, 0.72, -27.9),
            Vec3::new(25.0, 1.22, -74.9),
            Vec3::new(61.4, 1.05, 4.5),
            Vec3::new(41.9, watch_height + 0.9, -89.1),
        ];
        // Hand-lettered boards, after the places they warn about; the words
        // live in `lore::sign`. They face the way people arrive.
        let sign = |x: f32, z: f32, yaw: f32, w: f32, text: u8| SignSite {
            pos: p(x, z),
            yaw,
            size: p(w, w * 0.5),
            text,
        };
        let signs = vec![
            sign(-8.0, 25.2, 0.0, 2.8, 0),
            sign(7.6, 25.0, 0.0, 2.2, 1),
            sign(-13.0, 26.2, 0.0, 2.4, 9),
            sign(-4.6, 7.2, 0.0, 2.3, 2),
            sign(-41.5, -15.0, 0.7, 2.2, 3),
            sign(-16.5, -44.2, 0.35, 2.0, 4),
            sign(39.2, 2.8, 0.0, 2.4, 5),
            sign(14.5, -57.5, 0.5, 2.3, 6),
            sign(22.5, -60.5, -0.3, 2.1, 7),
            sign(46.0, 28.4, -1.35, 2.0, 8),
            sign(-43.0, -76.2, 0.2, 2.3, 10),
            sign(46.6, -56.5, 0.25, 1.7, 11),
        ];
        // Notes lie on the furniture they are found on: the corral barrel
        // top and the altar table top.
        let corral_note = p(32.4, -6.6);
        let barrel_top = props
            .iter()
            .find(|q| q.kind == PropKind::Barrel && q.center.distance(corral_note) <= q.half.x)
            .map_or(0., |q| q.height);
        let notes = vec![
            NoteSite {
                pos: Vec3::new(2.75, 0.8, -4.3),
                id: 0,
            },
            NoteSite {
                pos: Vec3::new(-15.4, 0.95, -11.6),
                id: 1,
            },
            NoteSite {
                pos: Vec3::new(corral_note.x, barrel_top, corral_note.y),
                id: 2,
            },
            NoteSite {
                pos: Vec3::new(-20.6, ALTAR_TABLE_TOP, -46.3),
                id: 3,
            },
            NoteSite {
                pos: Vec3::new(41.4, watch_height + 0.9, -89.5),
                id: 4,
            },
            NoteSite {
                pos: Vec3::new(27.4, 1.25, -75.4),
                id: 5,
            },
            NoteSite {
                pos: Vec3::new(-38.4, TOWER_GROUND + 1.1, -29.8),
                id: 6,
            },
            // The rest of the story, where people left it: pasted up at the
            // gate, on the cart, on the house shelf (a photograph and the
            // radio), on the coop, the windmill barrel, the corral hay, the
            // fields barrel, in the tool shed, on the stilt hut's crates, at
            // the tower's foot, and in the truck's cargo.
            NoteSite {
                pos: Vec3::new(-8.0, 1.42, 24.8),
                id: 7,
            },
            NoteSite {
                pos: Vec3::new(-11.0, 1.27, 6.5),
                id: 8,
            },
            NoteSite {
                pos: Vec3::new(-4.74, 1.82, -4.5),
                id: 9,
            },
            NoteSite {
                pos: Vec3::new(-4.74, 1.82, -5.4),
                id: 10,
            },
            NoteSite {
                pos: Vec3::new(-17.0, 1.12, -7.5),
                id: 11,
            },
            NoteSite {
                pos: Vec3::new(-42.0, TOWER_GROUND + 1.42, -24.0),
                id: 12,
            },
            NoteSite {
                pos: Vec3::new(41.0, 1.27, -26.0),
                id: 13,
            },
            NoteSite {
                pos: Vec3::new(64.0, 0.92, 1.0),
                id: 14,
            },
            NoteSite {
                pos: Vec3::new(-14.5, 0.9, -2.0),
                id: 15,
            },
            NoteSite {
                pos: Vec3::new(29.0, 1.32, -71.0),
                id: 16,
            },
            NoteSite {
                pos: Vec3::new(54.0, 0.78, -84.0),
                id: 17,
            },
            NoteSite {
                pos: Vec3::new(50.0, 0.92, 34.0),
                id: 18,
            },
            NoteSite {
                pos: Vec3::new(-11.8, 0.9, -5.0),
                id: 19,
            },
        ];
        let lamps = {
            let mut v = Vec::new();
            let mut lamp = |x, y, z| {
                v.push(Lamp {
                    pos: Vec3::new(x, y, z),
                    radius: 8.0,
                    powered: false,
                    circuit: 0,
                })
            };
            // Main entry: two on the gate posts, one on the sign.
            lamp(-4.15, 2.7, 22.7);
            lamp(4.15, 2.7, 22.7);
            lamp(-8.4, 1.9, 23.2);
            // Ranch: well post, casita, depósito, cocina, yard sign.
            lamp(9., 2.3, 13.5);
            lamp(-13., 2.05, -0.3);
            lamp(13., 2.05, -8.6);
            lamp(-15., 2.05, -8.7);
            lamp(-4.6, 1.9, 6.6);
            // Water tower: base, high platform and pump house.
            lamp(-46., TOWER_GROUND + 1.9, -22.3);
            lamp(-47.2, TOWER_GROUND + 9.6, -25.);
            lamp(-40.6, TOWER_GROUND + 2.2, -26.8);
            // Corral: gate posts and shelter.
            lamp(26., 2.5, -6.7);
            lamp(44., 2.5, -6.7);
            lamp(28., 2.6, -21.3);
            lamp(35., 2.6, -21.3);
            lamp(42., 2.6, -21.3);
            // Shrine: a lantern on a limb over the altar and two candle clusters.
            lamp(-18.6, 2.6, -45.9);
            lamp(-20.8, 0.9, -46.0);
            lamp(-19.2, 0.9, -47.7);
            // Fields shelter.
            lamp(62.5, 2.1, 5.2);
            // Caño: hut and pier.
            lamp(24.2, 3.0, -71.5);
            lamp(27.8, 3.0, -71.5);
            lamp(19.2, 2.4, -64.);
            // Watchtower: cabin, base shed, ramp foot.
            lamp(43., watch_height + 2.1, -88.);
            lamp(52., 2.1, -85.7);
            lamp(43.9, 1.9, -57.);
            // A lonely lantern on the marsh fence.
            lamp(-40., 1.8, -86.4);
            // Extraction bridge lanterns.
            lamp(60.9, 2.5, 27.5);
            lamp(60.9, 2.5, 34.9);
            // Power poles: dark until the windmill pump runs.
            let mut powered = |x: f32, z: f32| {
                // West of the gate and the yard: the hacienda's line; the
                // corral and the middle road: the second; east: the bridge's.
                let circuit = if z < 20.0 {
                    if x < 15.0 { 0 } else { 1 }
                } else if x < 0.0 {
                    0
                } else if x < 30.0 {
                    1
                } else {
                    2
                };
                v.push(Lamp {
                    pos: Vec3::new(x, 5.4, z),
                    radius: 11.0,
                    powered: true,
                    circuit,
                })
            };
            for x in pole_xs {
                powered(x, 26.6);
            }
            for c in [p(-6., 12.), p(8., 12.), p(24., -3.), p(46., -3.), p(66., 26.6)] {
                powered(c.x, c.y);
            }
            v
        };
        let mut palms = Vec::new();
        let mut groves = Vec::new();
        let mut rng = Rng::new(0x5A17_A0DE);
        let mut d = Self {
            landmarks,
            routes,
            sheds,
            rails,
            ruins,
            props,
            water,
            shallows,
            surfaces,
            grass,
            pads,
            palms: Vec::new(),
            groves: Vec::new(),
            lamps,
            relics,
            relic_sites,
            aji,
            batteries,
            notes,
            signs,
            pump: Vec3::new(-46.0, TOWER_GROUND + 1.25, -22.05),
            beacon: Vec3::new(43.0, watch_height + 1.5, -91.0),
            ignition: Vec3::new(55.7, 1.45, 30.9),
            lockbox: Vec3::new(-37.0, TOWER_GROUND + 0.98, -25.0),
            dog_post: Vec2::new(-7.5, -8.5),
            panel: Vec3::new(-44.2, TOWER_GROUND + 1.35, -21.2),
            truck,
            tower_half: p(2.3, 2.3),
            tower_height: 9.0,
            tank_radius: 1.75,
            tower_ground: TOWER_GROUND,
            watch_deck,
            watch_ramp,
            watch_height,
            watch_side: watch_deck.min + Vec2::new(-8.0, 10.0),
        };
        // Hand-placed landmark palms and groves, then a seeded scatter that
        // keeps routes, structures and water clear. Trunks are solid and hide
        // a tall enemy, so the scatter is part of the authored footprint.
        for c in [
            p(-65., -10.),
            p(-62., -32.),
            p(-47., -45.),
            p(-9., 18.),
            p(24., 0.),
            p(51., -29.),
            p(80., -17.),
            p(66., -76.),
            p(31., -99.),
            p(-32., -73.),
            p(-68., -93.),
            p(-17., -95.),
        ] {
            palms.push(c);
        }
        for c in [p(8., -23.), p(-32., -13.), p(52., -36.), p(-29., -65.), p(9., -88.)] {
            groves.push(c);
        }
        let b = rect(-74., -107., 88., 21.);
        let mut tries = 0;
        while (palms.len() < 64 || groves.len() < 22) && tries < 20000 {
            tries += 1;
            let c = p(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y));
            if !d.open_for_tree(c, &palms, &groves) {
                continue;
            }
            // Denser toward the edges, like the reference silhouette.
            let edge = (c.x - b.min.x).min(b.max.x - c.x).min(c.y - b.min.y).min(b.max.y - c.y);
            let keep = (0.35 + 0.65 * (1.0 - (edge / 26.0).clamp(0.0, 1.0))) * (0.55 + fbm2(c.x * 0.05, c.y * 0.05, 9));
            if rng.f32() > keep {
                continue;
            }
            if rng.f32() < 0.68 && palms.len() < 64 {
                palms.push(c);
            } else if groves.len() < 22 {
                groves.push(c);
            }
        }
        d.palms = palms;
        d.groves = groves;
        d
    }

    /// Is `c` a legal trunk position: off routes, water, structures, props,
    /// landmarks' clearings and other trunks.
    fn open_for_tree(&self, c: Vec2, palms: &[Vec2], groves: &[Vec2]) -> bool {
        if self.route_distance(c) < 3.2 {
            return false;
        }
        if self
            .water
            .iter()
            .chain(&self.shallows)
            .any(|w| grow(*w, 2.5).contains(c))
        {
            return false;
        }
        if self.grass.iter().any(|g| g.contains(c)) && c.x > 40. {
            return false;
        }
        if self.landmarks.iter().any(|l| c.distance(l.center) < l.clearing + 4.0) {
            return false;
        }
        if self
            .sheds
            .iter()
            .any(|s| grow(Rect2::from_center(s.center, s.half), 4.0).contains(c))
        {
            return false;
        }
        if self
            .props
            .iter()
            .any(|q| grow(Rect2::from_center(q.center, q.half), 3.0).contains(c))
        {
            return false;
        }
        if self.rails.iter().any(|r| segment_point_distance(r.a, r.b, c) < 2.4) {
            return false;
        }
        if self.surfaces.iter().any(|s| grow(s.rect, 3.0).contains(c)) {
            return false;
        }
        if self.lamps.iter().any(|l| Vec2::new(l.pos.x, l.pos.z).distance(c) < 2.5) {
            return false;
        }
        if palms.iter().chain(groves).any(|o| o.distance(c) < 5.5) {
            return false;
        }
        true
    }

    pub fn water_at(&self, p: Vec2) -> bool {
        self.water.iter().any(|r| r.contains(p))
    }
    pub fn shallow_at(&self, p: Vec2) -> bool {
        self.shallows.iter().any(|r| r.contains(p))
    }
    pub fn tall_grass_at(&self, p: Vec2) -> bool {
        self.grass.iter().any(|r| r.contains(p))
    }
    /// Height above the ground of the barrel or crate top under `p`, if any.
    pub fn prop_top(&self, p: Vec2) -> Option<f32> {
        self.props
            .iter()
            .filter(|q| match q.kind {
                PropKind::Barrel => q.center.distance(p) <= q.half.x,
                PropKind::Crates => Rect2::from_center(q.center, q.half).contains(p),
                _ => false,
            })
            .map(|q| q.height)
            .reduce(f32::max)
    }

    /// Highest authored deck/ramp/bridge height at `p`, or 0.
    pub fn surface_height(&self, p: Vec2) -> f32 {
        self.surfaces
            .iter()
            .filter(|s| s.rect.contains(p))
            .map(|s| s.height(p))
            .fold(0., f32::max)
    }
    pub fn surface_at(&self, p: Vec2) -> Option<&Surface> {
        self.surfaces.iter().find(|s| s.rect.contains(p))
    }
    pub fn route_distance(&self, p: Vec2) -> f32 {
        self.routes
            .iter()
            .flat_map(|r| {
                r.points
                    .windows(2)
                    .map(move |w| segment_point_distance(w[0], w[1], p) - r.width * 0.5)
            })
            .fold(f32::MAX, f32::min)
    }
    /// Distance to the nearest *worn trail* edge (for ground painting).
    pub fn trail_distance(&self, p: Vec2) -> f32 {
        self.routes
            .iter()
            .filter(|r| r.trail)
            .flat_map(|r| {
                r.points
                    .windows(2)
                    .map(move |w| segment_point_distance(w[0], w[1], p) - r.width * 0.5)
            })
            .fold(f32::MAX, f32::min)
    }

    pub fn cows(&self) -> impl Iterator<Item = &Prop> {
        self.props.iter().filter(|q| q.kind == PropKind::Cow)
    }

    /// Terrain height. Gently rolling, dead flat on pads and roads, dished
    /// into water, and rising beyond the boundary toward the horizon.
    pub fn terrain(&self, p: Vec2, bounds: Rect2) -> f32 {
        let roll = (fbm2(p.x * 0.045, p.y * 0.045, 71) - 0.5) * 0.7;
        let mut h = roll;
        for pad in &self.pads {
            let t = 1.0 - smooth(pad.distance(p) / pad.blend);
            h += (pad.height - h) * t;
        }
        let out = (bounds.min.x - p.x)
            .max(p.x - bounds.max.x)
            .max(bounds.min.y - p.y)
            .max(0.0);
        if out > 0.0 {
            let ramp = smooth(out / 46.0);
            h += ramp * (fbm2(p.x * 0.03, p.y * 0.03, 19) * 5.5 - 0.8);
        }
        for r in &self.water {
            if r.contains(p) {
                // The waterline wanders up to ~1.7 m inside the fence line;
                // walkability still follows the authored rectangle.
                let recede = fbm2(p.x * 0.31, p.y * 0.31, 33) * 1.7;
                h = h.min(-0.65 * ((inner_edge(*r, p) - recede) / 1.1).clamp(0.0, 1.0));
            }
        }
        for r in &self.shallows {
            if r.contains(p) {
                h = h.min(-0.16 * (inner_edge(*r, p) / 1.0).clamp(0.0, 1.0));
            }
        }
        h
    }

    pub(super) fn blockers(&self, out: &mut Vec<Blocker>) {
        let add = |out: &mut Vec<Blocker>, rect, sight, kind| {
            out.push(Blocker {
                shape: Shape::Rect(rect),
                sight,
                kind,
            })
        };
        for s in &self.sheds {
            let r = Rect2::from_center(s.center, s.half);
            if s.enclosed {
                add(
                    out,
                    rect(r.min.x, r.min.y, r.max.x, r.min.y + 0.16),
                    Sight::Blocks,
                    BlockerKind::Wall,
                );
                add(
                    out,
                    rect(r.min.x, r.min.y, r.min.x + 0.16, r.max.y),
                    Sight::Blocks,
                    BlockerKind::Wall,
                );
                add(
                    out,
                    rect(r.max.x - 0.16, r.min.y, r.max.x, r.max.y),
                    Sight::Blocks,
                    BlockerKind::Wall,
                );
            } else {
                for x in [r.min.x, r.max.x] {
                    for z in [r.min.y, r.max.y] {
                        add(
                            out,
                            Rect2::from_center(p(x, z), Vec2::splat(0.13)),
                            Sight::Clear,
                            BlockerKind::Post,
                        );
                    }
                }
            }
        }
        for rail in &self.rails {
            add(
                out,
                Rect2::new(
                    rail.a.min(rail.b) - Vec2::splat(0.12),
                    rail.a.max(rail.b) + Vec2::splat(0.12),
                ),
                Sight::Clear,
                BlockerKind::Fence,
            );
        }
        for prop in &self.props {
            match prop.kind {
                PropKind::TankTower => {
                    for (sx, sz) in [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)] {
                        out.push(Blocker {
                            shape: Shape::Circle {
                                center: prop.center + Vec2::new(sx, sz) * (prop.half - Vec2::splat(0.2)),
                                radius: 0.17,
                            },
                            sight: Sight::Clear,
                            kind: BlockerKind::Post,
                        });
                    }
                }
                PropKind::Pole => out.push(Blocker {
                    shape: Shape::Circle {
                        center: prop.center,
                        radius: 0.17,
                    },
                    sight: Sight::Clear,
                    kind: BlockerKind::Post,
                }),
                PropKind::Cow => out.push(Blocker {
                    shape: Shape::Circle {
                        center: prop.center,
                        radius: prop.half.x,
                    },
                    sight: Sight::Clear,
                    kind: BlockerKind::Furniture,
                }),
                _ => add(
                    out,
                    Rect2::from_center(prop.center, prop.half),
                    if prop.blocks_sight() {
                        Sight::Blocks
                    } else {
                        Sight::Clear
                    },
                    BlockerKind::Furniture,
                ),
            }
        }
        for &center in self.palms.iter().chain(&self.groves) {
            out.push(Blocker {
                shape: Shape::Circle { center, radius: 0.35 },
                sight: Sight::Blocks,
                kind: BlockerKind::Trunk,
            });
        }
        // Carve only authored crossings and shallows out of deep water;
        // everything else has a continuous solid bank. A graph edge is not a
        // substitute for this.
        for water in &self.water {
            let mut parts = vec![*water];
            for cut in self
                .surfaces
                .iter()
                .map(|s| s.rect)
                .chain(self.shallows.iter().copied())
            {
                let mut next = Vec::new();
                for r in parts {
                    subtract(r, cut, &mut next);
                }
                parts = next;
            }
            for r in parts {
                add(out, r, Sight::Clear, BlockerKind::Bank);
            }
        }
        let bridge = self.surfaces[0].rect;
        for x in [bridge.min.x, bridge.max.x] {
            add(
                out,
                rect(x - 0.12, bridge.min.y, x + 0.12, bridge.max.y),
                Sight::Clear,
                BlockerKind::Fence,
            );
        }
        // The extraction bridge is walled on both sides.
        let road_bridge = self.surfaces[4].rect;
        for z in [road_bridge.min.y, road_bridge.max.y] {
            add(
                out,
                rect(road_bridge.min.x, z - 0.12, road_bridge.max.x, z + 0.12),
                Sight::Clear,
                BlockerKind::Fence,
            );
        }
        // Ramp sides, platform perimeter and closed underside are one 2D
        // footprint. No drop-offs, jumping or stacked navigation levels.
        let d = self.watch_deck;
        let r = self.watch_ramp;
        for edge in [
            rect(d.min.x - 0.12, d.min.y - 0.12, d.max.x + 0.12, d.min.y + 0.12),
            rect(d.min.x - 0.12, d.min.y, d.min.x + 0.12, d.max.y),
            rect(d.max.x - 0.12, d.min.y, d.max.x + 0.12, d.max.y),
            rect(d.min.x, d.max.y - 0.12, r.min.x, d.max.y + 0.12),
            rect(r.max.x, d.max.y - 0.12, d.max.x, d.max.y + 0.12),
            rect(r.min.x - 0.12, r.min.y, r.min.x + 0.12, r.max.y),
            rect(r.max.x - 0.12, r.min.y, r.max.x + 0.12, r.max.y),
        ] {
            add(out, edge, Sight::Clear, BlockerKind::Fence);
        }
        let t = self.landmark(LandmarkId::WaterTower).center;
        for x in [-self.tower_half.x, self.tower_half.x] {
            for z in [-self.tower_half.y, self.tower_half.y] {
                add(
                    out,
                    Rect2::from_center(t + p(x, z), Vec2::splat(0.2)),
                    Sight::Clear,
                    BlockerKind::Post,
                );
            }
        }
    }
}

fn grow(r: Rect2, by: f32) -> Rect2 {
    Rect2::new(r.min - Vec2::splat(by), r.max + Vec2::splat(by))
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn inner_edge(r: Rect2, p: Vec2) -> f32 {
    (p.x - r.min.x).min(r.max.x - p.x).min(p.y - r.min.y).min(r.max.y - p.y)
}

fn subtract(r: Rect2, cut: Rect2, out: &mut Vec<Rect2>) {
    let lo = r.min.max(cut.min);
    let hi = r.max.min(cut.max);
    if lo.x >= hi.x || lo.y >= hi.y {
        out.push(r);
        return;
    }
    for (a, b) in [
        (r.min, p(lo.x, r.max.y)),
        (p(hi.x, r.min.y), r.max),
        (p(lo.x, r.min.y), p(hi.x, lo.y)),
        (p(lo.x, hi.y), p(hi.x, r.max.y)),
    ] {
        if b.x > a.x && b.y > a.y {
            out.push(Rect2::new(a, b));
        }
    }
}

impl Layout {
    /// The one authored map, every bundle in its authored hiding place.
    pub fn new() -> Self {
        Self::assemble(District::authored())
    }

    /// The map for one night: each bundle in one of its hiding places,
    /// chosen by the seed (the same on every machine in a shared session).
    pub fn with_seed(seed: u64) -> Self {
        let mut layout = Self::new();
        let mut rng = crate::rng::Rng::fork(seed, 0xB0E5);
        let d = &mut layout.district;
        for (i, sites) in d.relic_sites.iter().enumerate() {
            d.relics[i] = sites[rng.below(sites.len())];
        }
        layout
    }

    /// Ground height under `p`: terrain and any authored deck, ramp or bridge.
    pub fn surface_height(&self, p: Vec2) -> f32 {
        let surface = self.district.surface_height(p);
        let terrain = self.terrain(p);
        if surface > 0.0 { surface.max(terrain) } else { terrain }
    }

    /// Terrain alone (what grass and posts stand on).
    pub fn terrain(&self, p: Vec2) -> f32 {
        self.district.terrain(p, self.bounds)
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}
