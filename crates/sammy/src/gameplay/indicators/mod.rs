//! Gameplay visuals tied to the AI: the warning light on Sammy when a lion starts a
//! chase (`Warning_attack`, player.wdl:1091), ground shadows (`InitOmbreLion`,
//! `InitOmbreSammy`), the Eye of Truth pickup (`InitEyeOfTruth`, mm1.wdl:4954-5103)
//! with lion range rings (`DisplayEnnemyRange`, lion.wdl:1586-1653), and the
//! expanding rings of a friend call (`Circle_CallFriends`, lion.wdl:1015-1070).

use bevy::prelude::*;
use formats::wmb::Object;
use super::enemies::{CallRing, Enemy, WarningAttack};
use super::pickups::{PickupPos, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::platform::audio::Sfx;
use crate::render::coords::{self, QUANTS_PER_UNIT};
use crate::gameplay::state::GameState;
use crate::level::GameAssets;
use crate::render::lights::DynamicLight;
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

/// Warning light, blob shadows, the Eye of Truth and the range/call rings.
pub struct IndicatorsPlugin;

impl Plugin for IndicatorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                warning_light.in_set(GameSet::WarningLight),
                spawn_shadows.in_set(GameSet::SpawnShadows),
                place_shadows.in_set(GameSet::PlaceShadows),
                run_eye.in_set(GameSet::Eye),
                spawn_rings.in_set(GameSet::SpawnRings),
                update_rings.in_set(GameSet::UpdateRings),
                run_call_rings.in_set(GameSet::CallRings),
            ),
        );
    }
}
