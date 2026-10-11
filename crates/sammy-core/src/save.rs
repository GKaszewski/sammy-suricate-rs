//! Persistent progress (docs/hud-saves.md §4-5): the port's `BestSco0` + `levelN.SAV`
//! in one serialisable model. Where it is stored is the frontend's business.

use formats::sav::{self, ids};
use serde::{Deserialize, Serialize};

use crate::levels::{FIRST_OF_WORLD, LAST_OF_WORLD, LEVELS, NBR_LEVELS};

/// `LevelsInfo[n]`; stored as its WDL number (0 / 1 / 2).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(into = "u8", try_from = "u8")]
pub enum LevelState {
    /// No such level (or its file is missing).
    #[default]
    Absent,
    /// Present, not reached yet.
    Locked,
    /// Reached or won: playable.
    Unlocked,
}

impl LevelState {
    /// From a `LevelsExist` value of an original save.
    pub fn from_wdl(v: f32) -> Self {
        if v >= 2.0 {
            Self::Unlocked
        } else if v != 0.0 {
            Self::Locked
        } else {
            Self::Absent
        }
    }
}

impl From<LevelState> for u8 {
    fn from(s: LevelState) -> u8 {
        s as u8
    }
}

impl TryFrom<u8> for LevelState {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Absent),
            1 => Ok(Self::Locked),
            2 => Ok(Self::Unlocked),
            _ => Err(format!("bad level state {v}")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SaveGame {
    pub version: u32,
    /// `LevelsInfo[0..50]`.
    pub level_state: Vec<LevelState>,
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
    /// Port additions: window mode, resolutions and UI size.
    #[serde(default)]
    pub display: DisplaySettings,
    /// Never written to disk (tests, replays).
    #[serde(skip)]
    pub readonly: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct DisplaySettings {
    pub mode: DisplayMode,
    /// Windowed size in physical pixels.
    pub window: [u32; 2],
    /// Exclusive fullscreen video mode; `None` = the desktop's.
    pub fullscreen: Option<[u32; 2]>,
    /// 0 large (biggest whole-number scale that fits), 1 medium, 2 small.
    pub ui_size: u8,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self { mode: DisplayMode::Windowed, window: [1280, 720], fullscreen: None, ui_size: 0 }
    }
}

impl Default for SaveGame {
    fn default() -> Self {
        // A fresh install: every shipped level present but locked, the first of each world open.
        let mut level_state = vec![LevelState::Absent; NBR_LEVELS];
        for (slot, _) in LEVELS {
            level_state[*slot as usize] = LevelState::Locked;
        }
        for first in FIRST_OF_WORLD {
            level_state[first as usize] = LevelState::Unlocked;
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
            last_levels: LAST_OF_WORLD,
            music: 100.0,
            sound: 100.0,
            joypad: false,
            display: DisplaySettings::default(),
            readonly: false,
        }
    }
}

impl SaveGame {
    pub fn state(&self, slot: u32) -> LevelState {
        self.level_state.get(slot as usize).copied().unwrap_or(LevelState::Absent)
    }

    /// Playable from the menu. Registered builds also open 21 / 37 once 20 / 36 are
    /// open (interfac.wdl:103-131); the port counts as registered.
    pub fn playable(&self, slot: u32) -> bool {
        use LevelState::*;
        self.state(slot) == Unlocked
            || (slot == 21 && self.state(20) == Unlocked && self.state(21) != Absent)
            || (slot == 37 && self.state(36) == Unlocked && self.state(37) != Absent)
    }

    /// `ValideLevel` (InitLvl.wdl:239-258).
    pub fn validate(&mut self, slot: u32) {
        if let Some(s) = self.level_state.get_mut(slot as usize) {
            *s = LevelState::Unlocked;
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

    /// Pads the per-level arrays of an older or hand-edited save.
    pub fn fix_lengths(&mut self) {
        self.level_state.resize(NBR_LEVELS, LevelState::Absent);
        self.best_scores.resize(NBR_LEVELS, 0);
        self.checkpoints.resize(NBR_LEVELS, None);
    }

    /// Imports the original game's saves (docs/hud-saves.md §5.2); `find` reads a save
    /// file of the game directory by name (case-insensitive).
    pub fn import(find: impl Fn(&str) -> Option<Vec<u8>>) -> Option<Self> {
        let best = sav::parse(&find("BestSco0.SAV")?).ok()?;
        let mut s = Self::default();
        // Only element N of levelN.SAV counts (sammypolish.wdl:274-276).
        for n in 1..NBR_LEVELS {
            if let Some(file) = find(&format!("level{n}.SAV")).and_then(|d| sav::parse(&d).ok()) {
                let v = file.array(ids::LEVELS_EXIST).get(n).copied().unwrap_or(0.0);
                s.level_state[n] = LevelState::from_wdl(v);
            } else {
                s.level_state[n] = LevelState::Absent;
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
    fn level_states_stay_numbers_in_json() {
        let s = SaveGame::default();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"level_state\":[0,2,1,"));
        let back: SaveGame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.state(1), LevelState::Unlocked);
        assert_eq!(back.state(2), LevelState::Locked);
        assert_eq!(back.state(13), LevelState::Absent);
    }
}
