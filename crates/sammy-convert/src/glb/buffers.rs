//! Buffer views and accessors.

use super::*;

impl Glb {
    pub(super) fn push_view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
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

    pub(super) fn accessor_vec<const N: usize>(&mut self, data: &[[f32; N]], with_bounds: bool, target: Option<u32>) -> usize {
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

}
