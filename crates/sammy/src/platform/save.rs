//! Where progress is stored: `save.json` in the user data directory, imported from the
//! original game's `.SAV` files on first run. The model is [`sammy_core::save`].

use std::path::{Path, PathBuf};

use bevy::prelude::*;

pub use sammy_core::levels::NBR_LEVELS;
pub use sammy_core::save::{DisplayMode, DisplaySettings, LevelState, SaveGame};

/// The original game's saves in `dir`.
fn import(dir: &Path) -> Option<SaveGame> {
    SaveGame::import(|name| {
        let entry = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))?;
        std::fs::read(entry.path()).ok()
    })
}

/// `$XDG_DATA_HOME/sammy-suricate/save.json` (or `~/.local/share/...`); `SAMMY_SAVE` overrides.
pub fn save_path() -> PathBuf {
    if let Some(p) = std::env::var_os("SAMMY_SAVE") {
        return p.into();
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("sammy-suricate").join("save.json")
}

/// Loads the port's save, else imports the original game's, else starts fresh.
pub fn load(game_dir: &Path) -> SaveGame {
    let path = save_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<SaveGame>(&text) {
            Ok(mut s) => {
                s.fix_lengths();
                return s;
            }
            Err(e) => warn!("ignoring unreadable save {}: {e}", path.display()),
        }
    }
    match import(game_dir) {
        Some(s) => {
            info!("imported the original game's saves from {}", game_dir.display());
            write(&s);
            s
        }
        None => SaveGame::default(),
    }
}

pub fn write(save: &SaveGame) {
    // Scripted debug runs never touch the player's save.
    if save.readonly || std::env::var_os("SAMMY_SCREENSHOT").is_some() || std::env::var_os("SAMMY_NOSAVE").is_some() {
        return;
    }
    let path = save_path();
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(&path, serde_json::to_string_pretty(save).unwrap_or_default()));
    if let Err(e) = result {
        warn!("could not write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_user_progress_when_present() {
        let dir = Path::new(crate::DEFAULT_GAME_DIR);
        let Some(s) = import(dir) else { return };
        assert!(s.playable(1) && s.playable(17) && s.playable(33));
        assert_eq!(s.state(14), LevelState::Absent);
    }
}
