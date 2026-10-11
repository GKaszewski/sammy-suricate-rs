//! Level flow (mm1.wdl:542-1156, Loads.wdl): the level table, loading and
//! unloading, the level-start camera sweep, and win/lose progression.

use bevy::prelude::*;
use crate::gameplay::player::{JumpParams, TICKS};
use crate::camera::{CameraParams, FollowState};
use crate::gameplay::state::GameState;
use crate::level::Spawner;
use crate::render::material::LevelMaterial;
use crate::platform::vfs::GameFiles;
pub use sammy_core::levels::{EXTRO_SLOT, LEVELS, file_for, next_slot, slot_for};
pub use sammy_core::flow::Phase;

mod components;
mod resources;
mod systems;

pub use components::*;
pub use resources::*;
pub use systems::*;

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .init_resource::<PendingLoad>()
            .init_resource::<crate::level::GameAssets>()
            .add_systems(PreUpdate, load_pending)
            .add_systems(Update, run_flow.in_set(crate::schedule::GameSet::Flow));
    }
}
