//! Custom world material reproducing Acknex A4 surface shading (texture x lightmap).

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Acknex lightmaps are 2x overbright: mid-grey means full texture brightness.
pub const LIGHTMAP_SCALE: f32 = 2.0;
/// Brightness of flat (non-lightmapped) surfaces.
pub const FLAT_LIGHT: f32 = 1.0;

pub struct LevelMaterialPlugin;

impl Plugin for LevelMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "level.wesl");
        app.add_plugins(MaterialPlugin::<LevelMaterial>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LevelMaterial {
    /// x = lightmap scale, y = flat brightness.
    #[uniform(0)]
    pub params: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub base: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub lightmap: Option<Handle<Image>>,
}

impl LevelMaterial {
    pub fn new(base: Option<Handle<Image>>, lightmap: Option<Handle<Image>>) -> Self {
        Self { params: Vec4::new(LIGHTMAP_SCALE, FLAT_LIGHT, 0.0, 0.0), base, lightmap }
    }
}

impl Material for LevelMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://sammy/level.wesl".into()
    }
}
