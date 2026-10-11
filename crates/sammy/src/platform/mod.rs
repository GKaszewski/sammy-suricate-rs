//! The machine around the game: game files, save storage ([`sammy_platform`], shared
//! with other frontends), sound and music, input devices, windows and display modes,
//! debug tools.

pub mod audio;
#[cfg(feature = "debug-tools")]
pub mod debug;
pub mod display;
pub mod input;
#[cfg(feature = "music")]
pub mod music;
pub mod storage;
pub mod vfs;

#[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
pub use sammy_platform::locate;
pub use sammy_platform::{game_files, save};

