//! Health, lives and hazards:
//! - `InitTileDeath`/`TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava`/`Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitRestart`
//!   checkpoints (mm1.wdl:3771), `InitBoost`
//! - the Eye of Truth (`InitEyeOfTruth`, mm1.wdl:4954-5103) and the warning light on
//!   Sammy when a lion starts a chase (`Warning_attack`, player.wdl:1091).

use formats::wmb::Object;

use crate::sim::world::{Light, World};
use super::{Dt, Effect, Event, Sim};
use crate::eye::{EXTRA_HEIGHT, EyePickup, EyeStage};
use crate::health::{BIG_HEART_SPIN, BOOST_AWAY_SHRINK, BOOST_HIDDEN, DeathEvent, Dying, HEART_AWAY_SHRINK, HEART_HIDDEN_BELOW, HEART_REGROW, SmallHeart, Victim};
use crate::id::Id;
use crate::pickups::in_scan;
use crate::platforms::CAMERA_SNAP;
use crate::player::State;

mod collect;
mod death;
mod eye;
mod hearts;

pub(super) use self::{collect::*, death::*, eye::*, hearts::*};

/// Big heart flash: colour, start range, shrink per tick, cut-off range.
const LIFE_FLASH: ([f32; 3], f32, f32, f32) = ([50.0, 250.0, 10.0], 300.0, 40.0, 64.0);
/// Boost pickup flash.
const BOOST_FLASH: ([f32; 3], f32, f32, f32) = ([150.0, 50.0, 100.0], 200.0, 10.0, 64.0);
/// One `particle_Getlife` every `GET_LIFE_EVERY` ticks, `GET_LIFE_COUNT` times
/// (mm1.wdl:2696-2706).
const GET_LIFE_COUNT: usize = 6;
const GET_LIFE_EVERY: f32 = 2.0;
/// The warning light: colour, start range (`LightRange1`); it shrinks
/// `WARNING_FADE_RATE` per tick and goes out at `WARNING_FADE_END`.
const WARNING_COLOR: [f32; 3] = [250.0, 100.0, 0.0];
pub const WARNING_RANGE: f32 = 300.0;
const WARNING_FADE_RATE: f32 = 5.0;
const WARNING_FADE_END: f32 = 100.0;
