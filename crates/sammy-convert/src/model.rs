//! `sammy-convert model`: MDL3/MDL5 -> .glb with morph-target vertex animation.
//!
//! Layout of each output file:
//! - one node + mesh; base geometry = frame 0, every other frame is a morph target
//!   (position deltas, target names = frame names);
//! - one animation per frame group (`walk01..walk32` -> `walk`), keyed at
//!   [`FRAME_RATE`] with linear blending (Acknex interpolates between frames too);
//! - skin 0 embedded as base colour; extra skins written as `<name>_skin<i>.png`;
//! - node extras: `frames` (names) and `animations` (group -> frame indices), so a
//!   runtime can drive frames with the game's own percent-per-tick speeds.

use std::path::{Path, PathBuf};

use anyhow::Context;
use formats::mdl::{self, SkinFormat};
use serde_json::json;

use crate::glb::{to_gltf, Glb, Node, Primitive};
use crate::images::write_png;

/// Keyframe rate for exported animations: one model frame per Acknex tick (1/16 s).
/// The game actually drives cycles by script (`set_cycle` with a 0..100 percent
/// step advanced by `speed * time`), so real speeds vary per animation.
pub const FRAME_RATE: f32 = 16.0;

pub fn run(out: &Path, files: &[PathBuf]) -> anyhow::Result<()> {
    std::fs::create_dir_all(out)?;
    for f in files {
        let data = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
        let model = mdl::parse(&data).with_context(|| format!("parsing {}", f.display()))?;
        let name = f.file_stem().and_then(|s| s.to_str()).unwrap_or("model").to_string();
        convert(&model, &name, out).with_context(|| format!("converting {}", f.display()))?;
        println!(
            "{name}: {} verts, {} tris, {} frames, {} skins, anims [{}]",
            model.frames.first().map_or(0, |fr| fr.positions.len()),
            model.triangles.len(),
            model.frames.len(),
            model.skins.len(),
            model.animations().iter().map(|a| format!("{}:{}", a.name, a.frames.len())).collect::<Vec<_>>().join(" ")
        );
    }
    Ok(())
}

fn convert(model: &mdl::Mdl, name: &str, out: &Path) -> anyhow::Result<()> {
    let mut glb = Glb::default();
    let Some(base_frame) = model.frames.first() else {
        anyhow::bail!("model has no frames");
    };
    let vb = model.vertex_buffer();
    let base: Vec<[f32; 3]> = vb.positions(base_frame).into_iter().map(to_gltf).collect();
    let normals: Vec<[f32; 3]> = vb.normals(base_frame).into_iter().map(to_gltf).collect();

    let targets: Vec<Vec<[f32; 3]>> = model.frames[1..]
        .iter()
        .map(|fr| {
            vb.positions(fr)
                .into_iter()
                .zip(&base)
                .map(|(p, b)| {
                    let p = to_gltf(p);
                    [p[0] - b[0], p[1] - b[1], p[2] - b[2]]
                })
                .collect()
        })
        .collect();
    let target_names: Vec<String> = model.frames[1..].iter().map(|f| f.name.clone()).collect();

    let material = model.skins.first().map(|skin| {
        let tex = glb.add_texture(&format!("{name}_skin0"), &skin.image, false);
        glb.add_material(name, Some(tex), None, skin.format == SkinFormat::Argb4444)
    });
    for (i, skin) in model.skins.iter().enumerate().skip(1) {
        write_png(&out.join(format!("{name}_skin{i}.png")), &skin.image)?;
    }

    let prim = Primitive {
        positions: &base,
        normals: Some(&normals),
        uv0: Some(&vb.uvs),
        uv1: None,
        indices: &vb.indices,
        material,
        morph_targets: &targets,
    };
    let mesh = glb.add_mesh(name, &[prim], &target_names);

    let anims = model.animations();
    let extras = json!({
        "frames": model.frames.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        "animations": anims.iter().map(|a| (a.name.clone(), json!(a.frames))).collect::<serde_json::Map<_, _>>(),
        "skins": model.skins.len(),
        "frameRate": FRAME_RATE,
    });
    let node = glb.add_node(Node { name: name.to_string(), mesh: Some(mesh), extras: Some(extras), ..Default::default() }, true);

    if !targets.is_empty() {
        for anim in &anims {
            let times: Vec<f32> = (0..anim.frames.len()).map(|k| k as f32 / FRAME_RATE).collect();
            let weights: Vec<Vec<f32>> = anim
                .frames
                .iter()
                .map(|&f| {
                    let mut w = vec![0.0; targets.len()];
                    if f > 0 {
                        w[f - 1] = 1.0;
                    }
                    w
                })
                .collect();
            glb.add_weights_animation(&anim.name, node, &times, &weights);
        }
    }

    glb.write(&out.join(format!("{name}.glb")))
}
