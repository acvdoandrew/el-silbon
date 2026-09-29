//! The extraction truck and the derelict ranch pickup: original low-poly
//! vintage vehicles, merged per material. The truck is a live entity (its
//! lamps come on and its frame shivers while the engine runs); the pickup is
//! scenery.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::SpawnCtx;
use super::mesh::{MeshBuilder, Rgba, Ring, scale_rgb, srgb};
use crate::geometry::district::PropKind;

/// The extraction truck's root: it shivers while the engine runs.
#[derive(Component)]
pub struct TruckRoot {
    pub rest: Vec3,
}

/// A truck lamp that shines only while the engine runs.
#[derive(Component)]
pub struct TruckLamp {
    pub base: f32,
}

/// The lens materials the truck switches when the engine starts.
#[derive(Resource, Clone)]
pub struct TruckAssets {
    pub headlamp: Handle<StandardMaterial>,
    pub taillamp: Handle<StandardMaterial>,
}

#[derive(Default)]
struct Parts {
    paint: MeshBuilder,
    canvas: MeshBuilder,
    rubber: MeshBuilder,
    metal: MeshBuilder,
    glass: MeshBuilder,
    wood: MeshBuilder,
    lens: MeshBuilder,
    tail: MeshBuilder,
    cargo: MeshBuilder,
}

fn bx(m: &mut MeshBuilder, c: Vec3, half: Vec3, tint: Rgba) {
    m.cuboid(c, Quat::IDENTITY, half, 1.0, Vec2::ZERO, tint);
}

fn bm(m: &mut MeshBuilder, a: Vec3, b: Vec3, w: f32, h: f32, tint: Rgba) {
    m.beam(a, b, w, h, 0.0, 1.0, Vec2::ZERO, tint);
}

/// A cylinder along Z, `outer` being the end that gets the cap.
fn drum(m: &mut MeshBuilder, center: Vec3, radius: f32, half_w: f32, outer: f32, tint: Rgba) {
    let z = |k: f32| center + Vec3::Z * (half_w * k * outer);
    let rings = [
        Ring {
            center: z(-1.0),
            radius: radius * 0.88,
            color: tint,
        },
        Ring {
            center: z(-0.8),
            radius,
            color: tint,
        },
        Ring {
            center: z(0.8),
            radius,
            color: tint,
        },
        Ring {
            center: z(1.0),
            radius: radius * 0.88,
            color: tint,
        },
    ];
    m.tube(&rings, 16, 1.0, 1.0, true, &|_, _| 1.0);
}

/// A lens or hub disc facing +X (or -X) with a slight dome.
fn lamp_disc(m: &mut MeshBuilder, center: Vec3, radius: f32, depth: f32, dir: f32) {
    let rings = [
        Ring {
            center,
            radius,
            color: [1.0; 4],
        },
        Ring {
            center: center + Vec3::X * depth * dir,
            radius: radius * 0.9,
            color: [1.0; 4],
        },
    ];
    m.tube(&rings, 12, 1.0, 1.0, true, &|_, _| 1.0);
}

fn wheel(p: &mut Parts, at: Vec3, radius: f32, side: f32) {
    let rubber = srgb(0.06, 0.06, 0.055);
    drum(&mut p.rubber, at, radius, 0.16, side, rubber);
    drum(
        &mut p.metal,
        at + Vec3::Z * side * 0.03,
        radius * 0.5,
        0.17,
        side,
        srgb(0.3, 0.27, 0.22),
    );
    // Lug nuts.
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU;
        bx(
            &mut p.metal,
            at + Vec3::new(a.cos() * radius * 0.28, a.sin() * radius * 0.28, side * 0.2),
            Vec3::splat(0.02),
            srgb(0.35, 0.32, 0.28),
        );
    }
}

/// A vintage cargo truck facing +X on the ground plane.
fn truck(p: &mut Parts) {
    let paint = srgb(0.34, 0.38, 0.27);
    let paint_worn = srgb(0.42, 0.36, 0.27);
    let dark = srgb(0.1, 0.1, 0.09);
    let canvas = srgb(0.5, 0.47, 0.36);

    // Ladder frame and cross members.
    for z in [-0.62, 0.62] {
        bm(
            &mut p.metal,
            Vec3::new(-2.65, 0.7, z),
            Vec3::new(2.6, 0.7, z),
            0.14,
            0.2,
            dark,
        );
    }
    for x in [-2.4, -1.0, 0.3, 1.6] {
        bm(
            &mut p.metal,
            Vec3::new(x, 0.68, -0.62),
            Vec3::new(x, 0.68, 0.62),
            0.1,
            0.12,
            dark,
        );
    }
    // Wheels: a tandem at the back, a steer axle under the hood.
    for (x, r) in [(-1.7, 0.6), (-0.5, 0.6), (1.85, 0.58)] {
        for side in [-1.0, 1.0] {
            wheel(p, Vec3::new(x, r, side * 1.0), r, side);
        }
        bm(
            &mut p.metal,
            Vec3::new(x, r, -1.0),
            Vec3::new(x, r, 1.0),
            0.09,
            0.09,
            dark,
        );
    }
    // Bed: planked floor, side boards, headboard, tailgate.
    bx(
        &mut p.wood,
        Vec3::new(-1.3, 0.98, 0.0),
        Vec3::new(1.45, 0.05, 0.98),
        srgb(0.5, 0.4, 0.3),
    );
    for z in [-0.98, 0.98] {
        bx(
            &mut p.paint,
            Vec3::new(-1.3, 1.3, z),
            Vec3::new(1.45, 0.29, 0.04),
            paint_worn,
        );
        bm(
            &mut p.metal,
            Vec3::new(-2.75, 1.6, z),
            Vec3::new(0.15, 1.6, z),
            0.06,
            0.06,
            dark,
        );
    }
    bx(
        &mut p.paint,
        Vec3::new(0.12, 1.38, 0.0),
        Vec3::new(0.04, 0.4, 0.98),
        paint,
    );
    bx(
        &mut p.paint,
        Vec3::new(-2.72, 1.26, 0.0),
        Vec3::new(0.03, 0.26, 0.94),
        paint_worn,
    );
    // Canvas tilt over hoops.
    let length = 2.9;
    let x0 = -2.7;
    p.canvas.grid(
        Vec3::new(x0, 1.55, -0.95),
        Vec3::X * length,
        Vec3::Z * 1.9,
        24,
        18,
        Vec2::splat(0.6),
        &|u, v| {
            let s = 2.0 * v - 1.0;
            let arch = (1.0 - s * s).max(0.0).sqrt();
            // Sag between hoops, a little more at the back.
            let sag = -0.04 * (u * 5.0 * std::f32::consts::PI).sin().abs();
            (
                Vec3::new(0.0, 0.72 * arch + sag - 0.5 * s * s * 0.0, 0.0),
                scale_rgb(canvas, 0.85 + 0.15 * arch),
            )
        },
    );
    for k in 0..5 {
        let x = x0 + 0.1 + k as f32 * (length - 0.2) / 4.0;
        let pts: Vec<Vec3> = (0..=10)
            .map(|i| {
                let a = i as f32 / 10.0 * std::f32::consts::PI;
                Vec3::new(x, 1.55 + 0.72 * a.sin(), -0.95 * a.cos())
            })
            .collect();
        for w in pts.windows(2) {
            bm(&mut p.wood, w[0], w[1], 0.05, 0.05, srgb(0.3, 0.24, 0.18));
        }
    }
    // Cab: lower body, greenhouse, roof.
    bx(
        &mut p.paint,
        Vec3::new(0.9, 1.32, 0.0),
        Vec3::new(0.62, 0.36, 0.95),
        paint,
    );
    bx(
        &mut p.paint,
        Vec3::new(0.85, 1.9, 0.0),
        Vec3::new(0.55, 0.26, 0.86),
        paint,
    );
    bx(
        &mut p.paint,
        Vec3::new(0.85, 2.17, 0.0),
        Vec3::new(0.66, 0.04, 0.92),
        paint_worn,
    );
    // Windscreen (leaning back) and side lights.
    p.glass.quad(
        [
            Vec3::new(1.4, 1.68, 0.78),
            Vec3::new(1.4, 1.68, -0.78),
            Vec3::new(1.3, 2.1, -0.78),
            Vec3::new(1.3, 2.1, 0.78),
        ],
        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
        [1.0; 4],
    );
    for z in [-0.875, 0.875] {
        let n = if z < 0.0 { -1.0 } else { 1.0 };
        let q = [
            Vec3::new(0.4, 1.72, z),
            Vec3::new(1.25, 1.72, z),
            Vec3::new(1.25, 2.08, z),
            Vec3::new(0.4, 2.08, z),
        ];
        let mut q = q;
        if n < 0.0 {
            q.swap(0, 1);
            q.swap(2, 3);
        }
        p.glass.quad(q, [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y], [1.0; 4]);
        // Door seam, handle, running board and mirror.
        bm(
            &mut p.metal,
            Vec3::new(0.35, 1.0, z * 1.01),
            Vec3::new(0.35, 2.1, z * 1.01),
            0.02,
            0.02,
            dark,
        );
        bm(
            &mut p.metal,
            Vec3::new(1.3, 1.0, z * 1.01),
            Vec3::new(1.3, 2.1, z * 1.01),
            0.02,
            0.02,
            dark,
        );
        bx(
            &mut p.metal,
            Vec3::new(0.5, 1.5, z * 1.03),
            Vec3::new(0.06, 0.02, 0.02),
            srgb(0.5, 0.45, 0.38),
        );
        bx(
            &mut p.metal,
            Vec3::new(0.85, 0.85, z * 1.12),
            Vec3::new(0.62, 0.025, 0.14),
            dark,
        );
        bm(
            &mut p.metal,
            Vec3::new(1.32, 1.72, z * 0.97),
            Vec3::new(1.36, 1.85, z * 1.24),
            0.025,
            0.025,
            dark,
        );
        bx(
            &mut p.metal,
            Vec3::new(1.36, 1.86, z * 1.27),
            Vec3::new(0.02, 0.1, 0.07),
            dark,
        );
    }
    // Hood, wings, grille, bumper.
    bx(
        &mut p.paint,
        Vec3::new(2.13, 1.32, 0.0),
        Vec3::new(0.56, 0.24, 0.6),
        paint,
    );
    for z in [-0.85, 0.85] {
        bx(
            &mut p.paint,
            Vec3::new(2.0, 1.06, z),
            Vec3::new(0.78, 0.05, 0.2),
            paint_worn,
        );
        bx(
            &mut p.paint,
            Vec3::new(1.85, 1.16, z * 1.05),
            Vec3::new(0.5, 0.045, 0.16),
            paint_worn,
        );
    }
    bx(
        &mut p.metal,
        Vec3::new(2.7, 1.28, 0.0),
        Vec3::new(0.03, 0.27, 0.42),
        dark,
    );
    for k in 0..7 {
        let z = -0.36 + k as f32 * 0.12;
        bm(
            &mut p.metal,
            Vec3::new(2.74, 1.06, z),
            Vec3::new(2.74, 1.5, z),
            0.03,
            0.04,
            srgb(0.4, 0.37, 0.32),
        );
    }
    bm(
        &mut p.metal,
        Vec3::new(2.8, 0.8, -1.0),
        Vec3::new(2.8, 0.8, 1.0),
        0.1,
        0.14,
        srgb(0.3, 0.27, 0.22),
    );
    bm(
        &mut p.metal,
        Vec3::new(-2.8, 0.85, -0.95),
        Vec3::new(-2.8, 0.85, 0.95),
        0.08,
        0.12,
        srgb(0.3, 0.27, 0.22),
    );
    // Headlamps and taillamps.
    for z in [-0.7, 0.7] {
        bx(
            &mut p.metal,
            Vec3::new(2.62, 1.22, z),
            Vec3::new(0.06, 0.16, 0.16),
            dark,
        );
        lamp_disc(&mut p.lens, Vec3::new(2.66, 1.22, z), 0.13, 0.05, 1.0);
        lamp_disc(&mut p.tail, Vec3::new(-2.74, 1.12, z * 1.15), 0.06, -0.04, 1.0);
    }
    // Exhaust stack behind the cab and a fuel can on the running board.
    bm(
        &mut p.metal,
        Vec3::new(0.2, 0.9, -1.02),
        Vec3::new(0.2, 2.45, -1.02),
        0.09,
        0.09,
        dark,
    );
    bx(
        &mut p.metal,
        Vec3::new(0.7, 0.98, -1.15),
        Vec3::new(0.14, 0.2, 0.06),
        srgb(0.28, 0.32, 0.22),
    );
    // Rust patches and a tow hook.
    for (x, y, z, w) in [(0.9, 1.05, 0.96, 0.3), (2.15, 1.5, 0.3, 0.25), (-1.2, 1.5, -0.99, 0.5)] {
        bx(
            &mut p.paint,
            Vec3::new(x, y, z),
            Vec3::new(w, 0.1, 0.006),
            srgb(0.5, 0.26, 0.14),
        );
    }
    bm(
        &mut p.metal,
        Vec3::new(2.8, 0.7, 0.0),
        Vec3::new(3.0, 0.62, 0.0),
        0.05,
        0.05,
        dark,
    );
}

/// A rusted 1950s pickup with a tarp over its load, facing +X.
fn pickup(p: &mut Parts) {
    let paint = srgb(0.3, 0.27, 0.22);
    let worn = srgb(0.42, 0.28, 0.2);
    let dark = srgb(0.09, 0.09, 0.08);
    for (x, r) in [(-1.35, 0.5), (1.4, 0.5)] {
        for side in [-1.0, 1.0] {
            wheel(p, Vec3::new(x, r, side * 0.92), r, side);
        }
    }
    for z in [-0.6, 0.6] {
        bm(
            &mut p.metal,
            Vec3::new(-2.3, 0.65, z),
            Vec3::new(2.3, 0.65, z),
            0.12,
            0.16,
            dark,
        );
    }
    // Cab set back, long hood, open bed with a tarp.
    bx(
        &mut p.paint,
        Vec3::new(0.2, 1.15, 0.0),
        Vec3::new(0.7, 0.42, 0.88),
        paint,
    );
    bx(&mut p.paint, Vec3::new(0.1, 1.8, 0.0), Vec3::new(0.55, 0.27, 0.8), worn);
    bx(
        &mut p.paint,
        Vec3::new(0.1, 2.08, 0.0),
        Vec3::new(0.6, 0.035, 0.86),
        paint,
    );
    p.glass.quad(
        [
            Vec3::new(0.62, 1.55, 0.72),
            Vec3::new(0.62, 1.55, -0.72),
            Vec3::new(0.55, 1.98, -0.72),
            Vec3::new(0.55, 1.98, 0.72),
        ],
        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
        [1.0; 4],
    );
    bx(
        &mut p.paint,
        Vec3::new(1.55, 1.12, 0.0),
        Vec3::new(0.85, 0.32, 0.6),
        worn,
    );
    for z in [-0.82, 0.82] {
        bx(&mut p.paint, Vec3::new(1.5, 0.88, z), Vec3::new(0.95, 0.05, 0.2), paint);
    }
    bx(
        &mut p.metal,
        Vec3::new(2.4, 1.05, 0.0),
        Vec3::new(0.03, 0.27, 0.42),
        dark,
    );
    bm(
        &mut p.metal,
        Vec3::new(2.5, 0.72, -0.9),
        Vec3::new(2.5, 0.72, 0.9),
        0.09,
        0.12,
        srgb(0.3, 0.27, 0.22),
    );
    for z in [-0.6, 0.6] {
        lamp_disc(&mut p.metal, Vec3::new(2.35, 1.0, z), 0.12, 0.06, 1.0);
    }
    bx(
        &mut p.wood,
        Vec3::new(-1.35, 0.9, 0.0),
        Vec3::new(1.05, 0.04, 0.86),
        srgb(0.4, 0.32, 0.24),
    );
    for z in [-0.86, 0.86] {
        bx(
            &mut p.paint,
            Vec3::new(-1.35, 1.12, z),
            Vec3::new(1.05, 0.24, 0.035),
            worn,
        );
    }
    bx(
        &mut p.paint,
        Vec3::new(-2.38, 1.1, 0.0),
        Vec3::new(0.03, 0.22, 0.83),
        paint,
    );
    // Tarp over crates, roped down.
    p.canvas.grid(
        Vec3::new(-2.25, 1.02, -0.8),
        Vec3::X * 1.8,
        Vec3::Z * 1.6,
        14,
        10,
        Vec2::splat(0.6),
        &|u, v| {
            let s = 2.0 * v - 1.0;
            let lump = (u * 7.0).sin() * 0.04 + (v * 9.0).cos() * 0.03;
            (
                Vec3::new(
                    0.0,
                    0.5 * (1.0 - s * s).max(0.0).sqrt() * (0.6 + 0.4 * (u * 3.1).sin().abs()) + lump,
                    0.0,
                ),
                srgb(0.22, 0.26, 0.2),
            )
        },
    );
    for x in [-2.0, -1.5, -1.0, -0.6] {
        bm(
            &mut p.wood,
            Vec3::new(x, 1.0, -0.82),
            Vec3::new(x, 1.0, 0.82),
            0.02,
            0.02,
            srgb(0.4, 0.32, 0.2),
        );
    }
    bx(
        &mut p.cargo,
        Vec3::new(-2.1, 1.1, 0.35),
        Vec3::new(0.22, 0.14, 0.2),
        srgb(0.4, 0.3, 0.2),
    );
}

fn spawn_parts(
    ctx: &mut SpawnCtx,
    name: &'static str,
    parts: Parts,
    transform: Transform,
    materials: &Materials,
) -> Entity {
    let Parts {
        paint,
        canvas,
        rubber,
        metal,
        glass,
        wood,
        lens,
        tail,
        cargo,
    } = parts;
    let layers = [
        (paint, &materials.paint, true),
        (canvas, &materials.canvas, true),
        (rubber, &materials.rubber, true),
        (metal, &materials.metal, true),
        (glass, &materials.glass, false),
        (wood, &materials.wood, true),
        (lens, &materials.headlamp, false),
        (tail, &materials.taillamp, false),
        (cargo, &materials.wood, true),
    ];
    let root = ctx
        .commands
        .spawn((Name::new(name), transform, Visibility::default()))
        .id();
    for (mesh, material, casts) in layers {
        if mesh.is_empty() {
            continue;
        }
        let handle = ctx.meshes.add(mesh.build());
        let mut e = ctx.commands.spawn((
            Mesh3d(handle),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
            ChildOf(root),
        ));
        if !casts {
            e.insert(NotShadowCaster);
        }
    }
    root
}

struct Materials {
    paint: Handle<StandardMaterial>,
    canvas: Handle<StandardMaterial>,
    rubber: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    headlamp: Handle<StandardMaterial>,
    taillamp: Handle<StandardMaterial>,
}

pub fn spawn(ctx: &mut SpawnCtx, materials: &mut Assets<StandardMaterial>) {
    let mats = Materials {
        paint: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.5,
            metallic: 0.3,
            reflectance: 0.5,
            ..default()
        }),
        canvas: ctx.palette.cloth.clone(),
        rubber: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        }),
        metal: ctx.palette.tin.clone(),
        glass: ctx.palette.glass.clone(),
        wood: ctx.palette.wood.clone(),
        headlamp: materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.82, 0.7),
            emissive: LinearRgba::BLACK,
            perceptual_roughness: 0.15,
            ..default()
        }),
        taillamp: materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.05, 0.04),
            emissive: LinearRgba::BLACK,
            perceptual_roughness: 0.2,
            ..default()
        }),
    };
    let d = &ctx.layout.district;
    // The extraction truck, nose toward the bridge.
    let t = d.truck;
    let rest = Vec3::new(t.center.x, 0.0, t.center.y);
    let mut parts = Parts::default();
    truck(&mut parts);
    let root = spawn_parts(ctx, "extraction truck", parts, Transform::from_translation(rest), &mats);
    ctx.commands.entity(root).insert(TruckRoot { rest });
    // Headlamps and taillamps: dark until the engine runs.
    for z in [-0.7, 0.7] {
        ctx.commands.spawn((
            Name::new("truck headlamp"),
            SpotLight {
                color: Color::srgb(1.0, 0.9, 0.72),
                intensity: 0.0,
                range: 45.0,
                radius: 0.1,
                inner_angle: 0.22,
                outer_angle: 0.55,
                shadow_maps_enabled: false,
                ..default()
            },
            TruckLamp { base: 420_000.0 },
            // Points +X (a spot light looks down its local -Z).
            Transform::from_translation(Vec3::new(2.7, 1.22, z))
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            ChildOf(root),
        ));
    }
    for z in [-0.92, 0.92] {
        ctx.commands.spawn((
            Name::new("truck taillamp"),
            PointLight {
                color: Color::srgb(1.0, 0.1, 0.06),
                intensity: 0.0,
                range: 5.0,
                radius: 0.05,
                shadow_maps_enabled: false,
                ..default()
            },
            TruckLamp { base: 30_000.0 },
            Transform::from_translation(Vec3::new(-2.9, 1.12, z)),
            ChildOf(root),
        ));
    }
    ctx.commands.insert_resource(TruckAssets {
        headlamp: mats.headlamp.clone(),
        taillamp: mats.taillamp.clone(),
    });

    // The rusted pickup in the ranch yard, nose toward the house.
    for prop in d.props.iter().filter(|p| p.kind == PropKind::Pickup) {
        let mut parts = Parts::default();
        pickup(&mut parts);
        let at = Vec3::new(prop.center.x, 0.0, prop.center.y);
        spawn_parts(
            ctx,
            "ranch pickup",
            parts,
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(std::f32::consts::PI - 0.15)),
            &mats,
        );
    }
}
