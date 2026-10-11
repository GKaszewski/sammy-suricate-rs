//! Colour keys: a bitmask of red (1), yellow (2) and blue (4); mixed doors and keys
//! combine them (orange 3, magenta 5, green 6, exit 7). The master key opens all.

/// Colour value of the master key, granted on reaching the score goal.
pub const MASTER_KEY: u32 = 255;
pub const RED: u32 = 1;
pub const YELLOW: u32 = 2;
pub const BLUE: u32 = 4;

/// A primary colour key (red, yellow or blue).
pub fn is_primary(key: u32) -> bool {
    matches!(key, RED | YELLOW | BLUE)
}

/// Whether `key` opens a lock of colour `lock` (`movePortail` / engine locks): the
/// master key always, `exact` locks only their own colour, others also any primary
/// colour they contain.
pub fn fits(key: u32, lock: u32, exact: bool) -> bool {
    if key == MASTER_KEY {
        return true;
    }
    if exact {
        return key == lock;
    }
    key & lock != 0 && (key == lock || is_primary(key))
}
