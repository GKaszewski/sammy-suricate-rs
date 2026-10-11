//! How a menu item looks for the save's progress.

use super::*;

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
pub(super) const LOCKED_ALPHA: f32 = 50.0;

/// Slots that have an icon in the three worlds' grids.
pub(super) fn in_world(slot: u32) -> bool {
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
