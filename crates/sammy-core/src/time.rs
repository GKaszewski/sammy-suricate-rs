//! Acknex time: game logic advances in ticks of 1/16 s; a frame advances by a
//! fractional number of ticks (the WDL `time` variable).

/// Ticks per second.
pub const TICKS_PER_SEC: f32 = 16.0;

/// Converts seconds to ticks.
pub fn ticks(seconds: f32) -> f32 {
    seconds * TICKS_PER_SEC
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

    pub fn advance(&mut self, t: f32) {
        self.acc += t;
        self.ticks = (self.acc.floor() as u32).min(Self::MAX_TICKS);
        self.acc -= self.ticks as f32;
        self.acc = self.acc.min(1.0);
    }
}
