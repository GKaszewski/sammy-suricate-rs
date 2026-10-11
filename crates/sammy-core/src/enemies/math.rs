//! Angle and vector helpers of the AI (f32, as the original).

/// f32 `frc()` (§1.1: unpack with f32 maths to reproduce the shipped data).
pub(super) fn frc(x: f32) -> f32 {
    x - x.trunc()
}

pub(super) fn ang(a: f32) -> f32 {
    let a = (a + 180.0).rem_euclid(360.0) - 180.0;
    if a == -180.0 { 180.0 } else { a }
}

/// `To_Angle`: (pan, tilt, length) of a vector.
pub(super) fn to_angle(v: [f32; 3]) -> (f32, f32, f32) {
    let h = (v[0] * v[0] + v[1] * v[1]).sqrt();
    (v[1].atan2(v[0]).to_degrees(), v[2].atan2(h).to_degrees(), (h * h + v[2] * v[2]).sqrt())
}

pub(super) use crate::angle::forward;

/// Scan cone test (§2.5): within `range`, ±h/2 of `pan`, ±v/2 of `tilt`.
pub(super) fn in_cone(from: [f32; 3], pan: f32, tilt: f32, h: f32, v: f32, range: f32, p: [f32; 3]) -> bool {
    let (dp, dt, d) = to_angle([p[0] - from[0], p[1] - from[1], p[2] - from[2]]);
    d <= range && ang(dp - pan).abs() <= h / 2.0 && ang(dt - tilt).abs() <= v / 2.0
}
