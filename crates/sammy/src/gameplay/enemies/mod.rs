//! Lions, scorpions, the elephant and wasp enemies as Bevy entities: the AI is
//! [`sammy_core::enemies`]; these systems feed it the frame and show the result.

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::enemies::{CALL_RINGS, Impact, Out, Shared, World};
use super::player::{Player, PlayerTouched, State as PlayerState, TICKS};
use super::wasps::{Wasp, WaspHit};
use crate::platform::audio::Sfx;
use crate::gameplay::collision::{BoxCollider, CollisionWorld, id};
use crate::render::coords;
use crate::gameplay::state::{GameState, Rng};
use crate::render::model::ModelAnimator;
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

/// Lions, scorpions, the elephant and wasp enemies.
pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LionGlobals>()
            .init_resource::<AiClock>()
            .init_resource::<FriendCalls>()
            .add_message::<WarningAttack>()
            .add_systems(Update, home_wasps.in_set(GameSet::WaspHoming))
            .add_systems(
                Update,
                (
                    add_player_collider.in_set(GameSet::PlayerCollider),
                    sync_player_collider.in_set(GameSet::SyncPlayerCollider),
                    advance_clock.in_set(GameSet::AiClock),
                    tick_enemies.in_set(GameSet::EnemyAi),
                    bump_clock.in_set(GameSet::BumpClock),
                    add_enemy_colliders.in_set(GameSet::EnemyColliders),
                    run_start_triggers.in_set(GameSet::StartTriggers),
                    sync_enemy_colliders.in_set(GameSet::SyncEnemyColliders),
                    fade_enemies.in_set(GameSet::FadeEnemies),
                    place_enemies.in_set(GameSet::PlaceEnemies),
                ),
            )
            .add_systems(Update, reset_calls.in_set(GameSet::CallReset));
    }
}
