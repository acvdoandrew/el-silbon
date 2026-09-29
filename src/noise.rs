//! Deterministic value noise on the ground plane. Pure (no ECS), so the
//! authored terrain in `geometry` and the meshes in `world` agree exactly.

fn hash(ix: i32, iz: i32, seed: u32) -> f32 {
    let mut h =
        (ix as u32).wrapping_mul(0x27D4_EB2D) ^ (iz as u32).wrapping_mul(0x1656_67B1) ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Smooth world-space value noise in 0..1.
pub fn noise2(x: f32, z: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let fx = x - xi as f32;
    let fz = z - zi as f32;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sz) = (s(fx), s(fz));
    let a = hash(xi, zi, seed);
    let b = hash(xi + 1, zi, seed);
    let c = hash(xi, zi + 1, seed);
    let d = hash(xi + 1, zi + 1, seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sz
}

/// Two-octave world noise in 0..1.
pub fn fbm2(x: f32, z: f32, seed: u32) -> f32 {
    noise2(x, z, seed) * 0.65 + noise2(x * 2.1, z * 2.1, seed ^ 0x5151) * 0.35
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        for i in 0..200 {
            let (x, z) = (i as f32 * 0.37 - 30.0, i as f32 * 0.91 + 4.0);
            let a = fbm2(x, z, 7);
            assert_eq!(a, fbm2(x, z, 7));
            assert!((0.0..=1.0).contains(&a));
        }
    }
}
