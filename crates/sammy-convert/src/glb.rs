//! Minimal glTF 2.0 binary (.glb) writer: meshes, embedded PNG textures,
//! node hierarchy and morph targets (used for MDL vertex animation).
//!
//! Callers supply data already converted to glTF's Y-up, right-handed space.

use serde_json::{json, Value};

use formats::image::RgbaImage;

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

impl Glb {
    fn push_view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut view = json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if let Some(t) = target {
            view["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.buffer_views.push(view);
        self.buffer_views.len() - 1
    }

    fn accessor_vec<const N: usize>(&mut self, data: &[[f32; N]], with_bounds: bool, target: Option<u32>) -> usize {
        let bytes: Vec<u8> = data.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.push_view(&bytes, target);
        let ty = match N {
            1 => "SCALAR",
            2 => "VEC2",
            3 => "VEC3",
            4 => "VEC4",
            _ => unreachable!(),
        };
        let mut acc = json!({ "bufferView": view, "componentType": FLOAT, "count": data.len(), "type": ty });
        if with_bounds && !data.is_empty() {
            let mut min = [f32::MAX; N];
            let mut max = [f32::MIN; N];
            for v in data {
                for i in 0..N {
                    min[i] = min[i].min(v[i]);
                    max[i] = max[i].max(v[i]);
                }
            }
            acc["min"] = json!(min.to_vec());
            acc["max"] = json!(max.to_vec());
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }

    /// Embeds an RGBA image as PNG and returns its texture index.
    pub fn add_texture(&mut self, name: &str, img: &RgbaImage, nearest: bool) -> usize {
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, img.width, img.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&img.data).unwrap();
        }
        let view = self.push_view(&png_bytes, None);
        self.images.push(json!({ "name": name, "bufferView": view, "mimeType": "image/png" }));
        let sampler_idx = if nearest { 0 } else { 1 };
        self.textures.push(json!({ "source": self.images.len() - 1, "sampler": sampler_idx }));
        self.textures.len() - 1
    }

    /// Adds an unlit-capable material. `lightmap` goes into the occlusion slot on UV1
    /// (best widely-supported approximation of a baked lightmap).
    pub fn add_material(&mut self, name: &str, base: Option<usize>, lightmap: Option<usize>, alpha_mask: bool) -> usize {
        let mut m = json!({
            "name": name,
            "pbrMetallicRoughness": { "metallicFactor": 0.0, "roughnessFactor": 1.0 },
        });
        if let Some(t) = base {
            m["pbrMetallicRoughness"]["baseColorTexture"] = json!({ "index": t });
        }
        if let Some(t) = lightmap {
            m["occlusionTexture"] = json!({ "index": t, "texCoord": 1 });
        }
        if alpha_mask {
            m["alphaMode"] = json!("MASK");
            m["doubleSided"] = json!(true);
        }
        self.materials.push(m);
        self.materials.len() - 1
    }

    pub fn add_mesh(&mut self, name: &str, prims: &[Primitive], target_names: &[String]) -> usize {
        let mut out = Vec::new();
        for p in prims {
            let mut attrs = serde_json::Map::new();
            attrs.insert("POSITION".into(), json!(self.accessor_vec(p.positions, true, Some(ARRAY_BUFFER))));
            if let Some(n) = p.normals {
                attrs.insert("NORMAL".into(), json!(self.accessor_vec(n, false, Some(ARRAY_BUFFER))));
            }
            if let Some(uv) = p.uv0 {
                attrs.insert("TEXCOORD_0".into(), json!(self.accessor_vec(uv, false, Some(ARRAY_BUFFER))));
            }
            if let Some(uv) = p.uv1 {
                attrs.insert("TEXCOORD_1".into(), json!(self.accessor_vec(uv, false, Some(ARRAY_BUFFER))));
            }
            let idx_bytes: Vec<u8> = p.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let view = self.push_view(&idx_bytes, Some(ELEMENT_ARRAY_BUFFER));
            self.accessors.push(json!({ "bufferView": view, "componentType": UNSIGNED_INT, "count": p.indices.len(), "type": "SCALAR" }));
            let mut prim = json!({ "attributes": attrs, "indices": self.accessors.len() - 1 });
            if let Some(m) = p.material {
                prim["material"] = json!(m);
            }
            if !p.morph_targets.is_empty() {
                let targets: Vec<Value> = p
                    .morph_targets
                    .iter()
                    .map(|t| json!({ "POSITION": self.accessor_vec(t, true, Some(ARRAY_BUFFER)) }))
                    .collect();
                prim["targets"] = json!(targets);
            }
            out.push(prim);
        }
        let mut mesh = json!({ "name": name, "primitives": out });
        if !target_names.is_empty() {
            mesh["weights"] = json!(vec![0.0f32; target_names.len()]);
            mesh["extras"] = json!({ "targetNames": target_names });
        }
        self.meshes.push(mesh);
        self.meshes.len() - 1
    }

    pub fn add_node(&mut self, node: Node, root: bool) -> usize {
        let mut n = json!({ "name": node.name });
        if let Some(m) = node.mesh {
            n["mesh"] = json!(m);
        }
        if let Some(t) = node.translation {
            n["translation"] = json!(t);
        }
        if let Some(r) = node.rotation {
            n["rotation"] = json!(r);
        }
        if let Some(s) = node.scale {
            n["scale"] = json!(s);
        }
        if !node.children.is_empty() {
            n["children"] = json!(node.children);
        }
        if let Some(e) = node.extras {
            n["extras"] = e;
        }
        self.nodes.push(n);
        let i = self.nodes.len() - 1;
        if root {
            self.root_nodes.push(i);
        }
        i
    }

    /// Adds a morph-weight animation: `times[k]` is the time of keyframe k and
    /// `weights[k]` holds one weight per morph target (step interpolation is
    /// avoided; use LINEAR to match Acknex frame blending).
    pub fn add_weights_animation(&mut self, name: &str, node: usize, times: &[f32], weights: &[Vec<f32>]) {
        let t: Vec<[f32; 1]> = times.iter().map(|&x| [x]).collect();
        let input = self.accessor_vec(&t, true, None);
        let flat: Vec<[f32; 1]> = weights.iter().flatten().map(|&x| [x]).collect();
        let output = self.accessor_vec(&flat, false, None);
        self.animations.push(json!({
            "name": name,
            "samplers": [{ "input": input, "output": output, "interpolation": "LINEAR" }],
            "channels": [{ "sampler": 0, "target": { "node": node, "path": "weights" } }],
        }));
    }

    pub fn write(self, path: &std::path::Path) -> anyhow::Result<()> {
        let mut doc = json!({
            "asset": { "version": "2.0", "generator": "sammy-convert" },
            "buffers": [{ "byteLength": self.bin.len() }],
            "bufferViews": self.buffer_views,
            "accessors": self.accessors,
            "meshes": self.meshes,
            "nodes": self.nodes,
            "scenes": [{ "nodes": self.root_nodes }],
            "scene": 0,
            "samplers": [
                { "magFilter": 9728, "minFilter": 9984, "wrapS": 10497, "wrapT": 10497 },
                { "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 },
            ],
        });
        for (k, v) in [("images", self.images), ("textures", self.textures), ("materials", self.materials), ("animations", self.animations)] {
            if !v.is_empty() {
                doc[k] = json!(v);
            }
        }
        let mut json_bytes = serde_json::to_vec(&doc)?;
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let mut bin = self.bin;
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend(b"glTF");
        out.extend(2u32.to_le_bytes());
        out.extend((total as u32).to_le_bytes());
        out.extend((json_bytes.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json_bytes);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
        std::fs::write(path, out)?;
        Ok(())
    }
}

/// Acknex (X fwd, Y left, Z up) -> glTF (X left, Y up, Z fwd): a cyclic axis
/// permutation, so handedness is preserved and model fronts face glTF +Z.
pub fn to_gltf([x, y, z]: [f32; 3]) -> [f32; 3] {
    [y, z, x]
}
