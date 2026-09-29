//! The map (M): a top-down chart drawn once from `Layout`, with live markers
//! for the party and their pings. It never shows bones, peppers or him.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::*;
use crate::app::LayoutRes;
use crate::geometry::{Layout, Rect2};
use crate::noise::fbm2;
use crate::player::Player;

const PX_PER_M: f32 = 4.0;
const MAX_PINGS: usize = 4;
const DOT: f32 = 12.0;

#[derive(Resource)]
pub(crate) struct MapImage {
    handle: Handle<Image>,
    origin: Vec2,
    extent: Vec2,
}

impl MapImage {
    /// 0..1 across and down the chart for a ground point.
    fn uv(&self, p: Vec2) -> Vec2 {
        (p - self.origin) / self.extent
    }
}

#[derive(Component)]
pub(crate) struct MapPanel;
#[derive(Component)]
pub(crate) struct MapDot(usize);
#[derive(Component)]
pub(crate) struct MapPing(usize);

struct Canvas {
    w: usize,
    h: usize,
    origin: Vec2,
    px: Vec<[f32; 3]>,
}

type Rgba = [f32; 4];

impl Canvas {
    fn new(origin: Vec2, extent: Vec2) -> Self {
        let w = (extent.x * PX_PER_M).ceil() as usize;
        let h = (extent.y * PX_PER_M).ceil() as usize;
        Self {
            w,
            h,
            origin,
            px: vec![[0.0; 3]; w * h],
        }
    }

    fn to_px(&self, p: Vec2) -> Vec2 {
        (p - self.origin) * PX_PER_M
    }

    fn blend(&mut self, x: usize, y: usize, c: Rgba, coverage: f32) {
        let a = c[3] * coverage;
        if a <= 0.0 {
            return;
        }
        let d = &mut self.px[y * self.w + x];
        for i in 0..3 {
            d[i] += (c[i] - d[i]) * a;
        }
    }

    fn span(&self, lo: Vec2, hi: Vec2) -> (usize, usize, usize, usize) {
        let x0 = lo.x.floor().max(0.0) as usize;
        let y0 = lo.y.floor().max(0.0) as usize;
        let x1 = (hi.x.ceil().max(0.0) as usize).min(self.w);
        let y1 = (hi.y.ceil().max(0.0) as usize).min(self.h);
        (x0, y0, x1, y1)
    }

    fn rect(&mut self, r: Rect2, c: Rgba) {
        let (lo, hi) = (self.to_px(r.min), self.to_px(r.max));
        let (x0, y0, x1, y1) = self.span(lo, hi);
        for y in y0..y1 {
            for x in x0..x1 {
                // Soft edges: partial coverage at the borders.
                let cx = ((x as f32 + 1.0).min(hi.x) - (x as f32).max(lo.x)).clamp(0.0, 1.0);
                let cy = ((y as f32 + 1.0).min(hi.y) - (y as f32).max(lo.y)).clamp(0.0, 1.0);
                self.blend(x, y, c, cx * cy);
            }
        }
    }

    fn circle(&mut self, center: Vec2, radius: f32, c: Rgba) {
        let (cp, rp) = (self.to_px(center), radius * PX_PER_M);
        let (x0, y0, x1, y1) = self.span(cp - Vec2::splat(rp + 1.0), cp + Vec2::splat(rp + 1.0));
        for y in y0..y1 {
            for x in x0..x1 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(cp);
                self.blend(x, y, c, (rp - d + 0.5).clamp(0.0, 1.0));
            }
        }
    }

    fn segment(&mut self, a: Vec2, b: Vec2, width: f32, c: Rgba) {
        let (pa, pb) = (self.to_px(a), self.to_px(b));
        let half = width * 0.5 * PX_PER_M;
        let (x0, y0, x1, y1) = self.span(
            pa.min(pb) - Vec2::splat(half + 1.0),
            pa.max(pb) + Vec2::splat(half + 1.0),
        );
        let ab = pb - pa;
        let len2 = ab.length_squared().max(1e-6);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let t = ((p - pa).dot(ab) / len2).clamp(0.0, 1.0);
                let d = p.distance(pa + ab * t);
                self.blend(x, y, c, (half - d + 0.5).clamp(0.0, 1.0));
            }
        }
    }

    fn outline(&mut self, r: Rect2, width: f32, c: Rgba) {
        let (a, b) = (r.min, r.max);
        let corners = [a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
        for i in 0..4 {
            self.segment(corners[i], corners[(i + 1) % 4], width, c);
        }
    }

    fn into_image(self) -> Image {
        let mut data = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            for c in p {
                data.push((c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            }
            data.push(255);
        }
        let mut image = Image::new(
            Extent3d {
                width: self.w as u32,
                height: self.h as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        image
    }
}

fn paint(layout: &Layout) -> (Canvas, Vec2, Vec2) {
    let pad = Vec2::splat(6.0);
    let origin = layout.bounds.min - pad;
    let extent = layout.bounds.max - layout.bounds.min + pad * 2.0;
    let mut cv = Canvas::new(origin, extent);
    let d = &layout.district;

    // Paper and the faintly mottled land.
    for y in 0..cv.h {
        for x in 0..cv.w {
            let p = origin + Vec2::new(x as f32, y as f32) / PX_PER_M;
            let n = fbm2(p.x * 0.045, p.y * 0.045, 7) - 0.5;
            let g = fbm2(p.x * 0.4, p.y * 0.4, 3) - 0.5;
            cv.px[y * cv.w + x] = [0.15 + n * 0.07 + g * 0.02, 0.185 + n * 0.08 + g * 0.02, 0.12 + n * 0.05];
        }
    }
    // Everything beyond the walkable bounds is dark.
    let dark = [0.03, 0.04, 0.035, 0.82];
    let b = layout.bounds;
    let (lo, hi) = (origin, origin + extent);
    cv.rect(Rect2::new(lo, Vec2::new(hi.x, b.min.y)), dark);
    cv.rect(Rect2::new(Vec2::new(lo.x, b.max.y), hi), dark);
    cv.rect(Rect2::new(Vec2::new(lo.x, b.min.y), Vec2::new(b.min.x, b.max.y)), dark);
    cv.rect(Rect2::new(Vec2::new(b.max.x, b.min.y), Vec2::new(hi.x, b.max.y)), dark);

    // 20 m graticule.
    let grid = [0.5, 0.55, 0.45, 0.16];
    let mut gx = (b.min.x / 20.0).ceil() * 20.0;
    while gx < b.max.x {
        cv.segment(Vec2::new(gx, b.min.y), Vec2::new(gx, b.max.y), 0.14, grid);
        gx += 20.0;
    }
    let mut gz = (b.min.y / 20.0).ceil() * 20.0;
    while gz < b.max.y {
        cv.segment(Vec2::new(b.min.x, gz), Vec2::new(b.max.x, gz), 0.14, grid);
        gz += 20.0;
    }

    for r in &d.grass {
        cv.rect(*r, [0.26, 0.34, 0.15, 0.85]);
    }
    for r in &d.water {
        cv.rect(*r, [0.07, 0.15, 0.22, 1.0]);
    }
    for r in &d.shallows {
        cv.rect(*r, [0.13, 0.26, 0.3, 1.0]);
    }
    // Groves and lone palms.
    for &g in &d.groves {
        cv.circle(g, 3.2, [0.1, 0.22, 0.11, 0.9]);
    }
    for &g in &d.palms {
        cv.circle(g, 1.1, [0.1, 0.24, 0.12, 0.9]);
    }
    // The dirt road and every worn route.
    cv.rect(layout.road, [0.4, 0.33, 0.22, 1.0]);
    for route in d.routes.iter().filter(|r| r.trail) {
        for w in route.points.windows(2) {
            cv.segment(w[0], w[1], route.width.max(1.4), [0.36, 0.3, 0.2, 0.95]);
        }
    }
    // Walkable planks.
    for s in &d.surfaces {
        cv.rect(s.rect, [0.5, 0.39, 0.24, 1.0]);
        cv.outline(s.rect, 0.25, [0.22, 0.16, 0.1, 1.0]);
    }
    // Property lines and yards.
    for r in &d.rails {
        cv.segment(r.a, r.b, 0.3, [0.06, 0.05, 0.04, 0.8]);
    }
    // Buildings.
    let wall = [0.05, 0.04, 0.035, 1.0];
    let roof = [0.42, 0.33, 0.26, 1.0];
    let fp = layout.house.footprint;
    cv.rect(fp, roof);
    cv.outline(fp, 0.35, wall);
    for s in &d.sheds {
        let r = Rect2::from_center(s.center, s.half);
        cv.rect(r, roof);
        cv.outline(r, 0.3, wall);
    }
    for p in &d.props {
        let r = Rect2::from_center(p.center, p.half);
        use crate::geometry::district::PropKind::*;
        match p.kind {
            Truck | Pickup => {
                cv.rect(r, [0.55, 0.3, 0.14, 1.0]);
                cv.outline(r, 0.25, wall);
            }
            TankTower | Well => cv.circle(p.center, p.half.x.max(p.half.y), [0.5, 0.52, 0.5, 1.0]),
            Pole | Cow => {}
            _ => cv.rect(r, [0.34, 0.27, 0.2, 1.0]),
        }
    }
    // The ceiba.
    cv.circle(
        layout.ceiba.center,
        layout.ceiba.canopy_radius.min(8.0),
        [0.08, 0.2, 0.1, 0.55],
    );
    cv.circle(
        layout.ceiba.center,
        layout.ceiba.trunk_radius + 0.3,
        [0.28, 0.2, 0.13, 1.0],
    );
    // Lamps.
    for l in &layout.light_sources {
        cv.circle(Vec2::new(l.pos.x, l.pos.z), 0.55, [1.0, 0.72, 0.3, 0.95]);
    }
    (cv, origin, extent)
}

pub(crate) fn build_map_image(mut commands: Commands, mut images: ResMut<Assets<Image>>, layout: Res<LayoutRes>) {
    let (canvas, origin, extent) = paint(&layout.0);
    let handle = images.add(canvas.into_image());
    commands.insert_resource(MapImage { handle, origin, extent });
}

pub(crate) fn spawn_map_panel(root: &mut ChildSpawnerCommands<'_>, f: &Fonts, map: &MapImage, layout: &Layout) {
    let ratio = map.extent.x / map.extent.y;
    root.spawn((
        MapPanel,
        overlay(),
        Visibility::Hidden,
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        GlobalZIndex(8),
    ))
    .with_children(|o| {
        o.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            height: percent(94),
            ..default()
        })
        .with_children(|c| {
            c.spawn((Text::new("HACIENDA SANTA ROSA"), font(&f.serif, 20.0), TextColor(AMBER)));
            c.spawn((
                Node {
                    height: percent(92),
                    aspect_ratio: Some(ratio),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.8, 0.62, 0.38, 0.6)),
                ImageNode::new(map.handle.clone()),
            ))
            .with_children(|frame| {
                for l in &layout.district.landmarks {
                    let uv = map.uv(l.center);
                    frame.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(uv.x * 100.0),
                            top: percent(uv.y * 100.0),
                            width: px(0),
                            height: px(0),
                            overflow: Overflow::visible(),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        children![(
                            Text::new(l.name),
                            font(&f.serif, 14.0),
                            TextColor(INK),
                            TextShadow::default(),
                            Node {
                                position_type: PositionType::Absolute,
                                top: px(-8),
                                ..default()
                            },
                        )],
                    ));
                }
                for i in 0..MAX_PINGS {
                    frame.spawn((
                        MapPing(i),
                        Visibility::Hidden,
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(10),
                            height: px(10),
                            margin: UiRect::all(px(-5)),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        BorderColor::all(Color::srgb(1.0, 0.9, 0.35)),
                        UiTransform::from_rotation(Rot2::degrees(45.0)),
                    ));
                }
                for i in 0..MAX_PLAYERS_ON_MAP {
                    frame
                        .spawn((
                            MapDot(i),
                            Visibility::Hidden,
                            Node {
                                position_type: PositionType::Absolute,
                                width: px(DOT * 2.0),
                                height: px(DOT * 2.0),
                                margin: UiRect::all(px(-DOT)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            UiTransform::IDENTITY,
                        ))
                        .with_children(|dot| {
                            dot.spawn((
                                Node {
                                    width: px(DOT * 0.7),
                                    height: px(DOT * 0.7),
                                    border_radius: BorderRadius::MAX,
                                    border: UiRect::all(px(1)),
                                    ..default()
                                },
                                BorderColor::all(Color::BLACK),
                                BackgroundColor(INK),
                            ));
                            // The nose: which way they face.
                            dot.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: px(0),
                                    width: px(5),
                                    height: px(7),
                                    ..default()
                                },
                                BackgroundColor(INK),
                            ));
                        });
                }
            });
            c.spawn((
                Text::new("N is up · you are the bright marker · V marks a spot for everyone · M closes"),
                font(&f.sans, 13.0),
                TextColor(DIM),
            ));
        });
    });
}

const MAX_PLAYERS_ON_MAP: usize = crate::net::protocol::MAX_PLAYERS;

pub(crate) fn toggle_map(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<Flow>>,
    launch: Res<Launch>,
    mut map: ResMut<MapOpen>,
) {
    if *state.get() != Flow::Playing {
        map.0 = false;
        return;
    }
    if !launch.smoke && !launch.photos && keys.just_pressed(KeyCode::KeyM) {
        map.0 = !map.0;
    }
}

pub(crate) fn update_map(map: Res<MapOpen>, state: Res<State<Flow>>, mut q: Query<&mut Visibility, With<MapPanel>>) {
    let open = map.0 && *state.get() == Flow::Playing;
    for mut v in &mut q {
        set_vis(&mut v, open);
    }
}

pub(crate) fn update_markers(
    map: Res<MapOpen>,
    image: Res<MapImage>,
    net: Res<Network>,
    player: Single<&Player>,
    time: Res<Time<Real>>,
    mut dots: Query<(&MapDot, &mut Node, &mut Visibility, &mut UiTransform, &Children), Without<MapPing>>,
    mut pings: Query<(&MapPing, &mut Node, &mut Visibility), Without<MapDot>>,
    mut backgrounds: Query<&mut BackgroundColor>,
) {
    if !map.0 {
        return;
    }
    let me = net.id();
    let players = net.snapshot().map_or(&[][..], |s| s.players.as_slice());
    for (dot, mut node, mut vis, mut tf, kids) in &mut dots {
        let Some(p) = players.get(dot.0) else {
            set_vis(&mut vis, false);
            continue;
        };
        let (pos, yaw) = if Some(p.id) == me {
            (player.pose.pos, player.pose.yaw)
        } else {
            (Vec2::from_array(p.position), p.yaw)
        };
        let uv = image.uv(pos);
        node.left = percent(uv.x * 100.0);
        node.top = percent(uv.y * 100.0);
        tf.rotation = Rot2::radians(-yaw);
        // The downed blink red.
        let blink = p.status == 1 && (time.elapsed_secs() * 4.0).sin() > 0.0;
        let color = match p.status {
            1 if blink => RED,
            1 => Color::srgb(0.5, 0.15, 0.12),
            2 => Color::srgb(0.3, 0.3, 0.3),
            _ => player_color(dot.0),
        };
        set_vis(&mut vis, true);
        for kid in kids.iter() {
            if let Ok(mut bg) = backgrounds.get_mut(kid) {
                bg.0 = color;
            }
        }
    }
    let list = net.snapshot().map_or(&[][..], |s| s.pings.as_slice());
    for (ping, mut node, mut vis) in &mut pings {
        match list.get(ping.0) {
            Some(p) => {
                let uv = image.uv(Vec2::new(p.pos[0], p.pos[2]));
                node.left = percent(uv.x * 100.0);
                node.top = percent(uv.y * 100.0);
                set_vis(&mut vis, true);
            }
            None => set_vis(&mut vis, false),
        }
    }
}
