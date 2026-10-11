//! OpenGL ES 2 drawing: programs for the level's texture × lightmap surfaces, lit
//! models and sprites, and flat 2D for the HUD, textures and meshes. Everything is in the
//! game's native coordinates (X forward, Y left, Z up, quants).
//!
//! Colours are computed like the Bevy frontend, which lights in linear space and shows
//! sRGB: in gamma space that is `texture × light^(1/2.2)`.

use glam::{Mat4, Vec3};
use glow::HasContext;

mod draw;
mod mesh;
mod program;
mod shaders;
mod texture;

pub use self::{mesh::*, program::*, texture::*};
use self::shaders::*;

/// Most dynamic lights a draw takes (those reaching it, nearest first).
pub const MAX_LIGHTS: usize = 8;
/// Acknex lightmaps are 2× overbright: mid-grey means full texture brightness.
const LIGHTMAP_SCALE: f32 = 2.0;
/// Lightmaps are stored gamma-encoded (`(2v)^(1/2.2)`), divided by its maximum.
const GAMMA_MAX: f32 = 1.370_351; // 2^(1/2.2)
/// Alpha below this is discarded (colour-keyed sprites and skins).
pub const ALPHA_CUTOFF: f32 = 0.5;

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

/// The dynamic lights of a draw: `count` pairs of position + range, colour (0..1).
#[derive(Clone, Copy, Debug, Default)]
pub struct Lights {
    pub data: [[f32; 4]; MAX_LIGHTS * 2],
    pub count: usize,
}

pub struct Gfx {
    pub gl: glow::Context,
    /// Level: lightmap only, and with 1..=MAX_LIGHTS dynamic lights.
    world: Program,
    world_dynamic: Vec<Program>,
    /// Models: opaque, and colour-keyed (alpha test).
    model: Program,
    model_cutoff: Program,
    particle: Program,
    shadow: Program,
    ui: Program,
    pub white: Texture,
    /// The lightmap of unlit level batches (flat full light).
    flat_light: Texture,
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
        const WORLD_UNIFORMS: &[&str] = &["u_mvp", "u_tex", "u_lightmap", "u_lights", "u_light_count"];
        const MODEL_UNIFORMS: &[&str] = &["u_mvp", "u_model", "u_tex", "u_color", "u_uv_offset", "u_cutoff", "u_unlit", "u_sun", "u_ambient", "u_lights", "u_light_count"];
        let world_attribs = &["a_pos", "a_uv", "a_uv2"];
        let model_attribs = &["a_pos", "a_normal", "a_uv"];
        let world = Program::new(&gl, WORLD_VS, WORLD_FS, "", world_attribs, WORLD_UNIFORMS)?;
        let world_dynamic = (1..=MAX_LIGHTS).map(|n| Program::new(&gl, WORLD_VS, WORLD_FS, &format!("#define DYNAMIC {n}\n"), world_attribs, WORLD_UNIFORMS)).collect::<anyhow::Result<_>>()?;
        let model = Program::new(&gl, MODEL_VS, MODEL_FS, "", model_attribs, MODEL_UNIFORMS)?;
        let model_cutoff = Program::new(&gl, MODEL_VS, MODEL_FS, "#define CUTOFF\n", model_attribs, MODEL_UNIFORMS)?;
        let ui = Program::new(&gl, UI_VS, UI_FS, "", &["a_pos", "a_uv"], &["u_mvp", "u_tex", "u_color"])?;
        let particle = Program::new(&gl, PARTICLE_VS, PARTICLE_FS, "", &["a_pos", "a_uv", "a_color"], &["u_mvp", "u_tex"])?;
        // The shadow mesh is a model mesh: normals and UVs are bound but unused.
        let shadow = Program::new(&gl, SHADOW_VS, SHADOW_FS, "", &["a_pos", "a_normal", "a_uv"], &["u_vp", "u_model", "u_sun", "u_color"])?;
        let white = upload(&gl, &formats::image::RgbaImage { width: 1, height: 1, data: vec![255; 4] }, false, true);
        let flat = (255.0 / GAMMA_MAX).round() as u8;
        let flat_light = upload(&gl, &formats::image::RgbaImage { width: 1, height: 1, data: vec![flat, flat, flat, 255] }, false, true);
        Ok(Self {
            gl,
            world,
            world_dynamic,
            model,
            model_cutoff,
            particle,
            shadow,
            ui,
            white,
            flat_light,
            sun: Vec3::from(SUN_DIRECTION).normalize() * SUN_STRENGTH,
            ambient: Vec3::splat(AMBIENT),
        })
    }
}
