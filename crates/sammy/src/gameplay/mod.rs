//! The game rules on Bevy: one plugin per feature, each feeding [`sammy_core`] and
//! showing the result. Level objects are dispatched to them by action name
//! ([`dispatch`]).

pub mod collision;
pub mod decor;
pub mod dispatch;
pub mod extro;
pub mod doors;
pub mod enemies;
pub mod indicators;
pub mod gems;
pub mod health;
pub mod pickups;
pub mod platforms;
pub mod player;
pub mod state;
pub mod titles;
pub mod wasps;

use bevy::prelude::*;
use crate::gameplay::state::GameState;


/// All gameplay features.
pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameState>()
            .init_resource::<crate::gameplay::state::Rng>()
            .add_message::<crate::gameplay::state::ResetScan>()
            .add_plugins((
                platforms::PlatformsPlugin,
                doors::DoorsPlugin,
                player::PlayerPlugin,
                pickups::PickupsPlugin,
                health::HealthPlugin,
                wasps::WaspsPlugin,
                enemies::EnemiesPlugin,
                extro::ExtroPlugin,
                indicators::IndicatorsPlugin,
                titles::TitlesPlugin,
                decor::DecorPlugin,
            ));
    }
}

pub use dispatch::{ActionInit, init, is_marker, is_passable, is_playing};
