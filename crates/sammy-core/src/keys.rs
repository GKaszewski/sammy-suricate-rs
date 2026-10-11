//! Colour keys: a bitmask of red (1), yellow (2) and blue (4); mixed doors and keys
//! combine them (orange 3, magenta 5, green 6, exit 7). The master key opens all.

use core::ops::BitOr;

/// A key, lock or gem colour (WDL Skill6 values).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct KeyColor(pub u32);

impl KeyColor {
    /// No key held.
    pub const NONE: Self = Self(0);
    pub const RED: Self = Self(1);
    pub const YELLOW: Self = Self(2);
    pub const ORANGE: Self = Self(3);
    pub const BLUE: Self = Self(4);
    pub const MAGENTA: Self = Self(5);
    pub const GREEN: Self = Self(6);
    /// The exit door (red + yellow + blue).
    pub const EXIT: Self = Self(7);
    /// The master key, granted on reaching the score goal.
    pub const MASTER: Self = Self(255);

    /// The WDL number.
    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// A primary colour key (red, yellow or blue).
    pub fn is_primary(self) -> bool {
        matches!(self, Self::RED | Self::YELLOW | Self::BLUE)
    }

    /// Whether this key opens a lock of colour `lock` (`movePortail` / engine locks):
    /// the master key always, `exact` locks only their own colour, others also any
    /// primary colour they contain.
    pub fn fits(self, lock: Self, exact: bool) -> bool {
        if self == Self::MASTER {
            return true;
        }
        if exact {
            return self == lock;
        }
        self.0 & lock.0 != 0 && (self == lock || self.is_primary())
    }
}

/// The WDL number.
impl core::fmt::Display for KeyColor {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// Primaries mix (red + yellow = orange ...).
impl BitOr for KeyColor {
    type Output = Self;
    fn bitor(self, o: Self) -> Self {
        Self(self.0 | o.0)
    }
}

/// Colour value of the master key, granted on reaching the score goal.
pub const MASTER_KEY: KeyColor = KeyColor::MASTER;
pub const RED: KeyColor = KeyColor::RED;
pub const YELLOW: KeyColor = KeyColor::YELLOW;
pub const BLUE: KeyColor = KeyColor::BLUE;

/// What happens when Sammy picks up a key gem (`Key_Hit`, gems.wdl:212-350).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyOutcome {
    Taken,
    /// Two different primaries combined (red+yellow=orange, red+blue=magenta, yellow+blue=green).
    Mixed,
    /// The previously held key is thrown back out.
    Ejected(KeyColor),
}

/// The key held after picking up `gem` while holding `held`.
pub fn take_key(held: KeyColor, gem: KeyColor) -> (KeyColor, KeyOutcome) {
    match held {
        KeyColor::NONE => (gem, KeyOutcome::Taken),
        _ if held.is_primary() && gem.is_primary() && held != gem => (held | gem, KeyOutcome::Mixed),
        _ => (gem, KeyOutcome::Ejected(held)),
    }
}

/// Sprite of a key gem colour (17-frame spinning gems).
pub fn gem_sprite(color: KeyColor) -> Option<&'static str> {
    Some(match color {
        KeyColor::RED => "gemre+17.pcx",
        KeyColor::YELLOW => "gemye+17.pcx",
        KeyColor::ORANGE => "gemor+17.pcx",
        KeyColor::BLUE => "gembl+17.pcx",
        KeyColor::MAGENTA => "gemma+17.pcx",
        KeyColor::GREEN => "gemve+17.pcx",
        KeyColor::MASTER => "gemmu+17.pcx",
        _ => return None,
    })
}

/// Full-strength light colour channel of a key light.
const LIGHT_FULL: f32 = 250.0;
/// Orange key light green channel.
const LIGHT_ORANGE_G: f32 = 100.0;

/// Light colour (0..255) of a key colour (`GemLight` / `DoorLight`, gems.wdl:454-477).
pub fn light_color(key: KeyColor) -> [f32; 3] {
    match key {
        KeyColor::RED => [LIGHT_FULL, 0.0, 0.0],
        KeyColor::YELLOW => [LIGHT_FULL, LIGHT_FULL, 0.0],
        KeyColor::ORANGE => [LIGHT_FULL, LIGHT_ORANGE_G, 0.0],
        KeyColor::BLUE => [0.0, 0.0, LIGHT_FULL],
        KeyColor::MAGENTA => [LIGHT_FULL, 0.0, LIGHT_FULL],
        KeyColor::GREEN => [0.0, LIGHT_FULL, 0.0],
        _ => [LIGHT_FULL, LIGHT_FULL, LIGHT_FULL],
    }
}

#[cfg(test)]
mod tests;
