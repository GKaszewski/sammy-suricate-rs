//! The 3D main menu's interaction (`menu_greenpepper.WMB`, interfac.wdl; docs/menu.md
//! §3): picking the game's menu items with the mouse ray or the pad focus, clicks,
//! tooltips, the cursor, and the wide-screen walls of the level grids. The items, the
//! views and the camera are the game's ([`sammy_core::sim::MenuState`]).

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use crate::camera::MainCamera;
use crate::game::{Game, GameEvent, GameSystems};
use crate::platform::audio::Sfx;
use crate::platform::vfs::GameFiles;
use crate::render::coords;
use crate::scene::Mirrored;
use crate::ui::hud::load_ui_image;
use crate::ui::text::{BitmapText, FontKind, Strings};
use sammy_core::flow::Phase;
use sammy_core::id::Id;
use sammy_core::sim::Event;
pub use sammy_core::levels::MENU_FILE;
pub use sammy_core::sim::MENU_ARC;

mod components;
mod resources;
mod systems;

pub(crate) use components::*;
pub use resources::*;
pub use systems::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_message::<MenuChoice>()
            .add_systems(Startup, setup_overlay)
            .add_systems(Update, (debug_start, pick).chain().run_if(in_menu).before(GameSystems::Tick))
            // Choices act after the frame's tick: a chosen level loads on the next one
            // (so the click does not also skip its start).
            .add_systems(Update, (act, overlay.run_if(in_menu), show_cursor, hide_hud, extend_grid_walls).after(GameSystems::Tick));
    }
}
