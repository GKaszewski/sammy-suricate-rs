//! OpenGL ES 2 drawing: three programs (the level's texture × lightmap surfaces, lit
//! models and sprites, flat 2D for the HUD), textures and meshes. Everything is in the
//! game's native coordinates (X forward, Y left, Z up, quants).
//!
//! Colours are computed like the Bevy frontend, which lights in linear space and shows
//! sRGB: in gamma space that is `texture × light^(1/2.2)`.

use glam::{Mat4, Vec3};
use glow::HasContext;

/// Most dynamic lights a draw takes (the nearest to the camera).
pub const MAX_LIGHTS: usize = 8;
/// Acknex lightmaps are 2× overbright: mid-grey means full texture brightness.
const LIGHTMAP_SCALE: f32 = 2.0;
/// Alpha below this is discarded (colour-keyed sprites and skins).
pub const ALPHA_CUTOFF: f32 = 0.5;

const WORLD_VS: &str = r#"
attribute vec3 a_pos; attribute vec2 a_uv; attribute vec2 a_uv2;
uniform mat4 u_mvp;
varying vec2 v_uv; varying vec2 v_uv2; varying vec3 v_world;
void main() { v_uv = a_uv; v_uv2 = a_uv2; v_world = a_pos; gl_Position = u_mvp * vec4(a_pos, 1.0); }
"#;

const WORLD_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex; uniform sampler2D u_lightmap;
uniform float u_lit;
uniform vec4 u_lights[16];
varying vec2 v_uv; varying vec2 v_uv2; varying vec3 v_world;
void main() {
    vec4 c = texture2D(u_tex, v_uv);
    vec3 light = u_lit > 0.5 ? texture2D(u_lightmap, v_uv2).rgb * LIGHTMAP_SCALE : vec3(1.0);
    for (int i = 0; i < 8; i++) {
        vec4 p = u_lights[i * 2];
        if (p.w > 0.0) light += u_lights[i * 2 + 1].rgb * max(0.0, 1.0 - distance(v_world, p.xyz) / p.w) * 2.0;
    }
    gl_FragColor = vec4(c.rgb * pow(light, vec3(1.0 / 2.2)), 1.0);
}
"#;

const MODEL_VS: &str = r#"
attribute vec3 a_pos; attribute vec3 a_normal; attribute vec2 a_uv;
uniform mat4 u_mvp; uniform mat4 u_model;
varying vec2 v_uv; varying vec3 v_normal; varying vec3 v_world;
void main() {
    v_uv = a_uv;
    v_normal = mat3(u_model[0].xyz, u_model[1].xyz, u_model[2].xyz) * a_normal;
    v_world = (u_model * vec4(a_pos, 1.0)).xyz;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

const MODEL_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec4 u_color; uniform vec2 u_uv_offset;
uniform float u_cutoff; uniform float u_unlit;
uniform vec3 u_sun; uniform vec3 u_ambient;
uniform vec4 u_lights[16];
varying vec2 v_uv; varying vec3 v_normal; varying vec3 v_world;
void main() {
    vec4 c = texture2D(u_tex, v_uv + u_uv_offset) * u_color;
    if (c.a < u_cutoff) discard;
    vec3 light = vec3(1.0);
    if (u_unlit < 0.5) {
        vec3 n = normalize(v_normal);
        light = u_ambient + vec3(max(dot(n, u_sun), 0.0));
        for (int i = 0; i < 8; i++) {
            vec4 p = u_lights[i * 2];
            if (p.w > 0.0) light += u_lights[i * 2 + 1].rgb * max(0.0, 1.0 - distance(v_world, p.xyz) / p.w) * 2.0;
        }
    }
    gl_FragColor = vec4(c.rgb * pow(light, vec3(1.0 / 2.2)), c.a);
}
"#;

const UI_VS: &str = r#"
attribute vec2 a_pos; attribute vec2 a_uv;
uniform mat4 u_mvp;
varying vec2 v_uv;
void main() { v_uv = a_uv; gl_Position = u_mvp * vec4(a_pos, 0.0, 1.0); }
"#;

const UI_FS: &str = r#"
precision mediump float;
uniform sampler2D u_tex; uniform vec4 u_color;
varying vec2 v_uv;
void main() { vec4 c = texture2D(u_tex, v_uv) * u_color; if (c.a < 0.01) discard; gl_FragColor = c; }
"#;

/// A linked program and its uniforms by name.
pub struct Program {
    pub program: glow::Program,
    uniforms: Vec<(&'static str, Option<glow::UniformLocation>)>,
}

impl Program {
    fn new(gl: &glow::Context, vs: &str, fs: &str, attribs: &[&str], uniforms: &[&'static str]) -> anyhow::Result<Self> {
        let header = format!("#define LIGHTMAP_SCALE {LIGHTMAP_SCALE:.1}\n");
        unsafe {
            let program = gl.create_program().map_err(anyhow::Error::msg)?;
            for (kind, src) in [(glow::VERTEX_SHADER, vs.to_owned()), (glow::FRAGMENT_SHADER, header.clone() + fs)] {
                let s = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
                gl.shader_source(s, &src);
                gl.compile_shader(s);
                if !gl.get_shader_compile_status(s) {
                    anyhow::bail!("shader: {}", gl.get_shader_info_log(s));
                }
                gl.attach_shader(program, s);
            }
            for (i, a) in attribs.iter().enumerate() {
                gl.bind_attrib_location(program, i as u32, a);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                anyhow::bail!("program: {}", gl.get_program_info_log(program));
            }
            let uniforms = uniforms.iter().map(|u| (*u, gl.get_uniform_location(program, u))).collect();
            Ok(Self { program, uniforms })
        }
    }

    pub fn loc(&self, name: &str) -> Option<&glow::UniformLocation> {
        self.uniforms.iter().find(|(n, _)| *n == name).and_then(|(_, l)| l.as_ref())
    }
}

/// A GPU texture and its size.
#[derive(Clone, Copy, Debug)]
pub struct Texture {
    pub tex: glow::Texture,
    pub width: u32,
    pub height: u32,
}

/// Vertex layout of a mesh: float counts per attribute, in attribute order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout {
    /// pos 3, uv 2, uv2 2 (level).
    World,
    /// pos 3, normal 3, uv 2 (models, prefabs, sprites).
    Model,
    /// pos 2, uv 2 (HUD).
    Ui,
}

impl Layout {
    fn sizes(self) -> &'static [i32] {
        match self {
            Layout::World => &[3, 2, 2],
            Layout::Model => &[3, 3, 2],
            Layout::Ui => &[2, 2],
        }
    }

    pub fn floats(self) -> usize {
        self.sizes().iter().sum::<i32>() as usize
    }
}

/// An indexed triangle mesh (interleaved floats, u32 indices).
pub struct Mesh {
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    count: i32,
    layout: Layout,
}

fn bytes<T>(v: &[T]) -> &[u8] {
    // Plain-old-data slices (f32 / u32) as bytes for upload.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

impl Mesh {
    pub fn new(gl: &glow::Context, layout: Layout, vertices: &[f32], indices: &[u32], dynamic: bool) -> Self {
        unsafe {
            let vbo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(vertices), if dynamic { glow::DYNAMIC_DRAW } else { glow::STATIC_DRAW });
            let ibo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes(indices), glow::STATIC_DRAW);
            Self { vbo, ibo, count: indices.len() as i32, layout }
        }
    }

    /// Replaces the vertices (same count).
    pub fn update(&self, gl: &glow::Context, vertices: &[f32]) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes(vertices));
        }
    }

    pub fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            let stride = self.layout.floats() as i32 * 4;
            let mut offset = 0;
            for (i, &n) in self.layout.sizes().iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32);
                gl.vertex_attrib_pointer_f32(i as u32, n, glow::FLOAT, false, stride, offset);
                offset += n * 4;
            }
            gl.draw_elements(glow::TRIANGLES, self.count, glow::UNSIGNED_INT, 0);
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
        }
    }
}

/// How a model or sprite is drawn.
#[derive(Clone, Copy, Debug)]
pub struct Shade {
    /// RGBA multiplier (alpha = opacity).
    pub color: [f32; 4],
    pub unlit: bool,
    /// Alpha test (colour-keyed) rather than blending.
    pub cutoff: f32,
    /// Horizontal UV shift (sprite strips).
    pub uv_offset: f32,
}

impl Default for Shade {
    fn default() -> Self {
        Self { color: [1.0; 4], unlit: false, cutoff: ALPHA_CUTOFF, uv_offset: 0.0 }
    }
}

/// Dynamic lights for a frame: position + range, colour (0..1).
pub type Lights = [[f32; 4]; MAX_LIGHTS * 2];

pub struct Gfx {
    pub gl: glow::Context,
    world: Program,
    model: Program,
    ui: Program,
    pub white: Texture,
    /// Direction toward the sun and the ambient light for models.
    pub sun: Vec3,
    pub ambient: Vec3,
}

/// The fallback scene light, matched to the Bevy frontend's look.
const SUN_DIRECTION: [f32; 3] = [-1.5, -1.0, 2.0];
const SUN_STRENGTH: f32 = 0.75;
const AMBIENT: f32 = 0.45;

impl Gfx {
    pub fn new(gl: glow::Context) -> anyhow::Result<Self> {
        let world = Program::new(&gl, WORLD_VS, WORLD_FS, &["a_pos", "a_uv", "a_uv2"], &["u_mvp", "u_tex", "u_lightmap", "u_lit", "u_lights"])?;
        let model = Program::new(
            &gl,
            MODEL_VS,
            MODEL_FS,
            &["a_pos", "a_normal", "a_uv"],
            &["u_mvp", "u_model", "u_tex", "u_color", "u_uv_offset", "u_cutoff", "u_unlit", "u_sun", "u_ambient", "u_lights"],
        )?;
        let ui = Program::new(&gl, UI_VS, UI_FS, &["a_pos", "a_uv"], &["u_mvp", "u_tex", "u_color"])?;
        let white = upload(&gl, &formats::image::RgbaImage { width: 1, height: 1, data: vec![255; 4] }, false, true);
        Ok(Self { gl, world, model, ui, white, sun: Vec3::from(SUN_DIRECTION).normalize() * SUN_STRENGTH, ambient: Vec3::splat(AMBIENT) })
    }

    /// Uploads an image (`repeat` for level surfaces, clamped otherwise).
    pub fn texture(&self, img: &formats::image::RgbaImage, repeat: bool, linear: bool) -> Texture {
        upload(&self.gl, img, repeat, linear)
    }

    pub fn delete_texture(&self, t: Texture) {
        unsafe { self.gl.delete_texture(t.tex) };
    }

    fn bind(&self, unit: u32, t: Texture) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(t.tex));
        }
    }

    /// A level batch: texture × lightmap (or flat light) plus dynamic lights.
    pub fn draw_world(&self, mesh: &Mesh, view_proj: &Mat4, tex: Texture, lightmap: Option<Texture>, lights: &Lights) {
        let (gl, p) = (&self.gl, &self.world);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &view_proj.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_1_i32(p.loc("u_lightmap"), 1);
            gl.uniform_1_f32(p.loc("u_lit"), if lightmap.is_some() { 1.0 } else { 0.0 });
            gl.uniform_4_f32_slice(p.loc("u_lights"), lights.as_flattened());
        }
        self.bind(0, tex);
        self.bind(1, lightmap.unwrap_or(self.white));
        mesh.draw(gl);
    }

    /// A model, prefab part or sprite at `model` (world transform).
    pub fn draw_model(&self, mesh: &Mesh, view_proj: &Mat4, model: &Mat4, tex: Texture, shade: &Shade, lights: &Lights) {
        let (gl, p) = (&self.gl, &self.model);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &(*view_proj * *model).to_cols_array());
            gl.uniform_matrix_4_f32_slice(p.loc("u_model"), false, &model.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_4_f32_slice(p.loc("u_color"), &shade.color);
            gl.uniform_2_f32(p.loc("u_uv_offset"), shade.uv_offset, 0.0);
            gl.uniform_1_f32(p.loc("u_cutoff"), shade.cutoff);
            gl.uniform_1_f32(p.loc("u_unlit"), if shade.unlit { 1.0 } else { 0.0 });
            gl.uniform_3_f32_slice(p.loc("u_sun"), &self.sun.to_array());
            gl.uniform_3_f32_slice(p.loc("u_ambient"), &self.ambient.to_array());
            gl.uniform_4_f32_slice(p.loc("u_lights"), lights.as_flattened());
        }
        self.bind(0, tex);
        mesh.draw(gl);
    }

    /// A 2D quad mesh in screen pixels (`proj` maps pixels to clip space).
    pub fn draw_ui(&self, mesh: &Mesh, proj: &Mat4, tex: Texture, color: [f32; 4]) {
        let (gl, p) = (&self.gl, &self.ui);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &proj.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_4_f32_slice(p.loc("u_color"), &color);
        }
        self.bind(0, tex);
        mesh.draw(&self.gl);
    }
}

/// Uploads an RGBA image as a 2D texture.
fn upload(gl: &glow::Context, img: &formats::image::RgbaImage, repeat: bool, linear: bool) -> Texture {
    unsafe {
        let tex = gl.create_texture().expect("texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        let wrap = if repeat { glow::REPEAT } else { glow::CLAMP_TO_EDGE } as i32;
        let filter = if linear { glow::LINEAR } else { glow::NEAREST } as i32;
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            img.width as i32,
            img.height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&img.data)),
        );
        Texture { tex, width: img.width, height: img.height }
    }
}
