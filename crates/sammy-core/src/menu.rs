//! Main menu rules (`MainMenuSammy`, interfac.wdl:23-624; docs/menu.md §3): what each
//! item does, how the 3D signs bob and spin, and how level icons look from the save.

use formats::wmb::Object;

use crate::levels::file_for;
use crate::save::{LevelState, SaveGame};

/// A level grid of the menu (`Pos*Levels` markers) or the root view.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum View {
    #[default]
    Root,
    Tutorial,
    Jump,
    Think,
    Bonus,
}

impl View {
    /// The `ChoixMenu` code of the grid (0 for the root).
    pub fn code(self) -> u32 {
        match self {
            View::Root => 0,
            View::Tutorial => 2,
            View::Jump => 3,
            View::Think => 4,
            View::Bonus => 50,
        }
    }

    pub fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => View::Root,
            2 => View::Tutorial,
            3 => View::Jump,
            4 => View::Think,
            50 => View::Bonus,
            _ => return None,
        })
    }
}

/// A menu item's Skill1 (`ChoixMenu`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    /// 0: decoration, not clickable.
    Decor,
    /// 1: "Ostatnio grany poziom": resume the last level.
    Play,
    /// 2-4, 50: open a level grid.
    Grid(View),
    /// 5.
    Options,
    /// 6.
    Quit,
    /// 20: back to the root view.
    Back,
    /// 100 + N: level icon.
    Level(u32),
}

/// Level icons are `LEVEL_ICON + slot`.
pub const LEVEL_ICON: u32 = 100;

impl Choice {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Choice::Play,
            5 => Choice::Options,
            6 => Choice::Quit,
            20 => Choice::Back,
            c if c > LEVEL_ICON => Choice::Level(c - LEVEL_ICON),
            c => View::from_code(c).filter(|v| *v != View::Root).map_or(Choice::Decor, Choice::Grid),
        }
    }
}

/// What the frontend must do for a choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    /// Load this level (the save was updated: write it).
    StartLevel(&'static str),
    ShowView(View),
    OpenOptions,
    /// Ask "quit?".
    Quit,
    Nothing,
}

/// `MainMenuSammy`'s `ChoixMenu` handling.
pub fn act(choice: Choice, save: &mut SaveGame) -> MenuAction {
    match choice {
        Choice::Play => start_level(save, save.last_level),
        Choice::Grid(v) => MenuAction::ShowView(v),
        Choice::Back => MenuAction::ShowView(View::Root),
        Choice::Options => MenuAction::OpenOptions,
        Choice::Quit => MenuAction::Quit,
        Choice::Level(slot) => start_level(save, slot),
        Choice::Decor => MenuAction::Nothing,
    }
}

/// `GestionPartie2` (mm1.wdl:531-540): `NeverPlayed += 1`, then `DebutPartie`.
pub fn start_level(save: &mut SaveGame, slot: u32) -> MenuAction {
    let Some(file) = file_for(slot) else { return MenuAction::Nothing };
    save.games_started += 1;
    save.last_level = slot;
    MenuAction::StartLevel(file)
}

/// How an item looks given the save (interfac.wdl:40-175).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Look {
    /// Not shown (and not clickable).
    Hidden,
    /// Shown at this opacity (percent).
    Shown { alpha: f32 },
    /// A level that cannot be played yet: a 50 % ghost, darkened if it does not exist.
    Locked { alpha: f32, dark: bool },
}

/// Opacity of locked level icons.
const LOCKED_ALPHA: f32 = 50.0;

/// Slots that have an icon in the three worlds' grids.
fn in_world(slot: u32) -> bool {
    matches!(slot, 1..=12 | 17..=28 | 33..=44)
}

/// `InitMainMenu`'s visibility rules: Play only once a level was ever started, Bonus
/// only when unlocked, absent bonus slots vanish, locked levels are ghosts.
pub fn look(choice: Choice, base_alpha: f32, save: &SaveGame) -> Look {
    let hidden = match choice {
        Choice::Play => save.games_started == 0,
        Choice::Grid(View::Bonus) => save.bonus_state == 0 || (save.bonus_state == 1 && !save.extro_seen),
        Choice::Level(n) => save.state(n) == LevelState::Absent && !in_world(n),
        _ => false,
    };
    if hidden {
        return Look::Hidden;
    }
    match choice {
        Choice::Level(n) if !save.playable(n) => Look::Locked { alpha: LOCKED_ALPHA, dark: save.state(n) != LevelState::Locked },
        _ => Look::Shown { alpha: base_alpha },
    }
}

/// Bob and spin of an `InitMainMenu` / `InitMainTitle` item (interfac.wdl:74-198,
/// :977-1014): it eases toward ±Skill7 around its origin, flipping when close, and
/// spins by Skill2-4 per tick.
#[derive(Clone, Debug)]
pub struct Bob {
    org: [f32; 3],
    pub angles: [f32; 3],
    spin: [f32; 3],
    /// int(Skill7): amplitude; frac(Skill7): easing per tick.
    hover: f32,
    damping: f32,
    z: f32,
    target: f32,
    dir: f32,
    /// `InitMainTitle` flips at 8 quants instead of 4.
    title: bool,
}

/// Flip distance of menu items and of titles.
const FLIP: f32 = 4.0;
const TITLE_FLIP: f32 = 8.0;
/// Flag3 items are 80 % opaque.
const FLAG3_ALPHA: f32 = 80.0;

impl Bob {
    pub fn from_object(obj: &Object, title: bool) -> Self {
        let s7 = obj.skill(7);
        Self {
            org: obj.origin,
            angles: obj.angles,
            spin: [obj.skill(2), obj.skill(3), obj.skill(4)],
            hover: s7.trunc(),
            damping: s7 - s7.trunc(),
            z: obj.origin[2],
            target: obj.origin[2] + s7.trunc(),
            dir: 1.0,
            title,
        }
    }

    /// Items with Skill7 = 0 (Quit, Back, icons) stay still.
    pub fn moves(&self) -> bool {
        self.hover != 0.0
    }

    /// One frame; returns the position.
    pub fn step(&mut self, t: f32) -> [f32; 3] {
        self.z += (self.target - self.z) * self.damping * t;
        for i in 0..3 {
            self.angles[i] += self.spin[i] * t;
        }
        let threshold = if self.title { TITLE_FLIP } else { FLIP };
        if (self.z - self.target).abs() < threshold {
            self.dir = -self.dir;
            self.target = self.org[2] + self.hover * self.dir;
        }
        [self.org[0], self.org[1], self.z]
    }
}

/// An item's own opacity (Flag3 = 80 %).
pub fn base_alpha(obj: &Object) -> f32 {
    if obj.flag(3) { FLAG3_ALPHA } else { 100.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        assert_eq!(Choice::from_code(1), Choice::Play);
        assert_eq!(Choice::from_code(3), Choice::Grid(View::Jump));
        assert_eq!(Choice::from_code(105), Choice::Level(5));
        assert_eq!(Choice::from_code(0), Choice::Decor);
    }

    #[test]
    fn locked_icons_are_ghosts() {
        let save = SaveGame::default();
        assert_eq!(look(Choice::Level(1), 100.0, &save), Look::Shown { alpha: 100.0 });
        assert_eq!(look(Choice::Level(2), 100.0, &save), Look::Locked { alpha: 50.0, dark: false });
        assert_eq!(look(Choice::Play, 100.0, &save), Look::Hidden);
    }
}
