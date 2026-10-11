//! How the game presents its state, for any frontend: what the HUD shows where
//! ([`hud`]), the boot intro's logos and fog ([`intro`]), the menu's focus, tooltips
//! and grid walls ([`menu`]), the options overlay ([`options`]) and how text is laid
//! out ([`text`]). Frontends draw the result with their own renderer.

pub mod effects;
pub mod hud;
pub mod intro;
pub mod menu;
pub mod options;
pub mod particles;
pub mod text;
