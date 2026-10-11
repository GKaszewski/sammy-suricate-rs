//! Decoration ([`sammy_core::decor`]): sky domes that follow the camera and sprites
//! snapped onto the floor.

use bevy::prelude::*;
use formats::wmb::Object;
use super::ActionInit;
use crate::gameplay::collision::Hull;
use crate::render::coords;
pub use sammy_core::decor::{init_arbre, init_decor, init_mdl, on_ground};

mod components;
mod spawn;
mod systems;

pub use components::*;
pub use spawn::*;
pub use systems::*;
