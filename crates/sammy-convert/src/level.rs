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

fn convert(level: &Wmb, name: &str, out: &Path) -> anyhow::Result<Stats> {
    let mesh = level.build_mesh();
    let mut glb = Glb::default();

    let lightmap_tex = mesh.lightmap.as_ref().map(|img| glb.add_texture(&format!("{name}_lightmap"), img, false));
    if let Some(img) = &mesh.lightmap {
        write_png(&out.join(format!("{name}.lightmap.png")), img)?;
    }

    // Textures/materials are created lazily, once per (texture, lit) pair.
    let mut tex_ids: Vec<Option<usize>> = vec![None; level.textures.len()];
    let mut materials: Vec<((Option<usize>, bool), usize)> = Vec::new();
    let mut children = Vec::new();
    let (mut drawn, mut lit) = (0, 0);

    for (mi, model) in mesh.models.iter().enumerate() {
        let mut prim_data = Vec::new();
        for b in &model.batches {
            drawn += b.faces.len();
            if b.lit {
                lit += b.faces.len();
            }
            let key = (b.texture, b.lit);
            let mat = match materials.iter().find(|(k, _)| *k == key) {
                Some(&(_, m)) => m,
                None => {
                    let base = b.texture.map(|t| {
                        *tex_ids[t].get_or_insert_with(|| {
                            let tex = &level.textures[t];
                            glb.add_texture(&tex.name, &tex.image, false)
                        })
                    });
                    let tex_name = b.texture.map_or("untextured", |t| level.textures[t].name.as_str());
                    let mat_name = if b.lit { tex_name.to_string() } else { format!("{tex_name}_flat") };
                    let m = glb.add_material(&mat_name, base, if b.lit { lightmap_tex } else { None }, false);
                    materials.push((key, m));
                    m
                }
            };
            let positions: Vec<[f32; 3]> = b.positions.iter().copied().map(to_gltf).collect();
            let normals: Vec<[f32; 3]> = b.normals.iter().copied().map(to_gltf).collect();
            // The axis permutation preserves handedness, so CCW winding stays CCW.
            prim_data.push((positions, normals, mat, b));
        }
        if prim_data.is_empty() {
            continue;
        }
        let prims: Vec<Primitive> = prim_data
            .iter()
            .map(|(p, n, m, b)| Primitive {
                positions: p,
                normals: Some(n),
                uv0: Some(&b.uvs),
                uv1: b.lit.then_some(b.lightmap_uvs.as_slice()),
                indices: &b.indices,
                material: Some(*m),
                morph_targets: &[],
            })
            .collect();
        let mesh_name = if mi == 0 { format!("{name}_world") } else { format!("{name}_model{mi}") };
        let mesh_id = glb.add_mesh(&mesh_name, &prims, &[]);
        children.push(glb.add_node(Node { name: mesh_name, mesh: Some(mesh_id), ..Default::default() }, false));
    }

    let mut entity_json = Vec::new();
    for (i, e) in level.entities.iter().enumerate() {
        let extras = entity_to_json(e);
        children.push(glb.add_node(entity_node(i, e, extras.clone()), false));
        entity_json.push(extras);
    }

    glb.add_node(Node { name: name.to_string(), children, ..Default::default() }, true);
    glb.write(&out.join(format!("{name}.glb")))?;

    let doc = json!({
        "file": format!("{name}.wmb"),
        "coordinates": "acknex native: X forward, Y left, Z up; angles = pan, tilt, roll (degrees)",
        "bounds": level.models.first().map(|m| json!({ "mins": m.mins, "maxs": m.maxs })),
        "entities": entity_json,
    });
    std::fs::write(out.join(format!("{name}.entities.json")), serde_json::to_string_pretty(&doc)?)?;

    Ok(Stats { drawn, lit, lightmap: mesh.lightmap.as_ref().map(|i| (i.width, i.height)) })
}

fn entity_node(index: usize, e: &Entity, extras: Value) -> Node {
    let (name, origin, angles, scale) = match e {
        Entity::Position { origin, angles, name } => (name.clone(), *origin, *angles, None),
        Entity::Light { origin, .. } => (format!("light_{index}"), *origin, [0.0; 3], None),
        Entity::Object(o) => {
            let n = if o.name.is_empty() { format!("entity_{index}") } else { o.name.clone() };
            (n, o.origin, o.angles, Some(o.scale))
        }
        Entity::Info { .. } => ("level_info".into(), [0.0; 3], [0.0; 3], None),
        Entity::Unknown { kind, .. } => (format!("unknown{kind}_{index}"), [0.0; 3], [0.0; 3], None),
    };
    Node {
        name,
        translation: Some(to_gltf(origin)),
        rotation: (angles != [0.0; 3]).then(|| gltf_rotation(angles)),
        // Local axes are permuted like positions (scale_x = forward = glTF Z).
        scale: scale.map(to_gltf),
        extras: Some(extras),
        ..Default::default()
    }
}

/// Acknex pan/tilt/roll (degrees) -> glTF quaternion `[x, y, z, w]`.
///
/// Native rotation = Rz(pan) * Ry(-tilt) * Rx(roll): pan turns counter-clockwise
/// seen from above, positive tilt pitches the nose (+X) up. The roll sign is
/// assumed (unverified). The quaternion's vector part is permuted like positions.
pub fn gltf_rotation([pan, tilt, roll]: [f32; 3]) -> [f32; 4] {
    let q = |axis: [f32; 3], deg: f32| {
        let h = deg.to_radians() * 0.5;
        let s = h.sin();
        [axis[0] * s, axis[1] * s, axis[2] * s, h.cos()]
    };
    let mul = |a: [f32; 4], b: [f32; 4]| {
        [
            a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
            a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
            a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
        ]
    };
    let n = mul(mul(q([0.0, 0.0, 1.0], pan), q([0.0, 1.0, 0.0], -tilt)), q([1.0, 0.0, 0.0], roll));
    let v = to_gltf([n[0], n[1], n[2]]);
    [v[0], v[1], v[2], n[3]]
}

fn entity_to_json(e: &Entity) -> Value {
    match e {
        Entity::Position { origin, angles, name } => json!({
            "type": 1, "kind": "position", "name": name, "origin": origin, "angles": angles,
        }),
        Entity::Light { origin, color, range, flags } => json!({
            "type": 2, "kind": "light", "origin": origin, "color_percent": color, "range": range, "flags": flags,
        }),
        Entity::Object(o) => {
            let (names, unknown_bits) = wmb::entity_flag_names(o.flags);
            let mut v = json!({
                "type": o.kind,
                "kind": "entity",
                "name": o.name,
                "file": o.file,
                "action": o.action,
                "origin": o.origin,
                "angles": o.angles,
                "scale": o.scale,
                "skills": o.skills,
                "flags": o.flags,
                "flag_names": names,
                "flags_unknown_bits": unknown_bits,
                "ambient": o.ambient,
            });
            for (k, val) in [
                ("albedo", o.albedo.map(|x| json!(x))),
                ("path", o.path.map(|x| json!(x))),
                ("entity2", o.entity2.map(|x| json!(x))),
                ("material", o.material.as_ref().map(|x| json!(x))),
                ("string1", o.string1.as_ref().map(|x| json!(x))),
                ("string2", o.string2.as_ref().map(|x| json!(x))),
            ] {
                if let Some(val) = val {
                    v[k] = val;
                }
            }
            v
        }
        Entity::Info { azimuth, elevation, raw } => json!({
            "type": 5, "kind": "info", "sun_azimuth": azimuth, "sun_elevation": elevation, "raw_tail": hex(raw),
        }),
        Entity::Unknown { kind, raw } => json!({ "type": kind, "kind": "unknown", "raw": hex(raw) }),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
