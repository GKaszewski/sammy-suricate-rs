//! Virtual file system over the original game install: loose files in the game
//! directory take priority over entries in `*.wrs` archives (Acknex lookup order).
//! Names are matched case-insensitively, as on the original Windows install.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;

#[derive(Resource)]
pub struct GameFiles {
    loose: HashMap<String, PathBuf>,
    archived: HashMap<String, Vec<u8>>,
}

impl GameFiles {
    pub fn open(dir: &Path) -> anyhow::Result<Self> {
        let mut loose = HashMap::new();
        let mut archived = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_lowercase()) else { continue };
            if name.ends_with(".wrs") {
                let data = std::fs::read(&path)?;
                for e in formats::wrs::parse(&data)? {
                    archived.insert(e.name.to_lowercase(), e.data());
                }
            } else if path.is_file() {
                loose.insert(name, path);
            }
        }
        Ok(Self { loose, archived })
    }

    pub fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        let key = name.to_lowercase();
        if let Some(p) = self.loose.get(&key) {
            return Ok(std::fs::read(p)?);
        }
        self.archived
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("game file not found: {name}"))
    }

}
