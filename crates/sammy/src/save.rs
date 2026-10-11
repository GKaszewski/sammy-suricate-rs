//! Persistent progress (docs/hud-saves.md §4-5): the port's `BestSco0` + `levelN.SAV`
//! in one JSON file. On first run the original game's `.SAV` files are imported.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use formats::sav::{self, ids};
use serde::{Deserialize, Serialize};

use crate::flow::LEVELS;

pub const NBR_LEVELS: usize = 50;

/// `LevelsInfo` values.
pub const ABSENT: u8 = 0;
pub const LOCKED: u8 = 1;
pub const UNLOCKED: u8 = 2;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SaveGame {
    pub version: u32,
    /// `LevelsInfo[1..=49]`: 0 absent, 1 locked, 2 unlocked/won.
    pub level_state: Vec<u8>,
    /// `BestScores`.
    pub best_scores: Vec<i32>,
    /// `Player1RestartX/Y/Z`: checkpoint to start from (A4 coords).
    pub checkpoints: Vec<Option<[f32; 3]>>,
    /// `LevelEnCours`: the level "Play" resumes.
    pub last_level: u32,
    pub previous_level: u32,
    /// `NbrLevelsReach`.
    pub levels_reached: u32,
    /// `NeverPlayed`: games started (0 hides "Play").
    pub games_started: u32,
    /// `ExtroVue`.
    pub extro_seen: bool,
    /// `BonusState` / `BonusActif`.
    pub bonus_state: u8,
    /// `NumberLastTut/Jump/ThinkLevel` (12 / 28 / 44 as shipped).
    pub last_levels: [u32; 3],
    /// Option sliders 0..100.
    pub music: f32,
    pub sound: f32,
    pub joypad: bool,
}

impl Default for SaveGame {
    fn default() -> Self {
        // A fresh install: every shipped level present but locked, the first of each world open.
        let mut level_state = vec![ABSENT; NBR_LEVELS];
        for (slot, _) in LEVELS {
            level_state[*slot as usize] = LOCKED;
        }
        for first in [1, 17, 33] {
            level_state[first] = UNLOCKED;
        }
        Self {
            version: 1,
            level_state,
            best_scores: vec![0; NBR_LEVELS],
            checkpoints: vec![None; NBR_LEVELS],
            last_level: 1,
            previous_level: 0,
            levels_reached: 0,
            games_started: 0,
            extro_seen: false,
            bonus_state: 0,
            last_levels: [12, 28, 44],
            music: 100.0,
            sound: 100.0,
            joypad: false,
        }
    }
}

impl SaveGame {
    pub fn state(&self, slot: u32) -> u8 {
        self.level_state.get(slot as usize).copied().unwrap_or(ABSENT)
    }

    /// Playable from the menu. Registered builds also open 21 / 37 once 20 / 36 are
    /// open (interfac.wdl:103-131); the port counts as registered.
    pub fn playable(&self, slot: u32) -> bool {
        self.state(slot) == UNLOCKED
            || (slot == 21 && self.state(20) == UNLOCKED && self.state(21) != ABSENT)
            || (slot == 37 && self.state(36) == UNLOCKED && self.state(37) != ABSENT)
    }

    /// `ValideLevel` (InitLvl.wdl:239-258).
    pub fn validate(&mut self, slot: u32) {
        if let Some(s) = self.level_state.get_mut(slot as usize) {
            *s = UNLOCKED;
        }
    }

    /// End of a game (mm1.wdl:972-997): checkpoint (`StoreCurrentSkills`), best score,
    /// and on a win the level and the next one become playable.
    pub fn record_game(&mut self, slot: u32, score: i32, checkpoint: Option<[f32; 3]>, won: bool, next: Option<u32>) {
        let i = slot as usize;
        if i >= NBR_LEVELS {
            return;
        }
        self.checkpoints[i] = checkpoint;
        self.best_scores[i] = self.best_scores[i].max(score);
        self.previous_level = slot;
        if won {
            self.validate(slot);
            if let Some(n) = next {
                self.validate(n);
                self.last_level = n;
                self.levels_reached += 1;
            }
        } else {
            self.last_level = slot;
        }
    }

    fn fix_lengths(&mut self) {
        self.level_state.resize(NBR_LEVELS, ABSENT);
        self.best_scores.resize(NBR_LEVELS, 0);
        self.checkpoints.resize(NBR_LEVELS, None);
    }

    /// Imports the original game's saves (docs/hud-saves.md §5.2).
    pub fn import(dir: &Path) -> Option<Self> {
        let find = |name: &str| -> Option<Vec<u8>> {
            let entry = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))?;
            std::fs::read(entry.path()).ok()
        };
        let best = sav::parse(&find("BestSco0.SAV")?).ok()?;
        let mut s = Self::default();
        // Only element N of levelN.SAV counts (sammypolish.wdl:274-276).
        for n in 1..NBR_LEVELS {
            if let Some(file) = find(&format!("level{n}.SAV")).and_then(|d| sav::parse(&d).ok()) {
                let v = file.array(ids::LEVELS_EXIST).get(n).copied().unwrap_or(0.0);
                s.level_state[n] = if v >= 2.0 { UNLOCKED } else if v != 0.0 { LOCKED } else { ABSENT };
            } else {
                s.level_state[n] = ABSENT;
            }
        }
        if let Some(ver) = find("Ver1.SAV").and_then(|d| sav::parse(&d).ok()) {
            let get = |id| ver.var(id).map(|v| v as u32);
            if let (Some(t), Some(j), Some(k)) = (get(ids::LAST_TUT), get(ids::LAST_JUMP), get(ids::LAST_THINK)) {
                s.last_levels = [t, j, k];
            }
        }
        for (i, v) in best.array(ids::BEST_SCORES).iter().enumerate().take(NBR_LEVELS) {
            s.best_scores[i] = *v as i32;
        }
        let (xs, ys, zs) = (best.array(ids::RESTART_X), best.array(ids::RESTART_Y), best.array(ids::RESTART_Z));
        for i in 0..NBR_LEVELS.min(xs.len()) {
            if xs[i] != 0.0 {
                s.checkpoints[i] = Some([xs[i], ys.get(i).copied().unwrap_or(0.0), zs.get(i).copied().unwrap_or(0.0)]);
            }
        }
        let skill = |n: &str| best.skill(n).unwrap_or(0.0);
        s.last_level = (skill("LevelEnCours") as u32).clamp(1, 49);
        s.previous_level = skill("PreviousLevel") as u32;
        s.levels_reached = skill("NbrLevelsReach") as u32;
        s.games_started = skill("NeverPlayed") as u32;
        s.extro_seen = best.var(ids::EXTRO_VUE).unwrap_or(0.0) != 0.0;
        s.music = best.var(ids::SLIDER_MUSIC).unwrap_or(100.0);
        s.sound = best.var(ids::SLIDER_SOUND).unwrap_or(100.0);
        s.joypad = best.var(ids::JOYPAD).unwrap_or(0.0) != 0.0;
        Some(s)
    }
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
    match SaveGame::import(game_dir) {
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
    if std::env::var_os("SAMMY_SCREENSHOT").is_some() || std::env::var_os("SAMMY_NOSAVE").is_some() {
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
    fn win_unlocks_next_and_keeps_best() {
        let mut s = SaveGame::default();
        assert!(s.playable(1) && !s.playable(2) && s.playable(17));
        s.record_game(1, 300, None, true, Some(2));
        s.record_game(1, 100, Some([1.0, 2.0, 3.0]), false, None);
        assert!(s.playable(2));
        assert_eq!(s.best_scores[1], 300);
        assert_eq!(s.checkpoints[1], Some([1.0, 2.0, 3.0]));
        assert_eq!(s.last_level, 1);
        // Registered: 21 opens with 20.
        assert!(!s.playable(21));
        s.validate(20);
        assert!(s.playable(21));
    }

    #[test]
    fn imports_user_progress_when_present() {
        let dir = Path::new("/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate");
        let Some(s) = SaveGame::import(dir) else { return };
        assert!(s.playable(1) && s.playable(17) && s.playable(33));
        assert_eq!(s.state(14), ABSENT);
    }
}
