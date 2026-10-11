use super::*;

/// The original game's saves on this machine (if any) import as playable levels.
#[test]
fn imports_user_progress_when_present() {
    let storage = Storage::user();
    #[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
    let Some(files) = crate::locate::known(&storage).and_then(|d| GameFiles::open(&d).ok()) else { return };
    let Some(s) = SaveGame::import(|name| files.read(name).ok()) else { return };
    assert!(s.playable(1) && s.playable(17) && s.playable(33));
    assert_eq!(s.state(14), LevelState::Absent);
}
