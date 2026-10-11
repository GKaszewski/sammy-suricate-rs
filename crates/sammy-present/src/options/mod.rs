//! The options overlay and Yes/No dialog (options.wdl, paninter.wdl; docs/menu.md §4-5),
//! for any frontend: a 640×480 plate centred on screen with tabs (audio/video, help,
//! credits), volume sliders, display rows, help pages and the game buttons. [`Options`]
//! holds the state, [`Options::elements`] says what to draw, [`Options::update`] takes
//! the frame's input and returns what the frontend must do ([`Effect`]).

use sammy_core::save::{DisplayMode, SaveGame};

use crate::text::{FontKind, WdlStrings, cp1250};

mod input;
mod layout;
mod rows;
mod types;

pub use self::types::*;
pub use self::layout::*;
use self::rows::*;

/// Background plates.
const PLATE: &str = "fondopt1.pcx";
const DIALOG: &str = "fondyn.pcx";

/// The overlay's state.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub mode: Option<Mode>,
    /// `LastOptionChoose`: Help first (5), then remembered.
    pub tab: Tab,
    help_page: u32,
    pub dialog: Option<Question>,
    /// Focused control (pad / keys).
    pub focus: Option<Control>,
    dragging: Option<Slider>,
}

fn inside(p: [f32; 2], a: [f32; 4]) -> bool {
    p[0] >= a[0] && p[1] >= a[1] && p[0] < a[0] + a[2] && p[1] < a[1] + a[3]
}

fn centre(a: [f32; 4]) -> [f32; 2] {
    [a[0] + a[2] / 2.0, a[1] + a[3] / 2.0]
}

impl Options {
    /// The overlay or the dialog is up.
    pub fn open(&self) -> bool {
        self.mode.is_some() || self.dialog.is_some()
    }

    /// Opens the overlay; returns its effects (a level freezes).
    pub fn show(&mut self, mode: Mode, tab: Option<Tab>) -> Vec<Effect> {
        self.mode = Some(mode);
        self.help_page = self.help_page.max(1);
        if let Some(t) = tab {
            self.tab = t;
        }
        if mode != Mode::Menu { vec![Effect::Pause(true)] } else { Vec::new() }
    }

    /// The menu's Quit item: the Yes/No dialog without the overlay.
    pub fn ask_quit(&mut self) {
        if self.dialog.is_none() {
            self.dialog = Some(Question::Quit);
        }
    }

    fn close(&mut self, out: &mut Vec<Effect>) {
        if self.mode.take().is_some_and(|m| m != Mode::Menu) {
            out.push(Effect::Pause(false));
        }
        self.dialog = None;
        self.focus = None;
    }

}

#[cfg(test)]
mod tests;
