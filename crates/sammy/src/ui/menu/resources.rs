#[allow(unused_imports)]
use super::*;

/// The menu's interaction state (the view and the hovered item are the game's).
#[derive(Resource, Default)]
pub struct MenuState {
    /// `Texte_Version` is hidden by the first click (interfac.wdl:442).
    pub(crate) clicked_once: bool,
    /// A modal overlay (options, Yes/No) owns the mouse.
    pub modal: bool,
    /// Sparkle orbit angles of the hovered icon.
    pub(crate) sparkle: [f32; 2],
}

/// A click on a menu item (`ChoixMenu`).
#[derive(Message, Clone, Copy)]
pub struct MenuChoice(pub u32);
