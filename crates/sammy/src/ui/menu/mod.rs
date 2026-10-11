//! The 3D main menu (`menu_greenpepper.WMB`, interfac.wdl; docs/menu.md §3): title models
//! and level icons picked with the mouse, hard camera cuts to the level grids, tooltips.

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use formats::wmb::Object;
use crate::gameplay::ActionInit;
use crate::gameplay::player::TICKS;
use crate::platform::audio::Sfx;
use crate::camera::{CameraParams, MainCamera};
use crate::render::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::ui::hud::load_ui_image;
use crate::platform::save::{LevelState, SaveGame};
use sammy_core::menu::{Choice, Look, MenuAction};
use crate::ui::text::{BitmapText, FontKind, WdlStrings};
use crate::platform::vfs::GameFiles;
pub use sammy_core::levels::MENU_FILE;

mod components;
mod resources;
mod spawn;
mod systems;

pub use components::*;
pub use resources::*;
pub use spawn::*;
pub use systems::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_message::<MenuChoice>()
            .add_systems(Startup, setup_overlay)
            .add_systems(Update, (debug_start, style_items, animate_items, pick, act, overlay).chain().run_if(in_menu))
            .add_systems(PostUpdate, place_camera.run_if(in_menu).before(TransformSystems::Propagate))
            .add_systems(Update, (show_cursor, hide_hud, extend_grid_walls));
    }
}
