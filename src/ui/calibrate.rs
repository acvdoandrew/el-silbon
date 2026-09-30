//! The brightness calibration's picture: three hats on the night's black,
//! held just in front of the player's own eye while the menu's
//! calibration page is up. Each is unlit at a level taken from the night
//! (`display::HATS` times the fog's level), so the same grade, tonemapping,
//! bloom and vignette the night goes through reach them: the left one
//! should vanish, the middle one barely show, the right one read plainly.
//! Presentation only; the page's words and rows are in `menu`.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::menu::Menu;
use super::set_vis;
use crate::app::{Flow, GameSet};
use crate::player::Player;
use crate::world::mesh::{MeshBuilder, WHITE};

/// Where the hats stand across the screen (normalised device x, −1 at the
/// left edge) and the height of their brims' tips (normalised device y).
pub(crate) const HAT_X: [f32; 3] = [-0.4, 0.0, 0.4];
const HAT_Y: f32 = 0.25;
/// A hat's width, as a share of the screen's height.
const HAT_WIDTH: f32 = 0.22;
/// Where the labels under the hats begin (percent of the screen from the top).
pub(crate) const LABEL_TOP: f32 = (1.0 - HAT_Y) * 50.0 + 2.0;
/// How far in front of the eye the black and the hats hang (m): past the
/// near plane (0.05 m), nearer than the torch in hand (0.2 m) and the
/// satchel, so nothing of the night shows through.
const BACKDROP: f32 = 0.1;
const HATS: f32 = 0.098;

#[derive(Component)]
struct CalibrationCard;

/// One of the three hats, left to right.
#[derive(Component)]
struct Hat(usize);

/// A llanero's hat seen side-on, flat and facing the eye: a broad brim
/// drooping at both ends under a low, rounded crown. One unit wide,
/// centred, with the brim's tips at the origin.
fn hat() -> Mesh {
    let mut m = MeshBuilder::new();
    // The brim: a band across, thickest in the middle, its ends drooping.
    const SEGMENTS: usize = 16;
    let brim = |s: f32, top: bool| {
        let x = s - 0.5;
        let middle = 1.0 - (2.0 * x).powi(2);
        let under = 0.05 * middle;
        Vec3::new(x, if top { under + 0.02 + 0.05 * middle } else { under }, 0.0)
    };
    for i in 0..SEGMENTS {
        let (a, b) = (i as f32 / SEGMENTS as f32, (i + 1) as f32 / SEGMENTS as f32);
        m.quad(
            [brim(a, false), brim(b, false), brim(b, true), brim(a, true)],
            [
                Vec2::new(a, 1.0),
                Vec2::new(b, 1.0),
                Vec2::new(b, 0.0),
                Vec2::new(a, 0.0),
            ],
            WHITE,
        );
    }
    // The crown: tapering up from the brim to a rounded top, as a fan.
    let mut outline = vec![Vec2::new(-0.2, 0.1), Vec2::new(0.2, 0.1)];
    outline.extend((0..=8).map(|j| {
        let a = j as f32 / 8.0 * std::f32::consts::PI;
        Vec2::new(0.17 * a.cos(), 0.3 + 0.1 * a.sin())
    }));
    let centre = m.vertex(Vec3::new(0.0, 0.22, 0.0), Vec3::Z, Vec2::splat(0.5), WHITE);
    let rim: Vec<u32> = outline
        .iter()
        .map(|p| m.vertex(p.extend(0.0), Vec3::Z, *p, WHITE))
        .collect();
    for i in 0..rim.len() {
        m.tri(centre, rim[i], rim[(i + 1) % rim.len()]);
    }
    m.build()
}

/// The card, hidden, as a child of the player's camera (spawned in
/// `Startup`, hence `PostStartup` here).
fn spawn_card(
    mut commands: Commands,
    camera: Single<Entity, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Unlit and out of the fog: each writes its level straight into the
    // picture, before the grade, as the sky and the fog do.
    let mut level = |v: f32| {
        materials.add(StandardMaterial {
            base_color: Color::linear_rgb(v, v, v),
            unlit: true,
            fog_enabled: false,
            cull_mode: None,
            ..default()
        })
    };
    let black = level(0.0);
    let fog = crate::player::fog_level();
    let hats: Vec<_> = crate::display::HATS.iter().map(|&share| level(share * fog)).collect();
    let shape = meshes.add(hat());
    let backdrop = meshes.add(Rectangle::new(4.0, 4.0));
    let card = commands
        .spawn((
            Name::new("calibration card"),
            CalibrationCard,
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .with_children(|card| {
            card.spawn((
                Mesh3d(backdrop),
                MeshMaterial3d(black),
                Transform::from_xyz(0.0, 0.0, -BACKDROP),
                NotShadowCaster,
                NotShadowReceiver,
            ));
            for (i, material) in hats.into_iter().enumerate() {
                card.spawn((
                    Hat(i),
                    Mesh3d(shape.clone()),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.0, -HATS),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
            }
        })
        .id();
    commands.entity(*camera).add_child(card);
}

/// The card shows while the calibration page is up, its hats placed from
/// the live field of view and shape of the screen, so they stand over
/// their labels whatever either is.
fn show_card(
    state: Res<State<Flow>>,
    menu: Res<Menu>,
    mut card: Single<&mut Visibility, With<CalibrationCard>>,
    camera: Single<&Projection, With<Player>>,
    mut hats: Query<(&Hat, &mut Transform)>,
) {
    let up = menu.calibrating(*state.get());
    set_vis(&mut card, up);
    let Projection::Perspective(p) = *camera else {
        return;
    };
    if !up {
        return;
    }
    // Half the screen's height at the hats' distance.
    let half = HATS * (p.fov * 0.5).tan();
    for (hat, mut tf) in &mut hats {
        let want = Transform::from_xyz(HAT_X[hat.0] * half * p.aspect_ratio, HAT_Y * half, -HATS)
            .with_scale(Vec3::splat(HAT_WIDTH * 2.0 * half));
        if *tf != want {
            *tf = want;
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostStartup, spawn_card)
        .add_systems(Update, show_card.in_set(GameSet::Present));
}
