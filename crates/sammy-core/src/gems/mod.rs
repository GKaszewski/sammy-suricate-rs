//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5):
//! thrown-back keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` /
//! `LaunchNewKey`: boulders, Sammy's death gem), lifetimes, gravity for Flag7 gems,
//! surprise colours, boulders, and the gem that flies into a permanent door
//! (`CreateGemKeyAnim` / `AnimGemKey`).

use formats::trace::Trace;
use formats::wmb::Object;

use crate::physics::JumpParams;
use crate::rng::Rng;

mod boulder;
mod bounce;
mod door_gem;
mod fate;
mod flight;
mod lifetime;

pub use self::{boulder::*, door_gem::*, fate::*, flight::*, lifetime::*};

/// `HeightGem`: resting height of a gem above the floor.
pub const HEIGHT_GEM: f32 = 25.0;
/// `GemVerticalSpeed` / `GemVerticalFriction` (InitLvl.wdl:63-64).
pub const GEM_VSPEED: f32 = 10.0;
pub const GEM_VFRIC: f32 = 2.5;
/// `SpeedGemKey`: gem flying into a door, quants per tick.
pub const SPEED_GEM_KEY: f32 = 12.0;
/// A thrown key leaves 50 above Sammy and never rises more than 32 above that.
pub const THROW_HEIGHT: f32 = 50.0;

/// Lifetime blink: animation speed it starts at (frames per tick) and gains per blink.
pub const BLINK_START_SPEED: f32 = 3.0;
pub const BLINK_SPEEDUP: f32 = 0.1;

#[cfg(test)]
mod tests;
