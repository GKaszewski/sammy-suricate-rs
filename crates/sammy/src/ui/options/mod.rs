//! Options overlay and Yes/No dialog (options.wdl, paninter.wdl; docs/menu.md §4-5),
//! drawn on a centred 640×480 canvas. Opens with Esc in a level or from the menu item;
//! the game is frozen while it is open.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use crate::game::Game;
use sammy_core::flow::Phase;
use crate::ui::hud::load_ui_image;
use crate::ui::text::{BitmapText, FontKind, Strings};
use crate::platform::vfs::GameFiles;

mod components;
mod resources;
mod systems;

pub(crate) use components::*;
pub use resources::*;
pub use systems::*;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsRequest>()
            .init_resource::<OptionsState>()
            .init_resource::<Focus>()
            .add_systems(Update, (menu_requests, open_on_esc, open_close, build_tab, focus_nav, buttons, sliders, choices, choice_labels, dialog_keys, focus_frame).chain().after(crate::game::GameSystems::Tick));
    }
}
