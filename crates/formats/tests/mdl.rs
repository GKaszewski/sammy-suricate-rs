//! Parses every extracted model (skipped when `extracted/` is absent).

use std::path::PathBuf;

use formats::mdl::{self, SkinFormat, Version};

fn extracted() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    dir.is_dir().then_some(dir)
}

#[test]
fn parses_all_models() {
    let Some(dir) = extracted() else { return };
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("mdl")) {
            continue;
        }
        let m = mdl::parse(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!m.frames.is_empty() && !m.skins.is_empty(), "{}", path.display());
        let vb = m.vertex_buffer();
        assert_eq!(vb.indices.len(), m.triangles.len() * 3);
        assert!(vb.indices.iter().all(|&i| (i as usize) < vb.uvs.len()));
        n += 1;
    }
    assert_eq!(n, 37);
}

#[test]
fn suricat_layout() {
    let Some(dir) = extracted() else { return };
    let m = mdl::parse(&std::fs::read(dir.join("suricat1.MDL")).unwrap()).unwrap();
    assert_eq!(m.version, Version::Mdl3);
    assert_eq!(m.skin_size, [120, 480]);
    assert_eq!(m.skins[0].format, SkinFormat::Rgb565);
    assert_eq!((m.frames[0].positions.len(), m.triangles.len(), m.frames.len()), (394, 705, 553));
    let anims = m.animations();
    let walk = anims.iter().find(|a| a.name == "walkB").unwrap();
    assert_eq!(walk.frames.len(), 40);
    assert_eq!(m.frames[walk.frames[0]].name, "walkB01");
}

#[test]
fn mdl5_logo() {
    let Some(dir) = extracted() else { return };
    let m = mdl::parse(&std::fs::read(dir.join("titre.mdl")).unwrap()).unwrap();
    assert_eq!(m.version, Version::Mdl5);
    assert_eq!(m.skins[0].format, SkinFormat::Argb4444);
    assert_eq!(m.skin_size, [512, 184]);
}

#[test]
fn group_names() {
    assert_eq!(mdl::frame_group("walk01"), "walk");
    assert_eq!(mdl::frame_group("Frame 1"), "Frame");
    assert_eq!(mdl::frame_group("walk_trans"), "walk_trans");
}
