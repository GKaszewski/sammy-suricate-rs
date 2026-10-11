//! Bevy side of the global game state: resources wrapping the core types.

use bevy::prelude::*;

use crate::platform::audio::Sfx;

pub use sammy_core::game::{BOOST_MAX, MASTER_KEY, MAX_HEARTS, MAX_LIVES};

/// The reset scan (id 1000) sent by `RestartSammy`: level-placed keys and
/// checkpoints come back, runtime keys disappear.
#[derive(Message, Clone, Copy)]
pub struct ResetScan;

/// [`sammy_core::game::GameState`] as a resource.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct GameState(pub sammy_core::game::GameState);

impl GameState {
    /// Debug: SAMMY_KEY=<colour> starts with a key (255 = master).
    pub fn apply_debug_env(&mut self) {
        if let Some(k) = std::env::var("SAMMY_KEY").ok().and_then(|v| v.parse().ok()) {
            self.key = k;
        }
    }

    /// `AddPoints`, playing `ScoreReach_son` when the goal is reached.
    pub fn add_points(&mut self, points: i32, sfx: &mut Sfx) {
        if self.0.add_points(points) {
            sfx.play("ScoreReach_son");
        }
    }
}

/// The game's [`sammy_core::rng::Rng`]: seeded from `SAMMY_SEED`, else the clock.
#[derive(Resource, Deref, DerefMut)]
pub struct Rng(pub sammy_core::rng::Rng);

impl Default for Rng {
    fn default() -> Self {
        let seed = std::env::var("SAMMY_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
        });
        Self::seeded(seed)
    }
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Self(sammy_core::rng::Rng::seeded(seed))
    }
}
