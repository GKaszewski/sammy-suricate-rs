// A4 engine model shadow (`my.shadow = 1`): the model flattened onto the floor plane
// below it along the sun direction, drawn as a flat translucent dark silhouette.
import bevy_pbr::render::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
};

struct ShadowParams {
    // xyz: direction the sunlight travels (Bevy space, pointing down); w: floor height.
    sun: vec4<f32>,
    color: vec4<f32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: ShadowParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let v = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    var w = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0)).xyz;
    let height = max(w.y - params.sun.w, 0.0);
    w = w + params.sun.xyz * (height / max(-params.sun.y, 0.05));
    w.y = params.sun.w;
    out.world_position = vec4<f32>(w, 1.0);
    out.position = position_world_to_clip(w);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return params.color;
}
