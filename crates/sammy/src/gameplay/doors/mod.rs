//! Doors (`InitDoor`/`movePortail`/`OuvrePortail`, mm1.wdl:2886-3283) and the
//! exit tile (`InitExit`/`TileExit_Hit`, mm1.wdl:3956-3975).

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::doors::{DoorEvent, DoorView};
use super::player::{Player, TICKS};
use crate::platform::audio::Sfx;
use crate::gameplay::collision::Brush;
use crate::render::coords;
use crate::gameplay::state::{GameState, MASTER_KEY};
use super::is_playing;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

mod components;
mod spawn;
mod systems;

pub use components::*;
pub use spawn::*;
pub use systems::*;

/// Doors, their key gems and the exit tile.
pub struct DoorsPlugin;

impl Plugin for DoorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, run_doors.run_if(is_playing).in_set(GameSet::DoorScan))
            .add_systems(Update, run_exits.run_if(is_playing).in_set(GameSet::Exits))
            .add_systems(Update, run_door_gems.in_set(GameSet::DoorGems));
    }
}
