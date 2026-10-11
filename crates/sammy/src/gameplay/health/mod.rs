//! Health, lives and hazards:
//! - `InitTileDeath`/`TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava`/`Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitRestart` checkpoints (mm1.wdl:3771)

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::health::{BIG_HEART_SPIN, BOOST_AWAY_SHRINK, BOOST_HIDDEN, DeathEvent, HEART_AWAY_SHRINK, HEART_HIDDEN_BELOW, HEART_REGROW, Victim};
use super::pickups::{Collected, GemSprites, PickupPos, Respawnable, Shrinking, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::platform::audio::Sfx;
use crate::camera::FollowState;
use crate::render::coords;
use crate::gameplay::state::{GameState, ResetScan};
use crate::render::model::ModelAnimator;
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

/// Hazards, hearts, boosts, checkpoints and the death/respawn sequence.
pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, detect_deaths.run_if(is_playing).in_set(GameSet::Hazards))
            .add_systems(Update, collect_health.run_if(is_playing).in_set(GameSet::HealthPickups))
            .add_systems(Update, (spin_pickups.in_set(GameSet::SpinHealth), animate_hearts.in_set(GameSet::AnimateHearts), reset_checkpoints.in_set(GameSet::ResetCheckpoints)))
            .add_systems(Update, run_death.in_set(GameSet::Death));
    }
}
