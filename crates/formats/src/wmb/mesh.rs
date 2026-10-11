//! Render geometry: per-texture triangle batches with a packed lightmap atlas.

use alloc::vec::Vec;

use super::atlas::{Atlas, blit_padded};
use super::*;
use crate::image::RgbaImage;

impl Wmb {
    /// Builds render geometry for every BSP model (model 0 = world) plus a
    /// lightmap atlas shared by all lit batches.
    pub fn build_mesh(&self) -> LevelMesh {
        let rects: Vec<Option<LightmapRect>> = (0..self.faces.len())
            .map(|i| if self.face_visible(i) { self.face_lightmap_rect(i) } else { None })
            .collect();
        let atlas = Atlas::pack(&rects.iter().map(|r| r.map(|r| (r.width, r.height))).collect::<Vec<_>>());
        let lightmap = atlas.as_ref().map(|a| {
            let mut img = RgbaImage::new(a.width, a.height);
            for (rect, slot) in rects.iter().zip(&a.slots) {
                if let (Some(rect), Some(slot)) = (rect, slot) {
                    blit_padded(&mut img, &self.face_lightmap(rect), slot[0], slot[1]);
                }
            }
            img
        });

        let models = self
            .models
            .iter()
            .map(|m| {
                let mut batches: Vec<Batch> = Vec::new();
                let start = m.first_face as usize;
                let end = (start + m.num_faces as usize).min(self.faces.len());
                for (fi, rect) in rects.iter().enumerate().take(end).skip(start) {
                    if !self.face_visible(fi) || self.faces[fi].num_edges < 3 {
                        continue;
                    }
                    let tex = self.face_texture(fi);
                    let ti = &self.texinfos[self.faces[fi].texinfo as usize];
                    let lm = rect.zip(atlas.as_ref().and_then(|a| a.slots[fi]));
                    let lit = lm.is_some();
                    let b = match batches.iter_mut().position(|b| b.texture == tex && b.lit == lit) {
                        Some(i) => &mut batches[i],
                        None => {
                            batches.push(Batch { texture: tex, lit, ..Default::default() });
                            batches.last_mut().unwrap()
                        }
                    };
                    let normal = self.face_normal(fi);
                    let (tw, th) = tex.map_or((64.0, 64.0), |t| (self.textures[t].width as f32, self.textures[t].height as f32));
                    let base = b.positions.len() as u32;
                    for p in self.face_vertices(fi) {
                        let st = ti.st(p);
                        b.positions.push(p);
                        b.normals.push(normal);
                        b.uvs.push([st[0] / tw, st[1] / th]);
                        if let (Some((rect, slot)), Some(a)) = (lm, atlas.as_ref()) {
                            // Luxel i is centred at s = (origin + i) * 16.
                            let u = st[0] / LUXEL_SIZE - rect.origin[0] as f32 + slot[0] as f32 + 0.5;
                            let v = st[1] / LUXEL_SIZE - rect.origin[1] as f32 + slot[1] as f32 + 0.5;
                            b.lightmap_uvs.push([u / a.width as f32, v / a.height as f32]);
                        }
                    }
                    // Stored winding is clockwise seen from the front; emit CCW.
                    let n = self.faces[fi].num_edges as u32;
                    for k in 1..n - 1 {
                        b.indices.extend([base, base + k + 1, base + k]);
                    }
                    b.faces.push(fi as u32);
                }
                ModelMesh { batches }
            })
            .collect();
        LevelMesh { lightmap, models }
    }
}

pub struct LevelMesh {
    /// Packed lightmap atlas (RGBA8); `None` if the file has no lightmaps.
    pub lightmap: Option<RgbaImage>,
    /// One entry per BSP model; index 0 is the world.
    pub models: Vec<ModelMesh>,
}

pub struct ModelMesh {
    pub batches: Vec<Batch>,
}

/// Triangles sharing one texture and lighting mode.
#[derive(Default)]
pub struct Batch {
    /// Index into [`Wmb::textures`].
    pub texture: Option<usize>,
    /// True if every face has a lightmap (`lightmap_uvs` is filled).
    pub lit: bool,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Texture UVs normalised by texture size (repeating).
    pub uvs: Vec<[f32; 2]>,
    /// UVs into [`LevelMesh::lightmap`]; empty when `!lit`.
    pub lightmap_uvs: Vec<[f32; 2]>,
    /// Counter-clockwise triangles (front face, native coordinates).
    pub indices: Vec<u32>,
    /// Source face indices.
    pub faces: Vec<u32>,
}
