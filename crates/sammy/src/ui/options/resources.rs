#[allow(unused_imports)]
use super::*;

/// Requests from other systems (menu items, keys).
#[derive(Resource, Default)]
pub struct OptionsRequest {
    pub open: Option<OptionsMode>,
    /// Menu "Quit" item: the Yes/No dialog without the overlay.
    pub quit: bool,
}

#[derive(Resource, Default)]
pub struct OptionsState {
    pub mode: Option<OptionsMode>,
    /// `LastOptionChoose`: Help first (5), then remembered.
    pub(crate) tab: Tab,
    pub(crate) help_page: u32,
    pub(crate) dialog: Option<Question>,
    pub(crate) dirty: bool,
}

impl OptionsState {
    pub fn open(&self) -> bool {
        self.mode.is_some() || self.dialog.is_some()
    }
}

/// Gamepad / keyboard focus inside the overlay (shown while the pad is in use).
#[derive(Resource, Default)]
pub(crate) struct Focus(pub(crate) Option<Entity>);
