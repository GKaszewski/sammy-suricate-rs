#[allow(unused_imports)]
use super::*;

/// `InitMainMenu` / `InitMainTitle` entity (interfac.wdl:23-199, :950-1014).
#[derive(Component)]
pub struct MenuItem {
    /// Skill1 = `ChoixMenu` code (0 decor, 1 Play, 2-4 worlds, 5 Options, 6 Quit, 20 Back,
    /// 50 Bonus, 100+N level icon).
    pub code: u32,
    /// Bob and spin ([`sammy_core::menu::Bob`]).
    pub(crate) bob: sammy_core::menu::Bob,
    pub(crate) sprite: bool,
    pub(crate) alpha: f32,
    pub(crate) scale: Vec3,
    pub(crate) styled: bool,
    /// Icon of a locked / absent level (`LevelNotReady`).
    pub(crate) locked: bool,
    pub(crate) sparkle: [f32; 2],
}

/// `Pos*Levels` camera markers (interfac.wdl:903-923).
#[derive(Component)]
pub struct MenuMarker {
    pub(crate) code: u32,
    pub(crate) pos: [f32; 3],
    pub(crate) pan: f32,
    pub(crate) tilt: f32,
}

// ---------------------------------------------------------------- overlay

#[derive(Component)]
pub(crate) struct VersionText;

#[derive(Component)]
pub(crate) struct TooltipTitle;

#[derive(Component)]
pub(crate) struct TooltipMessage;

#[derive(Component)]
pub(crate) struct Cursor;
