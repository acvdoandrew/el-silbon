// Wind for grass and reeds: the vertex stage of StandardMaterial with a
// root-pinned sway. Lighting, fog and shadows stay the stock PBR fragment.
//
// Mesh attributes (written by world/flora.rs):
//   UV_0    = the blade's root in world (x, z): every vertex of a blade reads
//             the same wind, so blades bend instead of shearing.
//   COLOR.a = sway weight in metres of tip travel per unit of wind: zero at the
//             root, growing with the square of the height up the blade and with
//             the blade's length. The rgb is the usual vertex tint.
//
// Uniform: xy = downwind direction on the ground (unit), z = strength,
// w = tempo (1 = default).
//
// The wind is one coherent field of world position and the global clock:
// a soft breathing swell that rolls across the field, gust fronts that travel
// downwind as broad bands (wavering sideways so they are not straight lines),
// and a little flutter that only wakes inside a gust. Nothing on the CPU moves.

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> wind: vec4<f32>;

fn hash11(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Ground displacement of a blade tip (per unit weight) at `root`.
fn sway(root: vec2<f32>, t: f32) -> vec2<f32> {
    let d = wind.xy;
    let side = vec2<f32>(-d.y, d.x);
    let along = dot(root, d);
    let across = dot(root, side);
    let time = t * wind.w;

    // Broad gust fronts rolling downwind, bent by a slow sideways wander.
    let front = along * 0.075 - time * 0.55 + sin(across * 0.055 + time * 0.07) * 1.4;
    let band = 0.5 + 0.5 * sin(front);
    let gust = band * band * (3.0 - 2.0 * band);

    // Ambient breathing that always rolls through, so calm air still moves.
    let swell = 0.32 + 0.14 * sin(along * 0.29 - time * 1.25 + sin(across * 0.17) * 1.1);

    // Per-blade phase for flutter, strongest in gusts.
    let phase = hash11(floor(root * 23.0)) * 6.2831853;
    let flutter = sin(time * 3.3 + phase + along * 0.8) * (0.05 + 0.13 * gust);
    let lateral = sin(along * 0.21 - time * 0.85 + 1.7) * 0.16 * (0.4 + gust)
        + cos(time * 2.6 + phase) * 0.04 * gust;

    return d * (swell + 0.85 * gust + flutter) + side * lateral;
}

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));

    let weight = v.color.a * wind.z;
    let bend = sway(v.uv, globals.time) * weight;
    // A blade bends rather than stretching: what it gains in reach it loses in height.
    world = vec4<f32>(
        world.x + bend.x,
        world.y - dot(bend, bend) * 0.45,
        world.z + bend.y,
        world.w,
    );

    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(v.normal, v.instance_index);
    out.uv = v.uv;
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, v.tangent, v.instance_index);
#endif
    // The weight rode in on alpha; the blade itself is opaque.
    out.color = vec4<f32>(v.color.rgb, 1.0);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = v.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        v.instance_index, world_from_local[3]);
#endif
    return out;
}
