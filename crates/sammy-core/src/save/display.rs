//! Display settings kept in the save.

use super::*;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct DisplaySettings {
    pub mode: DisplayMode,
    /// Windowed size in physical pixels.
    pub window: [u32; 2],
    /// Exclusive fullscreen video mode; `None` = the desktop's.
    pub fullscreen: Option<[u32; 2]>,
    /// 0 large (biggest whole-number scale that fits), 1 medium, 2 small.
    pub ui_size: u8,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self { mode: DisplayMode::Windowed, window: [1280, 720], fullscreen: None, ui_size: 0 }
    }
}
