//! Prints a WMB's textures and per-texture face counts/bounds: `cargo run --example wmbinfo <file>`.
fn main() {
    let path = std::env::args().nth(1).expect("usage: wmbinfo <file.wmb>");
    let wmb = formats::wmb::parse(&std::fs::read(path).unwrap()).unwrap();
    let mesh = wmb.build_mesh();
    for (mi, m) in mesh.models.iter().enumerate() {
        for b in &m.batches {
            let name = b.texture.map_or("-", |t| wmb.textures[t].name.as_str());
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &b.positions {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
            }
            let maxi = b.indices.iter().max().copied().unwrap_or(0);
            println!("  verts={} uvs={} lmuvs={} normals={} max_index={maxi} uv0={:?} lm0={:?} n0={:?}", b.positions.len(), b.uvs.len(), b.lightmap_uvs.len(), b.normals.len(), b.uvs.first(), b.lightmap_uvs.first(), b.normals.first());
            println!("model {mi} {name:20} lit={} faces={:5} tris={:5} min={lo:?} max={hi:?}", b.lit, b.faces.len(), b.indices.len() / 3);
        }
    }
    let marked: std::collections::HashSet<u32> = wmb.mark_surfaces.iter().copied().collect();
    let mut refs = std::collections::BTreeMap::new();
    for (i, f) in wmb.faces.iter().enumerate() {
        let ti = &wmb.texinfos[f.texinfo as usize];
        let name = wmb.textures.get(ti.texture as usize).map_or("-", |t| t.name.as_str());
        let e = refs.entry(name.to_owned()).or_insert((0, 0));
        if marked.contains(&(i as u32)) { e.0 += 1 } else { e.1 += 1 }
    }
    for (n, (m, u)) in &refs {
        println!("marksurf tex={n:20} marked={m} unmarked={u}");
    }
    let mut seen = std::collections::BTreeMap::new();
    for f in &wmb.faces {
        let ti = &wmb.texinfos[f.texinfo as usize];
        let name = wmb.textures.get(ti.texture as usize).map_or("-", |t| t.name.as_str());
        *seen.entry((name.to_owned(), ti.flags)).or_insert(0) += 1;
    }
    for ((name, flags), n) in seen {
        println!("faces tex={name:20} flags={flags:#x} count={n}");
    }
    for t in &wmb.textures {
        println!("tex {} {}x{} {}bit", t.name, t.width, t.height, t.bits);
    }
}
