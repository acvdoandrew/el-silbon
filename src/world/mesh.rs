//! Small procedural mesh toolkit: boxes, tubes, lathes, blobs and ribbons
//! accumulated into one vertex/index buffer with per-vertex colour, so each
//! structure becomes a handful of merged meshes rather than hundreds of
//! entities.

use bevy::asset::RenderAssetUsages;
use bevy::math::{Quat, Vec2, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Linear RGBA vertex colour.
pub type Rgba = [f32; 4];

pub const WHITE: Rgba = [1.0, 1.0, 1.0, 1.0];

/// Convert an sRGB tint to a linear vertex colour.
pub fn srgb(r: f32, g: f32, b: f32) -> Rgba {
    [to_linear(r), to_linear(g), to_linear(b), 1.0]
}

pub fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn scale_rgb(c: Rgba, k: f32) -> Rgba {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

pub fn mix_rgb(a: Rgba, b: Rgba, t: f32) -> Rgba {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// One cross-section of a tube.
#[derive(Clone, Copy, Debug)]
pub struct Ring {
    pub center: Vec3,
    pub radius: f32,
    pub color: Rgba,
}

#[derive(Default)]
pub struct MeshBuilder {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.idx.is_empty()
    }

    pub fn vertex(&mut self, p: Vec3, n: Vec3, uv: Vec2, c: Rgba) -> u32 {
        let i = self.pos.len() as u32;
        self.pos.push(p.to_array());
        self.nrm.push(n.normalize_or(Vec3::Y).to_array());
        self.uv.push(uv.to_array());
        self.col.push(c);
        i
    }

    pub fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.idx.extend_from_slice(&[a, b, c]);
    }

    /// Quad from four vertex indices in counter-clockwise order (seen from
    /// the side the face should be visible from).
    pub fn quad_idx(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.idx.extend_from_slice(&[a, b, c, a, c, d]);
    }

    /// Flat quad, corners counter-clockwise seen from the front.
    pub fn quad(&mut self, p: [Vec3; 4], uv: [Vec2; 4], c: Rgba) {
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or(Vec3::Y);
        let a = self.vertex(p[0], n, uv[0], c);
        let b = self.vertex(p[1], n, uv[1], c);
        let cc = self.vertex(p[2], n, uv[2], c);
        let d = self.vertex(p[3], n, uv[3], c);
        self.quad_idx(a, b, cc, d);
    }

    /// Oriented box. UVs are in metres × `uv_scale`, offset by `uv_offset`,
    /// so textures keep a consistent world scale across different sizes.
    pub fn cuboid(&mut self, center: Vec3, rot: Quat, half: Vec3, uv_scale: f32, uv_offset: Vec2, c: Rgba) {
        let ax = rot * Vec3::X * half.x;
        let ay = rot * Vec3::Y * half.y;
        let az = rot * Vec3::Z * half.z;
        // (normal axis, u axis, v axis) with u × v pointing along the normal.
        let faces = [
            (ax, -az, ay),
            (-ax, az, ay),
            (az, ax, ay),
            (-az, -ax, ay),
            (ay, ax, -az),
            (-ay, ax, az),
        ];
        for (n, u, v) in faces {
            let lu = u.length() * 2.0 * uv_scale;
            let lv = v.length() * 2.0 * uv_scale;
            let c0 = center + n;
            let p = [c0 - u - v, c0 + u - v, c0 + u + v, c0 - u + v];
            let uvs = [
                uv_offset + Vec2::new(0.0, lv),
                uv_offset + Vec2::new(lu, lv),
                uv_offset + Vec2::new(lu, 0.0),
                uv_offset,
            ];
            let nn = n.normalize_or(Vec3::Y);
            let i0 = self.vertex(p[0], nn, uvs[0], c);
            let i1 = self.vertex(p[1], nn, uvs[1], c);
            let i2 = self.vertex(p[2], nn, uvs[2], c);
            let i3 = self.vertex(p[3], nn, uvs[3], c);
            self.quad_idx(i0, i1, i2, i3);
        }
    }

    /// Box from two corner points along an arbitrary axis (a beam between
    /// `a` and `b` with a square-ish cross-section `w` × `h`).
    pub fn beam(&mut self, a: Vec3, b: Vec3, w: f32, h: f32, roll: f32, uv_scale: f32, uv_offset: Vec2, c: Rgba) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return;
        }
        let rot = Quat::from_rotation_arc(Vec3::Z, d / len) * Quat::from_rotation_z(roll);
        self.cuboid(
            (a + b) * 0.5,
            rot,
            Vec3::new(w * 0.5, h * 0.5, len * 0.5),
            uv_scale,
            uv_offset,
            c,
        );
    }

    /// Tube through a sequence of rings, with parallel-transported frames.
    /// `wobble(ring, side)` scales each vertex radius (bark lumps, flutes).
    pub fn tube(
        &mut self,
        rings: &[Ring],
        sides: usize,
        uv_u: f32,
        uv_v_scale: f32,
        cap_end: bool,
        wobble: &dyn Fn(usize, usize) -> f32,
    ) {
        if rings.len() < 2 || sides < 3 {
            return;
        }
        let n = rings.len();
        let tangent = |i: usize| -> Vec3 {
            let a = rings[i.saturating_sub(1)].center;
            let b = rings[(i + 1).min(n - 1)].center;
            (b - a).normalize_or(Vec3::Y)
        };
        let t0 = tangent(0);
        let helper = if t0.dot(Vec3::X).abs() < 0.9 { Vec3::X } else { Vec3::Z };
        let mut normal = t0.cross(helper).normalize();
        let mut prev_t = t0;
        let mut v_acc = 0.0;
        let mut base_idx = Vec::with_capacity(n);
        for i in 0..n {
            let t = tangent(i);
            let q = Quat::from_rotation_arc(prev_t, t);
            normal = (q * normal).normalize_or(normal);
            prev_t = t;
            let binormal = t.cross(normal).normalize_or(Vec3::Z);
            if i > 0 {
                v_acc += rings[i].center.distance(rings[i - 1].center) * uv_v_scale;
            }
            let start = self.pos.len() as u32;
            base_idx.push(start);
            for s in 0..=sides {
                let a = s as f32 / sides as f32 * std::f32::consts::TAU;
                let dir = normal * a.cos() + binormal * a.sin();
                let r = rings[i].radius * wobble(i, s % sides);
                self.vertex(
                    rings[i].center + dir * r,
                    dir,
                    Vec2::new(s as f32 / sides as f32 * uv_u, v_acc),
                    rings[i].color,
                );
            }
        }
        for i in 0..n - 1 {
            let a0 = base_idx[i];
            let b0 = base_idx[i + 1];
            for s in 0..sides as u32 {
                // Outward-facing winding.
                self.quad_idx(a0 + s, a0 + s + 1, b0 + s + 1, b0 + s);
            }
        }
        if cap_end {
            let last = rings[n - 1];
            let t = tangent(n - 1);
            let center = self.vertex(
                last.center + t * last.radius * 0.35,
                t,
                Vec2::new(0.5, v_acc),
                last.color,
            );
            let b = base_idx[n - 1];
            for s in 0..sides as u32 {
                self.tri(b + s, b + s + 1, center);
            }
        }
    }

    /// Surface of revolution around +Y at `base`. `profile` is (radius, y)
    /// from bottom to top.
    pub fn lathe(&mut self, base: Vec3, profile: &[(f32, f32)], sides: usize, uv_v_scale: f32, c: Rgba) {
        if profile.len() < 2 {
            return;
        }
        let n = profile.len();
        let mut starts = Vec::with_capacity(n);
        let mut v_acc = 0.0;
        for i in 0..n {
            let (r, y) = profile[i];
            let (r0, y0) = profile[i.saturating_sub(1)];
            let (r1, y1) = profile[(i + 1).min(n - 1)];
            // Profile tangent → outward normal in the (r, y) plane.
            let dr = r1 - r0;
            let dy = y1 - y0;
            let pn = Vec2::new(dy, -dr).normalize_or(Vec2::X);
            if i > 0 {
                v_acc += Vec2::new(r - profile[i - 1].0, y - profile[i - 1].1).length() * uv_v_scale;
            }
            starts.push(self.pos.len() as u32);
            for s in 0..=sides {
                let a = s as f32 / sides as f32 * std::f32::consts::TAU;
                let (sa, ca) = a.sin_cos();
                let p = base + Vec3::new(ca * r, y, sa * r);
                let nrm = Vec3::new(ca * pn.x, pn.y, sa * pn.x);
                self.vertex(p, nrm, Vec2::new(s as f32 / sides as f32 * 2.0, v_acc), c);
            }
        }
        for i in 0..n - 1 {
            let a0 = starts[i];
            let b0 = starts[i + 1];
            for s in 0..sides as u32 {
                self.quad_idx(a0 + s, b0 + s, b0 + s + 1, a0 + s + 1);
            }
        }
    }

    /// Lumpy ellipsoid. `bump(dir)` scales the radius along a unit direction.
    pub fn blob(
        &mut self,
        center: Vec3,
        radii: Vec3,
        stacks: usize,
        sectors: usize,
        uv_scale: f32,
        c: Rgba,
        bump: &dyn Fn(Vec3) -> f32,
    ) {
        let start = self.pos.len() as u32;
        for i in 0..=stacks {
            let v = i as f32 / stacks as f32;
            let phi = v * std::f32::consts::PI;
            for j in 0..=sectors {
                let u = j as f32 / sectors as f32;
                let theta = u * std::f32::consts::TAU;
                let dir = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                let k = bump(dir);
                let p = center + dir * radii * k;
                // Ellipsoid normal (bumps are gentle, so this reads well).
                let n = (dir / radii).normalize_or(dir);
                self.vertex(p, n, Vec2::new(u * uv_scale * 2.0, v * uv_scale), c);
            }
        }
        let row = sectors as u32 + 1;
        for i in 0..stacks as u32 {
            for j in 0..sectors as u32 {
                let a = start + i * row + j;
                let b = a + row;
                self.quad_idx(a, a + 1, b + 1, b);
            }
        }
    }

    /// Thin tapered strip along a polyline (grass blades, fronds, fringes).
    /// `side` gives the flat axis of the strip at each point.
    pub fn ribbon(&mut self, points: &[Vec3], widths: &[f32], side: Vec3, colors: &[Rgba]) {
        if points.len() < 2 {
            return;
        }
        let n = points.len();
        let start = self.pos.len() as u32;
        for i in 0..n {
            let t = (points[(i + 1).min(n - 1)] - points[i.saturating_sub(1)]).normalize_or(Vec3::Y);
            let normal = side.cross(t).normalize_or(Vec3::Z);
            let w = widths[i.min(widths.len() - 1)] * 0.5;
            let c = colors[i.min(colors.len() - 1)];
            let v = i as f32 / (n - 1) as f32;
            self.vertex(points[i] - side * w, normal, Vec2::new(0.0, v), c);
            self.vertex(points[i] + side * w, normal, Vec2::new(1.0, v), c);
        }
        for i in 0..n as u32 - 1 {
            let a = start + i * 2;
            self.quad_idx(a, a + 1, a + 3, a + 2);
        }
    }

    /// Regular grid over a rectangle in a plane, displaced by `height`.
    /// Used for ground patches, roof sheets and cloth.
    pub fn grid(
        &mut self,
        origin: Vec3,
        u_axis: Vec3,
        v_axis: Vec3,
        nu: usize,
        nv: usize,
        uv_scale: Vec2,
        displace: &dyn Fn(f32, f32) -> (Vec3, Rgba),
    ) {
        let start = self.pos.len() as u32;
        let base_n = u_axis.cross(v_axis).normalize_or(Vec3::Y);
        let eps = 1e-3;
        for j in 0..=nv {
            for i in 0..=nu {
                let u = i as f32 / nu as f32;
                let v = j as f32 / nv as f32;
                let (off, c) = displace(u, v);
                let p = origin + u_axis * u + v_axis * v + off;
                // Normal from finite differences of the displaced surface.
                let (ou, _) = displace((u + eps).min(1.0), v);
                let (ov, _) = displace(u, (v + eps).min(1.0));
                let du = u_axis * eps + (ou - off);
                let dv = v_axis * eps + (ov - off);
                let mut n = du.cross(dv).normalize_or(base_n);
                if n.dot(base_n) < 0.0 {
                    n = -n;
                }
                let uv = Vec2::new(u * u_axis.length() * uv_scale.x, v * v_axis.length() * uv_scale.y);
                self.vertex(p, n, uv, c);
            }
        }
        let row = nu as u32 + 1;
        for j in 0..nv as u32 {
            for i in 0..nu as u32 {
                let a = start + j * row + i;
                let b = a + row;
                self.quad_idx(a, a + 1, b + 1, b);
            }
        }
    }

    pub fn build(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.col)
            .with_inserted_indices(Indices::U32(self.idx))
    }
}
