//! Keyboard and gamepads (the PS Vita's own buttons and sticks are a gamepad to SDL) as
//! the game's [`Controls`], with the Bevy frontend's mapping: arrows / left stick move,
//! Ctrl / A jump, Space / X fire, Shift / right shoulder boost, Alt / B U-turn, Enter /
//! Select confirm, Esc / Start options.

use std::collections::HashSet;

use sammy_core::input::Controls;
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

mod controls;
mod events;

/// Stick dead zone (of 1.0) and the tilt that counts as a menu move.
const DEADZONE: f32 = 0.2;
const STICK_NAV: f32 = 0.6;
/// Stick menu moves repeat after this long, then this often (seconds).
const NAV_DELAY: f32 = 0.4;
const NAV_REPEAT: f32 = 0.15;

/// The mouse in a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pointer {
    /// Window pixels; None while a gamepad is in use or before the mouse moved.
    pub pos: Option<[f32; 2]>,
    /// The left button: pressed this frame, held, released this frame.
    pub clicked: bool,
    pub held: bool,
    pub released: bool,
    /// Yes / No dialog keys this frame (J/T/Y, N).
    pub yes: bool,
    pub no: bool,
}

#[derive(Default)]
pub struct Input {
    /// Kept for the whole run: SDL shuts the subsystem down when its last handle drops.
    controllers: Option<sdl2::GameControllerSubsystem>,
    keys: HashSet<Keycode>,
    pressed: HashSet<Keycode>,
    pads: Vec<GameController>,
    buttons: HashSet<Button>,
    pad_pressed: HashSet<Button>,
    right_click: bool,
    left_click: bool,
    left_held: bool,
    left_released: bool,
    /// The mouse pointer (window pixels), once it moved.
    mouse: Option<[f32; 2]>,
    nav_dir: Option<[i32; 2]>,
    nav_timer: f32,
    /// The last input came from a gamepad.
    pad: bool,
    pub quit: bool,
    /// Seconds since start (for `SAMMY_PAD_NAV`).
    clock: f32,
}

impl Input {
    /// Gamepads are opened as SDL announces them (also those present at start).
    pub fn new(sdl: &sdl2::Sdl) -> Self {
        Self { controllers: sdl.game_controller().ok(), ..Self::default() }
    }

    /// The mouse this frame: pointer position (window pixels) and a left click.
    pub fn pointer(&self) -> Pointer {
        let hit = |k: &[Keycode]| k.iter().any(|k| self.pressed.contains(k));
        Pointer {
            pos: self.mouse.filter(|_| !self.pad),
            clicked: self.left_click,
            held: self.left_held,
            released: self.left_released,
            yes: hit(&[Keycode::J, Keycode::T, Keycode::Y]),
            no: hit(&[Keycode::N]),
        }
    }
}
