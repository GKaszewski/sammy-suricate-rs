//! The exit tile.

use super::*;
use crate::time::Ticks;

/// The exit tile: standing on it plays `Win_son`, and the level is won 8 ticks later.
#[derive(Clone, Debug, Default)]
pub struct ExitTile {
    pub(super) countdown: Option<Ticks>,
}

/// Ticks between stepping on the exit and `partieGagnee`.
pub(super) const EXIT_DELAY: Ticks = Ticks(8.0);

impl ExitTile {
    /// One frame; returns the sound to play, if any.
    pub fn step(&mut self, standing_on: bool, game: &mut GameState, t: Ticks) -> Option<&'static str> {
        match self.countdown {
            None if standing_on && !game.won => {
                self.countdown = Some(EXIT_DELAY);
                return Some("Win_son");
            }
            Some(left) if left - t <= Ticks::ZERO => {
                game.won = true;
                self.countdown = Some(Ticks(f32::INFINITY));
            }
            Some(left) => self.countdown = Some(left - t),
            None => {}
        }
        None
    }
}
