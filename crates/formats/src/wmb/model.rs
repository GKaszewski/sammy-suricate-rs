//! The decoded lumps: the level, its BSP and its surfaces.

use alloc::{string::String, vec::Vec};

use super::Entity;
use crate::image::RgbaImage;

#[derive(Clone, Debug)]
pub struct Wmb {
    /// Lump 0: per-level 8-bit rendering tables (starts with an RGB palette,
    /// followed by colour lookup tables; 181520 bytes when present, else empty).
    /// Not used for texture decoding; kept raw.
    pub palette_lump: Vec<u8>,
    pub planes: Vec<Plane>,
    pub textures: Vec<Texture>,
    pub vertices: Vec<[f32; 3]>,
    /// Raw PVS data (Quake run-length encoding); see [`Wmb::decompress_vis`].
    pub vis: Vec<u8>,
    pub nodes: Vec<Node>,
    pub texinfos: Vec<TexInfo>,
    pub faces: Vec<Face>,
    /// Raw RGB565 lightmap data, indexed by [`Face::light_offset`] (bytes).
    pub lightmaps: Vec<u8>,
    pub clipnodes: Vec<ClipNode>,
    pub leaves: Vec<Leaf>,
    pub mark_surfaces: Vec<u32>,
    pub edges: Vec<[u32; 2]>,
    pub surfedges: Vec<i32>,
    pub models: Vec<Model>,
    pub entities: Vec<Entity>,
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: [f32; 3],
    pub dist: f32,
    /// Quake plane type: 0/1/2 axial X/Y/Z, 3..5 non-axial.
    pub kind: i32,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// 16 (RGB565) or 8 (palettized, Quake palette).
    pub bits: u8,
    /// Mip level 0 decoded to RGBA. 8-bit textures use the palette given to
    /// [`parse_with_palette`], or a greyscale ramp if none.
    pub image: RgbaImage,
    /// Raw mip-0 palette indices for 8-bit textures (for re-colouring).
    pub indices: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug)]
pub struct TexInfo {
    /// `s = dot(pos, s[0..3]) + s[3]` in texels.
    pub s: [f32; 4],
    pub t: [f32; 4],
    pub texture: i32,
    /// See [`surface_flags`].
    pub flags: u32,
    /// Surface ambient (signed percent, -100..100).
    pub ambient: i8,
    /// Surface albedo (percent, default 50).
    pub albedo: u8,
}

impl TexInfo {
    pub fn st(&self, p: [f32; 3]) -> [f32; 2] {
        let s = p[0] * self.s[0] + p[1] * self.s[1] + p[2] * self.s[2] + self.s[3];
        let t = p[0] * self.t[0] + p[1] * self.t[1] + p[2] * self.t[2] + self.t[3];
        [s, t]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// Unknown leading u16 (small values 0..~30; not the plane index as in Quake).
    pub unknown: u16,
    /// Non-zero: face normal is the negated plane normal.
    pub side: u16,
    pub first_edge: u32,
    pub num_edges: u16,
    pub texinfo: u16,
    /// Light styles; 0 = normal lightmap, 255 = none.
    pub styles: [u8; 4],
    /// Byte offset into [`Wmb::lightmaps`], or -1.
    pub light_offset: i32,
    /// Plane index (stored last, unlike Quake).
    pub plane: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub plane: i32,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    /// >= 0: node index; < 0: leaf index `-(c + 1)`.
    pub children: [i32; 2],
    pub first_face: u32,
    pub num_faces: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ClipNode {
    pub plane: i32,
    /// >= 0: clipnode index; < 0: contents (see [`contents`]).
    pub children: [i32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Leaf {
    /// See [`contents`]; -240 also occurs (empty with an extra bit 0x10).
    pub contents: i32,
    /// Offset into [`Wmb::vis`], or -1.
    pub vis_offset: i32,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    /// Unknown bytes 20..28 (often repeat maxs.z; Quake has ambient levels here).
    pub unknown: [u8; 8],
    pub first_mark_surface: u32,
    pub num_mark_surfaces: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub origin: [f32; 3],
    /// [0] BSP node root, [1..3] clipnode roots for hulls 1..3.
    pub head_nodes: [i32; 4],
    /// Collision hull boxes (hull 0 = point, 1 = "narrow" ±16, 2 = "fat").
    pub hull_mins: [[i32; 3]; 4],
    pub hull_maxs: [[i32; 3]; 4],
    pub vis_leafs: i32,
    pub first_face: u32,
    pub num_faces: u32,
}
