//! Sammy's wasps (docs/wasps.md): ammo pickups (`InitAmmo1`, bullets.wdl:21-177),
//! firing (`PlayerTire`, player.wdl:271-384), flight and hits (`InitWasp`/`Wasp`/
//! `Wasp_Hit`, bullets.wdl:203-747), weak doors (`InitDoorWeak`, mm1.wdl:4466-4715) and
//! hives.

use formats::wmb::Object;

use super::spawn::MODEL_FPS;
use super::world::{Animator, Body, Light, Look, Visual, World};
use super::{Dt, Effect, Event, Sim, Space, WaspHit};
use crate::collision::{Hull, Space as _};
use crate::id::Id;
use crate::level::Presentation;
use crate::input::Controls;
use crate::player::State;
use crate::time::TICKS_PER_SEC;
use crate::wasps::{AMMO_COLORS, AmmoPickup, Corpse, DoorDamage, ESCAPE_SCALE, Escapee, FIRED, HIVE_WASP_ANIM, HiveWasp, KINDS, PICKUP_ANIM, PICKUP_FADE, PICKUP_FADE_SHRINK, PICKUP_SPIN, PICKUP_TILT, PLANKS, Plank, Shot, WING_ANIM, Wasp, WaspStep};

mod doors;
mod fire;
mod flight;
mod hives;
mod pickups;

pub(super) use self::{doors::*, fire::*, flight::*, hives::*, pickups::*};

/// Pickup flash on taking ammo: start range, shrink per tick, cut-off range.
const TAKE_FLASH: (f32, f32, f32) = (300.0, 15.0, 100.0);
/// Spread of a wasp's death burst (`wasp_explode`).
const WASP_BURST_SPREAD: f32 = 2.0;
/// The flying wasp model.
const WASP_MODEL: &str = "wasp1.mdl";
/// Wasp type 1, the "queen" (overlay look).
const QUEEN: u32 = 1;
