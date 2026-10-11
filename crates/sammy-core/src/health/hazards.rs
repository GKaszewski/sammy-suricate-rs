//! Which death starts: death tiles and running out of health (lava).

use super::*;
use crate::time::Ticks;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DeathKind {
    /// Deadly tile: lose a life, fade out, respawn (no death animation).
    Tile,
    /// Out of health: death animation, lose a life, blink, respawn.
    Kaputt,
}

/// Which death (if any) starts this frame: a death tile under Sammy, or no health left
/// (lava burns it meanwhile).
pub fn check_death(game: &mut GameState, on_death_tile: bool, on_lava: bool, t: Ticks) -> Option<DeathKind> {
    if on_lava {
        game.hearts -= LAVA_DAMAGE * t;
    }
    let kind = if on_death_tile {
        DeathKind::Tile
    } else if game.hearts <= 0.0 {
        DeathKind::Kaputt
    } else {
        return None;
    };
    game.hearts = game.hearts.max(0.0);
    Some(kind)
}
