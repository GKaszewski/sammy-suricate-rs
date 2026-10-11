//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): inside the menu scene a
//! scripted Sammy walks the path-node loop with the follow camera behind him while the
//! A5, publisher and Suricate logos fade in front of the camera. Node key 999, Enter,
//! Esc or the right mouse button end it: the camera glides to the menu pose.

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

mod components;
mod resources;
mod spawn;
mod systems;

pub use components::*;
pub use resources::*;
pub use spawn::*;
pub(crate) use systems::*;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Intro>()
            .add_systems(Update, (start, tick).chain())
            .add_systems(PostUpdate, (glide, place_logo).chain().after(crate::camera::follow).before(TransformSystems::Propagate));
    }
}
