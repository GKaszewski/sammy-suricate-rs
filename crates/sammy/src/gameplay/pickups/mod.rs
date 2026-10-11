//! Diamonds (`InitBonus3`/`Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey`/`Key_Hit`/`EjectOldKey`/`LaunchYouKey`, gems.wdl:40-960).
//! Both are picked up by Sammy's scan (id 1, radius `BlobZoneScan`).

use std::collections::HashMap;
use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::gems::HEIGHT_GEM;
use sammy_core::keys::{KeyOutcome, take_key};
use sammy_core::pickups::{KeySpec, NARROW_FLOOR_OFFSET, SHRINK_GONE, SHRINK_PER_TICK, SNAP_START, in_scan};
use super::player::{Player, TICKS};
use crate::platform::audio::Sfx;
use crate::gameplay::collision::{CollisionWorld, Hull};
use crate::render::coords;
use crate::gameplay::state::{GameState, MASTER_KEY, ResetScan};
pub use sammy_core::keys::gem_sprite as gem_file;
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

/// Diamonds, key gems (with the gem machinery of `gems`) and boulders.
pub struct PickupsPlugin;

impl Plugin for PickupsPlugin {
    fn build(&self, app: &mut App) {
        use super::gems;
        app.init_resource::<GemSprites>()
            .add_systems(Update, shrink.in_set(GameSet::Shrink))
            .add_systems(
                Update,
                (
                    gems::surprise_gems.in_set(GameSet::SurpriseGems),
                    gems::gem_gravity.in_set(GameSet::GemGravity),
                    gems::gem_lifetimes.in_set(GameSet::GemLifetimes),
                    gems::fly_gems.in_set(GameSet::GemFlight),
                ),
            )
            .add_systems(Update, (collect.run_if(is_playing).in_set(GameSet::Collect), snap_pickups.in_set(GameSet::SnapPickups)))
            .add_systems(Update, reset_keys.in_set(GameSet::KeyReset))
            .add_systems(Update, gems::run_boulders.run_if(is_playing).in_set(GameSet::Boulders));
    }
}
