//! Original procedural surface textures, generated at startup from the fixed
//! seed (tileable value noise, strokes, stencilled lettering). Every texture
//! gets a full box-filtered mip chain and a repeating anisotropic sampler.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::{Assets, Handle};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::rng::Rng;

/// Handles to every generated texture.
#[derive(Clone)]
pub struct Textures {
    pub ground: Handle<Image>,
    pub road: Handle<Image>,
    pub wood: Handle<Image>,
    pub wood_dark: Handle<Image>,
    pub zinc: Handle<Image>,
    pub bark: Handle<Image>,
    pub leaves: Handle<Image>,
    pub burlap: Handle<Image>,
    pub cloth: Handle<Image>,
    pub cloth_pale: Handle<Image>,
    pub straw: Handle<Image>,
    pub clay: Handle<Image>,
    pub paper_note: Handle<Image>,
    pub calendar: Handle<Image>,
    pub sign: Handle<Image>,
    pub halo: Handle<Image>,
    /// Tangent-space normals and glTF-style (G roughness, B metallic) maps.
    pub ground_n: Handle<Image>,
    pub ground_mr: Handle<Image>,
    pub road_n: Handle<Image>,
    pub road_mr: Handle<Image>,
    pub wood_n: Handle<Image>,
    pub wood_mr: Handle<Image>,
    pub wood_dark_n: Handle<Image>,
    pub wood_dark_mr: Handle<Image>,
    pub zinc_n: Handle<Image>,
    pub zinc_mr: Handle<Image>,
    pub bark_n: Handle<Image>,
    pub bark_mr: Handle<Image>,
    pub burlap_n: Handle<Image>,
    pub burlap_mr: Handle<Image>,
    pub straw_n: Handle<Image>,
    pub straw_mr: Handle<Image>,
    pub cloth_n: Handle<Image>,
    pub cloth_mr: Handle<Image>,
    pub cloth_pale_n: Handle<Image>,
    pub cloth_pale_mr: Handle<Image>,
    /// A flipbook of expanding rain rings, 8 frames side by side.
    pub ripples: Handle<Image>,
    pub water_n: Handle<Image>,
}

/// RGBA float canvas in sRGB space (0..1).
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize, fill: [f32; 4]) -> Self {
        Self {
            w,
            h,
            px: vec![fill; w * h],
        }
    }

    /// Blend a colour into a pixel (wrapping, so strokes tile).
    fn blend(&mut self, x: i32, y: i32, c: [f32; 3], a: f32) {
        let x = x.rem_euclid(self.w as i32) as usize;
        let y = y.rem_euclid(self.h as i32) as usize;
        let p = &mut self.px[y * self.w + x];
        for (dst, src) in p.iter_mut().zip(c) {
            *dst += (src - *dst) * a;
        }
    }

    fn for_each(&mut self, f: impl Fn(f32, f32, &mut [f32; 4])) {
        for y in 0..self.h {
            for x in 0..self.w {
                let u = x as f32 / self.w as f32;
                let v = y as f32 / self.h as f32;
                f(u, v, &mut self.px[y * self.w + x]);
            }
        }
    }

    fn into_image(self, images: &mut Assets<Image>, repeat: bool) -> Handle<Image> {
        self.into_image_as(images, repeat, TextureFormat::Rgba8UnormSrgb)
    }

    /// Data textures (normals, roughness) are linear, not sRGB.
    fn into_linear(self, images: &mut Assets<Image>, repeat: bool) -> Handle<Image> {
        self.into_image_as(images, repeat, TextureFormat::Rgba8Unorm)
    }

    /// A tangent-space normal map (Y up) from the luminance of this canvas,
    /// wrapping at the edges so it tiles.
    fn normal_map(&self, strength: f32) -> Canvas {
        let luma = |x: i32, y: i32| {
            let x = x.rem_euclid(self.w as i32) as usize;
            let y = y.rem_euclid(self.h as i32) as usize;
            let p = self.px[y * self.w + x];
            p[0] * 0.3 + p[1] * 0.59 + p[2] * 0.11
        };
        let mut out = Canvas::new(self.w, self.h, [0.5, 0.5, 1.0, 1.0]);
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                let nx = (luma(x - 1, y) - luma(x + 1, y)) * strength;
                let ny = (luma(x, y + 1) - luma(x, y - 1)) * strength;
                let len = (nx * nx + ny * ny + 1.0).sqrt();
                out.px[y as usize * self.w + x as usize] =
                    [nx / len * 0.5 + 0.5, ny / len * 0.5 + 0.5, 1.0 / len * 0.5 + 0.5, 1.0];
            }
        }
        out
    }

    fn into_image_as(self, images: &mut Assets<Image>, repeat: bool, format: TextureFormat) -> Handle<Image> {
        let mut levels: Vec<Vec<[f32; 4]>> = vec![self.px];
        let (mut w, mut h) = (self.w, self.h);
        while w > 1 || h > 1 {
            let nw = (w / 2).max(1);
            let nh = (h / 2).max(1);
            let prev = levels.last().expect("level 0 exists");
            let mut next = vec![[0.0; 4]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    let mut acc = [0.0f32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(w - 1);
                        let sy = (y * 2 + dy).min(h - 1);
                        let p = prev[sy * w + sx];
                        for (dst, src) in acc.iter_mut().zip(p) {
                            *dst += src * 0.25;
                        }
                    }
                    next[y * nw + x] = acc;
                }
            }
            levels.push(next);
            w = nw;
            h = nh;
        }
        let mip_count = levels.len() as u32;
        let to_bytes = |level: &[[f32; 4]]| -> Vec<u8> {
            let mut out = Vec::with_capacity(level.len() * 4);
            for p in level {
                for c in p {
                    out.push((c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
                }
            }
            out
        };
        let base = to_bytes(&levels[0]);
        let mut all = Vec::new();
        for level in &levels {
            all.extend(to_bytes(level));
        }
        let mut image = Image::new(
            Extent3d {
                width: self.w as u32,
                height: self.h as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            base,
            format,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.data = Some(all);
        image.texture_descriptor.mip_level_count = mip_count;
        let mode = if repeat {
            ImageAddressMode::Repeat
        } else {
            ImageAddressMode::ClampToEdge
        };
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: mode,
            address_mode_v: mode,
            address_mode_w: mode,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        images.add(image)
    }
}

// ----------------------------------------------------------------------------
// Tileable noise
// ----------------------------------------------------------------------------

fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h =
        (x as u32).wrapping_mul(0x8DA6_B343) ^ (y as u32).wrapping_mul(0xD816_3841) ^ seed.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Value noise on a torus of `period` cells: tiles perfectly on [0,1).
fn vnoise(u: f32, v: f32, period: i32, seed: u32) -> f32 {
    let x = u * period as f32;
    let y = v * period as f32;
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let w = |i: i32| i.rem_euclid(period);
    let a = hash2(w(xi), w(yi), seed);
    let b = hash2(w(xi + 1), w(yi), seed);
    let c = hash2(w(xi), w(yi + 1), seed);
    let d = hash2(w(xi + 1), w(yi + 1), seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sy
}

/// Fractal sum of tileable value noise, roughly 0..1.
fn fbm(u: f32, v: f32, period: i32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    let mut p = period;
    for o in 0..octaves {
        sum += vnoise(u, v, p, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        p *= 2;
    }
    sum / norm
}

/// Anisotropic tileable noise: `px` cells across, `py` cells down.
fn vnoise2(u: f32, v: f32, px: i32, py: i32, seed: u32) -> f32 {
    let x = u * px as f32;
    let y = v * py as f32;
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let wx = |i: i32| i.rem_euclid(px);
    let wy = |i: i32| i.rem_euclid(py);
    let a = hash2(wx(xi), wy(yi), seed);
    let b = hash2(wx(xi + 1), wy(yi), seed);
    let c = hash2(wx(xi), wy(yi + 1), seed);
    let d = hash2(wx(xi + 1), wy(yi + 1), seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sy
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn set(p: &mut [f32; 4], c: [f32; 3]) {
    p[0] = c[0];
    p[1] = c[1];
    p[2] = c[2];
    p[3] = 1.0;
}

// ----------------------------------------------------------------------------
// Stencil lettering (original 5×7 glyphs)
// ----------------------------------------------------------------------------

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '+' => [0b00100, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        _ => [0; 7],
    }
}

/// Paint text with square "brush" cells; `wear` erodes random cells.
fn stencil(c: &mut Canvas, text: &str, x0: i32, y0: i32, cell: i32, color: [f32; 3], wear: f32, rng: &mut Rng) {
    let mut x = x0;
    for ch in text.chars() {
        let rows = glyph(ch);
        for (ry, row) in rows.iter().enumerate() {
            for rx in 0..5i32 {
                if (row >> (4 - rx)) & 1 == 0 {
                    continue;
                }
                for py in 0..cell {
                    for px in 0..cell {
                        let a = if rng.f32() < wear { 0.25 } else { 0.92 };
                        c.blend(x + rx * cell + px, y0 + ry as i32 * cell + py, color, a);
                    }
                }
            }
        }
        x += cell * 6;
    }
}

// ----------------------------------------------------------------------------
// The textures
// ----------------------------------------------------------------------------

pub fn generate(images: &mut Assets<Image>, seed: u64) -> Textures {
    let s = (seed as u32) ^ ((seed >> 32) as u32);
    // Each albedo yields its normal map first, then is consumed by its image.
    let ground_albedo = ground(s, seed);
    let ground_n = ground_albedo.normal_map(2.6).into_linear(images, true);
    let road_albedo = road(s, seed);
    let road_n = road_albedo.normal_map(3.2).into_linear(images, true);
    let wood_albedo = wood(s, false);
    let wood_n = wood_albedo.normal_map(2.2).into_linear(images, true);
    let wood_dark_albedo = wood(s ^ 0x55, true);
    let wood_dark_n = wood_dark_albedo.normal_map(2.4).into_linear(images, true);
    let bark_albedo = bark(s);
    let bark_n = bark_albedo.normal_map(3.0).into_linear(images, true);
    let burlap_albedo = weave(s, [0.52, 0.42, 0.28], 24);
    let burlap_n = burlap_albedo.normal_map(2.6).into_linear(images, true);
    let straw_albedo = weave(s ^ 0x77, [0.46, 0.38, 0.24], 40);
    let straw_n = straw_albedo.normal_map(2.0).into_linear(images, true);
    let cloth_albedo = cloth(s, false);
    let cloth_n = cloth_albedo.normal_map(1.6).into_linear(images, true);
    let cloth_pale_albedo = cloth(s ^ 0x88, true);
    let cloth_pale_n = cloth_pale_albedo.normal_map(1.6).into_linear(images, true);
    Textures {
        ground: ground_albedo.into_image(images, true),
        road: road_albedo.into_image(images, true),
        wood: wood_albedo.into_image(images, true),
        wood_dark: wood_dark_albedo.into_image(images, true),
        zinc: zinc(s).into_image(images, true),
        bark: bark_albedo.into_image(images, true),
        leaves: leaves(s, seed).into_image(images, true),
        burlap: burlap_albedo.into_image(images, true),
        cloth: cloth_albedo.into_image(images, true),
        cloth_pale: cloth_pale_albedo.into_image(images, true),
        straw: straw_albedo.into_image(images, true),
        clay: clay(s).into_image(images, true),
        paper_note: note_paper(s, seed).into_image(images, false),
        calendar: calendar(s, seed).into_image(images, false),
        sign: sign(s, seed).into_image(images, false),
        halo: halo().into_image(images, false),
        ground_n,
        ground_mr: ground_mr(s).into_linear(images, true),
        road_n,
        road_mr: road_mr(s).into_linear(images, true),
        wood_n,
        wood_mr: wood_mr(s, false).into_linear(images, true),
        wood_dark_n,
        wood_dark_mr: wood_mr(s ^ 0x55, true).into_linear(images, true),
        zinc_n: zinc_height(s).normal_map(5.0).into_linear(images, true),
        zinc_mr: zinc_mr(s).into_linear(images, true),
        bark_n,
        bark_mr: bark_mr(s).into_linear(images, true),
        burlap_n,
        burlap_mr: fibre_mr(s, 0.94, 24).into_linear(images, true),
        straw_n,
        straw_mr: fibre_mr(s ^ 0x77, 0.86, 40).into_linear(images, true),
        cloth_n,
        cloth_mr: cloth_mr(s, false).into_linear(images, true),
        cloth_pale_n,
        cloth_pale_mr: cloth_mr(s ^ 0x88, true).into_linear(images, true),
        ripples: ripples(seed).into_linear(images, true),
        water_n: water_height(s).normal_map(3.4).into_linear(images, true),
    }
}

/// Dry llano grass over dark soil, with blade strokes.
fn ground(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(512, 512, [0.0; 4]);
    c.for_each(|u, v, p| {
        let broad = fbm(u, v, 4, 4, s);
        let fine = fbm(u, v, 32, 3, s ^ 0x1234);
        let soil = [0.115, 0.09, 0.065];
        let dry = [0.3, 0.3, 0.155];
        let green = [0.13, 0.215, 0.085];
        let grass = lerp3(green, dry, smooth(0.35, 0.7, broad));
        let bare = smooth(0.6, 0.78, fbm(u, v, 8, 3, s ^ 0x777));
        let mut col = lerp3(grass, soil, bare * 0.8);
        let k = 0.78 + 0.44 * fine;
        col = [col[0] * k, col[1] * k, col[2] * k];
        set(p, col);
    });
    let mut rng = Rng::fork(seed, 11);
    for _ in 0..9000 {
        let x = rng.range(0.0, 512.0);
        let y = rng.range(0.0, 512.0);
        let len = rng.range(5.0, 16.0);
        let ang = rng.range(-0.9, 0.9) - std::f32::consts::FRAC_PI_2;
        let light = rng.f32() < 0.55;
        let col = if light { [0.46, 0.5, 0.28] } else { [0.09, 0.12, 0.06] };
        let a = rng.range(0.25, 0.6);
        let steps = len as i32;
        for i in 0..steps {
            let t = i as f32;
            let px = (x + ang.cos() * t) as i32;
            let py = (y + ang.sin() * t) as i32;
            c.blend(px, py, col, a * (1.0 - t / len * 0.6));
        }
    }
    c
}

/// Packed road earth with pebbles.
fn road(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let n = fbm(u, v, 6, 5, s ^ 0x2222);
        let wet = smooth(0.62, 0.75, fbm(u, v, 3, 3, s ^ 0x9));
        let base = lerp3([0.3, 0.235, 0.17], [0.2, 0.155, 0.11], n);
        let col = lerp3(base, [0.11, 0.085, 0.065], wet * 0.7);
        set(p, col);
    });
    let mut rng = Rng::fork(seed, 12);
    for _ in 0..1400 {
        let x = rng.range(0.0, 256.0) as i32;
        let y = rng.range(0.0, 256.0) as i32;
        let r = rng.range(0.6, 2.4);
        let tone = rng.range(0.35, 0.7);
        let col = [tone, tone * 0.93, tone * 0.85];
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d <= r {
                    c.blend(x + dx, y + dy, col, 0.7 * (1.0 - d / (r + 0.5)));
                }
            }
        }
    }
    c
}

/// Weathered vertical-grain boards (silvered) or darker interior wood.
fn wood(s: u32, dark: bool) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let grain = vnoise2(u, v, 48, 3, s ^ 0x31) * 0.6 + vnoise2(u, v, 160, 8, s ^ 0x32) * 0.4;
        let streak = fbm(u, v, 6, 4, s ^ 0x33);
        let crack = smooth(0.82, 0.9, vnoise2(u, v, 96, 5, s ^ 0x34));
        let (light, mid, deep) = if dark {
            ([0.36, 0.26, 0.18], [0.26, 0.18, 0.12], [0.11, 0.08, 0.06])
        } else {
            ([0.58, 0.55, 0.5], [0.42, 0.37, 0.31], [0.17, 0.14, 0.11])
        };
        let mut col = lerp3(mid, light, grain * 0.8 + streak * 0.35 - 0.2);
        col = lerp3(col, deep, crack * 0.85);
        // Knots.
        let kx = (u * 3.0).fract() - 0.5;
        let ky = (v * 2.0).fract() - 0.5;
        let kd = (kx * kx * 9.0 + ky * ky * 3.0).sqrt();
        let knot = hash2((u * 3.0) as i32, (v * 2.0) as i32, s ^ 0x35);
        if knot > 0.72 {
            let ring = ((kd * 40.0).sin() * 0.5 + 0.5) * smooth(0.35, 0.05, kd);
            col = lerp3(col, deep, ring * 0.6);
        }
        if !dark {
            // Faint green-grey lichen near the bottom of boards.
            let lichen = smooth(0.7, 0.85, fbm(u, v, 10, 3, s ^ 0x36));
            col = lerp3(col, [0.47, 0.5, 0.4], lichen * 0.4);
        }
        set(p, col);
    });
    c
}

/// Corrugated zinc: bright ridges, rust patches and run-off streaks.
fn zinc(s: u32) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let ridge = (u * std::f32::consts::TAU * 8.0).sin() * 0.5 + 0.5;
        let metal = lerp3([0.38, 0.39, 0.4], [0.62, 0.63, 0.62], ridge * 0.6 + 0.2);
        let rust = smooth(0.48, 0.7, fbm(u, v, 5, 5, s ^ 0x41));
        let streak = smooth(0.55, 0.8, vnoise2(u, v, 40, 2, s ^ 0x42)) * smooth(0.2, 0.8, v);
        let rust_col = lerp3([0.45, 0.22, 0.1], [0.28, 0.15, 0.08], fbm(u, v, 20, 3, s ^ 0x43));
        let mut col = lerp3(metal, rust_col, (rust + streak * 0.6).min(1.0));
        let grime = fbm(u, v, 12, 3, s ^ 0x44);
        col = lerp3(col, [0.15, 0.13, 0.11], smooth(0.6, 0.9, grime) * 0.5);
        set(p, col);
    });
    c
}

/// Smooth grey-green ceiba bark with lenticels and pale lichen.
fn bark(s: u32) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let base = fbm(u, v, 4, 5, s ^ 0x51);
        let striae = vnoise2(u, v, 24, 2, s ^ 0x52);
        let mut col = lerp3([0.16, 0.145, 0.12], [0.31, 0.29, 0.24], base * 0.7 + striae * 0.3);
        // Horizontal lenticel dashes.
        let len = vnoise2(u, v, 20, 90, s ^ 0x53);
        col = lerp3(col, [0.22, 0.21, 0.18], smooth(0.8, 0.9, len) * 0.7);
        let lichen = smooth(0.62, 0.75, fbm(u, v, 9, 4, s ^ 0x54));
        col = lerp3(col, [0.42, 0.42, 0.34], lichen * 0.4);
        let moss = smooth(0.66, 0.8, fbm(u, v, 5, 3, s ^ 0x55));
        col = lerp3(col, [0.11, 0.2, 0.08], moss * 0.55);
        set(p, col);
    });
    c
}

/// Canopy foliage clusters.
fn leaves(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let n = fbm(u, v, 8, 4, s ^ 0x61);
        let col = lerp3([0.12, 0.17, 0.08], [0.3, 0.38, 0.18], n);
        set(p, col);
    });
    let mut rng = Rng::fork(seed, 13);
    for _ in 0..2600 {
        let x = rng.range(0.0, 256.0);
        let y = rng.range(0.0, 256.0);
        let ang = rng.range(0.0, std::f32::consts::TAU);
        let len = rng.range(3.0, 7.0);
        let tone = rng.range(0.0, 1.0);
        let col = lerp3([0.14, 0.2, 0.09], [0.42, 0.5, 0.24], tone);
        for i in 0..len as i32 {
            let t = i as f32;
            let w = (1.0 - (t / len * 2.0 - 1.0).abs()) * 2.0;
            for k in -(w as i32)..=(w as i32) {
                let px = x + ang.cos() * t - ang.sin() * k as f32;
                let py = y + ang.sin() * t + ang.cos() * k as f32;
                c.blend(px as i32, py as i32, col, 0.8);
            }
        }
    }
    c
}

/// Woven fibre (burlap satchel, hat straw).
fn weave(s: u32, tint: [f32; 3], threads: i32) -> Canvas {
    let mut c = Canvas::new(128, 128, [0.0; 4]);
    c.for_each(|u, v, p| {
        let tu = (u * threads as f32 * std::f32::consts::TAU).sin();
        let tv = (v * threads as f32 * std::f32::consts::TAU).sin();
        let over = if ((u * threads as f32) as i32 + (v * threads as f32) as i32) % 2 == 0 {
            tu.abs()
        } else {
            tv.abs()
        };
        let fibre = vnoise(u, v, 64, s ^ 0x71);
        let stain = fbm(u, v, 4, 3, s ^ 0x72);
        let k = 0.55 + over * 0.35 + fibre * 0.2 - smooth(0.6, 0.85, stain) * 0.25;
        set(p, [tint[0] * k, tint[1] * k, tint[2] * k]);
    });
    c
}

/// Sun-rotted cotton for the Silbón: dark for the trousers, a worn,
/// grime-streaked lighter weave for the shirt.
fn cloth(s: u32, pale: bool) -> Canvas {
    let mut c = Canvas::new(128, 128, [0.0; 4]);
    c.for_each(|u, v, p| {
        let thread = vnoise2(u, v, 128, 16, s ^ 0x81) * 0.5 + vnoise2(u, v, 16, 128, s ^ 0x82) * 0.5;
        let dirt = fbm(u, v, 4, 4, s ^ 0x83);
        let (lo, hi) = if pale {
            ([0.4, 0.38, 0.33], [0.66, 0.63, 0.56])
        } else {
            ([0.2, 0.18, 0.15], [0.33, 0.29, 0.23])
        };
        let mut col = lerp3(lo, hi, thread * 0.6 + dirt * 0.4);
        if pale {
            // Earthy streaks running down from the shoulders.
            let streak = smooth(0.6, 0.85, vnoise2(u, v, 12, 3, s ^ 0x84));
            col = lerp3(col, [0.3, 0.26, 0.2], streak * 0.6);
        }
        set(p, col);
    });
    c
}

/// Terracotta for the tinajas.
fn clay(s: u32) -> Canvas {
    let mut c = Canvas::new(128, 128, [0.0; 4]);
    c.for_each(|u, v, p| {
        let n = fbm(u, v, 6, 4, s ^ 0x91);
        let band = smooth(0.4, 0.5, (v * 6.0).fract()) * smooth(0.6, 0.5, (v * 6.0).fract());
        let mut col = lerp3([0.46, 0.25, 0.15], [0.62, 0.38, 0.24], n);
        col = lerp3(col, [0.3, 0.16, 0.1], band * 0.3);
        set(p, col);
    });
    c
}

/// Aged paper with lines of illegible hand (the real text is shown in the UI).
fn note_paper(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(128, 160, [0.0; 4]);
    c.for_each(|u, v, p| {
        let n = fbm(u, v, 4, 4, s ^ 0xA1);
        let edge = smooth(0.0, 0.08, u.min(1.0 - u).min(v).min(1.0 - v));
        let col = lerp3([0.5, 0.43, 0.3], [0.8, 0.74, 0.58], n * 0.5 + edge * 0.5);
        set(p, col);
    });
    let mut rng = Rng::fork(seed, 14);
    let mut y = 22.0;
    while y < 140.0 {
        let mut x = 14.0 + rng.range(0.0, 6.0);
        let end = 114.0 - rng.range(0.0, 30.0);
        while x < end {
            let word = rng.range(6.0, 16.0);
            let mut t = 0.0;
            while t < word {
                let wy = y + (t * 0.9 + rng.range(0.0, 1.0)).sin() * 2.2;
                c.blend((x + t) as i32, wy as i32, [0.12, 0.1, 0.12], 0.8);
                c.blend((x + t) as i32, wy as i32 + 1, [0.12, 0.1, 0.12], 0.4);
                t += 0.7;
            }
            x += word + rng.range(3.0, 6.0);
        }
        y += 13.0;
    }
    c
}

/// A wall calendar page: ABRIL 1998.
fn calendar(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(128, 160, [0.0; 4]);
    c.for_each(|u, v, p| {
        let n = fbm(u, v, 4, 3, s ^ 0xB1);
        let col = lerp3([0.62, 0.58, 0.48], [0.82, 0.78, 0.66], n);
        set(p, col);
    });
    // Faded print of a landscape band at the top.
    for y in 6..52 {
        for x in 6..122 {
            let u = x as f32 / 128.0;
            let hill = 34.0 + (u * 9.0).sin() * 5.0;
            let col = if (y as f32) < hill {
                [0.55, 0.45, 0.4]
            } else {
                [0.4, 0.45, 0.3]
            };
            c.blend(x, y, col, 0.6);
        }
    }
    let mut rng = Rng::fork(seed, 15);
    stencil(&mut c, "ABRIL", 10, 60, 2, [0.5, 0.12, 0.1], 0.1, &mut rng);
    stencil(&mut c, "1998", 76, 60, 2, [0.2, 0.18, 0.18], 0.1, &mut rng);
    for row in 0..5 {
        for col in 0..7 {
            let x0 = 10 + col * 16;
            let y0 = 82 + row * 14;
            for k in 0..12 {
                c.blend(x0 + k, y0, [0.3, 0.3, 0.3], 0.5);
                c.blend(x0, y0 + k.min(10), [0.3, 0.3, 0.3], 0.5);
            }
            if rng.f32() < 0.7 {
                c.blend(x0 + 5, y0 + 5, [0.2, 0.2, 0.2], 0.8);
                c.blend(x0 + 6, y0 + 5, [0.2, 0.2, 0.2], 0.8);
            }
        }
    }
    c
}

/// Hand-painted ranch sign: HATO LA CEIBA.
fn sign(s: u32, seed: u64) -> Canvas {
    let mut c = Canvas::new(512, 128, [0.0; 4]);
    c.for_each(|u, v, p| {
        let grain = vnoise2(u, v, 6, 40, s ^ 0xC1) * 0.5 + fbm(u, v, 8, 3, s ^ 0xC2) * 0.5;
        let paint = smooth(0.3, 0.45, fbm(u, v, 10, 4, s ^ 0xC3));
        let wood = lerp3([0.3, 0.25, 0.19], [0.46, 0.4, 0.32], grain);
        let col = lerp3(wood, [0.78, 0.76, 0.68], paint * 0.85);
        set(p, col);
    });
    let mut rng = Rng::fork(seed, 16);
    stencil(&mut c, "HATO LA CEIBA", 22, 36, 6, [0.42, 0.08, 0.06], 0.18, &mut rng);
    // Painted arrow toward the gate (left).
    for i in 0..40 {
        for k in -3..=3 {
            c.blend(470 - i, 100 + k, [0.15, 0.12, 0.1], 0.8);
        }
    }
    for i in 0..12 {
        for k in -i..=i {
            c.blend(430 - 12 + i, 100 + k, [0.15, 0.12, 0.1], 0.85);
        }
    }
    c
}

/// A weathered, hand-lettered board with up to three lines of stencilled
/// capitals (the words are in `lore::sign`).
pub(super) fn sign_lines(images: &mut Assets<Image>, lines: &[&str], seed: u64) -> Handle<Image> {
    let (w, h) = (1024usize, 512usize);
    let s = seed as u32;
    let mut c = Canvas::new(w, h, [0.0; 4]);
    c.for_each(|u, v, p| {
        let plank = (v * 4.0).floor() as u32;
        let grain = vnoise2(u, v, 5, 60, s ^ plank.wrapping_mul(977));
        let wear = fbm(u, v, 8, 4, s ^ 0xAB);
        let seam = smooth(0.44, 0.5, ((v * 4.0).fract() - 0.5).abs());
        let col = lerp3([0.15, 0.115, 0.085], [0.36, 0.29, 0.21], grain * 0.6 + wear * 0.4);
        set(p, lerp3(col, [0.04, 0.03, 0.025], seam * 0.85));
    });
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as i32;
    let cell = ((w as i32 - 70) / (longest * 6)).clamp(3, 10);
    let line_h = cell * 7;
    let gap = cell * 3;
    let n = lines.len().max(1) as i32;
    let mut y = (h as i32 - (n * line_h + (n - 1) * gap)) / 2;
    let mut rng = Rng::fork(seed, 0x5163);
    for line in lines {
        let len = line.chars().count() as i32;
        let x = (w as i32 - len * 6 * cell + cell) / 2;
        stencil(&mut c, line, x, y, cell, [0.86, 0.83, 0.72], 0.1, &mut rng);
        y += line_h + gap;
    }
    c.into_image(images, false)
}

/// Soft radial glow for the moon halo (alpha = brightness).
fn halo() -> Canvas {
    let mut c = Canvas::new(128, 128, [0.0; 4]);
    c.for_each(|u, v, p| {
        let d = ((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt() * 2.0;
        let a = (1.0 - d).clamp(0.0, 1.0).powf(2.2);
        *p = [0.75, 0.82, 1.0, a];
    });
    c
}

// ----------------------------------------------------------------------------
// Data maps: how wet things are, where they are bumpy, how water moves
// ----------------------------------------------------------------------------

/// Roughness (G) and metallic (B) for the llano: wet, glossy mud where the
/// soil shows through the grass, matte grass elsewhere.
fn ground_mr(s: u32) -> Canvas {
    let mut c = Canvas::new(512, 512, [0.0; 4]);
    c.for_each(|u, v, p| {
        let bare = smooth(0.6, 0.78, fbm(u, v, 8, 3, s ^ 0x777));
        let sheen = fbm(u, v, 16, 3, s ^ 0x5151);
        let rough = 0.9 - bare * (0.55 + 0.15 * sheen);
        *p = [1.0, rough, 0.0, 1.0];
    });
    c
}

/// Packed road earth: soaked, with dry pebbles standing proud.
fn road_mr(s: u32) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let wet = smooth(0.45, 0.75, fbm(u, v, 3, 3, s ^ 0x9));
        let fine = fbm(u, v, 24, 3, s ^ 0x3131);
        let rough = 0.62 - wet * 0.42 + fine * 0.2;
        *p = [1.0, rough.clamp(0.08, 0.95), 0.0, 1.0];
    });
    c
}

/// Data maps hold roughness in G and metallic in B (glTF layout).
fn data_map(w: usize, h: usize, f: impl Fn(f32, f32) -> [f32; 2]) -> Canvas {
    let mut c = Canvas::new(w, h, [0.0; 4]);
    c.for_each(|u, v, p| {
        let [rough, metal] = f(u, v);
        *p = [1.0, rough.clamp(0.05, 1.0), metal.clamp(0.0, 1.0), 1.0];
    });
    c
}

/// Boards: dry, open grain is rough; pitch-dark cracks and wet streaks are
/// slightly smoother. Uses the albedo's grain so the two line up.
fn wood_mr(s: u32, dark: bool) -> Canvas {
    data_map(256, 256, |u, v| {
        let grain = vnoise2(u, v, 48, 3, s ^ 0x31) * 0.6 + vnoise2(u, v, 160, 8, s ^ 0x32) * 0.4;
        let streak = fbm(u, v, 6, 4, s ^ 0x33);
        let crack = smooth(0.82, 0.9, vnoise2(u, v, 96, 5, s ^ 0x34));
        let base = if dark { 0.8 } else { 0.72 };
        [base + (grain - 0.5) * 0.24 - streak * 0.1 + crack * 0.12, 0.0]
    })
}

/// Zinc: smooth bright ridge crests, matte rust and grime, metal only where
/// the sheet is still bare.
fn zinc_mr(s: u32) -> Canvas {
    data_map(256, 256, |u, v| {
        let ridge = (u * std::f32::consts::TAU * 8.0).sin() * 0.5 + 0.5;
        let rust = smooth(0.48, 0.7, fbm(u, v, 5, 5, s ^ 0x41));
        let streak = smooth(0.55, 0.8, vnoise2(u, v, 40, 2, s ^ 0x42)) * smooth(0.2, 0.8, v);
        let rusted = (rust + streak * 0.6).min(1.0);
        let grime = smooth(0.6, 0.9, fbm(u, v, 12, 3, s ^ 0x44));
        let rough = 0.36 + (1.0 - ridge) * 0.14 + rusted * 0.45 + grime * 0.12;
        [rough, (1.0 - rusted) * (1.0 - grime * 0.5) * 0.55]
    })
}

/// Bark: dry ridges and lichen are matte, damp moss holds a faint sheen.
fn bark_mr(s: u32) -> Canvas {
    data_map(256, 256, |u, v| {
        let base = fbm(u, v, 4, 5, s ^ 0x51);
        let lichen = smooth(0.62, 0.75, fbm(u, v, 9, 4, s ^ 0x54));
        let moss = smooth(0.66, 0.8, fbm(u, v, 5, 3, s ^ 0x55));
        [0.7 + (base - 0.5) * 0.2 + lichen * 0.15 - moss * 0.2, 0.0]
    })
}

/// Woven fibre (burlap, straw): always matte, with the over/under weave and
/// fibre noise giving tiny variation.
fn fibre_mr(s: u32, base: f32, threads: i32) -> Canvas {
    data_map(128, 128, |u, v| {
        let fibre = vnoise(u, v, 64, s ^ 0x71);
        let stain = fbm(u, v, 4, 3, s ^ 0x72);
        let t = (u * threads as f32 * std::f32::consts::TAU).sin().abs();
        [
            base + (fibre - 0.5) * 0.1 - t * 0.04 - smooth(0.6, 0.85, stain) * 0.06,
            0.0,
        ]
    })
}

/// Cotton: rough thread, greasy grime a little smoother; pale shirt streaks
/// stay matte so it never glints like plastic.
fn cloth_mr(s: u32, pale: bool) -> Canvas {
    data_map(128, 128, |u, v| {
        let thread = vnoise2(u, v, 128, 16, s ^ 0x81) * 0.5 + vnoise2(u, v, 16, 128, s ^ 0x82) * 0.5;
        let dirt = fbm(u, v, 4, 4, s ^ 0x83);
        let base = if pale { 0.9 } else { 0.95 };
        [base - (thread - 0.5) * 0.12 - dirt * 0.08, 0.0]
    })
}

/// The corrugation of tin roofs as a height map (ridges along U).
fn zinc_height(s: u32) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let ridge = (u * std::f32::consts::TAU * 8.0).sin() * 0.5 + 0.5;
        let dent = fbm(u, v, 6, 3, s ^ 0x6161) * 0.25;
        let h = ridge * 0.75 + dent;
        *p = [h, h, h, 1.0];
    });
    c
}

/// Eight frames of raindrop rings expanding and fading, side by side. Each
/// frame is a normal map; stepping the UV across the strip animates every
/// puddle at once, and neighbouring tiles show different frames.
fn ripples(seed: u64) -> Canvas {
    const FRAMES: usize = 8;
    const TILE: usize = 96;
    let mut c = Canvas::new(FRAMES * TILE, TILE, [0.5, 0.5, 1.0, 1.0]);
    let mut rng = Rng::fork(seed, 0x91B);
    // Each ring is born at a random spot and frame and lives the whole loop.
    let rings: Vec<(f32, f32, usize)> = (0..7)
        .map(|_| (rng.range(0.25, 0.75), rng.range(0.25, 0.75), rng.below(FRAMES)))
        .collect();
    for f in 0..FRAMES {
        let mut height = vec![0.0f32; TILE * TILE];
        for &(cx, cy, born) in &rings {
            let age = ((f + FRAMES - born) % FRAMES) as f32 / FRAMES as f32;
            let radius = 0.04 + age * 0.22;
            let amp = (1.0 - age).powf(1.5);
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = x as f32 / TILE as f32 - cx;
                    let dy = y as f32 / TILE as f32 - cy;
                    let d = (dx * dx + dy * dy).sqrt();
                    let w = 0.022;
                    let k = (d - radius) / w;
                    height[y * TILE + x] += (k * 3.0).sin() * (-k * k).exp() * amp;
                }
            }
        }
        let at =
            |x: i32, y: i32| height[y.clamp(0, TILE as i32 - 1) as usize * TILE + x.clamp(0, TILE as i32 - 1) as usize];
        for y in 0..TILE as i32 {
            for x in 0..TILE as i32 {
                let nx = (at(x - 1, y) - at(x + 1, y)) * 2.2;
                let ny = (at(x, y + 1) - at(x, y - 1)) * 2.2;
                let len = (nx * nx + ny * ny + 1.0).sqrt();
                c.px[y as usize * FRAMES * TILE + f * TILE + x as usize] =
                    [nx / len * 0.5 + 0.5, ny / len * 0.5 + 0.5, 1.0 / len * 0.5 + 0.5, 1.0];
            }
        }
    }
    c
}

/// Long, lazy swell for open water, tileable.
fn water_height(s: u32) -> Canvas {
    let mut c = Canvas::new(256, 256, [0.0; 4]);
    c.for_each(|u, v, p| {
        let a = vnoise2(u, v, 4, 14, s ^ 0x7A1) * 0.5;
        let b = fbm(u, v, 8, 4, s ^ 0x7A2) * 0.5;
        let h = a + b;
        *p = [h, h, h, 1.0];
    });
    c
}
