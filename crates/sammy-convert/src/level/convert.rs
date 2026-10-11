//! One level to .glb, entities JSON and lightmap PNG.

use super::*;

pub(super) fn convert(level: &Wmb, name: &str, out: &Path) -> anyhow::Result<Stats> {
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

pub(super) fn entity_node(index: usize, e: &Entity, extras: Value) -> Node {
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
