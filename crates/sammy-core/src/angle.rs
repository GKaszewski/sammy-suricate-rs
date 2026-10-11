//! Angles in degrees, as in WDL.

/// `ang()`: wraps an angle into -180..180.
pub fn ang(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests;
