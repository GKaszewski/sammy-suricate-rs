//! MDL models as Bevy meshes with CPU keyframe blending (Acknex interpolates
//! vertex frames linearly; glTF morph targets would exceed Bevy's target limit).

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use formats::mdl::{Mdl, VertexBuffer};
use sammy_core::anim::{Anim, Scenes};

use crate::coords;

pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_models.in_set(crate::schedule::FrameSet::Animate));
    }
}

pub struct LoadedModel {
    pub mdl: Mdl,
    pub vb: VertexBuffer,
    /// Skin 1 (the default) material.
    pub material: Handle<StandardMaterial>,
    /// One material per skin (A4 `my.skin` is 1-based).
    pub skins: Vec<Handle<StandardMaterial>>,
    /// Per-frame vertex positions/normals in Bevy space, precomputed.
    frames: Vec<(Vec<[f32; 3]>, Vec<[f32; 3]>)>,
    /// Scene table for animation clocks.
    pub scenes: Arc<Scenes>,
}

impl LoadedModel {
    pub fn load(data: &[u8], images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>) -> anyhow::Result<Self> {
        let mdl = formats::mdl::parse(data)?;
        let vb = mdl.vertex_buffer();
        let frames = mdl
            .frames
            .iter()
            .map(|f| {
                let p = vb.positions(f).into_iter().map(|v| coords::pos(v).to_array()).collect();
                let n = vb.normals(f).into_iter().map(|v| coords::dir(v).to_array()).collect();
                (p, n)
            })
            .collect();
        let mut skins: Vec<Handle<StandardMaterial>> = mdl
            .skins
            .iter()
            .map(|s| {
                let alpha = s.image.data.chunks_exact(4).any(|p| p[3] < 255);
                materials.add(StandardMaterial {
                    base_color_texture: Some(images.add(crate::image::to_bevy(&s.image, true, false))),
                    perceptual_roughness: 1.0,
                    alpha_mode: if alpha { AlphaMode::Mask(0.5) } else { AlphaMode::Opaque },
                    ..default()
                })
            })
            .collect();
        if skins.is_empty() {
            skins.push(materials.add(StandardMaterial::default()));
        }
        let scenes = Arc::new(Scenes::of(&mdl));
        Ok(Self { mdl, vb, material: skins[0].clone(), skins, frames, scenes })
    }

    /// Material for A4 skin number `n` (1-based; 0 means the default skin 1).
    /// Skin `n` with A4 `overlay`: palette index 0 (or black for true-colour skins) is transparent.
    pub fn overlay_skin(&self, n: u32, images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        let Some(skin) = self.mdl.skins.get((n.max(1) as usize - 1).min(self.mdl.skins.len().saturating_sub(1))) else {
            return self.skin(n);
        };
        let mut img = skin.image.clone();
        match &skin.indices {
            Some(idx) => img.data.chunks_exact_mut(4).zip(idx).filter(|(_, i)| **i == 0).for_each(|(px, _)| px[3] = 0),
            None => img.data.chunks_exact_mut(4).filter(|px| crate::image::is_key(px)).for_each(|px| px[3] = 0),
        }
        materials.add(StandardMaterial {
            base_color_texture: Some(images.add(crate::image::to_bevy(&img, true, false))),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    }

    pub fn skin(&self, n: u32) -> Handle<StandardMaterial> {
        self.skins[(n.max(1) as usize - 1).min(self.skins.len() - 1)].clone()
    }

    pub fn build_mesh(&self, frame: usize) -> Mesh {
        let (p, n) = &self.frames[frame.min(self.frames.len() - 1)];
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, n.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.vb.uvs.clone())
            .with_inserted_indices(Indices::U32(self.vb.indices.clone()))
    }

    pub fn animation(&self, name: &str) -> Vec<usize> {
        self.scenes.frames(name)
    }
}

/// Plays an MDL frame sequence by blending vertex frames on the CPU; the clock is
/// [`Anim`] (dereferenced, so `play`, `finished` ... work directly).
#[derive(Component, Deref, DerefMut)]
pub struct ModelAnimator {
    pub model: Arc<LoadedModel>,
    #[deref]
    pub anim: Anim,
}

impl ModelAnimator {
    pub fn new(model: Arc<LoadedModel>, frames: Vec<usize>, fps: f32) -> Self {
        let anim = Anim::new(model.scenes.clone(), frames, fps);
        Self { model, anim }
    }
}

pub fn animate_models(time: Res<Time>, mut meshes: ResMut<Assets<Mesh>>, mut q: Query<(&mut ModelAnimator, &Mesh3d)>) {
    for (mut anim, mesh) in &mut q {
        let Some((a, b, f)) = anim.anim.advance(time.delta_secs()) else { continue };
        let Some(mut mesh) = meshes.get_mut(&mesh.0) else { continue };
        let (pa, na) = &anim.model.frames[a];
        let (pb, nb) = &anim.model.frames[b];
        let lerp = |x: &[[f32; 3]], y: &[[f32; 3]]| -> Vec<[f32; 3]> {
            x.iter().zip(y).map(|(x, y)| Vec3::from(*x).lerp(Vec3::from(*y), f).to_array()).collect()
        };
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(lerp(pa, pb)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float32x3(lerp(na, nb)));
    }
}
