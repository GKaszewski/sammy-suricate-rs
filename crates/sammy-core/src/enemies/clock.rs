//! The AI's fixed 16 Hz clock.

/// 16 Hz clock for the enemy AI.
#[derive(Clone, Debug, Default)]
pub struct AiClock {
    pub acc: f32,
    /// Ticks to run this frame.
    pub ticks: u32,
    /// `ticks1`: frame counter wrapping every 64 (mm1.wdl:893-903).
    pub ticks1: u32,
}

impl AiClock {
    /// Enemies wait for `PartieEncours == 1` (lion.wdl:143-148); at most 4 ticks per frame.
    pub fn advance(&mut self, t: f32, playing: bool) {
        if !playing {
            self.ticks = 0;
            return;
        }
        self.acc += t;
        self.ticks = (self.acc.floor() as u32).min(4);
        self.acc -= self.ticks as f32;
        self.acc = self.acc.min(1.0);
    }

    /// After the frame's ticks: `ticks1` wraps every 64.
    pub fn bump(&mut self) {
        self.ticks1 = (self.ticks1 + self.ticks) % 64;
    }
}
