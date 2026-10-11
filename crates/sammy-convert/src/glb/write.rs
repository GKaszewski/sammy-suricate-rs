//! Writing the .glb container.

use super::*;

impl Glb {
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
        while !bin.len().is_multiple_of(4) {
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
