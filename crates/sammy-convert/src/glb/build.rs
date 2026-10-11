//! Adding textures, materials, meshes, nodes and animations.

use super::*;

impl Glb {
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

}
