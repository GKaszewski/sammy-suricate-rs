//! Help signs ([`sammy_core::titles`]), drawn on top of the world (`Near`).

use bevy::prelude::*;
use formats::wmb::Object;
use super::player::TICKS;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

mod components;
mod spawn;
mod systems;

pub use components::*;
pub use spawn::*;
pub use systems::*;

/// Help signs.
pub struct TitlesPlugin;

impl Plugin for TitlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_titles.in_set(GameSet::Titles));
    }
}
