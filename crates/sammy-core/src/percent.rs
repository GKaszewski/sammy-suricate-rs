//! WDL percentages: opacity (`alpha`), volumes and fog run 0..100.

use core::ops::{Add, AddAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// A value in percent, 100 = full.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Percent(pub f32);

impl Percent {
    pub const ZERO: Self = Self(0.0);
    pub const FULL: Self = Self(100.0);

    /// The plain number (0..100).
    pub const fn get(self) -> f32 {
        self.0
    }

    /// As a fraction (0..1).
    pub fn fraction(self) -> f32 {
        self.0 / Self::FULL.0
    }

    /// Kept within 0..100.
    pub fn clamped(self) -> Self {
        Self(self.0.clamp(Self::ZERO.0, Self::FULL.0))
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Add for Percent {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0)
    }
}

impl Sub for Percent {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self(self.0 - o.0)
    }
}

impl AddAssign for Percent {
    fn add_assign(&mut self, o: Self) {
        self.0 += o.0;
    }
}

impl SubAssign for Percent {
    fn sub_assign(&mut self, o: Self) {
        self.0 -= o.0;
    }
}
