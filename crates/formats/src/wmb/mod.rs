//! WMB4 level / map-entity files (Acknex A4 BSP, a Quake BSP29 derivative).
//!
//! See `docs/formats/wmb4.md` for the byte-level layout. All data is kept in the
//! engine's native coordinate system (X forward, Y left, Z up, quants).
//!
//! [`parse`] decodes every lump into plain structs; [`Wmb::build_mesh`] turns the
//! faces into renderable per-texture triangle batches with a packed lightmap atlas.

mod atlas;
mod entity;
mod mesh;
mod model;
mod parse;
mod query;

pub use entity::{Entity, Object, entity_flag_names, entity_flags};
pub use mesh::{Batch, LevelMesh, ModelMesh};
pub use model::*;
pub use parse::{parse, parse_with_palette};
pub use query::LightmapRect;

/// Lump indices in the 16-entry directory that follows the magic.
pub mod lump {
    pub const PALETTE: usize = 0;
    pub const PLANES: usize = 1;
    pub const TEXTURES: usize = 2;
    pub const VERTICES: usize = 3;
    pub const VIS: usize = 4;
    pub const NODES: usize = 5;
    pub const TEXINFO: usize = 6;
    pub const FACES: usize = 7;
    pub const LIGHTMAPS: usize = 8;
    pub const CLIPNODES: usize = 9;
    pub const LEAVES: usize = 10;
    pub const MARKSURFACES: usize = 11;
    pub const EDGES: usize = 12;
    pub const SURFEDGES: usize = 13;
    pub const MODELS: usize = 14;
    pub const ENTITIES: usize = 15;
    pub const COUNT: usize = 16;
}

/// Texinfo flag bits (meaning inferred from usage statistics; see docs).
pub mod surface_flags {
    /// Face has no lightmap ("flat" surface, lit by sun/ambient only).
    pub const FLAT: u32 = 1;
    /// Unknown; set on most flat surfaces of prefabs and decorative brushes.
    pub const UNKNOWN_2: u32 = 2;
    /// Not rendered (dummy/clip brushes, outer level hull, kill zones).
    pub const INVISIBLE: u32 = 8;
    /// Unknown; appears both on lit and flat surfaces.
    pub const UNKNOWN_16: u32 = 16;
    /// Unknown; seen once (`STONE005_128`, flags 131).
    pub const UNKNOWN_128: u32 = 128;
}

/// Leaf/clipnode contents values.
pub mod contents {
    pub const SOLID: i32 = -128;
    pub const EMPTY: i32 = -256;
}

/// Luxel spacing in texture space (same as Quake).
pub const LUXEL_SIZE: f32 = 16.0;
