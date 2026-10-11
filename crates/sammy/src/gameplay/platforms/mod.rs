//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X at Skill2 quants/tick through
//! hidden rails in the level. On impact it either reflects (Flag3) or turns by
//! Skill4/5/7 and pauses Skill3 ticks. Platforms and bodies with the same link id
//! (Skill1) copy the engine's motion at a fixed offset, carrying whatever stands
//! on them. An engine with Skill6 is locked until Sammy stands on its platform
//! holding a matching key.

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::platforms::{TeleportStep, UNLOCKED_PLATFORM};
use super::health::set_alpha;
use super::player::{LevelCollision, Player, State, TICKS};
use crate::platform::audio::Sfx;
use crate::camera::FollowState;
use crate::gameplay::collision::{Brush, Hull};
use crate::render::coords;
use crate::gameplay::state::GameState;
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

/// Engines, platforms and teleport pads.
pub struct PlatformsPlugin;

impl Plugin for PlatformsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                link_platforms.in_set(GameSet::LinkPlatforms),
                run_engines.run_if(is_playing).in_set(GameSet::RunEngines),
                follow_engines.in_set(GameSet::FollowEngines),
                ride_platforms.in_set(GameSet::RidePlatforms),
            ),
        )
            .add_systems(Update, enter_teleports.run_if(is_playing).in_set(GameSet::TeleportEntry))
            .add_systems(Update, run_teleports.in_set(GameSet::Teleports));
    }
}
