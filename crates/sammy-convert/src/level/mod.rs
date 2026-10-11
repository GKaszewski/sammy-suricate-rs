//! `sammy-convert level`: WMB4 -> .glb + entities JSON.
//!
//! Per input `<name>.wmb` this writes:
//! - `<name>.glb`: one mesh per BSP model (model 0 = world) with one primitive per
//!   texture/lighting batch. Base texture on TEXCOORD_0, baked lightmap atlas in
//!   the occlusion slot on TEXCOORD_1. Faces flagged invisible are skipped.
//!   One empty node per entity (translation/rotation/scale converted to glTF
//!   space) with every entity field in `extras`.
//! - `<name>.entities.json`: the same entity data in native Acknex coordinates.
//! - `<name>.lightmap.png`: the lightmap atlas (only for files that have one).

use std::path::{Path, PathBuf};

use anyhow::Context;
use formats::wmb::{self, Entity, Wmb};
use serde_json::{json, Value};

use crate::glb::{to_gltf, Glb, Node, Primitive};
use crate::images::write_png;

mod convert;
mod entities;

pub use self::entities::gltf_rotation;
use self::{convert::*, entities::*};

pub fn run(out: &Path, files: &[PathBuf]) -> anyhow::Result<()> {
    std::fs::create_dir_all(out)?;
    for f in files {
        let data = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
        let palette = find_palette(f);
        let level = wmb::parse_with_palette(&data, palette.as_ref()).with_context(|| format!("parsing {}", f.display()))?;
        let name = f.file_stem().and_then(|s| s.to_str()).unwrap_or("level").to_string();
        let stats = convert(&level, &name, out).with_context(|| format!("converting {}", f.display()))?;
        println!(
            "{name}: {} faces ({} drawn, {} lit), {} textures, {} entities, lightmap {}",
            level.faces.len(),
            stats.drawn,
            stats.lit,
            level.textures.len(),
            level.entities.len(),
            stats.lightmap.map_or("none".into(), |(w, h)| format!("{w}x{h}")),
        );
    }
    Ok(())
}

/// Optional Quake-format palette (`palette.raw` / `palette.lmp`, 768 bytes RGB)
/// next to the input or in its parent directory, used for the few 8-bit
/// textures. Without it they are exported as greyscale.
fn find_palette(file: &Path) -> Option<[[u8; 3]; 256]> {
    let dir = file.parent()?;
    let candidates = [dir.to_path_buf(), dir.parent().map(Path::to_path_buf).unwrap_or_default()];
    for d in candidates {
        for name in ["palette.raw", "palette.lmp"] {
            if let Ok(b) = std::fs::read(d.join(name))
                && b.len() >= 768
            {
                return Some(std::array::from_fn(|i| [b[i * 3], b[i * 3 + 1], b[i * 3 + 2]]));
            }
        }
    }
    None
}

struct Stats {
    drawn: usize,
    lit: usize,
    lightmap: Option<(u32, u32)>,
}
