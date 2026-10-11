//! Persistent progress (docs/hud-saves.md §4-5): the port's `BestSco0` + `levelN.SAV`
//! in one serialisable model. Where it is stored is the frontend's business.

use formats::sav::{self, ids};
use serde::{Deserialize, Serialize};

use crate::levels::{FIRST_OF_WORLD, LAST_OF_WORLD, LEVELS, LevelSlot, NBR_LEVELS};
use crate::percent::Percent;
use crate::points::Points;

mod display;
mod level_state;

pub use self::{display::*, level_state::*};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SaveGame {
    pub version: u32,
    /// `LevelsInfo[0..50]`.
    pub level_state: Vec<LevelState>,
    /// `BestScores`.
    pub best_scores: Vec<Points>,
    /// `Player1RestartX/Y/Z`: checkpoint to start from (A4 coords).
    pub checkpoints: Vec<Option<[f32; 3]>>,
    /// `LevelEnCours`: the level "Play" resumes.
    pub last_level: LevelSlot,
    pub previous_level: LevelSlot,
    /// `NbrLevelsReach`.
    pub levels_reached: u32,
    /// `NeverPlayed`: games started (0 hides "Play").
    pub games_started: u32,
    /// `ExtroVue`.
    pub extro_seen: bool,
    /// `BonusState` / `BonusActif`.
    pub bonus_state: u8,
    /// `NumberLastTut/Jump/ThinkLevel` (12 / 28 / 44 as shipped).
    pub last_levels: [LevelSlot; 3],
    /// Option sliders 0..100.
    pub music: Percent,
    pub sound: Percent,
    pub joypad: bool,
    /// Port additions: window mode, resolutions and UI size.
    #[serde(default)]
    pub display: DisplaySettings,
    /// Never written to disk (tests, replays).
    #[serde(skip)]
    pub readonly: bool,
}

impl Default for SaveGame {
    fn default() -> Self {
        // A fresh install: every shipped level present but locked, the first of each world open.
        let mut level_state = vec![LevelState::Absent; NBR_LEVELS];
        for (slot, _) in LEVELS {
            level_state[slot.index()] = LevelState::Locked;
        }
        for first in FIRST_OF_WORLD {
            level_state[first.index()] = LevelState::Unlocked;
        }
        Self {
            version: 1,
            level_state,
            best_scores: vec![Points::ZERO; NBR_LEVELS],
            checkpoints: vec![None; NBR_LEVELS],
            last_level: FIRST_OF_WORLD[0],
            previous_level: LevelSlot::default(),
            levels_reached: 0,
            games_started: 0,
            extro_seen: false,
            bonus_state: 0,
            last_levels: LAST_OF_WORLD,
            music: Percent::FULL,
            sound: Percent::FULL,
            joypad: false,
            display: DisplaySettings::default(),
            readonly: false,
        }
    }
}

impl SaveGame {
    pub fn state(&self, slot: LevelSlot) -> LevelState {
        self.level_state.get(slot.index()).copied().unwrap_or(LevelState::Absent)
    }

    /// Playable from the menu. Registered builds also open 21 / 37 once 20 / 36 are
    /// open (interfac.wdl:103-131); the port counts as registered.
    pub fn playable(&self, slot: LevelSlot) -> bool {
        use LevelState::*;
        // A registered extra opens with the level before it.
        let extra = |s: u32| slot == LevelSlot(s) && self.state(LevelSlot(s - 1)) == Unlocked && self.state(slot) != Absent;
        self.state(slot) == Unlocked || extra(21) || extra(37)
    }

    /// `ValideLevel` (InitLvl.wdl:239-258).
    pub fn validate(&mut self, slot: LevelSlot) {
        if let Some(s) = self.level_state.get_mut(slot.index()) {
            *s = LevelState::Unlocked;
        }
    }

    /// End of a game (mm1.wdl:972-997): checkpoint (`StoreCurrentSkills`), best score,
    /// and on a win the level and the next one become playable.
    pub fn record_game(&mut self, slot: LevelSlot, score: Points, checkpoint: Option<[f32; 3]>, won: bool, next: Option<LevelSlot>) {
        let i = slot.index();
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
        self.best_scores.resize(NBR_LEVELS, Points::ZERO);
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
                s.last_levels = [t, j, k].map(LevelSlot);
            }
        }
        for (i, v) in best.array(ids::BEST_SCORES).iter().enumerate().take(NBR_LEVELS) {
            s.best_scores[i] = Points(*v as i32);
        }
        let (xs, ys, zs) = (best.array(ids::RESTART_X), best.array(ids::RESTART_Y), best.array(ids::RESTART_Z));
        for (i, &x) in xs.iter().enumerate().take(NBR_LEVELS) {
            if x != 0.0 {
                s.checkpoints[i] = Some([x, ys.get(i).copied().unwrap_or(0.0), zs.get(i).copied().unwrap_or(0.0)]);
            }
        }
        let skill = |n: &str| best.skill(n).unwrap_or(0.0);
        s.last_level = LevelSlot((skill("LevelEnCours") as u32).clamp(1, 49));
        s.previous_level = LevelSlot(skill("PreviousLevel") as u32);
        s.levels_reached = skill("NbrLevelsReach") as u32;
        s.games_started = skill("NeverPlayed") as u32;
        s.extro_seen = best.var(ids::EXTRO_VUE).unwrap_or(0.0) != 0.0;
        s.music = Percent(best.var(ids::SLIDER_MUSIC).unwrap_or(100.0));
        s.sound = Percent(best.var(ids::SLIDER_SOUND).unwrap_or(100.0));
        s.joypad = best.var(ids::JOYPAD).unwrap_or(0.0) != 0.0;
        Some(s)
    }
}

#[cfg(test)]
mod tests;
