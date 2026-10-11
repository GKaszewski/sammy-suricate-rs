//! Sammy as a Bevy entity: the [`sammy_core::player::Player`] component, his tuning
//! resources and the system that steps him.

use std::sync::Arc;
use bevy::prelude::*;
use formats::wmb::{Object, Wmb};
use sammy_core::player::{Frame, StepOut, Touch};
use crate::gameplay::collision::{CollisionWorld, entity, id};
use crate::render::coords;
use crate::render::model::ModelAnimator;
pub use sammy_core::player::{BLOB_HAUT, SCALE, State};
pub use sammy_core::time::TICKS_PER_SEC as TICKS;
pub use crate::gameplay::state::BOOST_MAX;
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

/// Sammy: tuning resources and movement.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JumpParams>()
            .init_resource::<PlayerTuning>()
            .add_message::<PlayerTouched>()
            .add_systems(Update, move_player.in_set(GameSet::Player));
    }
}
