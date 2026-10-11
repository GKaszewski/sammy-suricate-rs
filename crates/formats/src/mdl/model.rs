//! A decoded model: skins, triangles, frames and its vertex buffers.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use crate::image::RgbaImage;

use alloc::string::ToString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// A4 (`MDL3`): skin size in header, 8-bit frame vertices.
    Mdl3,
    /// A5 (`MDL5`): per-skin size, frames may use 16-bit vertices.
    Mdl5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinFormat {
    /// Type 0: 8-bit palette indices. No palette is stored in the file; `Skin::image`
    /// is a grayscale stand-in and the raw indices are kept in `Skin::indices`.
    Indexed8,
    /// Type 2: 16-bit RGB565.
    Rgb565,
    /// Type 3: 16-bit ARGB4444 (only seen in MDL5 files).
    Argb4444,
}

#[derive(Clone, Debug)]
pub struct Skin {
    pub format: SkinFormat,
    pub image: RgbaImage,
    /// Raw palette indices for `SkinFormat::Indexed8`.
    pub indices: Option<Vec<u8>>,
}

/// One triangle exactly as stored (clockwise front face, see module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle {
    /// Indices into each frame's `positions`/`normals`.
    pub xyz: [u16; 3],
    /// Indices into `Mdl::skin_verts` / `Mdl::uvs`.
    pub uv: [u16; 3],
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub name: String,
    /// Decoded bounding box (scale * packed + translate).
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub positions: Vec<[f32; 3]>,
    /// Outward unit normals: `-NORMALS[normal_indices[i]]`.
    pub normals: Vec<[f32; 3]>,
    pub normal_indices: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Mdl {
    pub version: Version,
    pub scale: [f32; 3],
    pub translate: [f32; 3],
    pub bounding_radius: f32,
    pub eye_position: [f32; 3],
    pub flags: u32,
    /// Skin size used to normalise UVs (header value for MDL3, first skin for MDL5).
    pub skin_size: [u32; 2],
    pub skins: Vec<Skin>,
    /// Texture coordinates in skin pixels, as stored.
    pub skin_verts: Vec<[i16; 2]>,
    /// `skin_verts` divided by `skin_size` (0..1, origin top-left, as glTF).
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<Triangle>,
    pub frames: Vec<Frame>,
}

/// Frames sharing a name prefix (trailing digits/whitespace stripped), e.g.
/// `walk01`..`walk32` -> `walk`. This is what `set_cycle`/`set_frame` address.
#[derive(Clone, Debug)]
pub struct Animation {
    pub name: String,
    /// Frame indices in file order.
    pub frames: Vec<usize>,
}

/// Render-ready vertex layout: one vertex per unique (xyz index, uv index) pair.
#[derive(Clone, Debug)]
pub struct VertexBuffer {
    /// For each output vertex, the index into a frame's `positions`/`normals`.
    pub xyz_index: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    /// Triangle list into the output vertices (CCW front faces).
    pub indices: Vec<u32>,
}

impl VertexBuffer {
    pub fn positions(&self, frame: &Frame) -> Vec<[f32; 3]> {
        self.xyz_index.iter().map(|&i| frame.positions[i as usize]).collect()
    }
    pub fn normals(&self, frame: &Frame) -> Vec<[f32; 3]> {
        self.xyz_index.iter().map(|&i| frame.normals[i as usize]).collect()
    }
}

/// Strips trailing digits and whitespace: `"walk01"` -> `"walk"`, `"Frame 1"` -> `"Frame"`.
pub fn frame_group(name: &str) -> &str {
    name.trim_end_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
}

impl Mdl {
    /// Groups frames by `frame_group(name)` in order of first appearance. Grouping is
    /// case-sensitive on the stored name; Acknex itself matches names case-insensitively.
    pub fn animations(&self) -> Vec<Animation> {
        let mut out: Vec<Animation> = Vec::new();
        let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
        for (i, f) in self.frames.iter().enumerate() {
            let key = frame_group(&f.name);
            let slot = *by_name.entry(key).or_insert_with(|| {
                out.push(Animation { name: key.to_string(), frames: Vec::new() });
                out.len() - 1
            });
            out[slot].frames.push(i);
        }
        out
    }

    pub fn vertex_buffer(&self) -> VertexBuffer {
        let mut map: BTreeMap<(u16, u16), u32> = BTreeMap::new();
        let mut vb = VertexBuffer { xyz_index: Vec::new(), uvs: Vec::new(), indices: Vec::new() };
        for t in &self.triangles {
            // Reverse the stored (clockwise) order to get CCW front faces.
            for k in [0, 2, 1] {
                let key = (t.xyz[k], t.uv[k]);
                let idx = *map.entry(key).or_insert_with(|| {
                    vb.xyz_index.push(key.0 as u32);
                    vb.uvs.push(self.uvs[key.1 as usize]);
                    vb.xyz_index.len() as u32 - 1
                });
                vb.indices.push(idx);
            }
        }
        vb
    }
}
