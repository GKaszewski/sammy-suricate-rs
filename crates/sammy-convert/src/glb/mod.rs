//! Minimal glTF 2.0 binary (.glb) writer: meshes, embedded PNG textures,
//! node hierarchy and morph targets (used for MDL vertex animation).
//!
//! Callers supply data already converted to glTF's Y-up, right-handed space.

use serde_json::{json, Value};

use formats::image::RgbaImage;

mod buffers;
mod build;
mod write;

#[derive(Default)]
pub struct Glb {
    bin: Vec<u8>,
    buffer_views: Vec<Value>,
    accessors: Vec<Value>,
    images: Vec<Value>,
    textures: Vec<Value>,
    materials: Vec<Value>,
    meshes: Vec<Value>,
    nodes: Vec<Value>,
    root_nodes: Vec<usize>,
    animations: Vec<Value>,
}

pub struct Primitive<'a> {
    pub positions: &'a [[f32; 3]],
    pub normals: Option<&'a [[f32; 3]]>,
    pub uv0: Option<&'a [[f32; 2]]>,
    /// Second UV set, e.g. lightmap coordinates.
    pub uv1: Option<&'a [[f32; 2]]>,
    pub indices: &'a [u32],
    pub material: Option<usize>,
    /// Morph targets as position *deltas* relative to `positions`.
    pub morph_targets: &'a [Vec<[f32; 3]>],
}

#[derive(Default)]
pub struct Node {
    pub name: String,
    pub mesh: Option<usize>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    pub children: Vec<usize>,
    pub extras: Option<Value>,
}

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;

/// Acknex (X fwd, Y left, Z up) -> glTF (X left, Y up, Z fwd): a cyclic axis
/// permutation, so handedness is preserved and model fronts face glTF +Z.
pub fn to_gltf([x, y, z]: [f32; 3]) -> [f32; 3] {
    [y, z, x]
}
