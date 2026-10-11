//! Sammy's wasps (docs/wasps.md): ammo pickups (`InitAmmo1`, bullets.wdl:21-177),
//! firing (`PlayerTire`, player.wdl:271-384), flight and hits (`InitWasp`/`Wasp`/
//! `Wasp_Hit`, bullets.wdl:203-747) and weak doors (`InitDoorWeak`, mm1.wdl:4466-4715).

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::wasps::{AMMO_COLORS, DoorDamage, FIRED, PICKUP_FADE, PICKUP_FADE_SHRINK, PICKUP_SPIN, PICKUP_TILT, PLANKS, WaspStep};
use super::pickups::{PickupPos, Shrinking};
use super::player::{Player, State, TICKS};
use crate::platform::audio::Sfx;
use crate::gameplay::collision::{Brush, CollisionWorld, Hull, entity, id};
use crate::render::coords;
use crate::gameplay::state::{GameState, Rng};
use crate::level::{GameAssets, LevelObject};
use crate::render::model::ModelAnimator;
use super::is_playing;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

mod components;
mod resources;
mod spawn;
mod systems;

pub use components::*;
pub use resources::*;
pub use spawn::*;
pub use systems::*;

/// Sammy's wasps: ammo, firing, flight, weak doors, planks and hives.
pub struct WaspsPlugin;

impl Plugin for WaspsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WaspHit>()
            .add_systems(
                Update,
                (
                    collect_ammo.run_if(is_playing).in_set(GameSet::CollectAmmo),
                    fire.in_set(GameSet::Fire),
                    spin_pickups.in_set(GameSet::SpinAmmo),
                    fly_wasps.in_set(GameSet::WaspFlight),
                    update_weak_doors.in_set(GameSet::WeakDoors),
                    fly_planks.in_set(GameSet::Planks),
                    run_hives.run_if(is_playing).in_set(GameSet::Hives),
                    fly_hive_wasps.in_set(GameSet::HiveWasps),
                    expire_pickups.in_set(GameSet::ExpireAmmo),
                ),
            );
    }
}
