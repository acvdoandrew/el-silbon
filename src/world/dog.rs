//! Tureco, as everyone sees him from the snapshot: a lean brown llanero dog.
//! Tied by a rope to the post behind the house until someone unties him;
//! then trotting at his friend's heels, head low and growling when the
//! Silbón is near, head thrown up when he barks.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::mesh::{MeshBuilder, Ring, srgb};
use crate::app::LayoutRes;
use crate::net::Network;

#[derive(Component)]
pub struct DogRoot {
    /// Leg swing phase, and how far he has gone since last frame.
    phase: f32,
    last: Vec2,
}

#[derive(Component)]
pub struct DogHead;

#[derive(Component)]
pub struct DogLeg(pub f32);

/// The model's wrists and hocks: the gait phase, and whether it is a
/// fore leg (the paw folds back) or a hind leg (the hock flexes).
#[derive(Component)]
pub struct DogPaw(pub f32, pub bool);

/// The model's jaw and tail (the procedural dog has neither).
#[derive(Component)]
pub struct DogJaw;
#[derive(Component)]
pub struct DogTail;

#[derive(Component)]
pub struct DogRope;

fn body_mesh() -> Mesh {
    let fur = srgb(0.3, 0.19, 0.1);
    let belly = srgb(0.5, 0.38, 0.25);
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::new(0.0, 0.52, 0.0),
        Vec3::new(0.15, 0.16, 0.36),
        8,
        12,
        1.0,
        fur,
        &|d: Vec3| {
            if d.y < -0.5 { 0.92 } else { 1.0 }
        },
    );
    m.blob(
        Vec3::new(0.0, 0.44, 0.02),
        Vec3::new(0.1, 0.07, 0.24),
        6,
        8,
        1.0,
        belly,
        &|_| 1.0,
    );
    // The tail, curled up a little behind.
    m.tube(
        &[
            Ring {
                center: Vec3::new(0.0, 0.6, 0.33),
                radius: 0.035,
                color: fur,
            },
            Ring {
                center: Vec3::new(0.0, 0.72, 0.45),
                radius: 0.028,
                color: fur,
            },
            Ring {
                center: Vec3::new(0.0, 0.8, 0.5),
                radius: 0.012,
                color: belly,
            },
        ],
        6,
        1.0,
        1.0,
        true,
        &|_, _| 1.0,
    );
    m.build()
}

fn head_mesh() -> Mesh {
    let fur = srgb(0.3, 0.19, 0.1);
    let dark = srgb(0.1, 0.07, 0.05);
    let mut m = MeshBuilder::new();
    m.blob(Vec3::ZERO, Vec3::new(0.09, 0.09, 0.1), 6, 10, 1.0, fur, &|_| 1.0);
    // Snout, nose, and two pricked ears.
    m.blob(
        Vec3::new(0.0, -0.02, -0.12),
        Vec3::new(0.05, 0.045, 0.08),
        5,
        8,
        1.0,
        fur,
        &|_| 1.0,
    );
    m.blob(
        Vec3::new(0.0, -0.005, -0.2),
        Vec3::splat(0.022),
        4,
        6,
        1.0,
        dark,
        &|_| 1.0,
    );
    for x in [-0.05_f32, 0.05] {
        m.tube(
            &[
                Ring {
                    center: Vec3::new(x, 0.06, 0.0),
                    radius: 0.03,
                    color: fur,
                },
                Ring {
                    center: Vec3::new(x * 1.3, 0.14, 0.01),
                    radius: 0.005,
                    color: dark,
                },
            ],
            5,
            1.0,
            1.0,
            true,
            &|_, _| 1.0,
        );
    }
    m.build()
}

fn leg_mesh() -> Mesh {
    let fur = srgb(0.26, 0.17, 0.09);
    let mut m = MeshBuilder::new();
    m.tube(
        &[
            Ring {
                center: Vec3::ZERO,
                radius: 0.035,
                color: fur,
            },
            Ring {
                center: Vec3::new(0.0, -0.22, 0.0),
                radius: 0.025,
                color: fur,
            },
            Ring {
                center: Vec3::new(0.0, -0.42, -0.02),
                radius: 0.022,
                color: fur,
            },
        ],
        6,
        1.0,
        1.0,
        true,
        &|_, _| 1.0,
    );
    m.build()
}

pub fn spawn_dog(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    layout: Res<LayoutRes>,
) {
    let post = layout.0.district.dog_post;
    let fur = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        ..default()
    });
    let rope = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.45, 0.3),
        perceptual_roughness: 0.9,
        ..default()
    });
    let ground = layout.0.surface_height(post);
    let at = Vec3::new(post.x, ground, post.y);
    // The post he is tied to, and the rope (stretched from post to collar).
    let mut stake = MeshBuilder::new();
    stake.cuboid(
        at + Vec3::new(-0.5, 0.45, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.05, 0.45, 0.05),
        1.0,
        Vec2::ZERO,
        srgb(0.4, 0.3, 0.2),
    );
    commands.spawn((
        Name::new("Tureco's post"),
        Mesh3d(meshes.add(stake.build())),
        MeshMaterial3d(rope.clone()),
    ));
    commands.spawn((
        Name::new("Tureco's rope"),
        DogRope,
        Mesh3d(meshes.add(Cuboid::new(0.02, 0.02, 1.0))),
        MeshMaterial3d(rope),
        NotShadowCaster,
        Transform::from_translation(at),
    ));
    let head = meshes.add(head_mesh());
    let leg = meshes.add(leg_mesh());
    commands
        .spawn((
            Name::new("Tureco"),
            DogRoot { phase: 0.0, last: post },
            Mesh3d(meshes.add(body_mesh())),
            MeshMaterial3d(fur.clone()),
            Transform::from_translation(at),
            Visibility::Inherited,
        ))
        .with_children(|d| {
            d.spawn((
                DogHead,
                Mesh3d(head),
                MeshMaterial3d(fur.clone()),
                Transform::from_xyz(0.0, 0.68, -0.38),
            ));
            for (i, (x, z)) in [(-0.08_f32, -0.24_f32), (0.08, -0.24), (-0.08, 0.24), (0.08, 0.24)]
                .into_iter()
                .enumerate()
            {
                let offset = if i == 0 || i == 3 { 0.0 } else { std::f32::consts::PI };
                d.spawn((
                    DogLeg(offset),
                    Mesh3d(leg.clone()),
                    MeshMaterial3d(fur.clone()),
                    Transform::from_xyz(x, 0.46, z),
                ));
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn animate_dog(
    time: Res<Time>,
    net: Res<Network>,
    layout: Res<LayoutRes>,
    mut jaw: Query<
        &mut Transform,
        (
            With<DogJaw>,
            Without<DogRoot>,
            Without<DogHead>,
            Without<DogLeg>,
            Without<DogRope>,
        ),
    >,
    mut tail: Query<
        &mut Transform,
        (
            With<DogTail>,
            Without<DogJaw>,
            Without<DogRoot>,
            Without<DogHead>,
            Without<DogLeg>,
            Without<DogRope>,
        ),
    >,
    root: Single<(&mut DogRoot, &mut Transform), (Without<DogHead>, Without<DogLeg>, Without<DogRope>)>,
    mut head: Query<&mut Transform, (With<DogHead>, Without<DogLeg>, Without<DogRope>)>,
    mut legs: Query<(&DogLeg, &mut Transform), (Without<DogHead>, Without<DogRope>)>,
    mut rope: Query<(&mut Transform, &mut Visibility), (With<DogRope>, Without<DogHead>, Without<DogLeg>)>,
    mut paws: Query<
        (&DogPaw, &mut Transform),
        (
            Without<DogLeg>,
            Without<DogHead>,
            Without<DogRope>,
            Without<DogJaw>,
            Without<DogTail>,
            Without<DogRoot>,
        ),
    >,
) {
    let Some(view) = net.snapshot().map(|s| s.dog) else {
        return;
    };
    let dt = time.delta_secs().max(1e-4);
    let t = time.elapsed_secs();
    let (mut dog, mut tf) = root.into_inner();
    let pos = Vec2::from_array(view.pos);
    let facing = Vec2::from_array(view.facing).normalize_or(Vec2::Y);
    let moved = pos.distance(dog.last);
    dog.last = pos;
    let pace = (moved / dt / 3.0).clamp(0.0, 1.0);
    dog.phase = (dog.phase + moved * 9.0) % std::f32::consts::TAU;
    let ground = layout.0.surface_height(pos);
    let tied = view.mood == 0;
    // Tied, he sits and waits by the post; free, he trots with a bob.
    let bob = 0.02 * (dog.phase * 2.0).sin().abs() * pace;
    let sit = if tied { -0.12 } else { 0.0 };
    let want = Vec3::new(pos.x, ground + bob + sit, pos.y);
    tf.translation = if tf.translation.distance(want) > 3.0 {
        want
    } else {
        tf.translation.lerp(want, (dt * 12.0).min(1.0))
    };
    let yaw = (-facing.x).atan2(-facing.y);
    let target = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(if tied { 0.25 } else { 0.0 });
    tf.rotation = tf.rotation.slerp(target, (dt * 8.0).min(1.0));
    for mut h in &mut head {
        let (dip, jerk) = match view.mood {
            2 => (-0.35, 0.0),
            3 => (0.2, 0.15 * (t * 30.0).sin()),
            _ => (0.05 * (t * 1.3).sin(), 0.0),
        };
        h.rotation = Quat::from_rotation_x(dip + jerk);
    }
    // He snaps his jaws when he barks; his tail wags when he follows and
    // drops, stiff, when he growls.
    for mut j in &mut jaw {
        let open = if view.mood == 3 {
            0.35 * (t * 18.0).sin().max(0.0)
        } else {
            0.0
        };
        j.rotation = Quat::from_rotation_x(open);
    }
    for mut tl in &mut tail {
        let (lift, wag) = match view.mood {
            1 => (0.5, 0.6 * (t * 11.0).sin() * (0.4 + 0.6 * pace)),
            2 | 3 => (-0.3, 0.05 * (t * 30.0).sin()),
            _ => (0.1, 0.15 * (t * 2.0).sin()),
        };
        tl.rotation = Quat::from_rotation_y(wag) * Quat::from_rotation_x(lift);
    }
    for (leg, mut l) in &mut legs {
        let swing = if tied {
            0.0
        } else {
            0.6 * (dog.phase + leg.0).sin() * pace
        };
        l.rotation = Quat::from_rotation_x(swing);
    }
    // The lower legs fold as each foot comes through: the fore paws flick
    // back, the hocks bend; tied, he sits on his haunches.
    for (paw, mut p) in &mut paws {
        let lift = (dog.phase + paw.0).cos().max(0.0) * pace;
        let fold = match (paw.1, tied) {
            (true, _) => -0.9 * lift,
            (false, true) => 0.5,
            (false, false) => 0.7 * lift,
        };
        p.rotation = Quat::from_rotation_x(fold);
    }
    // The rope runs from the post to his collar while he is tied.
    let post = layout.0.district.dog_post;
    let post3 = Vec3::new(post.x - 0.5, ground + 0.75, post.y);
    for (mut r, mut vis) in &mut rope {
        let want_vis = if tied {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
        if tied {
            let collar = tf.translation + tf.rotation * Vec3::new(0.0, 0.62, -0.3);
            let mid = (post3 + collar) * 0.5;
            let span = collar - post3;
            r.translation = mid;
            r.rotation = Quat::from_rotation_arc(Vec3::Z, span.normalize_or(Vec3::Z));
            r.scale = Vec3::new(1.0, 1.0, span.length().max(0.05));
        }
    }
}
