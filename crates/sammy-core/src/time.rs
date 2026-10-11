//! Acknex time: game logic advances in ticks of 1/16 s; a frame advances by a
//! fractional number of ticks (the WDL `time` variable). [`Ticks`] and [`Secs`] keep the
//! two units apart.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Ticks per second.
pub const TICKS_PER_SEC: f32 = 16.0;

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(pub f32);

        impl $name {
            pub const ZERO: Self = Self(0.0);

            /// The plain number.
            pub const fn get(self) -> f32 {
                self.0
            }

            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0))
            }

            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, o: Self) -> Self {
                Self(self.0 + o.0)
            }
        }

        impl Sub for $name {
            type Output = Self;
            fn sub(self, o: Self) -> Self {
                Self(self.0 - o.0)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, o: Self) {
                self.0 += o.0;
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, o: Self) {
                self.0 -= o.0;
            }
        }

        impl Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        /// Scaling a duration.
        impl Mul<f32> for $name {
            type Output = Self;
            fn mul(self, k: f32) -> Self {
                Self(self.0 * k)
            }
        }

        /// A rate per unit over a duration: the amount.
        impl Mul<$name> for f32 {
            type Output = f32;
            fn mul(self, d: $name) -> f32 {
                self * d.0
            }
        }

        /// Splitting a duration.
        impl Div<f32> for $name {
            type Output = Self;
            fn div(self, k: f32) -> Self {
                Self(self.0 / k)
            }
        }

        /// How many of one duration fit in another.
        impl Div for $name {
            type Output = f32;
            fn div(self, o: Self) -> f32 {
                self.0 / o.0
            }
        }
    };
}

unit! {
    /// A duration in A4 ticks (1/16 s), the unit of the game logic.
    Ticks
}

unit! {
    /// A duration in seconds, the unit of frames and animation clocks.
    Secs
}

impl Ticks {
    pub fn secs(self) -> Secs {
        Secs(self.0 / TICKS_PER_SEC)
    }
}

impl Secs {
    pub fn ticks(self) -> Ticks {
        Ticks(self.0 * TICKS_PER_SEC)
    }
}

/// A 16 Hz logic clock driven by fractional frame ticks: how many whole ticks to run
/// this frame (at most [`TickClock::MAX_TICKS`]), the rest carried over (capped at one).
#[derive(Clone, Debug, Default)]
pub struct TickClock {
    /// Fraction of a tick carried over (also the render interpolation factor).
    pub acc: f32,
    /// Ticks to run this frame.
    pub ticks: u32,
}

impl TickClock {
    pub const MAX_TICKS: u32 = 4;

    pub fn advance(&mut self, t: Ticks) {
        self.acc += t.0;
        self.ticks = (self.acc.floor() as u32).min(Self::MAX_TICKS);
        self.acc -= self.ticks as f32;
        self.acc = self.acc.min(1.0);
    }
}
