//! Score points (`ScoreJoueur1`, `ScoreGoal`, best scores).

use core::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

/// A number of points.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Points(pub i32);

impl Points {
    pub const ZERO: Self = Self(0);

    /// The plain number.
    pub const fn get(self) -> i32 {
        self.0
    }
}

impl Add for Points {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0)
    }
}

impl AddAssign for Points {
    fn add_assign(&mut self, o: Self) {
        self.0 += o.0;
    }
}

impl core::fmt::Display for Points {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.fmt(f)
    }
}
