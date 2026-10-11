//! Native A4 space: positions in quants, X forward, Y left, Z up.

use core::ops::{Index, IndexMut};

use crate::angle::Degrees;

/// A point in the level, in quants (native axes: X forward, Y left, Z up). Frontends
/// convert it to their own space at their boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos(pub [f32; 3]);

impl Pos {
    pub const ORIGIN: Self = Self([0.0; 3]);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self([x, y, z])
    }

    pub const fn to_array(self) -> [f32; 3] {
        self.0
    }

    /// The plain coordinates (as [`Pos::to_array`]).
    pub const fn get(self) -> [f32; 3] {
        self.0
    }
}

/// Where and facing which way Sammy (re)appears: a spawn point or a checkpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub pos: Pos,
    pub pan: Degrees,
}

impl Placement {
    pub const fn new(pos: [f32; 3], pan: f32) -> Self {
        Self { pos: Pos(pos), pan: Degrees(pan) }
    }
}

impl From<[f32; 3]> for Pos {
    fn from(p: [f32; 3]) -> Self {
        Self(p)
    }
}

impl From<Pos> for [f32; 3] {
    fn from(p: Pos) -> Self {
        p.0
    }
}

/// `pos[0..3]`: x, y, z.
impl Index<usize> for Pos {
    type Output = f32;
    fn index(&self, i: usize) -> &f32 {
        &self.0[i]
    }
}

impl IndexMut<usize> for Pos {
    fn index_mut(&mut self, i: usize) -> &mut f32 {
        &mut self.0[i]
    }
}
