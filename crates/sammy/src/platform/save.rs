//! The player's progress: `save.json` in the platform's [`Storage`], imported from the
//! original game's `.SAV` files (in its folder) on first run. The model is
//! [`sammy_core::save`].

use bevy::prelude::*;

use super::storage::Storage;
use super::vfs::GameFiles;

pub use sammy_core::levels::NBR_LEVELS;
pub use sammy_core::save::{DisplayMode, DisplaySettings, LevelState, SaveGame};

const SAVE: &str = "save.json";

/// Loads the port's save, else imports the original game's, else starts fresh.
pub fn load(storage: &Storage, files: &GameFiles) -> SaveGame {
    if let Some(data) = storage.read(SAVE) {
        match serde_json::from_slice::<SaveGame>(&data) {
            Ok(mut s) => {
                s.fix_lengths();
                return s;
            }
            Err(e) => warn!("ignoring an unreadable save: {e}"),
        }
    }
    match SaveGame::import(|name| files.read(name).ok()) {
        Some(s) => {
            info!("imported the original game's saves");
            write(storage, &s);
            s
        }
        None => SaveGame::default(),
    }
}

pub fn write(storage: &Storage, save: &SaveGame) {
    // Scripted debug runs never touch the player's save.
    if save.readonly || std::env::var_os("SAMMY_SCREENSHOT").is_some() || std::env::var_os("SAMMY_NOSAVE").is_some() {
        return;
    }
    if let Err(e) = storage.write(SAVE, serde_json::to_string_pretty(save).unwrap_or_default().as_bytes()) {
        warn!("could not write the save: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The original game's saves on this machine (if any) import as playable levels.
    #[test]
    fn imports_user_progress_when_present() {
        let storage = Storage::user();
        #[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
        let Some(files) = crate::platform::locate::known(&storage).and_then(|d| GameFiles::open(&d).ok()) else { return };
        let Some(s) = SaveGame::import(|name| files.read(name).ok()) else { return };
        assert!(s.playable(1) && s.playable(17) && s.playable(33));
        assert_eq!(s.state(14), LevelState::Absent);
    }
}
