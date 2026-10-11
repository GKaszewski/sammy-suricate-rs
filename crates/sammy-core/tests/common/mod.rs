//! The original game files for tests: `SAMMY_DIR`, else the folder the game
//! remembered on this machine. Tests skip without them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sammy_core::sim::Files;

/// Loose files over `*.wrs` archive entries, names case-insensitive.
pub struct GameFiles {
    loose: HashMap<String, PathBuf>,
    archived: HashMap<String, Vec<u8>>,
}

impl GameFiles {
    fn open(dir: &Path) -> Option<Self> {
        let mut loose = HashMap::new();
        let mut archived = HashMap::new();
        for entry in std::fs::read_dir(dir).ok()? {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_string_lossy().to_lowercase();
            if name.ends_with(".wrs") {
                for e in formats::wrs::parse(&std::fs::read(&path).ok()?).ok()? {
                    archived.insert(e.name.to_lowercase(), e.data());
                }
            } else if path.is_file() {
                loose.insert(name, path);
            }
        }
        Some(Self { loose, archived })
    }
}

impl Files for GameFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let key = name.to_lowercase();
        match self.loose.get(&key) {
            Some(p) => std::fs::read(p).ok(),
            None => self.archived.get(&key).cloned(),
        }
    }
}

pub fn files() -> Option<&'static GameFiles> {
    static FILES: std::sync::OnceLock<Option<GameFiles>> = std::sync::OnceLock::new();
    FILES
        .get_or_init(|| {
            GameFiles::open(&game_dir()?)
        })
        .as_ref()
}

/// `SAMMY_DIR`, else the game folder the game remembered on this machine
/// (`settings.json` in its storage, `SAMMY_HOME` or the user data folder).
fn game_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("SAMMY_DIR") {
        return Some(d.into());
    }
    let home = std::env::var_os("SAMMY_HOME").map(PathBuf::from).or_else(|| dirs::data_dir().map(|d| d.join("sammy-suricate")))?;
    let settings: serde_json::Value = serde_json::from_slice(&std::fs::read(home.join("settings.json")).ok()?).ok()?;
    settings.get("game_dir")?.as_str().map(PathBuf::from)
}

