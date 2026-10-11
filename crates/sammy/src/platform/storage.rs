//! Small named records the game keeps for the player (the save, settings), wherever the
//! platform keeps such things:
//! - desktop: the per-user data folder (`~/.local/share/sammy-suricate`,
//!   `%APPDATA%\sammy-suricate`, `~/Library/Application Support/sammy-suricate`), or
//!   `SAMMY_HOME`;
//! - PS Vita: `ux0:data/sammy-suricate`;
//! - web: the browser's storage (not implemented yet: nothing is kept).

use bevy::prelude::*;

/// The folder name under the platform's data directory.
#[cfg(not(target_arch = "wasm32"))]
const APP_DIR: &str = "sammy-suricate";

#[derive(Resource, Clone, Debug)]
pub struct Storage {
    #[cfg(not(target_arch = "wasm32"))]
    dir: Option<std::path::PathBuf>,
}

impl Storage {
    /// The player's storage on this platform.
    pub fn user() -> Self {
        #[cfg(target_os = "vita")]
        return Self { dir: Some(std::path::PathBuf::from("ux0:data").join(APP_DIR)) };
        #[cfg(all(not(target_os = "vita"), not(target_arch = "wasm32")))]
        return Self { dir: std::env::var_os("SAMMY_HOME").map(Into::into).or_else(|| dirs::data_dir().map(|d| d.join(APP_DIR))) };
        #[cfg(target_arch = "wasm32")]
        return Self {};
    }

    /// The folder records live in (None where storage is not a folder).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dir(&self) -> Option<&std::path::Path> {
        self.dir.as_deref()
    }

    pub fn read(&self, key: &str) -> Option<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        return std::fs::read(self.dir.as_ref()?.join(key)).ok();
        #[cfg(target_arch = "wasm32")]
        {
            let _ = key;
            None
        }
    }

    pub fn write(&self, key: &str, data: &[u8]) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(dir) = &self.dir else { return Err(std::io::ErrorKind::NotFound.into()) };
            std::fs::create_dir_all(dir)?;
            std::fs::write(dir.join(key), data)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (key, data);
            Err(std::io::ErrorKind::Unsupported.into())
        }
    }
}

/// Settings kept outside the save: where the game files are.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct Settings {
    /// The game folder the player picked (desktop).
    pub game_dir: Option<String>,
}

const SETTINGS: &str = "settings.json";

impl Settings {
    pub fn load(storage: &Storage) -> Self {
        storage.read(SETTINGS).and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default()
    }

    pub fn store(&self, storage: &Storage) {
        if let Err(e) = storage.write(SETTINGS, serde_json::to_string_pretty(self).unwrap_or_default().as_bytes()) {
            warn!("could not store the settings: {e}");
        }
    }
}
