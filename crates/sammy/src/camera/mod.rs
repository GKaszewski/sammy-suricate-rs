//! Cameras: the game's follow camera ([`sammy_core::camera`]) on Bevy's main camera,
//! the sky/near cameras that mirror it, the field of view, and a free-fly debug camera
//! (F1 toggles).

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::camera::{CameraPose, Subject};
use crate::gameplay::player::{LevelCollision, Player, TICKS};
use crate::gameplay::collision::id;
use crate::render::coords;
use crate::flow::{CurrentLevel, Phase};

mod components;
mod resources;
mod spawn;
mod systems;

pub use components::*;
pub use resources::*;
pub use spawn::*;
pub use systems::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<CameraParams>()
            .init_resource::<CameraRuntime>()
            .add_systems(Update, (reset_camera_on_respawn.in_set(crate::schedule::GameSet::CameraReset), camera_zones.in_set(crate::schedule::GameSet::CameraZones)))
            .add_systems(Update, (toggle_mode, fly.run_if(resource_equals(CameraMode::Fly)), apply_arc))
            .add_systems(
                PostUpdate,
                (follow.run_if(resource_equals(CameraMode::Follow)), copy_to_sky)
                    .chain()
                    .before(TransformSystems::Propagate),
            );
    }
}
