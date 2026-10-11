//! Acknex time: game logic advances in ticks of 1/16 s; a frame advances by a
//! fractional number of ticks (the WDL `time` variable).

/// Ticks per second.
pub const TICKS_PER_SEC: f32 = 16.0;

/// Converts seconds to ticks.
pub fn ticks(seconds: f32) -> f32 {
    seconds * TICKS_PER_SEC
}
