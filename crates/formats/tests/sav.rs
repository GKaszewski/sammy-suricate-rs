//! Decodes the user's original saves when the install is present (docs/hud-saves.md §4.6).

use formats::sav::{self, ids};

const DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

#[test]
fn decodes_original_saves() {
    let Ok(best) = std::fs::read(format!("{DIR}/BestSco0.SAV")) else { return };
    let s = sav::parse(&best).unwrap();
    assert_eq!(s.vars.len(), 18);
    assert_eq!(s.skills.len(), 38);
    let scores = s.array(ids::BEST_SCORES);
    assert_eq!(scores.len(), 50);
    assert!(s.skill("LevelEnCours").is_some());
    let ver = sav::parse(&std::fs::read(format!("{DIR}/Ver1.SAV")).unwrap()).unwrap();
    assert_eq!(ver.skill("LevelEnCours"), Some(999.0));
    assert_eq!(ver.var(ids::LAST_JUMP), Some(28.0));
}
