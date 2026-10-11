//! The exit tile.

use super::*;

/// The exit tile: standing on it plays `Win_son`, and the level is won 8 ticks later.
#[derive(Clone, Debug, Default)]
pub struct ExitTile {
    pub(super) countdown: Option<f32>,
}

/// Ticks between stepping on the exit and `partieGagnee`.
pub(super) const EXIT_DELAY: f32 = 8.0;

impl ExitTile {
    /// One frame; returns the sound to play, if any.
    pub fn step(&mut self, standing_on: bool, game: &mut GameState, t: f32) -> Option<&'static str> {
        match self.countdown {
            None if standing_on && !game.won => {
                self.countdown = Some(EXIT_DELAY);
                return Some("Win_son");
            }
            Some(left) if left - t <= 0.0 => {
                game.won = true;
                self.countdown = Some(f32::INFINITY);
            }
            Some(left) => self.countdown = Some(left - t),
            None => {}
        }
        None
    }
}
