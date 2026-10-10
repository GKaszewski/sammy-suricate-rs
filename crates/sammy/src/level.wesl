// Acknex-style world surface: texture * baked lightmap (2x overbright), no dynamic lighting.
// Flat (non-lightmapped) surfaces get a constant brightness instead.
import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var base_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var lightmap_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var lightmap_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(base_texture, base_sampler, in.uv);
    var light = vec3<f32>(params.y);
    @if(VERTEX_UVS_B) {
        light = textureSample(lightmap_texture, lightmap_sampler, in.uv_b).rgb * params.x;
    }
    return vec4<f32>(color.rgb * light, 1.0);
}
