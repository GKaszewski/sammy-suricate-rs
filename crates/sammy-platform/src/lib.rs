//! Sammy Suricate on a machine, for any frontend: the original game's files
//! ([`vfs::GameFiles`]), the player's storage and saves ([`storage`], [`save`]), finding
//! the game on a desktop ([`locate`]) and the sound table ([`sounds`]).

pub mod debug;
#[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
pub mod locate;
pub mod save;
pub mod sounds;
pub mod storage;
pub mod vfs;

use std::path::PathBuf;

pub use storage::{Settings, Storage};
pub use vfs::GameFiles;

/// The player's copy of the game on this platform: on desktops found or asked for
/// ([`locate`]), `explicit` (`--data`) first.
#[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
pub fn game_files(explicit: Option<PathBuf>, storage: &Storage) -> anyhow::Result<GameFiles> {
    locate::find(explicit, storage).map(|(files, _)| files)
}

/// The PS Vita reads the game from `ux0:data/sammy-suricate/data`.
#[cfg(target_os = "vita")]
pub fn game_files(explicit: Option<PathBuf>, storage: &Storage) -> anyhow::Result<GameFiles> {
    let dir = explicit.or_else(|| storage.dir().map(|d| d.join("data"))).ok_or_else(|| anyhow::anyhow!("no data folder"))?;
    GameFiles::open(&dir).ok().filter(GameFiles::is_game).ok_or_else(|| anyhow::anyhow!("no Sammy Suricate game files in {}", dir.display()))
}

/// On the web the game files come from the browser (picked once and kept there); not
/// implemented yet.
#[cfg(target_arch = "wasm32")]
pub fn game_files(_explicit: Option<PathBuf>, _storage: &Storage) -> anyhow::Result<GameFiles> {
    anyhow::bail!("the web build cannot load the game files yet")
}
