// Acknex-style world surface: texture * baked lightmap (2x overbright), no dynamic lighting.
// Flat (non-lightmapped) surfaces get a constant brightness instead.
import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var base_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var lightmap_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var lightmap_sampler: sampler;

struct Lights {
    data: array<vec4<f32>, 16>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var<uniform> lights: Lights;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(base_texture, base_sampler, in.uv);
    var light = vec3<f32>(params.y);
    @if(VERTEX_UVS_B) {
        light = textureSample(lightmap_texture, lightmap_sampler, in.uv_b).rgb * params.x;
    }
    // A4 dynamic lights: added on top of the static light, linear falloff to the range.
    for (var i = 0u; i < 8u; i++) {
        let p = lights.data[i * 2u];
        if p.w > 0.0 {
            let d = distance(in.world_position.xyz, p.xyz);
            light += lights.data[i * 2u + 1u].rgb * max(0.0, 1.0 - d / p.w) * 2.0;
        }
    }
    return vec4<f32>(color.rgb * light, 1.0);
}
