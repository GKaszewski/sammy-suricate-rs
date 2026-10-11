//! What the overlay shows and takes: modes, tabs, controls, effects and input.

use super::*;

/// The plate (mode-6 pixels).
pub const W: f32 = 640.0;
pub const H: f32 = 480.0;

/// `MenuOptionsGame`: 0 front-end, 1 level, 2 extro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Menu,
    Level,
    Extro,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    Audio,
    #[default]
    Help,
    Credits,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Question {
    Quit,
    Restart,
}

/// What a button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Tab(Tab),
    Resume,
    Back,
    Quit,
    Restart,
    PrevPage,
    NextPage,
    Yes,
    No,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slider {
    Music,
    Sound,
}

/// Port display settings, drawn as `<  value  >` rows in the old video slider's place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Mode,
    Size,
    UiSize,
}

/// Something the player can point at or focus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Button(Action),
    Slider(Slider),
    Row(Row),
}

/// What to draw: an image (region) or a text, at plate pixels; `control` marks the
/// focusable / clickable ones.
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Image { image: String, src: Option<[f32; 4]>, area: [f32; 4] },
    Text { font: FontKind, narrow: bool, bytes: Vec<u8>, at: [f32; 2], centre_in: Option<[f32; 4]> },
}

/// A drawable with its control (if any).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub element: Element,
    pub control: Option<(Control, [f32; 4])>,
}

/// What the frontend must do after an update.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Effect {
    /// `play_sound name, vol`.
    Sound(&'static str, f32),
    /// The game is frozen while the overlay is up in a level (`Freeze_mode`).
    Pause(bool),
    /// Back to the menu from a game (stored like a lost game).
    Abandon,
    /// `OuiRestartLevel`: no lives left → lose screen, same level again.
    GiveUp,
    /// `QuitSammy`: save, then exit.
    Exit,
    /// The settings changed (write the save; apply volumes / the display).
    SaveChanged,
}

/// The display choices a frontend offers: window sizes for the current mode and the size
/// in use. `None` where the display is fixed (handhelds).
#[derive(Clone, Debug, Default)]
pub struct DisplayInfo {
    pub sizes: Vec<[u32; 2]>,
    pub current: Option<[u32; 2]>,
}

/// Windowed sizes offered (those that fit the monitor).
pub(super) const WINDOW_SIZES: [[u32; 2]; 10] = [[640, 480], [800, 600], [1024, 768], [1280, 720], [1280, 960], [1366, 768], [1600, 900], [1920, 1080], [2560, 1440], [3840, 2160]];
/// Smallest fullscreen mode offered (the original art size).
pub(super) const MIN_FULLSCREEN: [u32; 2] = [640, 480];

impl DisplayInfo {
    /// The choices for `d`'s mode on a monitor of `desktop` size with these fullscreen
    /// video modes.
    pub fn new(d: &sammy_core::save::DisplaySettings, desktop: Option<[u32; 2]>, video_modes: &[[u32; 2]]) -> Self {
        let max = desktop.unwrap_or([u32::MAX; 2]);
        let sizes = match d.mode {
            DisplayMode::Windowed => WINDOW_SIZES.iter().copied().filter(|s| s[0] <= max[0] && s[1] <= max[1]).collect(),
            DisplayMode::Fullscreen => {
                let mut v = video_modes.to_vec();
                v.sort();
                v.dedup();
                v.retain(|s| s[0] >= MIN_FULLSCREEN[0] && s[1] >= MIN_FULLSCREEN[1]);
                v
            }
            DisplayMode::Borderless => Vec::new(),
        };
        let current = match d.mode {
            DisplayMode::Windowed => Some(d.window),
            DisplayMode::Borderless => desktop,
            DisplayMode::Fullscreen => d.fullscreen.or(desktop),
        };
        Self { sizes, current }
    }
}

/// One frame's input for the overlay, pointer in plate pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub pointer: Option<[f32; 2]>,
    /// Left button: pressed this frame, held, released this frame.
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
    /// A focus move (x right, y up).
    pub nav: Option<[i32; 2]>,
    /// Pad A / Enter on the focused item.
    pub accept: bool,
    /// Esc, Enter, Start or B: close the overlay or cancel the dialog.
    pub close: bool,
    /// Yes / No keys (J/T/Y, N).
    pub yes: bool,
    pub no: bool,
    /// The pad is in use (focus is shown and used).
    pub pad: bool,
}
