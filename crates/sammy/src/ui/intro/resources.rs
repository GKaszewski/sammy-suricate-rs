#[allow(unused_imports)]
use super::*;

#[derive(Resource, Default)]
pub struct Intro {
    /// The intro owns the camera and the menu is not interactive yet.
    pub active: bool,
    /// `MoveMenu`: the camera glides back to the menu pose.
    pub(crate) gliding: bool,
    pub(crate) acc: f32,
    pub(crate) ticks: f32,
    /// `Camera.fog` (0..100, black).
    pub(crate) fog: f32,
    pub(crate) logo: Logo,
    /// The boot load happened (only a menu at boot plays the intro).
    pub(crate) played: bool,
    pub(crate) loads: u32,
}

impl Intro {
    /// The follow camera drives the view (not while gliding).
    pub fn following(&self) -> bool {
        self.active && !self.gliding
    }
}
