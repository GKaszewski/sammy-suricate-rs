//! Parses every shipped WMB file (skipped when the extracted data is absent).

use std::path::{Path, PathBuf};

use formats::wmb::{self, Entity};

fn wmb_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    let mut out = Vec::new();
    for dir in [root.clone(), root.join("levels")] {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wmb")) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn parse_all() {
    let files = wmb_files();
    if files.is_empty() {
        eprintln!("no extracted data; skipping");
        return;
    }
    let mut lit_faces = 0usize;
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let w = wmb::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(w.models.len(), 1, "{}", path.display());
        let m = &w.models[0];
        assert_eq!(m.first_face, 0);
        assert_eq!(m.num_faces as usize, w.faces.len(), "{}", path.display());
        assert_eq!(m.vis_leafs as usize + 1, w.leaves.len(), "{}", path.display());

        // Lightmap rectangles must tile the lightmap lump exactly (4-byte padded).
        let mut lit: Vec<usize> = (0..w.faces.len()).filter(|&i| w.faces[i].light_offset >= 0).collect();
        lit.sort_by_key(|&i| w.faces[i].light_offset);
        for (k, &fi) in lit.iter().enumerate() {
            let r = w.face_lightmap_rect(fi).unwrap_or_else(|| panic!("{}: face {fi} rect", path.display()));
            let next = lit.get(k + 1).map_or(w.lightmaps.len(), |&n| w.faces[n].light_offset as usize);
            let size = (r.width * r.height * 2) as usize;
            assert!(next - r.offset == size.next_multiple_of(4) || (k + 1 == lit.len() && next - r.offset >= size),
                "{}: face {fi} lightmap {}x{} vs {} bytes", path.display(), r.width, r.height, next - r.offset);
        }
        lit_faces += lit.len();

        for e in &w.entities {
            assert!(!matches!(e, Entity::Unknown { .. }), "{}: unknown entity {}", path.display(), e.kind());
        }
        // Leaf 0 is solid; any point far outside is solid in hull 0.
        assert_eq!(w.leaves[0].contents, wmb::contents::SOLID);

        let mesh = w.build_mesh();
        let (mut tris, mut flipped) = (0usize, 0usize);
        for b in &mesh.models[0].batches {
            assert_eq!(b.positions.len(), b.uvs.len());
            assert!(!b.lit || b.lightmap_uvs.len() == b.positions.len());
            assert!(b.indices.iter().all(|&i| (i as usize) < b.positions.len()));
            // Emitted triangles are CCW around the face normal.
            for tri in b.indices.as_chunks::<3>().0 {
                let [a, bb, c] = [0, 1, 2].map(|k| b.positions[tri[k] as usize]);
                let e1 = [bb[0] - a[0], bb[1] - a[1], bb[2] - a[2]];
                let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let cr = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
                let n = b.normals[tri[0] as usize];
                let d = cr[0] * n[0] + cr[1] * n[1] + cr[2] * n[2];
                tris += 1;
                flipped += usize::from(d < -1.0);
            }
        }
        // A handful of faces in the shipped data are stored with reversed winding.
        assert!(flipped <= 3 + tris / 100, "{}: {flipped}/{tris} triangles flipped", path.display());
    }
    eprintln!("parsed {} files, {lit_faces} lit faces", files.len());
    assert!(files.len() >= 133, "expected all 133 WMB files, found {}", files.len());
}

#[test]
fn collision_and_vis_10001() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels/10001.wmb");
    let Ok(data) = std::fs::read(&path) else { return };
    let w = wmb::parse(&data).unwrap();
    let player = w
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Object(o) if o.action == "initmarche" => Some(o.origin),
            _ => None,
        })
        .unwrap();
    for hull in 0..3 {
        assert_ne!(w.point_contents(0, hull, player), wmb::contents::SOLID, "hull {hull}");
        // Inside the 16-quant floor brush (z -16..0) is solid in every hull.
        assert_eq!(w.point_contents(0, hull, [player[0], player[1], -8.0]), wmb::contents::SOLID, "hull {hull}");
    }
    let visleafs = w.models[0].vis_leafs as usize;
    for leaf in 1..w.leaves.len() {
        let row = w.decompress_vis(leaf).unwrap();
        assert_eq!(row.len(), visleafs.div_ceil(8));
    }
}
