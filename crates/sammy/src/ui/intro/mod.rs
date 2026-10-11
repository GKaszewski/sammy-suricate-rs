//! Boot intro presentation (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): the A5,
//! publisher and Suricate logos in front of the camera and the black fog. The intro
//! itself (the walk, the timeline, the glide) is the game's
//! ([`sammy_core::sim::IntroState`]).

use bevy::prelude::*;
use crate::camera::MainCamera;
use crate::game::Game;
use crate::platform::vfs::GameFiles;
use crate::render::coords;
use crate::scene::{Loader, NEAR_LAYER};

mod components;
mod systems;

pub(crate) use components::*;
pub(crate) use systems::*;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, place_logo.after(crate::camera::CameraSystems::Place).before(TransformSystems::Propagate));
    }
}
