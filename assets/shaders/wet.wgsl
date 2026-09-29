// Standing water under rain: StandardMaterial lit as usual, with raindrop
// rings computed from world position and the global clock. Every drop lives
// in a jittered cell, is reborn elsewhere in that cell each cycle, expands as
// a thin ring and fades. The rings tilt the normal (so lamps and sky glint in
// them) and carry a faint sky-coloured glint of their own, so they read on a
// black night surface. Nothing on the CPU moves per frame.
//
// Uniform: x = cell size in metres, y = tempo (drop cycles scale with it),
// z = ring strength (0 = still water), w = distance in metres where rings have
// faded away (they alias into noise beyond it).

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, globals},
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> ripple: vec4<f32>;

fn hash21(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

fn hash11(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// x,y: horizontal slope the rings add to the surface; z: ring glint.
fn rings(p: vec2<f32>, t: f32) -> vec3<f32> {
    let size = ripple.x;
    let base = floor(p / size);
    var slope = vec2<f32>(0.0);
    var glint = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let cell = base + vec2<f32>(f32(i), f32(j));
            // Each cell has its own tempo, so drops never march in step.
            let period = (1.1 + 1.3 * hash11(cell + 7.7)) / ripple.y;
            let clock = t / period + hash11(cell + 3.1);
            let cycle = floor(clock);
            let age = clock - cycle;
            // Only some cycles bring a drop, so the surface is not a lattice.
            if (hash11(cell * 1.7 + cycle * 5.3) > 0.62) {
                continue;
            }
            let spot = hash21(cell + cycle * 17.31);
            let center = (cell + 0.12 + 0.76 * spot) * size;
            let delta = p - center;
            let dist = length(delta);
            let radius = age * size * 0.6;
            let width = 0.028 + 0.03 * age;
            let x = (dist - radius) / width;
            let fade = (1.0 - age) * (1.0 - age) * smoothstep(0.0, 0.05, age);
            let env = exp(-x * x);
            // Height ~ env * cos(2x); slope is its derivative across the ring.
            let dh = env * (-2.0 * x * cos(2.0 * x) - 2.0 * sin(2.0 * x)) / width;
            let dir = delta / max(dist, 0.001);
            slope += dir * dh * fade * 0.004;
            glint += env * fade;
        }
    }
    return vec3<f32>(slope, glint);
}

@fragment
fn fragment(in_vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    // The swell normal map creeps across the surface (stock uv_transform is identity).
    var in = in_vertex;
    in.uv = in_vertex.uv + vec2<f32>(globals.time * 0.011, globals.time * 0.017);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let dist = distance(view.world_position.xyz, in.world_position.xyz);
    let near = 1.0 - smoothstep(ripple.w * 0.55, ripple.w, dist);
    let r = rings(in.world_position.xz, globals.time);
    let k = ripple.z * near;
    pbr_input.N = normalize(pbr_input.N + vec3<f32>(r.x, 0.0, r.y) * k * 2.0);
    // A cold glint on each ring; lighting scales it by the surface's alpha.
    pbr_input.material.emissive = vec4<f32>(
        pbr_input.material.emissive.rgb + vec3<f32>(0.20, 0.30, 0.38) * r.z * k * 0.16,
        pbr_input.material.emissive.a,
    );

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
