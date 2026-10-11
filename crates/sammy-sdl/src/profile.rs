//! The frame-time profile in the log.

use crate::scene;

/// The profile is logged every this many frames.
pub const LOG_FRAMES: u64 = 60;

/// Where the frame time goes, logged every [`LOG_FRAMES`] frames: SDL events and input,
/// the game tick, the music's share of real time (audio thread), scene
/// sync (animated meshes), draw submission, and the buffer swap (waiting on the GPU
/// and vsync), with the scene's counts.
#[derive(Default)]
pub struct Profile {
    pub(super) tick: f64,
    pub(super) sync: f64,
    pub(super) draw: f64,
    pub(super) swap: f64,
    pub(super) input: f64,
    pub(super) since: f64,
}

impl Profile {
    pub fn log(&mut self, frames: u64, now: f64, s: scene::Stats, music: f64) {
        let n = LOG_FRAMES as f64;
        let ms = |v: f64| v / n * 1000.0;
        let fps = n / (now - self.since).max(1e-6);
        log::info!(
            "frame {frames} at {now:.1} s: {fps:.0} fps | ms tick {:.1} sync {:.1} draw {:.1} swap {:.1} input {:.1} | music {:.1}% | drawn {} culled {} animated {} lights {}",
            ms(self.tick),
            ms(self.sync),
            ms(self.draw),
            ms(self.swap),
            ms(self.input),
            music * 100.0,
            s.drawn,
            s.culled,
            s.animated,
            s.lights
        );
        *self = Self { since: now, ..Self::default() };
    }
}
