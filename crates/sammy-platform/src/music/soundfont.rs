//! Finding and parsing a General MIDI SoundFont.

use std::path::PathBuf;
use std::sync::Arc;

use rustysynth::SoundFont;

use crate::{GameFiles, Storage};

/// `SAMMY_SOUNDFONT`, else the first `.sf2` next to the executable (releases bundle
/// one; on the Vita in the package), in the player's storage, among the game files, or in `/usr/share/soundfonts`.
/// Returns the file's bytes and a name for messages.
pub fn find_soundfont(storage: &Storage, files: &GameFiles) -> Option<(String, Vec<u8>)> {
    let from_path = |p: PathBuf| std::fs::read(&p).ok().map(|d| (p.display().to_string(), d));
    let first_in = |d: PathBuf| {
        let mut v: Vec<_> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sf2"))).collect();
        v.sort();
        v.into_iter().next()
    };
    if let Ok(p) = std::env::var("SAMMY_SOUNDFONT") {
        return from_path(p.into());
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    #[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
    dirs.extend(crate::locate::beside_executable());
    // The Vita package itself.
    #[cfg(target_os = "vita")]
    dirs.push("app0:".into());
    dirs.extend(storage.dir().map(PathBuf::from));
    if let Some(p) = dirs.into_iter().find_map(first_in) {
        return from_path(p);
    }
    if let Some(name) = files.find_extension("sf2") {
        return files.read(&name).ok().map(|d| (name, d));
    }
    #[cfg(target_os = "linux")]
    if let Some(p) = first_in("/usr/share/soundfonts".into()) {
        return from_path(p);
    }
    None
}

/// Parses a SoundFont.
pub fn parse_soundfont(data: &[u8]) -> anyhow::Result<Arc<SoundFont>> {
    SoundFont::new(&mut &data[..]).map(Arc::new).map_err(|e| anyhow::anyhow!("{e:?}"))
}
