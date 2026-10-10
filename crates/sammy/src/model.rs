//! MDL models as Bevy meshes with CPU keyframe blending (Acknex interpolates
//! vertex frames linearly; glTF morph targets would exceed Bevy's target limit).

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use formats::mdl::{Mdl, VertexBuffer};

use crate::coords;

pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_models);
    }
}

pub struct LoadedModel {
    pub mdl: Mdl,
    pub vb: VertexBuffer,
    pub material: Handle<StandardMaterial>,
    /// Per-frame vertex positions/normals in Bevy space, precomputed.
    frames: Vec<(Vec<[f32; 3]>, Vec<[f32; 3]>)>,
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
        let skin = mdl.skins.first().map(|s| images.add(crate::image::to_bevy(&s.image, true, false)));
        let alpha = mdl.skins.first().is_some_and(|s| s.image.data.chunks_exact(4).any(|p| p[3] < 255));
        let material = materials.add(StandardMaterial {
            base_color_texture: skin,
            perceptual_roughness: 1.0,
            alpha_mode: if alpha { AlphaMode::Mask(0.5) } else { AlphaMode::Opaque },
            ..default()
        });
        Ok(Self { mdl, vb, material, frames })
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
        let name = name.to_lowercase();
        self.mdl
            .animations()
            .into_iter()
            .find(|a| a.name.to_lowercase() == name)
            .map(|a| a.frames)
            .unwrap_or_default()
    }
}

/// Plays an MDL frame sequence by blending vertex frames on the CPU.
#[derive(Component)]
pub struct ModelAnimator {
    pub model: Arc<LoadedModel>,
    pub frames: Vec<usize>,
    /// Frames per second (Acknex default: one frame per 1/16 s tick).
    pub fps: f32,
    pub time: f32,
}

fn animate_models(time: Res<Time>, mut meshes: ResMut<Assets<Mesh>>, mut q: Query<(&mut ModelAnimator, &Mesh3d)>) {
    for (mut anim, mesh) in &mut q {
        if anim.frames.len() < 2 {
            continue;
        }
        anim.time += time.delta_secs() * anim.fps;
        let n = anim.frames.len() as f32;
        let t = anim.time.rem_euclid(n);
        let (a, b) = (anim.frames[t as usize], anim.frames[(t as usize + 1) % anim.frames.len()]);
        let f = t.fract();
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
