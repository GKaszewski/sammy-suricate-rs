//! Menu choices and what they do.

use super::*;

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
    Level(LevelSlot),
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
            c if c > LEVEL_ICON => Choice::Level(LevelSlot(c - LEVEL_ICON)),
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
pub fn start_level(save: &mut SaveGame, slot: LevelSlot) -> MenuAction {
    let Some(file) = file_for(slot) else { return MenuAction::Nothing };
    save.games_started += 1;
    save.last_level = slot;
    MenuAction::StartLevel(file)
}
