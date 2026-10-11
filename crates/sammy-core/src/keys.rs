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

/// What happens when Sammy picks up a key gem (`Key_Hit`, gems.wdl:212-350).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyOutcome {
    Taken,
    /// Two different primaries combined (red+yellow=orange, red+blue=magenta, yellow+blue=green).
    Mixed,
    /// The previously held key is thrown back out.
    Ejected(u32),
}

/// The key held after picking up `gem` while holding `held`.
pub fn take_key(held: u32, gem: u32) -> (u32, KeyOutcome) {
    match held {
        0 => (gem, KeyOutcome::Taken),
        _ if is_primary(held) && is_primary(gem) && held != gem => (held | gem, KeyOutcome::Mixed),
        _ => (gem, KeyOutcome::Ejected(held)),
    }
}

/// Sprite of a key gem colour (17-frame spinning gems).
pub fn gem_sprite(color: u32) -> Option<&'static str> {
    Some(match color {
        1 => "gemre+17.pcx",
        2 => "gemye+17.pcx",
        3 => "gemor+17.pcx",
        4 => "gembl+17.pcx",
        5 => "gemma+17.pcx",
        6 => "gemve+17.pcx",
        MASTER_KEY => "gemmu+17.pcx",
        _ => return None,
    })
}

/// Full-strength light colour channel of a key light.
const LIGHT_FULL: f32 = 250.0;
/// Orange key light green channel.
const LIGHT_ORANGE_G: f32 = 100.0;

/// Light colour (0..255) of a key colour (`GemLight` / `DoorLight`, gems.wdl:454-477).
pub fn light_color(key: u32) -> [f32; 3] {
    match key {
        1 => [LIGHT_FULL, 0.0, 0.0],
        2 => [LIGHT_FULL, LIGHT_FULL, 0.0],
        3 => [LIGHT_FULL, LIGHT_ORANGE_G, 0.0],
        4 => [0.0, 0.0, LIGHT_FULL],
        5 => [LIGHT_FULL, 0.0, LIGHT_FULL],
        6 => [0.0, LIGHT_FULL, 0.0],
        _ => [LIGHT_FULL, LIGHT_FULL, LIGHT_FULL],
    }
}

#[cfg(test)]
mod tests;
