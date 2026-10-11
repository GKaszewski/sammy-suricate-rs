//! Angles in degrees, as in WDL: [`Degrees`], and an entity's orientation
//! ([`Angles`]: pan about Z counter-clockwise from +X, tilt nose up, roll).

use core::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// `ang()`: wraps an angle into -180..180.
pub fn ang(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

/// The unit vector of a heading: `pan` and `tilt` in degrees, native axes.
pub fn forward(pan: f32, tilt: f32) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

/// An angle in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl Degrees {
    pub const ZERO: Self = Self(0.0);

    /// The plain number of degrees.
    pub const fn get(self) -> f32 {
        self.0
    }

    pub fn radians(self) -> f32 {
        self.0.to_radians()
    }

    /// Wrapped into -180..180 ([`ang`]).
    pub fn wrapped(self) -> Self {
        Self(ang(self.0))
    }
}

impl Add for Degrees {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0)
    }
}

impl Sub for Degrees {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self(self.0 - o.0)
    }
}

impl AddAssign for Degrees {
    fn add_assign(&mut self, o: Self) {
        self.0 += o.0;
    }
}

impl SubAssign for Degrees {
    fn sub_assign(&mut self, o: Self) {
        self.0 -= o.0;
    }
}

impl Neg for Degrees {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Mul<f32> for Degrees {
    type Output = Self;
    fn mul(self, k: f32) -> Self {
        Self(self.0 * k)
    }
}

/// An entity's orientation (WDL `pan`, `tilt`, `roll`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Angles {
    pub pan: Degrees,
    pub tilt: Degrees,
    pub roll: Degrees,
}

impl Angles {
    pub const ZERO: Self = Self { pan: Degrees::ZERO, tilt: Degrees::ZERO, roll: Degrees::ZERO };

    pub const fn new(pan: f32, tilt: f32, roll: f32) -> Self {
        Self { pan: Degrees(pan), tilt: Degrees(tilt), roll: Degrees(roll) }
    }

    /// Facing `pan`, level.
    pub const fn pan(pan: f32) -> Self {
        Self::new(pan, 0.0, 0.0)
    }

    /// As stored in level files and WDL vectors: `[pan, tilt, roll]`.
    pub const fn from_array([pan, tilt, roll]: [f32; 3]) -> Self {
        Self::new(pan, tilt, roll)
    }

    pub const fn to_array(self) -> [f32; 3] {
        [self.pan.0, self.tilt.0, self.roll.0]
    }

    /// Turns each axis by `by` degrees (pan, tilt, roll).
    pub fn turn(&mut self, by: [f32; 3]) {
        self.pan.0 += by[0];
        self.tilt.0 += by[1];
        self.roll.0 += by[2];
    }

    /// The facing direction.
    pub fn forward(self) -> [f32; 3] {
        forward(self.pan.0, self.tilt.0)
    }
}

impl From<[f32; 3]> for Angles {
    fn from(a: [f32; 3]) -> Self {
        Self::from_array(a)
    }
}

#[cfg(test)]
mod tests;
