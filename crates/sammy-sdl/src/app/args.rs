//! The command line: `[--data <game folder>] [slot | file.wmb | menu]`.

use std::path::PathBuf;

/// What to start with.
pub struct Args {
    /// The game folder (`--data`), else it is looked for.
    pub data: Option<PathBuf>,
    /// The level: a slot, a `.wmb` file or `menu`; else `SAMMY_LEVEL` (e.g. from the
    /// Vita's env.txt), else the menu.
    pub level: Option<String>,
}

impl Args {
    pub fn from_env() -> Self {
        let mut argv: Vec<String> = std::env::args().skip(1).collect();
        let data = argv.iter().position(|a| a == "--data").map(|i| {
            let dir = argv.get(i + 1).cloned().unwrap_or_default();
            argv.drain(i..(i + 2).min(argv.len()));
            PathBuf::from(dir)
        });
        let level = argv.first().cloned().or_else(|| std::env::var("SAMMY_LEVEL").ok());
        Self { data, level }
    }

    /// The level file to load first.
    pub fn level_file(&self) -> String {
        match self.level.as_deref() {
            None | Some("menu") => sammy_core::levels::MENU_FILE.to_owned(),
            Some(l) => l.parse::<u32>().ok().map(sammy_core::levels::LevelSlot).and_then(sammy_core::levels::file_for).map_or_else(|| l.to_owned(), str::to_owned),
        }
    }
}
