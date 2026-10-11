//! How the game presents its state, for any frontend: what the HUD shows where
//! ([`hud`]), the menu's focus, tooltips and grid walls ([`menu`]) and how text is laid
//! out ([`text`]). Frontends draw the result with their own renderer.

pub mod hud;
pub mod menu;
pub mod text;
