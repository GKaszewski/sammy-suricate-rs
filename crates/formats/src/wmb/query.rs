//! Questions about the decoded level: face geometry, lightmaps, visibility and
//! point contents.

use alloc::vec::Vec;

use super::*;
use crate::image::RgbaImage;

impl Wmb {
    /// Vertex positions of a face in winding order (as stored: clockwise when
    /// viewed from the front, like Quake).
    pub fn face_vertices(&self, face: usize) -> impl Iterator<Item = [f32; 3]> + '_ {
        let f = &self.faces[face];
        let se = &self.surfedges[f.first_edge as usize..][..f.num_edges as usize];
        se.iter().map(|&e| {
            let edge = self.edges[e.unsigned_abs() as usize];
            self.vertices[if e >= 0 { edge[0] } else { edge[1] } as usize]
        })
    }

    pub fn face_normal(&self, face: usize) -> [f32; 3] {
        let f = &self.faces[face];
        let n = self.planes[f.plane as usize].normal;
        if f.side != 0 { [-n[0], -n[1], -n[2]] } else { n }
    }

    /// Texture of a face (None if the texinfo references a missing texture).
    pub fn face_texture(&self, face: usize) -> Option<usize> {
        let ti = &self.texinfos[self.faces[face].texinfo as usize];
        usize::try_from(ti.texture).ok().filter(|&t| t < self.textures.len())
    }

    /// Whether the face is drawn (texinfo not flagged [`surface_flags::INVISIBLE`]).
    pub fn face_visible(&self, face: usize) -> bool {
        self.texinfos[self.faces[face].texinfo as usize].flags & surface_flags::INVISIBLE == 0
    }

    /// Lightmap rectangle of a face: luxel origin `(floor(min s/16), floor(min t/16))`
    /// and size in luxels. `None` if the face has no lightmap.
    ///
    /// Must be computed in f32 exactly like this to reproduce the compiler's
    /// rounding (verified against every face of every shipped level).
    pub fn face_lightmap_rect(&self, face: usize) -> Option<LightmapRect> {
        let f = &self.faces[face];
        if f.light_offset < 0 || f.styles[0] == 255 {
            return None;
        }
        let ti = &self.texinfos[f.texinfo as usize];
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for p in self.face_vertices(face) {
            let st = ti.st(p);
            for k in 0..2 {
                min[k] = min[k].min(st[k]);
                max[k] = max[k].max(st[k]);
            }
        }
        let lo = [libm::floorf(min[0] / LUXEL_SIZE), libm::floorf(min[1] / LUXEL_SIZE)];
        let hi = [libm::ceilf(max[0] / LUXEL_SIZE), libm::ceilf(max[1] / LUXEL_SIZE)];
        let w = (hi[0] - lo[0]) as u32 + 1;
        let h = (hi[1] - lo[1]) as u32 + 1;
        let offset = f.light_offset as usize;
        // Entries are padded to 4 bytes except possibly the last.
        if offset + (w * h * 2) as usize > self.lightmaps.len() || w > 1024 || h > 1024 {
            return None;
        }
        Some(LightmapRect { offset, origin: [lo[0] as i32, lo[1] as i32], width: w, height: h })
    }

    /// Decodes one face lightmap to RGBA.
    pub fn face_lightmap(&self, rect: &LightmapRect) -> RgbaImage {
        let n = (rect.width * rect.height) as usize;
        let raw = &self.lightmaps[rect.offset..rect.offset + n * 2];
        RgbaImage::from_rgb565(rect.width, rect.height, raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])))
    }

    /// Decompresses the PVS row of a leaf (Quake RLE: a 0 byte is followed by a
    /// count of zero bytes). Bit `i` set = leaf `i + 1` visible.
    pub fn decompress_vis(&self, leaf: usize) -> Option<Vec<u8>> {
        let leaf = self.leaves.get(leaf)?;
        let visleafs = self.models.first().map_or(0, |m| m.vis_leafs.max(0) as usize);
        let row = visleafs.div_ceil(8);
        let mut src = self.vis.get(usize::try_from(leaf.vis_offset).ok()?..)?.iter();
        let mut out = Vec::with_capacity(row);
        while out.len() < row {
            match *src.next()? {
                0 => {
                    let n = *src.next()? as usize;
                    out.extend(core::iter::repeat_n(0, n.min(row - out.len())));
                }
                b => out.push(b),
            }
        }
        Some(out)
    }

    /// Point-in-hull test: returns the contents at `p` for collision hull
    /// `hull` (0 = BSP nodes/leaves, 1..3 = clipnodes) of `model`.
    pub fn point_contents(&self, model: usize, hull: usize, p: [f32; 3]) -> i32 {
        let Some(m) = self.models.get(model) else { return contents::EMPTY };
        let mut n = m.head_nodes[hull.min(3)];
        let side = |plane: i32| -> usize {
            let pl = &self.planes[plane as usize];
            let d = pl.normal[0] * p[0] + pl.normal[1] * p[1] + pl.normal[2] * p[2] - pl.dist;
            usize::from(d < 0.0)
        };
        if hull == 0 {
            while n >= 0 {
                let node = &self.nodes[n as usize];
                n = node.children[side(node.plane)];
            }
            self.leaves.get((-n - 1) as usize).map_or(contents::SOLID, |l| l.contents)
        } else {
            while n >= 0 {
                let c = &self.clipnodes[n as usize];
                n = c.children[side(c.plane)];
            }
            n
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LightmapRect {
    /// Byte offset into [`Wmb::lightmaps`].
    pub offset: usize,
    /// Luxel-grid origin, i.e. `floor(min_st / 16)`.
    pub origin: [i32; 2],
    pub width: u32,
    pub height: u32,
}
