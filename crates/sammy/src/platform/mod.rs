//! The machine around the game: game files, save storage, sound and music, input
//! devices, windows and display modes, debug tools.

pub mod audio;
#[cfg(feature = "debug-tools")]
pub mod debug;
pub mod display;
pub mod input;
#[cfg(feature = "music")]
pub mod music;
pub mod save;
pub mod vfs;
