//! The original game's files, from wherever the platform has them: a folder on disk
//! (desktop, PS Vita) or files handed over in memory (web, a zip). Loose files take
//! priority over entries in `*.wrs` archives (Acknex lookup order); names are matched
//! case-insensitively, as on the original Windows install.

use std::collections::HashMap;
use std::sync::Arc;

/// Files every copy of the game has (checked before using a folder).
const REQUIRED: &[&str] = &["sons.wdl", sammy_core::levels::MENU_FILE];

/// Where a file's bytes are.
#[derive(Clone)]
enum Source {
    /// A loose file on disk, read when asked for.
    #[cfg(not(target_arch = "wasm32"))]
    Path(std::path::PathBuf),
    Bytes(Arc<[u8]>),
}

#[derive(Default)]
pub struct GameFiles {
    loose: HashMap<String, Source>,
    archived: HashMap<String, Arc<[u8]>>,
}

impl GameFiles {
    /// The game in a folder (its loose files and `*.wrs` archives).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(dir: &std::path::Path) -> anyhow::Result<Self> {
        let mut files = Self::default();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_lowercase()) else { continue };
            if name.ends_with(".wrs") {
                files.add_archive(&std::fs::read(&path)?)?;
            } else if path.is_file() {
                files.loose.insert(name, Source::Path(path));
            }
        }
        Ok(files)
    }

    /// The game from files in memory, e.g. picked in a browser or read from a zip.
    pub fn from_files(files: impl IntoIterator<Item = (String, Vec<u8>)>) -> anyhow::Result<Self> {
        let mut out = Self::default();
        for (name, data) in files {
            // Only the file name counts (a picked folder may come with its path).
            let name = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_lowercase();
            if name.ends_with(".wrs") {
                out.add_archive(&data)?;
            } else {
                out.loose.insert(name, Source::Bytes(data.into()));
            }
        }
        Ok(out)
    }

    fn add_archive(&mut self, data: &[u8]) -> anyhow::Result<()> {
        for e in formats::wrs::parse(data)? {
            self.archived.insert(e.name.to_lowercase(), e.data().into());
        }
        Ok(())
    }

    /// Whether these look like Sammy Suricate's files.
    pub fn is_game(&self) -> bool {
        REQUIRED.iter().all(|f| self.contains(f))
    }

    /// The first loose file (by name) with this extension.
    pub fn find_extension(&self, ext: &str) -> Option<String> {
        let ext = format!(".{}", ext.to_lowercase());
        let mut names: Vec<&String> = self.loose.keys().filter(|n| n.ends_with(&ext)).collect();
        names.sort();
        names.first().map(|n| (*n).clone())
    }

    pub fn contains(&self, name: &str) -> bool {
        let key = name.to_lowercase();
        self.loose.contains_key(&key) || self.archived.contains_key(&key)
    }

    pub fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        let key = name.to_lowercase();
        match self.loose.get(&key) {
            #[cfg(not(target_arch = "wasm32"))]
            Some(Source::Path(p)) => Ok(std::fs::read(p)?),
            Some(Source::Bytes(b)) => Ok(b.to_vec()),
            None => self.archived.get(&key).map(|b| b.to_vec()).ok_or_else(|| anyhow::anyhow!("game file not found: {name}")),
        }
    }
}

impl sammy_core::sim::Files for GameFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        GameFiles::read(self, name).ok()
    }
}
