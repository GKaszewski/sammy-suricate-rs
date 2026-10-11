//! The 3D main menu's interaction (`menu_greenpepper.WMB`, interfac.wdl; docs/menu.md
//! §3): picking the game's menu items with the mouse ray or the pad focus, clicks,
//! tooltips, the cursor, and the wide-screen walls of the level grids. The items, the
//! views and the camera are the game's ([`sammy_core::sim::MenuState`]).

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use sammy_core::flow::Phase;
use sammy_core::id::Id;
pub use sammy_core::levels::MENU_FILE;
use sammy_core::sim::Event;
pub use sammy_core::sim::MENU_ARC;

use crate::camera::MainCamera;
use crate::game::{Game, GameEvent, GameSystems};
use crate::platform::audio::Sfx;
use crate::platform::vfs::GameFiles;
use crate::render::coords;
use crate::scene::Mirrored;
use crate::ui::hud::load_ui_image;
use crate::ui::text::{BitmapText, FontKind, Strings};

mod choice;
mod components;
mod grid;
mod overlay;
mod picking;
mod resources;

pub(crate) use components::*;
pub use resources::*;
pub use self::choice::*;
pub(crate) use self::{grid::*, overlay::*, picking::*};

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
