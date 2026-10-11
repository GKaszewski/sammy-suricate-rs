#[allow(unused_imports)]
use super::*;

/// What the menu shows and what is under the cursor.
#[derive(Resource, Default)]
pub struct MenuState {
    /// 0 = root view, else the code of the level grid shown (2, 3, 4, 50).
    pub view: u32,
    pub(crate) hovered: Option<Entity>,
    /// `Texte_Version` is hidden by the first click (interfac.wdl:442).
    pub(crate) clicked_once: bool,
    /// A modal overlay (options, Yes/No) owns the mouse.
    pub modal: bool,
}

/// A click on a menu item (`ChoixMenu`).
#[derive(Message, Clone, Copy)]
pub struct MenuChoice(pub u32);
