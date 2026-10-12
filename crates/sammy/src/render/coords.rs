//! Acknex space (X forward, Y left, Z up, right-handed, units = quants) to Bevy
//! space (X right, Y up, -Z forward). Proper rotation, so winding is preserved.

use bevy::prelude::*;

/// World scale: quants per Bevy unit. Sammy is ~50 quants tall.
pub const QUANTS_PER_UNIT: f32 = 32.0;

pub fn pos(p: impl Into<sammy_core::space::Pos>) -> Vec3 {
    let [x, y, z] = p.into().get();
    Vec3::new(-y, z, -x) / QUANTS_PER_UNIT
}

/// Inverse of [`pos`]: Bevy position back to native Acknex quants.
pub fn native(v: Vec3) -> [f32; 3] {
    [-v.z * QUANTS_PER_UNIT, -v.x * QUANTS_PER_UNIT, v.y * QUANTS_PER_UNIT]
}

/// A native direction (unit length kept) in Bevy space.
pub fn dir([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(-y, z, -x)
}

/// Inverse of [`dir`]: a Bevy direction in native axes, unscaled (not a position).
pub fn native_dir(v: Vec3) -> [f32; 3] {
    [-v.z, -v.x, v.y]
}

/// Acknex euler angles in degrees: pan (around Z, CCW from above), tilt (pitch up),
/// roll. Applied pan, then tilt, then roll.
pub fn rotation(angles: impl Into<sammy_core::angle::Angles>) -> Quat {
    let [pan, tilt, roll] = angles.into().to_array();
    // In Bevy space: Acknex Z (up) = +Y, Acknex Y (left) = -X, Acknex X (fwd) = -Z.
    Quat::from_rotation_y(pan.to_radians())
        * Quat::from_rotation_x(tilt.to_radians())
        * Quat::from_rotation_z(-roll.to_radians())
}
