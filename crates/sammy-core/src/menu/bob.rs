//! Menu items bobbing and flipping.

use super::*;

/// Bob and spin of an `InitMainMenu` / `InitMainTitle` item (interfac.wdl:74-198,
/// :977-1014): it eases toward ±Skill7 around its origin, flipping when close, and
/// spins by Skill2-4 per tick.
#[derive(Clone, Debug)]
pub struct Bob {
    pub(super) org: [f32; 3],
    pub angles: [f32; 3],
    pub(super) spin: [f32; 3],
    /// int(Skill7): amplitude; frac(Skill7): easing per tick.
    pub(super) hover: f32,
    pub(super) damping: f32,
    pub(super) z: f32,
    pub(super) target: f32,
    pub(super) dir: f32,
    /// `InitMainTitle` flips at 8 quants instead of 4.
    pub(super) title: bool,
}

/// Flip distance of menu items and of titles.
pub(super) const FLIP: f32 = 4.0;
pub(super) const TITLE_FLIP: f32 = 8.0;
/// Flag3 items are 80 % opaque.
pub(super) const FLAG3_ALPHA: f32 = 80.0;

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
