//! The boot intro's presentation (`PlaceTitle`, Intro2.wdl; docs/menu.md §2): which
//! logo is shown, where, and how opaque, and the black fog over the view. The intro
//! itself is the game's ([`sammy_core::sim::IntroState`]).

use sammy_core::sim::Sim;

/// Logo opacity and fog are percentages.
const FULL_PERCENT: f32 = 100.0;
/// Model logos (`publogo`, `surilogo`) are shown at this scale; the sprite logo at 1.
pub const LOGO_MODEL_SCALE: f32 = 1.5;

/// A logo's art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoArt {
    /// A camera-facing image (TGA).
    Image(&'static str),
    /// A model, drawn with its first skin and frame.
    Model(&'static str),
}

/// The logo in front of the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Logo {
    /// Which logo (1 A5, 2 publisher, 3 Suricate); changes when the art does.
    pub id: u8,
    pub art: LogoArt,
    /// Native position: `dist` quants ahead of the camera, level with it.
    pub pos: [f32; 3],
    pub scale: f32,
    /// Opacity 0..1.
    pub alpha: f32,
}

/// What the intro shows this frame: the logo (if any) for a camera at `cam` looking
/// along `forward`, and the black fog's opacity 0..1 (drawn under the logo).
pub fn overlay(sim: &Sim, cam: [f32; 3], forward: [f32; 3]) -> (Option<Logo>, f32) {
    let (shown, dist, alpha, fog) = sim.intro.overlay();
    let a = forward[1].atan2(forward[0]);
    let pos = [cam[0] + a.cos() * dist, cam[1] + a.sin() * dist, cam[2]];
    let logo = match shown {
        1 => Some((LogoArt::Image("slogoa5.tga"), 1.0)),
        2 => Some((LogoArt::Model("publogo.mdl"), LOGO_MODEL_SCALE)),
        3 => Some((LogoArt::Model("surilogo.mdl"), LOGO_MODEL_SCALE)),
        _ => None,
    };
    (logo.map(|(art, scale)| Logo { id: shown, art, pos, scale, alpha: alpha / FULL_PERCENT }), fog / FULL_PERCENT)
}
