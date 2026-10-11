//! Extro level as Bevy systems: the `pov_Syn` camera target, id-25 scanners and the
//! scripted triggers, from [`sammy_core::extro`].

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::extro::{Body, TriggerKind, Wanted};
use super::enemies::Enemy;
use super::player::{Player, State};
use crate::camera::{CameraParams, FollowState};
use crate::gameplay::collision::{entity, id};
pub use sammy_core::extro::RANGE_SCAN_25;
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

/// The extro's camera target, scanners and scripted triggers.
pub struct ExtroPlugin;

impl Plugin for ExtroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pov>()
            .init_resource::<Scanners>()
            .init_resource::<TriggerClock>()
            .add_systems(PreUpdate, (update_pov, trigger_clock))
            .add_systems(Update, run_triggers.in_set(GameSet::ExtroTriggers))
            .add_systems(Update, scripted_anims.in_set(GameSet::ScriptedAnims))
            .add_systems(Update, fade_out.in_set(GameSet::Flow));
    }
}
