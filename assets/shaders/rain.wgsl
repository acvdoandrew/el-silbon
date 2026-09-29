// Falling rain: a thin veil of soft, short camera-facing streaks in one mesh.
// The box of rain is anchored to the camera in world space (each streak wraps
// around it) and the fall is animated here from a phase the CPU integrates
// from the shared storm clock, so every player's rain moves with the same sky
// and changing the storm's strength never makes streaks jump.
//
// Mesh attributes: POSITION = the streak's anchor inside the box in [0,1)^3,
// UV_0 = (-1..1 across, 0 head .. 1 tail), COLOR = (speed, brightness, length)
// jitter.
//
// Blending is AlphaMode::Premultiplied: out = src.rgb + dst * (1 - src.a).
// The fragment therefore returns colour ALREADY multiplied by its coverage;
// returning the bare tint (as this shader once did) would add the full tint
// colour to every pixel of every quad, however transparent, and blot the
// night with bright bars.

#import bevy_pbr::{
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

// rgb tint, a = overall streak opacity (the storm's strength).
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> tint: vec4<f32>;
// Box size (xyz) and fall speed (w).
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> volume: vec4<f32>;
// Wind (x, z), streak length, streak width.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> motion: vec4<f32>;
// Distance the rain has drifted (x, y, z; y is negative: down), and the
// number of sheltered volumes in `shelters` (w).
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> flow: vec4<f32>;
// Roofed places, two vec4 each: [2i] = (min x, min z, max x, max z) and
// [2i+1].x = the height of the roof above them. No rain falls below it.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> shelters: array<vec4<f32>, 32>;

// Streaks fade in over this range of distance from the lens, and out again
// toward the far side of the box: rain is texture in the middle distance, never
// a curtain over the view or over whatever stands in the light.
const NEAR_START: f32 = 3.5;
const NEAR_END: f32 = 9.0;
const FAR_START: f32 = 10.5;
const FAR_END: f32 = 14.5;
// How softly rain gives way at the edge and top of a roof.
const SHELTER_SOFT: f32 = 0.45;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fade: f32,
};

// 1 where a point is under a roof, easing to 0 across its edges.
fn covered(p: vec3<f32>) -> f32 {
    var under = 0.0;
    let count = u32(flow.w);
    for (var i = 0u; i < count; i = i + 1u) {
        let r = shelters[2u * i];
        let top = shelters[2u * i + 1u].x;
        let dx = max(max(r.x - p.x, p.x - r.z), 0.0);
        let dz = max(max(r.y - p.z, p.z - r.w), 0.0);
        let inside = 1.0 - smoothstep(0.0, SHELTER_SOFT, length(vec2<f32>(dx, dz)));
        let below = 1.0 - smoothstep(top - SHELTER_SOFT, top, p.y);
        under = max(under, inside * below);
    }
    return under;
}

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let size = volume.xyz;
    let cam = view.world_position;
    let speed = volume.w * (0.85 + 0.3 * v.color.x);
    let vel = vec3<f32>(motion.x, -speed, motion.y);
    // Anchor, carried by the fall and wrapped into the box around the camera.
    let raw = v.position * size + vec3<f32>(flow.x, flow.y * (0.85 + 0.3 * v.color.x), flow.z);
    let k = ceil((cam - size * 0.5 - raw) / size);
    let head = raw + k * size;
    let dir = normalize(vel);
    let len = motion.z * (0.7 + 0.6 * v.color.z);
    let to_cam = normalize(cam - head);
    var side = cross(dir, to_cam);
    if (dot(side, side) < 1e-6) {
        side = vec3<f32>(1.0, 0.0, 0.0);
    }
    side = normalize(side);
    let world = head - dir * (v.uv.y * len) + side * (v.uv.x * motion.w * 0.5);
    out.clip = position_world_to_clip(world);
    out.uv = v.uv;
    // Fade near the lens, toward the far side and edges of the box, and
    // under every roof.
    let rel = (head - cam) / size;
    let edge = 1.0 - smoothstep(0.30, 0.48, max(abs(rel.x), max(abs(rel.y), abs(rel.z))));
    let d = distance(cam, head);
    let depth = smoothstep(NEAR_START, NEAR_END, d) * (1.0 - smoothstep(FAR_START, FAR_END, d));
    out.fade = edge * depth * (1.0 - covered(head)) * (0.4 + 0.6 * v.color.y);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft across the width, eased in at the head and fading to nothing at
    // the tail.
    let across = 1.0 - abs(in.uv.x);
    let head = smoothstep(0.0, 0.2, in.uv.y);
    let tail = pow(1.0 - in.uv.y, 1.3);
    let a = clamp(pow(across, 1.6) * head * tail * in.fade * tint.a, 0.0, 1.0);
    return vec4<f32>(tint.rgb * a, a);
}
