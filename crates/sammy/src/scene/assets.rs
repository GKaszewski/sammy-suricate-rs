//! Render assets loaded from the game files on demand and kept across levels: MDL
//! models, WMB prefabs and PCX sprites.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use formats::wmb::{LevelMesh, Wmb};

use crate::platform::vfs::GameFiles;
use crate::render::coords;
use crate::render::model::LoadedModel;

/// Sprite alpha-mask cut-off.
const SPRITE_ALPHA_CUTOFF: f32 = 0.5;

/// A sprite quad (one frame of its strip), its material and frame count.
#[derive(Clone)]
pub struct SpriteAsset {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub frames: u32,
}

/// A prefab's parts (one mesh per texture).
pub type PrefabParts = Vec<(Handle<Mesh>, Handle<StandardMaterial>)>;

/// Everything loaded so far, by lower-case file name (failures remembered too).
#[derive(Resource, Default)]
pub struct SceneAssets {
    models: HashMap<String, Option<Arc<LoadedModel>>>,
    prefabs: HashMap<String, Option<PrefabParts>>,
    sprites: HashMap<String, Option<SpriteAsset>>,
}

/// Loads render assets on demand.
#[derive(SystemParam)]
pub struct Loader<'w> {
    pub assets: ResMut<'w, SceneAssets>,
    pub files: Res<'w, GameFiles>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl Loader<'_> {
    pub fn model(&mut self, file: &str) -> Option<Arc<LoadedModel>> {
        let file = file.to_lowercase();
        if let Some(m) = self.assets.models.get(&file) {
            return m.clone();
        }
        let m = self.files.read(&file).ok().and_then(|d| LoadedModel::load(&d, &mut self.images, &mut self.materials).map(Arc::new).ok());
        if m.is_none() {
            warn!("missing model {file}");
        }
        self.assets.models.insert(file, m.clone());
        m
    }

    /// Prefab WMBs (doors, platforms...) have no lightmaps; Acknex lights them
    /// dynamically like models, so they use a lit StandardMaterial.
    pub fn prefab(&mut self, file: &str) -> Option<PrefabParts> {
        let file = file.to_lowercase();
        if let Some(p) = self.assets.prefabs.get(&file) {
            return p.clone();
        }
        let p = self.files.read(&file).ok().and_then(|d| formats::wmb::parse(&d).ok()).map(|w| {
            let level = w.build_mesh();
            let mut mats: HashMap<Option<AssetId<Image>>, Handle<StandardMaterial>> = HashMap::new();
            batch_meshes(&w, &level, false, &[], &mut self.meshes, &mut self.images)
                .into_iter()
                .map(|(mesh, tex, _)| {
                    let material = mats
                        .entry(tex.as_ref().map(|t| t.id()))
                        .or_insert_with(|| self.materials.add(StandardMaterial { base_color_texture: tex, perceptual_roughness: 1.0, reflectance: 0.0, ..default() }))
                        .clone();
                    (mesh, material)
                })
                .collect()
        });
        if p.is_none() {
            warn!("missing prefab {file}");
        }
        self.assets.prefabs.insert(file, p.clone());
        p
    }

    /// Sprites are upright quads sized in quants = pixels; `name+N.pcx` is an N-frame
    /// horizontal strip shown one frame at a time.
    pub fn sprite(&mut self, file: &str) -> Option<SpriteAsset> {
        let file = file.to_lowercase();
        if let Some(s) = self.assets.sprites.get(&file) {
            return s.clone();
        }
        let s = self.load_sprite(&file);
        if s.is_none() {
            warn!("missing sprite {file}");
        }
        self.assets.sprites.insert(file, s.clone());
        s
    }

    fn load_sprite(&mut self, file: &str) -> Option<SpriteAsset> {
        let pcx = formats::pcx::parse(&self.files.read(file).ok()?).ok()?;
        let frames = sammy_core::sim::assets::sprite_frames(file);
        let mut img = pcx.image.clone();
        // Acknex sprites are colour-keyed: palette index 0 (8-bit) or pure black (24-bit).
        if let Some(idx) = &pcx.indices {
            for (px, &i) in img.data.as_chunks_mut::<4>().0.iter_mut().zip(idx) {
                if i == 0 {
                    px[3] = 0;
                }
            }
        } else {
            for px in img.data.as_chunks_mut::<4>().0.iter_mut() {
                if crate::render::image::is_key(px) {
                    px[3] = 0;
                }
            }
        }
        let w = img.width as f32 / frames as f32 / coords::QUANTS_PER_UNIT;
        let h = img.height as f32 / coords::QUANTS_PER_UNIT;
        let u1 = 1.0 / frames as f32;
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[-w / 2.0, -h / 2.0, 0.0], [w / 2.0, -h / 2.0, 0.0], [w / 2.0, h / 2.0, 0.0], [-w / 2.0, h / 2.0, 0.0]],
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], [u1, 1.0], [u1, 0.0], [0.0, 0.0]])
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let material = self.materials.add(StandardMaterial {
            base_color_texture: Some(self.images.add(crate::render::image::to_bevy(&img, true, false))),
            alpha_mode: AlphaMode::Mask(SPRITE_ALPHA_CUTOFF),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        Some(SpriteAsset { mesh: self.meshes.add(mesh), material, frames })
    }

    /// An unlit, double-sided copy of a material (sky domes, overlays).
    pub fn unlit(&mut self, base: &Handle<StandardMaterial>) -> Handle<StandardMaterial> {
        let mut m = self.materials.get(base).cloned().unwrap_or_default();
        m.unlit = true;
        m.cull_mode = None;
        self.materials.add(m)
    }
}

/// One Bevy mesh per texture batch: (mesh, texture, has lightmap UVs). Batches whose
/// texture is in `skip` are left out.
pub fn batch_meshes(
    wmb: &Wmb,
    level: &LevelMesh,
    use_lightmap: bool,
    skip: &[&str],
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
) -> Vec<(Handle<Mesh>, Option<Handle<Image>>, bool)> {
    let textures: Vec<Handle<Image>> = wmb.textures.iter().map(|t| images.add(crate::render::image::to_bevy(&t.image, true, true))).collect();
    let mut out = Vec::new();
    for batch in level.models.iter().flat_map(|m| &m.batches) {
        let tex_name = batch.texture.map_or("", |t| wmb.textures[t].name.as_str());
        if skip.iter().any(|s| tex_name.eq_ignore_ascii_case(s)) || (std::env::var_os("SAMMY_NODUMMY").is_some() && tex_name.to_lowercase().contains("dummy")) {
            continue;
        }
        let lit = batch.lit && use_lightmap;
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, batch.positions.iter().map(|&p| coords::pos(p)).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, batch.normals.iter().map(|&n| coords::dir(n)).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, batch.uvs.clone())
            .with_inserted_indices(Indices::U32(batch.indices.clone()));
        if lit {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, batch.lightmap_uvs.clone());
        }
        out.push((meshes.add(mesh), batch.texture.map(|t| textures[t].clone()), lit));
    }
    out
}
