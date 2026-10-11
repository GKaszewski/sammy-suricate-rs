//! GLSL ES 1.0 sources of the programs.

// Shaders are kept cheap for tile-based handheld GPUs: the level is texture × a lightmap
// with the gamma curve baked in (dynamic lights add in gamma space, in chunks they reach), models
// are lit per vertex like A4's Gouraud models, and only colour-keyed textures use the
// `discard` variant (which costs hidden-surface removal).

pub(super) const WORLD_VS: &str = r#"
attribute vec3 a_pos; attribute vec2 a_uv; attribute vec2 a_uv2;
uniform mat4 u_mvp;
varying vec2 v_uv; varying vec2 v_uv2; varying vec3 v_world;
void main() { v_uv = a_uv; v_uv2 = a_uv2; v_world = a_pos; gl_Position = u_mvp * vec4(a_pos, 1.0); }
"#;

pub(super) const WORLD_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex; uniform sampler2D u_lightmap;
uniform vec4 u_lights[16]; uniform int u_light_count;
varying vec2 v_uv; varying vec2 v_uv2; varying vec3 v_world;
void main() {
    vec4 c = texture2D(u_tex, v_uv);
    vec3 light = texture2D(u_lightmap, v_uv2).rgb * GAMMA_MAX;
#ifdef DYNAMIC
    // Added on top of the lightmap in gamma space (as A4's 16-bit renderer did): no pow.
    // DYNAMIC is the exact count: a fixed loop the GPU compiler can unroll.
    for (int i = 0; i < DYNAMIC; i++) {
        vec4 p = u_lights[i * 2];
        light += u_lights[i * 2 + 1].rgb * max(0.0, 1.0 - distance(v_world, p.xyz) / p.w) * GAMMA_MAX;
    }
#endif
    gl_FragColor = vec4(c.rgb * light, 1.0);
}
"#;

pub(super) const MODEL_VS: &str = r#"
attribute vec3 a_pos; attribute vec3 a_normal; attribute vec2 a_uv;
uniform mat4 u_mvp; uniform mat4 u_model;
uniform float u_unlit; uniform vec3 u_sun; uniform vec3 u_ambient;
uniform vec4 u_lights[16]; uniform int u_light_count;
varying vec2 v_uv; varying vec3 v_light;
void main() {
    v_uv = a_uv;
    v_light = vec3(1.0);
    if (u_unlit < 0.5) {
        vec3 n = normalize(mat3(u_model[0].xyz, u_model[1].xyz, u_model[2].xyz) * a_normal);
        vec3 world = (u_model * vec4(a_pos, 1.0)).xyz;
        vec3 light = u_ambient + vec3(max(dot(n, u_sun), 0.0));
        for (int i = 0; i < 8; i++) {
            if (i >= u_light_count) break;
            vec4 p = u_lights[i * 2];
            light += u_lights[i * 2 + 1].rgb * max(0.0, 1.0 - distance(world, p.xyz) / p.w) * 2.0;
        }
        v_light = pow(light, vec3(1.0 / 2.2));
    }
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

pub(super) const MODEL_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec4 u_color; uniform vec2 u_uv_offset; uniform float u_cutoff;
varying vec2 v_uv; varying vec3 v_light;
void main() {
    vec4 c = texture2D(u_tex, v_uv + u_uv_offset) * u_color;
#ifdef CUTOFF
    if (c.a < u_cutoff) discard;
#endif
    gl_FragColor = vec4(c.rgb * v_light, c.a);
}
"#;

pub(super) const PARTICLE_VS: &str = r#"
attribute vec3 a_pos; attribute vec2 a_uv; attribute vec4 a_color;
uniform mat4 u_mvp;
varying vec2 v_uv; varying vec4 v_color;
void main() { v_uv = a_uv; v_color = a_color; gl_Position = u_mvp * vec4(a_pos, 1.0); }
"#;

pub(super) const PARTICLE_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
varying vec2 v_uv; varying vec4 v_color;
void main() { gl_FragColor = texture2D(u_tex, v_uv) * v_color; }
"#;

// Engine model shadows: the mesh flattened along the sun direction onto the floor
// plane under its owner, 50 % black; the stencil keeps overlaps from darkening twice.
pub(super) const SHADOW_VS: &str = r#"
attribute vec3 a_pos;
uniform mat4 u_vp; uniform mat4 u_model;
uniform vec4 u_sun;
void main() {
    vec3 w = (u_model * vec4(a_pos, 1.0)).xyz;
    w -= u_sun.xyz * ((w.z - u_sun.w) / u_sun.z);
    gl_Position = u_vp * vec4(w, 1.0);
}
"#;

pub(super) const SHADOW_FS: &str = r#"
precision mediump float;
uniform vec4 u_color;
void main() { gl_FragColor = u_color; }
"#;

pub(super) const UI_VS: &str = r#"
attribute vec2 a_pos; attribute vec2 a_uv;
uniform mat4 u_mvp;
varying vec2 v_uv;
void main() { v_uv = a_uv; gl_Position = u_mvp * vec4(a_pos, 0.0, 1.0); }
"#;

pub(super) const UI_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex; uniform vec4 u_color;
varying vec2 v_uv;
void main() { vec4 c = texture2D(u_tex, v_uv) * u_color; if (c.a < 0.01) discard; gl_FragColor = c; }
"#;
