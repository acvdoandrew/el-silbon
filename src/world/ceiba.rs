//! The great ceiba: a fluted grey column on tall buttress roots, heavy limbs
//! spreading almost flat, and a broad umbrella crown. The root hollow that
//! receives the bones faces the house, exactly where the layout says.

use bevy::prelude::*;

use super::SpawnCtx;
use super::mesh::{MeshBuilder, Rgba, Ring, scale_rgb, srgb};
use super::noise2;
use crate::rng::Rng;

pub fn spawn(ctx: &mut SpawnCtx) {
    let tree = ctx.layout.ceiba.clone();
    let mut rng = ctx.rng(81);
    let mut bark = MeshBuilder::new();
    let mut crown = MeshBuilder::new();
    let mut vines = MeshBuilder::new();
    let mut stones = MeshBuilder::new();
    let c = Vec3::new(tree.center.x, 0.0, tree.center.y);
    let bark_col = srgb(1.0, 1.0, 0.98);
    let seed = ctx.seed as u32;

    // Trunk: flared foot, fluted column, slight lean.
    let top_y = tree.height * 0.6;
    let lean = Vec3::new(0.35, 0.0, -0.25);
    let rings: Vec<Ring> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0;
            let y = -0.3 + (top_y + 0.3) * t;
            let flare = (1.0 - (y / 2.2).clamp(0.0, 1.0)).powf(2.0) * 0.9;
            let r = tree.trunk_radius * (1.02 - 0.3 * t) + flare;
            Ring {
                center: c + Vec3::Y * y + lean * t * t,
                radius: r,
                color: scale_rgb(bark_col, 0.85 + 0.15 * t),
            }
        })
        .collect();
    bark.tube(&rings, 24, 6.0, 0.18, false, &|i, s| {
        let a = s as f32 / 24.0 * std::f32::consts::TAU;
        1.0 + 0.07 * (a * 7.0 + i as f32 * 0.15).sin() + 0.05 * (noise2(a * 3.0, i as f32 * 0.5, seed) - 0.5)
    });

    // Buttress roots.
    for (k, &a) in tree.root_angles.iter().enumerate() {
        let reach = tree.root_reach * rng.range(0.85, 1.12);
        let height = rng.range(2.4, 3.6);
        let curve = rng.signed(0.25);
        let tint = scale_rgb(bark_col, 0.8 + 0.03 * (k % 5) as f32);
        buttress(&mut bark, c, a, 1.2, reach + 0.4, height, 0.5, curve, tint);
    }

    // Limbs: heavy, almost horizontal, rising at the tips; they reach out to
    // carry the umbrella crown over the layout's canopy radius.
    let crown_top = c + Vec3::Y * top_y + lean;
    let reach = tree.canopy_radius / 14.0;
    let limb_count = 7;
    let mut tips: Vec<(Vec3, Vec3)> = Vec::new();
    for l in 0..limb_count {
        let a = l as f32 / limb_count as f32 * std::f32::consts::TAU + rng.signed(0.35);
        let out = Vec3::new(a.cos(), 0.0, a.sin());
        let start = crown_top + Vec3::Y * rng.range(-1.2, 0.8) + out * 0.6;
        let len = rng.range(7.5, 10.5) * reach;
        let rise = rng.range(0.18, 0.42);
        let (end, dir) = limb(&mut bark, start, out, rise, len, 1.0, 0.32, &mut rng, bark_col);
        tips.push((end, dir));
        for _ in 0..rng.range(2.0, 4.0) as usize {
            let t = rng.range(0.45, 0.9);
            let from = start.lerp(end, t) + Vec3::Y * (rise * len * t * 0.3);
            let side = Vec3::new(-dir.z, 0.0, dir.x) * if rng.f32() < 0.5 { 1.0 } else { -1.0 };
            let sub_dir = (dir + side * rng.range(0.4, 1.0)).normalize();
            let (se, sd) = limb(
                &mut bark,
                from,
                sub_dir,
                rng.range(0.25, 0.6),
                rng.range(3.5, 6.0) * reach,
                0.3 * (1.0 - t * 0.4),
                0.08,
                &mut rng,
                bark_col,
            );
            tips.push((se, sd));
        }
    }

    // Umbrella crown: broad, flat-topped layers of foliage clumps over the
    // limb tips and a fill ring, with gaps that let the limbs show.
    let leaf_col = srgb(1.0, 1.0, 1.0);
    let clump = |mb: &mut MeshBuilder, p: Vec3, r: f32, rng: &mut Rng| {
        let s = rng.next_u64() as u32;
        let tint = scale_rgb(leaf_col, rng.range(0.85, 1.15));
        mb.blob(p, Vec3::new(r, r * 0.42, r), 8, 14, 0.35, tint, &move |d: Vec3| {
            0.78 + 0.4 * noise2(d.x * 2.5 + 11.0, d.z * 2.5 + d.y * 2.0, s)
        });
    };
    for &(tip, _) in &tips {
        let p = tip + Vec3::Y * rng.range(0.6, 1.4);
        let r = rng.range(2.8, 4.4) * reach;
        clump(&mut crown, p, r, &mut rng);
    }
    for i in 0..18 {
        let a = i as f32 / 18.0 * std::f32::consts::TAU + rng.signed(0.2);
        let d = rng.range(tree.canopy_radius * 0.3, tree.canopy_radius * 0.85);
        let p = crown_top + Vec3::new(a.cos() * d, rng.range(3.5, 6.5), a.sin() * d);
        let r = rng.range(3.0, 4.6) * reach;
        clump(&mut crown, p, r, &mut rng);
    }
    clump(&mut crown, crown_top + Vec3::Y * 7.0, 5.0 * reach, &mut rng);

    // Lianas hanging from the limbs.
    let vine_col = srgb(0.45, 0.4, 0.3);
    for &(tip, dir) in tips.iter().take(12) {
        let from = tip - dir * rng.range(0.5, 2.5);
        let len = rng.range(2.5, 6.0);
        let sway = Vec3::new(rng.signed(0.4), 0.0, rng.signed(0.4));
        let pts: Vec<Vec3> = (0..6)
            .map(|k| {
                let t = k as f32 / 5.0;
                from + Vec3::NEG_Y * len * t + sway * t * t
            })
            .collect();
        vines.ribbon(
            &pts,
            &[0.05, 0.04, 0.035, 0.03, 0.025, 0.01],
            Vec3::new(dir.z, 0.0, -dir.x),
            &[vine_col],
        );
    }

    // The hollow: a ring of fieldstones and dark earth between two roots.
    let off = tree.offering;
    let to_house = (Vec3::new(off.x, 0.0, off.z) - c).normalize();
    let side = Vec3::new(-to_house.z, 0.0, to_house.x);
    for k in 0..9 {
        let t = k as f32 / 8.0 - 0.5;
        let p = Vec3::new(off.x, 0.0, off.z) + side * t * 1.3 + to_house * (0.45 - t * t * 1.4);
        let r = rng.range(0.1, 0.18);
        let s = rng.next_u64() as u32;
        stones.blob(
            p + Vec3::Y * r * 0.3,
            Vec3::new(r, r * 0.7, r * 1.1),
            5,
            8,
            1.0,
            scale_rgb(srgb(1.0, 1.0, 1.0), rng.range(0.7, 1.1)),
            &move |d: Vec3| 0.85 + 0.3 * noise2(d.x * 3.0, d.z * 3.0, s),
        );
    }

    let bark_mat = ctx.palette.bark.clone();
    let leaves = ctx.palette.leaves.clone();
    let wood = ctx.palette.wood.clone();
    let stone = ctx.palette.stone.clone();
    ctx.static_mesh("ceiba bark", bark, &bark_mat);
    ctx.static_mesh("ceiba crown", crown, &leaves);
    ctx.static_mesh("ceiba lianas", vines, &wood);
    ctx.static_mesh("hollow stones", stones, &stone);
}

/// A curving limb; returns its end point and final direction.
fn limb(
    mb: &mut MeshBuilder,
    start: Vec3,
    out: Vec3,
    rise: f32,
    len: f32,
    r0: f32,
    r1: f32,
    rng: &mut Rng,
    color: Rgba,
) -> (Vec3, Vec3) {
    let steps = 8;
    let wiggle = Vec3::new(rng.signed(0.6), 0.0, rng.signed(0.6));
    let mut rings = Vec::with_capacity(steps + 1);
    let mut p = start;
    let mut dir = (out + Vec3::Y * rise * 0.3).normalize();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        rings.push(Ring {
            center: p,
            radius: r0 + (r1 - r0) * t,
            color: scale_rgb(color, 0.9 + 0.1 * t),
        });
        // Bend upward toward the tip, with a little meander.
        dir = (dir + Vec3::Y * rise * 0.22 * t + wiggle * 0.05).normalize();
        p += dir * (len / steps as f32);
    }
    mb.tube(&rings, 10, 2.0, 0.3, true, &|_, _| 1.0);
    let end = rings.last().map(|r| r.center).unwrap_or(start);
    (end, dir)
}

/// A buttress fin radiating from the trunk: tall and thin, tapering outward.
fn buttress(
    mb: &mut MeshBuilder,
    center: Vec3,
    angle: f32,
    inner: f32,
    reach: f32,
    height: f32,
    thick: f32,
    curve: f32,
    color: Rgba,
) {
    let ns = 10;
    let nt = 6;
    let at = |s: f32| {
        let a = angle + curve * s * s;
        Vec3::new(a.cos(), 0.0, a.sin())
    };
    let point = |s: f32, t: f32| -> (Vec3, Vec3, f32) {
        let dir = at(s);
        let dist = inner + (reach - inner) * s;
        let top = height * (1.0 - s).powf(1.6) + 0.1;
        // Concave upper edge like a sail.
        let y = top * t - 0.25 * (1.0 - t) * s;
        let w = thick * (1.0 - 0.7 * s) * (1.0 - 0.5 * t) + 0.04;
        let perp = Vec3::new(-dir.z, 0.0, dir.x);
        (center + dir * dist + Vec3::Y * y, perp, w)
    };
    for side in [-1.0_f32, 1.0] {
        let start = mb_vertex_grid(
            mb,
            ns,
            nt,
            |s, t| {
                let (p, perp, w) = point(s, t);
                (p + perp * side * w * 0.5, perp * side)
            },
            color,
        );
        let row = ns as u32 + 1;
        for j in 0..nt as u32 {
            for i in 0..ns as u32 {
                let a = start + j * row + i;
                let b = a + row;
                if side > 0.0 {
                    mb.quad_idx(a, a + 1, b + 1, b);
                } else {
                    mb.quad_idx(a, b, b + 1, a + 1);
                }
            }
        }
    }
    // Rounded top edge.
    let mut prev: Option<(u32, u32)> = None;
    for i in 0..=ns {
        let s = i as f32 / ns as f32;
        let (p, perp, w) = point(s, 1.0);
        let l = mb.vertex(p - perp * w * 0.5, Vec3::Y, Vec2::new(0.0, s * 4.0), color);
        let r = mb.vertex(p + perp * w * 0.5, Vec3::Y, Vec2::new(0.3, s * 4.0), color);
        if let Some((pl, pr)) = prev {
            mb.quad_idx(pl, pr, r, l);
        }
        prev = Some((l, r));
    }
}

/// Add a (ns+1)×(nt+1) vertex grid; returns the first index.
fn mb_vertex_grid(
    mb: &mut MeshBuilder,
    ns: usize,
    nt: usize,
    f: impl Fn(f32, f32) -> (Vec3, Vec3),
    color: Rgba,
) -> u32 {
    let mut first = None;
    for j in 0..=nt {
        for i in 0..=ns {
            let s = i as f32 / ns as f32;
            let t = j as f32 / nt as f32;
            let (p, n) = f(s, t);
            let idx = mb.vertex(p, n, Vec2::new(s * 3.0, t * 2.0), color);
            first.get_or_insert(idx);
        }
    }
    first.unwrap_or(0)
}
