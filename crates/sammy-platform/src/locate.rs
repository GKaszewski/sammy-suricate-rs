//! Finding the player's copy of Sammy Suricate on a desktop, in order:
//! 1. `--data <folder>` on the command line, or `SAMMY_DIR`;
//! 2. next to the executable (the game folder itself, or a `data` folder in it; on
//!    macOS also the app bundle's `Resources`);
//! 3. the folder picked on an earlier run (remembered in the settings);
//! 4. a folder dialog: the choice is checked and remembered.
//!
//! The PS Vita reads a fixed folder (`ux0:data/sammy-suricate/data`); the web gets its
//! files from the browser.

use std::path::{Path, PathBuf};

use log::info;

use crate::storage::{Settings, Storage};
use crate::vfs::GameFiles;

/// Opens the game in `dir` if it is one.
fn open(dir: &Path) -> Option<GameFiles> {
    GameFiles::open(dir).ok().filter(GameFiles::is_game)
}

/// Folders next to the executable that may hold the game.
pub fn beside_executable() -> Vec<PathBuf> {
    let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else { return Vec::new() };
    vec![exe.join("data"), exe.clone(), exe.join("../Resources/data")]
}

/// The game folder known without asking: `SAMMY_DIR`, else the remembered one.
pub fn known(storage: &Storage) -> Option<PathBuf> {
    std::env::var_os("SAMMY_DIR").map(PathBuf::from).or_else(|| Settings::load(storage).game_dir.map(PathBuf::from))
}

/// The game's files and the folder they came from; an error if the player cancels.
pub fn find(explicit: Option<PathBuf>, storage: &Storage) -> anyhow::Result<(GameFiles, PathBuf)> {
    if let Some(dir) = explicit.or_else(|| std::env::var_os("SAMMY_DIR").map(PathBuf::from)) {
        return open(&dir).map(|f| (f, dir.clone())).ok_or_else(|| anyhow::anyhow!("no Sammy Suricate game files in {}", dir.display()));
    }
    let mut settings = Settings::load(storage);
    let remembered = settings.game_dir.as_ref().map(PathBuf::from);
    for dir in beside_executable().into_iter().chain(remembered) {
        if let Some(f) = open(&dir) {
            return Ok((f, dir));
        }
    }
    // Ask, until the player picks a game folder or cancels.
    let mut title = "Select your Sammy Suricate folder";
    loop {
        let Some(dir) = rfd::FileDialog::new().set_title(title).pick_folder() else {
            anyhow::bail!(
                "Sammy Suricate's game files are needed. Put this program in the game's folder, \
                 pick the folder when asked, or start it with --data <folder>."
            );
        };
        if let Some(f) = open(&dir) {
            info!("using the game files in {}", dir.display());
            settings.game_dir = Some(dir.to_string_lossy().into_owned());
            settings.store(storage);
            return Ok((f, dir));
        }
        title = "That folder has no Sammy Suricate files. Select the game's folder";
    }
}
