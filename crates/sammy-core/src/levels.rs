//! The level table (Loads.wdl, docs/gameplay.md §3.1).

/// `InitLevelN` → WMB (Loads.wdl; docs/gameplay.md §3.1). Bonus slots 13-16,
/// 29-32 and 45-48 point at files that never shipped and are skipped.
pub const LEVELS: &[(u32, &str)] = &[
    (1, "10001.wmb"),
    (2, "10002.wmb"),
    (3, "10003.wmb"),
    (4, "10004.wmb"),
    (5, "10005.wmb"),
    (6, "10006.wmb"),
    (7, "10007.wmb"),
    (8, "10008.wmb"),
    (9, "10009.wmb"),
    (10, "10012.wmb"),
    (11, "10010.wmb"),
    (12, "10011.wmb"),
    (17, "101o_j.wmb"),
    (18, "104M_j.wmb"),
    (19, "103J_j.wmb"),
    (20, "112e_j.wmb"),
    (21, "107i_j.wmb"),
    (22, "174a_j07.wmb"),
    (23, "111r_j.wmb"),
    (24, "172c_j06.wmb"),
    (25, "175c_t.wmb"),
    (26, "177e_j.wmb"),
    (27, "108i_j.wmb"),
    (28, "179f_t.wmb"),
    (33, "106g_t.wmb"),
    (34, "171c_t04.wmb"),
    (35, "110n_t.wmb"),
    (36, "180c_t.wmb"),
    (37, "170b_t00.wmb"),
    (38, "102N_t.wmb"),
    (39, "173b_t05.wmb"),
    (40, "114t_t.wmb"),
    (41, "178d_t.wmb"),
    (42, "176g_t.wmb"),
    (43, "113q_t.wmb"),
    (44, "109o_t.wmb"),
    (49, "extro.wmb"),
];

pub fn file_for(slot: u32) -> Option<&'static str> {
    LEVELS.iter().find(|(s, _)| *s == slot).map(|(_, f)| *f)
}

pub fn slot_for(file: &str) -> Option<u32> {
    LEVELS.iter().find(|(_, f)| f.eq_ignore_ascii_case(file)).map(|(s, _)| *s)
}

/// The extro's slot (`LevelEnCours = 49`).
pub const EXTRO_SLOT: u32 = 49;

/// The main menu scene.
pub const MENU_FILE: &str = "menu_greenpepper.wmb";

/// Last level of the tutorial, jump and think worlds (`NumberLast*LevelOrg`).
pub const LAST_OF_WORLD: [u32; 3] = [12, 28, 44];

/// First level of each world (open on a fresh install).
pub const FIRST_OF_WORLD: [u32; 3] = [1, 17, 33];

/// Slots of the save arrays (`LevelsInfo[0..50]`).
pub const NBR_LEVELS: usize = 50;

/// `SearchNextLevel`: the next existing slot (worlds chain into each other, 44 leads
/// to the extro, the extro back to 1).
pub fn next_slot(slot: u32) -> u32 {
    match slot {
        12 => FIRST_OF_WORLD[1],
        28 => FIRST_OF_WORLD[2],
        44 => EXTRO_SLOT,
        EXTRO_SLOT => FIRST_OF_WORLD[0],
        s => LEVELS.iter().map(|(n, _)| *n).find(|&n| n > s).unwrap_or(1),
    }
}

#[cfg(test)]
mod tests;
