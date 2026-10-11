//! Health, lives and hazards:
//! - `InitTileDeath` / `TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava` / `Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitBoost`
//!   (mm1.wdl:4315), `InitRestart` checkpoints (mm1.wdl:3771)

use formats::wmb::Object;

use crate::anim::Anim;
use crate::game::{GameState, MAX_HEARTS, MAX_LIVES};
use crate::gems::{Fate, Launch};
use crate::time::TICKS_PER_SEC;

mod death;
mod pickups;

pub use self::{death::*, pickups::*};

/// Lava burns this much health per tick.
pub const LAVA_DAMAGE: f32 = 1.0;
/// `InitSmallHeart` Skill2 default.
const SMALL_HEART_DEFAULT: f32 = 12.0;
/// A heart touched at full health ducks away for this long.
pub const HEART_AWAY: f32 = 48.0;
/// ... shrinking ×0.8 per tick, hidden below 0.1, then growing back ×1.5 per tick.
pub const HEART_AWAY_SHRINK: f32 = 0.8;
pub const HEART_HIDDEN_BELOW: f32 = 0.1;
pub const HEART_REGROW: f32 = 1.5;
/// `InitBoost` Skill2 default (boost units refilled).
const BOOST_DEFAULT: f32 = 1.0;
/// A boost pickup touched while the boost is full: shrinks ×0.9 per tick for 40 ticks,
/// is hidden for 64, then comes back.
pub const BOOST_HIDDEN: f32 = 64.0;
pub const BOOST_AWAY: f32 = BOOST_HIDDEN + 40.0;
pub const BOOST_AWAY_SHRINK: f32 = 0.9;
/// A checkpoint respawns Sammy this far above it.
pub const CHECKPOINT_RESPAWN_HEIGHT: f32 = 150.0;
/// Big hearts spin this many degrees per tick.
pub const BIG_HEART_SPIN: f32 = 10.0;

#[cfg(test)]
mod tests;
