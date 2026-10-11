//! What the game shows that is not a game entity, for any frontend: enemy ground
//! shadows and range rings (`InitOmbreLion`, `DisplayEnnemyRange`), friend-call rings
//! (`Circle_CallFriends`), the +50/+100 score popups (`FlyingPCX`), the lose title
//! (`InitTitleLoose`), the camera-space HUD wasp and Eye of Truth plate (docs/hud-saves.md
//! §1.6-1.7) and the sun angle of the engine model shadows. [`Extras`] keeps their state
//! and lists them each frame as plain items in native coordinates.

use sammy_core::flow::Phase;
use sammy_core::sim::{Event, Sim};

use sammy_core::points::Points;

use crate::particles::Particles;

mod advance;
mod draw;

/// Lion shadow: `shadlio2.pcx` (48 quants square) at 1.2 × the lion's scale, 0.5 above
/// the floor, turned with the lion's pan + 90, half transparent.
const SHADOW_FILE: &str = "shadlio2.pcx";
const SHADOW_SCALE: f32 = 1.2;
const SHADOW_SIZE: f32 = 48.0;
const SHADOW_LIFT: f32 = 0.5;
const SHADOW_PAN_OFFSET: f32 = 90.0;
const SHADOW_ALPHA: f32 = 0.5;
/// Range rings: `circl100.mdl` skins 1..=`RING_SKINS`, 1000 quants across at scale 1.
pub const RING_FILE: &str = "circl100.mdl";
const RING_SKINS: u32 = 3;
const RING_DIAMETER: f32 = 1000.0;
/// `Circle_CallFriends`: scale grows `CALL_GROW` per tick until `CALL_EASE_FROM`, then
/// eases (`CALL_EASE_RATE`) toward `CALL_EASE_TARGET`; the frame runs `CALL_FRAME_RATE`
/// per tick; the ring ends at frame `CALL_END_FRAME` or scale `CALL_END_SCALE`.
const CALL_START_SCALE: f32 = 0.01;
const CALL_GROW: f32 = 0.2;
const CALL_EASE_FROM: f32 = 3.0;
const CALL_EASE_TARGET: f32 = 5.1;
const CALL_EASE_RATE: f32 = 0.5;
const CALL_FRAME_RATE: f32 = 4.0;
const CALL_END_FRAME: f32 = 100.0;
const CALL_END_SCALE: f32 = 5.0;
/// `Bonus3_Hit` popups: points → (sprite, life in ticks). The popup starts `POPUP_LIFT`
/// above the pickup, flies at `POPUP_SPEED` per tick toward the camera tilted up by
/// `POPUP_TILT_DEG` (`InvDirCamera`), and plays its strip at this many frames per tick.
const POPUPS: [(Points, &str, f32); 2] = [(Points(50), "plus50+6.pcx", 32.0), (Points(100), "plu100+2.pcx", 48.0)];
const POPUP_LIFT: f32 = 32.0;
const POPUP_SPEED: f32 = 40.0;
const POPUP_TILT_DEG: f32 = 20.0;
const POPUP_FRAMES_PER_TICK: f32 = 0.2;
/// The lose title (`InitTitleLoose`, mm1.wdl:5314-5393) starts `LOSE_START_DEPTH` ahead
/// moving `LOSE_START_SPEED` per tick, slows by `LOSE_DECEL` (floor `LOSE_MIN_SPEED`)
/// and stops past `LOSE_STOP_DEPTH`; it sits `LOSE_RAISE` above the view axis, turned
/// to face the camera, at scale `LOSE_SCALE`, lit amber, smoking.
const LOSE_FILE: &str = "lose.mdl";
const LOSE_START_DEPTH: f32 = 20.0;
const LOSE_START_SPEED: f32 = 20.0;
const LOSE_DECEL: f32 = 0.8;
const LOSE_MIN_SPEED: f32 = 2.0;
const LOSE_STOP_DEPTH: f32 = 128.0;
const LOSE_RAISE: f32 = 20.0;
const LOSE_SCALE: f32 = 0.32;
const LOSE_LIGHT: ([f32; 3], f32) = ([250.0, 200.0, 50.0], 500.0);
/// HUD wasp (`InitCurrentPanWasp`, panels.wdl:1411-1461): scale per wasp type, mode-6
/// screen pixel (camera space (100,-45,-35)), depth in quants, start pan (also its
/// pitch) and spin per tick.
const WASP_FILE: &str = "panwasp.mdl";
const WASP_SCALE: [f32; 7] = [0.3, 0.23, 0.2, 0.2, 0.25, 0.3, 0.2];
const WASP_SCREEN: [f32; 2] = [569.0, 434.0];
const WASP_DEPTH: f32 = 100.0;
const WASP_PAN_START: f32 = 90.0;
const WASP_SPIN: f32 = 2.0;
/// Eye of Truth plate: mode-6 screen pixel (camera space (200,-45,-72)), depth, additive.
const EYE_FILE: &str = "paneye.mdl";
const EYE_SCREEN: [f32; 2] = [445.0, 440.0];
const EYE_DEPTH: f32 = 200.0;
/// `sun_angle` (pan, tilt) of the model shadows: 135/85 in levels (mm1.wdl:618-620),
/// pan 90 in the extro (Extro.wdl:81); it circles at 10°/tick on the win/lose screen
/// (mm1.wdl:966).
const SUN: [f32; 2] = [135.0, 85.0];
const EXTRO_SUN_PAN: f32 = 90.0;
const SUN_SPIN_RATE: f32 = 10.0;

/// How an item blends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    Alpha,
    /// A4 `flare`.
    Add,
}

/// Something to draw in the world (native coordinates; angles are A4 pan/tilt/roll).
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A model at one frame (0-based, fractional frames blend), unlit.
    Model { file: &'static str, skin: u32, frame: f32, pos: [f32; 3], angles: [f32; 3], scale: [f32; 3], blend: Blend },
    /// A model drawn over the level (near layer), lit by its own light.
    Title { file: &'static str, pos: [f32; 3], angles: [f32; 3], scale: f32, light: ([f32; 3], f32) },
    /// A flat square lying on the ground, `size` quants across, turned by `pan`.
    Decal { file: &'static str, pos: [f32; 3], pan: f32, size: f32, alpha: f32 },
    /// A camera-facing sprite strip at one frame.
    Sprite { file: &'static str, frame: u32, pos: [f32; 3] },
}

/// A model glued to the camera, next to the 2D HUD: placed at a mode-6 screen pixel
/// (`px`, x counted from the right edge when negative) and `depth` quants along the
/// view, turned by `angles` relative to the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct HudModel {
    pub file: &'static str,
    pub skin: u32,
    pub px: [f32; 2],
    pub depth: f32,
    pub angles: [f32; 3],
    pub scale: f32,
    pub blend: Blend,
}

/// The camera as the extras need it: position and A4 pan/tilt.
#[derive(Clone, Copy, Debug, Default)]
pub struct View {
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
}

impl View {
    /// From a position looking along `forward`.
    pub fn looking(pos: [f32; 3], forward: [f32; 3]) -> Self {
        let h = forward[0].hypot(forward[1]);
        Self { pos, pan: forward[1].atan2(forward[0]).to_degrees(), tilt: forward[2].atan2(h).to_degrees() }
    }

    fn forward(&self) -> [f32; 3] {
        let (p, t) = (self.pan.to_radians(), self.tilt.to_radians());
        [t.cos() * p.cos(), t.cos() * p.sin(), t.sin()]
    }

    fn up(&self) -> [f32; 3] {
        let (p, t) = (self.pan.to_radians(), self.tilt.to_radians());
        [-t.sin() * p.cos(), -t.sin() * p.sin(), t.cos()]
    }
}

#[derive(Clone, Debug)]
struct CallRing {
    delay: f32,
    scale: f32,
    frame: f32,
    origin: [f32; 3],
    skin: u32,
}

#[derive(Clone, Debug)]
struct Popup {
    file: &'static str,
    pos: [f32; 3],
    dir: [f32; 3],
    life: f32,
    frame: f32,
    frames: u32,
}

#[derive(Clone, Copy, Debug)]
struct LoseTitle {
    depth: f32,
    speed: f32,
    pos: [f32; 3],
}

/// The extras' state.
#[derive(Clone, Debug)]
pub struct Extras {
    calls: Vec<CallRing>,
    popups: Vec<Popup>,
    lose: Option<LoseTitle>,
    wasp_pan: f32,
    /// `sun_angle` (pan, tilt) in degrees: the direction the sun is seen from.
    pub sun: [f32; 2],
    view: View,
}

impl Default for Extras {
    fn default() -> Self {
        Self { calls: Vec::new(), popups: Vec::new(), lose: None, wasp_pan: WASP_PAN_START, sun: SUN, view: View::default() }
    }
}
