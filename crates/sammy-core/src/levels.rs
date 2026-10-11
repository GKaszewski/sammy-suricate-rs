//! The level table (Loads.wdl, docs/gameplay.md §3.1).

use serde::{Deserialize, Serialize};

/// A level's slot (`LevelEnCours`, 1..49): its index in the save arrays and the menu.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LevelSlot(pub u32);

impl LevelSlot {
    /// Index into the save arrays.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The level file, if the slot has one.
    pub fn file(self) -> Option<&'static str> {
        file_for(self)
    }
}

impl core::fmt::Display for LevelSlot {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// `InitLevelN` → WMB (Loads.wdl; docs/gameplay.md §3.1). Bonus slots 13-16,
/// 29-32 and 45-48 point at files that never shipped and are skipped.
pub const LEVELS: &[(LevelSlot, &str)] = &[
    (LevelSlot(1), "10001.wmb"),
    (LevelSlot(2), "10002.wmb"),
    (LevelSlot(3), "10003.wmb"),
    (LevelSlot(4), "10004.wmb"),
    (LevelSlot(5), "10005.wmb"),
    (LevelSlot(6), "10006.wmb"),
    (LevelSlot(7), "10007.wmb"),
    (LevelSlot(8), "10008.wmb"),
    (LevelSlot(9), "10009.wmb"),
    (LevelSlot(10), "10012.wmb"),
    (LevelSlot(11), "10010.wmb"),
    (LevelSlot(12), "10011.wmb"),
    (LevelSlot(17), "101o_j.wmb"),
    (LevelSlot(18), "104M_j.wmb"),
    (LevelSlot(19), "103J_j.wmb"),
    (LevelSlot(20), "112e_j.wmb"),
    (LevelSlot(21), "107i_j.wmb"),
    (LevelSlot(22), "174a_j07.wmb"),
    (LevelSlot(23), "111r_j.wmb"),
    (LevelSlot(24), "172c_j06.wmb"),
    (LevelSlot(25), "175c_t.wmb"),
    (LevelSlot(26), "177e_j.wmb"),
    (LevelSlot(27), "108i_j.wmb"),
    (LevelSlot(28), "179f_t.wmb"),
    (LevelSlot(33), "106g_t.wmb"),
    (LevelSlot(34), "171c_t04.wmb"),
    (LevelSlot(35), "110n_t.wmb"),
    (LevelSlot(36), "180c_t.wmb"),
    (LevelSlot(37), "170b_t00.wmb"),
    (LevelSlot(38), "102N_t.wmb"),
    (LevelSlot(39), "173b_t05.wmb"),
    (LevelSlot(40), "114t_t.wmb"),
    (LevelSlot(41), "178d_t.wmb"),
    (LevelSlot(42), "176g_t.wmb"),
    (LevelSlot(43), "113q_t.wmb"),
    (LevelSlot(44), "109o_t.wmb"),
    (LevelSlot(49), "extro.wmb"),
];

pub fn file_for(slot: LevelSlot) -> Option<&'static str> {
    LEVELS.iter().find(|(s, _)| *s == slot).map(|(_, f)| *f)
}

pub fn slot_for(file: &str) -> Option<LevelSlot> {
    LEVELS.iter().find(|(_, f)| f.eq_ignore_ascii_case(file)).map(|(s, _)| *s)
}

/// The extro's slot (`LevelEnCours = 49`).
pub const EXTRO_SLOT: LevelSlot = LevelSlot(49);

/// The main menu scene.
pub const MENU_FILE: &str = "menu_greenpepper.wmb";

/// Last level of the tutorial, jump and think worlds (`NumberLast*LevelOrg`).
pub const LAST_OF_WORLD: [LevelSlot; 3] = [LevelSlot(12), LevelSlot(28), LevelSlot(44)];

/// First level of each world (open on a fresh install).
pub const FIRST_OF_WORLD: [LevelSlot; 3] = [LevelSlot(1), LevelSlot(17), LevelSlot(33)];

/// Slots of the save arrays (`LevelsInfo[0..50]`).
pub const NBR_LEVELS: usize = 50;

/// `SearchNextLevel`: the next existing slot (worlds chain into each other, 44 leads
/// to the extro, the extro back to 1).
pub fn next_slot(slot: LevelSlot) -> LevelSlot {
    match slot {
        LevelSlot(12) => FIRST_OF_WORLD[1],
        LevelSlot(28) => FIRST_OF_WORLD[2],
        LevelSlot(44) => EXTRO_SLOT,
        EXTRO_SLOT => FIRST_OF_WORLD[0],
        s => LEVELS.iter().map(|(n, _)| *n).find(|&n| n > s).unwrap_or(FIRST_OF_WORLD[0]),
    }
}

#[cfg(test)]
mod tests;
