//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5):
//! thrown-back keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` /
//! `LaunchNewKey`: boulders, Sammy's death gem), lifetimes with blinking,
//! gravity/hover for Flag7 gems, surprise colours, boulders, and the gem that
//! flies into a permanent door (`CreateGemKeyAnim` / `AnimGemKey`).

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::gems::{HEIGHT_GEM, Rest};
use super::pickups::{GemSprites, KeyGem, LevelKey, PickupPos, Respawnable, Shrinking};
use super::player::{JumpParams, Player, TICKS};
use super::platforms::Platform;
use super::wasps::WaspHit;
use crate::platform::audio::Sfx;
use crate::gameplay::collision::{CollisionWorld, Hull};
use crate::render::coords;
use crate::gameplay::state::Rng;
use crate::level::{Billboard, SpriteAnim};

mod components;
mod spawn;
mod systems;

pub use components::*;
pub use spawn::*;
pub use systems::*;
