//! Options overlay and Yes/No dialog on Bevy UI: the shared model
//! ([`sammy_present::options`]) drawn as image and text nodes on a centred 640×480
//! plate. Opens with Esc in a level or from the menu items; the game is frozen while
//! it is open in a level.

use std::collections::HashMap;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryMonitor, PrimaryWindow};
use sammy_core::flow::Phase;
use sammy_present::options::{DisplayInfo, Effect, Element, FRAME_BORDER, FRAME_FILL_ALPHA, FRAME_PAD, FRAME_RGB, H, Input, Item, OVERLAY_ALPHA, Options, Tab, W};

use crate::game::{Game, GameEvent, GameSystems};
use crate::platform::vfs::GameFiles;
use crate::ui::hud::load_ui_image;
use crate::ui::text::{BitmapText, Strings};

pub use sammy_present::options::Mode as OptionsMode;

/// Stacking: overlay, focus frame.
const Z_OVERLAY: i32 = 50;
const Z_FOCUS_FRAME: i32 = 70;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsRequest>()
            .init_resource::<OptionsState>()
            .add_systems(Update, (menu_requests, open_on_esc, open, update, render).chain().after(GameSystems::Tick));
    }
}

/// Requests from other systems (menu items, keys).
#[derive(Resource, Default)]
pub struct OptionsRequest {
    pub open: Option<OptionsMode>,
    /// Menu "Quit" item: the Yes/No dialog without the overlay.
    pub quit: bool,
}

/// The overlay's state.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct OptionsState(pub Options);

#[derive(Component)]
struct OptionsRoot;

#[derive(Component)]
struct Plate;

#[derive(Component)]
struct FocusFrame;

mod open;
mod render;
mod update;

use self::{open::*, render::*, update::*};
