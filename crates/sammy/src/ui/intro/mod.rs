//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): inside the menu scene a
//! scripted Sammy walks the path-node loop with the follow camera behind him while the
//! A5, publisher and Suricate logos fade in front of the camera. Node key 999, Enter,
//! Esc or the right mouse button end it: the camera glides to the menu pose.
#![cfg_attr(not(feature = "intro"), allow(unused_imports))]

use crate::gameplay::collision::Hull;
use bevy::prelude::*;
use formats::wmb::Object;
use crate::gameplay::player::{BLOB_HAUT, TICKS};
use crate::camera::{CameraParams, FollowState, MainCamera};
use crate::render::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::level::{GameAssets, NEAR_LAYER};
use crate::platform::vfs::GameFiles;
use sammy_core::intro::{END_KEY, FRICTION, FRICTION_AT, GLIDE_FOG_CLEAR, Logo, MENU_ORIGIN as ORIGIN, PLUS_TILT};

// Without the `intro` feature only the `Intro` state is used.
#[cfg_attr(not(feature = "intro"), allow(dead_code))]
mod components;
#[cfg_attr(not(feature = "intro"), allow(dead_code))]
mod resources;
mod spawn;
#[cfg(feature = "intro")]
mod systems;

pub use components::*;
pub use resources::*;
pub use spawn::*;
#[cfg(feature = "intro")]
pub(crate) use systems::*;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        // The state is always there (other features ask whether an intro runs).
        app.init_resource::<Intro>();
        #[cfg(feature = "intro")]
        app.add_systems(Update, (start, tick).chain())
            .add_systems(PostUpdate, (glide, place_logo).chain().after(crate::camera::follow).before(TransformSystems::Propagate));
    }
}
