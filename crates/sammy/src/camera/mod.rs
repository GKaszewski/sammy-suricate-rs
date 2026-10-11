//! Cameras: the main camera placed where the game's camera is
//! ([`sammy_core::sim::Camera`]), the sky/near cameras that mirror it, the field of view,
//! and a free-fly debug camera (F1 toggles).

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use sammy_core::camera::CameraPose;

use crate::game::Game;
use crate::render::coords;

mod components;
mod resources;
mod systems;

pub use components::*;
pub use resources::*;
pub use systems::*;

/// Where the cameras are placed (PostUpdate, before transforms propagate).
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CameraSystems {
    Place,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .add_systems(Update, (toggle_mode, fly.run_if(resource_equals(CameraMode::Fly)), apply_arc))
            .add_systems(
                PostUpdate,
                (place.run_if(resource_equals(CameraMode::Follow)), copy_to_sky).chain().in_set(CameraSystems::Place).before(TransformSystems::Propagate),
            );
    }
}
