//! Keyboard and gamepads (the PS Vita's own buttons and sticks are a gamepad to SDL) as
//! the game's [`Controls`], with the Bevy frontend's mapping: arrows / left stick move,
//! Ctrl / A jump, Space / X fire, Shift / right shoulder boost, Alt / B U-turn, Enter /
//! Select confirm, Esc / Start options.

use std::collections::HashSet;

use sammy_core::input::Controls;
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

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

    /// Takes one SDL event.
    pub fn event(&mut self, e: &Event) {
        match e {
            Event::Quit { .. } => self.quit = true,
            Event::KeyDown { keycode: Some(k), repeat: false, .. } => {
                self.keys.insert(*k);
                self.pressed.insert(*k);
                self.pad = false;
            }
            Event::KeyUp { keycode: Some(k), .. } => {
                self.keys.remove(k);
            }
            Event::MouseButtonDown { mouse_btn: sdl2::mouse::MouseButton::Right, .. } => self.right_click = true,
            Event::MouseButtonUp { mouse_btn: sdl2::mouse::MouseButton::Left, .. } => {
                self.left_held = false;
                self.left_released = true;
            }
            Event::MouseButtonDown { mouse_btn: sdl2::mouse::MouseButton::Left, x, y, .. } => {
                self.left_click = true;
                self.left_held = true;
                self.mouse = Some([*x as f32, *y as f32]);
                self.pad = false;
            }
            Event::MouseMotion { x, y, .. } => {
                self.mouse = Some([*x as f32, *y as f32]);
                self.pad = false;
            }
            Event::ControllerButtonDown { button, .. } => {
                self.buttons.insert(*button);
                self.pad_pressed.insert(*button);
                self.pad = true;
            }
            Event::ControllerButtonUp { button, .. } => {
                self.buttons.remove(button);
            }
            Event::ControllerDeviceRemoved { which, .. } => self.pads.retain(|p| p.instance_id() != *which),
            Event::ControllerDeviceAdded { which, .. } => {
                if let Some(Ok(c)) = self.controllers.as_ref().map(|g| g.open(*which))
                    && !self.pads.iter().any(|p| p.instance_id() == c.instance_id())
                {
                    log::info!("gamepad {}", c.name());
                    self.pads.push(c);
                }
            }
            _ => {}
        }
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

    /// This frame's controls; clears the frame's presses.
    pub fn controls(&mut self, dt: f32) -> Controls {
        let key = |k: &[Keycode]| k.iter().any(|k| self.keys.contains(k));
        let hit = |k: &[Keycode]| k.iter().any(|k| self.pressed.contains(k));
        let btn = |b: Button| self.buttons.contains(&b);
        let tap = |b: Button| self.pad_pressed.contains(&b);
        let axis = |pos: Keycode, neg: Keycode| key(&[pos]) as i32 as f32 - key(&[neg]) as i32 as f32;
        let mut c = Controls {
            turn: axis(Keycode::Right, Keycode::Left),
            forward: axis(Keycode::Up, Keycode::Down),
            jump: key(&[Keycode::LCtrl, Keycode::RCtrl]) || btn(Button::A),
            jump_hit: hit(&[Keycode::LCtrl, Keycode::RCtrl]) || tap(Button::A),
            fire_hit: hit(&[Keycode::Space]) || tap(Button::X),
            boost: key(&[Keycode::LShift, Keycode::RShift]) || btn(Button::RightShoulder),
            uturn_hit: hit(&[Keycode::LAlt, Keycode::RAlt]) || tap(Button::B),
            confirm: hit(&[Keycode::Return, Keycode::KpEnter]) || tap(Button::Back),
            options: hit(&[Keycode::Escape]) || tap(Button::Start),
            back: self.right_click || tap(Button::B),
            accept: tap(Button::A),
            any: !self.pressed.is_empty() || !self.pad_pressed.is_empty(),
            pad: self.pad,
            nav: None,
        };
        // Sticks and the D-pad move Sammy too.
        let mut stick = [0.0f32; 2];
        for p in &self.pads {
            let a = |ax: Axis| {
                let v = p.axis(ax) as f32 / i16::MAX as f32;
                if v.abs() < DEADZONE { 0.0 } else { (v - DEADZONE * v.signum()) / (1.0 - DEADZONE) }
            };
            stick = [stick[0] + a(Axis::LeftX), stick[1] - a(Axis::LeftY)];
        }
        let dpad = [btn(Button::DPadRight) as i32 as f32 - btn(Button::DPadLeft) as i32 as f32, btn(Button::DPadUp) as i32 as f32 - btn(Button::DPadDown) as i32 as f32];
        c.turn = (c.turn + stick[0] + dpad[0]).clamp(-1.0, 1.0);
        c.forward = (c.forward + stick[1] + dpad[1]).clamp(-1.0, 1.0);
        // Menu moves: arrows and D-pad presses, the stick with key repeat.
        for (k, d) in [(Keycode::Left, [-1, 0]), (Keycode::Right, [1, 0]), (Keycode::Up, [0, 1]), (Keycode::Down, [0, -1])] {
            if hit(&[k]) {
                c.nav = Some(d);
            }
        }
        for (b, d) in [(Button::DPadLeft, [-1, 0]), (Button::DPadRight, [1, 0]), (Button::DPadUp, [0, 1]), (Button::DPadDown, [0, -1])] {
            if tap(b) {
                c.nav = Some(d);
            }
        }
        let stick_dir = (stick[0].hypot(stick[1]) > STICK_NAV).then(|| if stick[0].abs() > stick[1].abs() { [stick[0].signum() as i32, 0] } else { [0, stick[1].signum() as i32] });
        match stick_dir {
            Some(d) if self.nav_dir != Some(d) => {
                self.nav_dir = Some(d);
                self.nav_timer = NAV_DELAY;
                c.nav = c.nav.or(Some(d));
            }
            Some(d) => {
                self.nav_timer -= dt;
                if self.nav_timer <= 0.0 {
                    self.nav_timer = NAV_REPEAT;
                    c.nav = c.nav.or(Some(d));
                }
            }
            None => self.nav_dir = None,
        }
        if stick_dir.is_some() {
            self.pad = true;
            c.pad = true;
        }
        // Debug: SAMMY_PAD_NAV=right,down,a,b,... presses one pad input every step.
        self.clock += dt;
        let (cmd, running) = sammy_platform::debug::pad_nav(self.clock, dt);
        match cmd.as_deref() {
            Some("left") => c.nav = Some([-1, 0]),
            Some("right") => c.nav = Some([1, 0]),
            Some("up") => c.nav = Some([0, 1]),
            Some("down") => c.nav = Some([0, -1]),
            Some("a") => (c.accept, c.jump_hit, c.any) = (true, true, true),
            Some("b") => (c.back, c.uturn_hit) = (true, true),
            Some("start") => c.options = true,
            _ => {}
        }
        if running {
            self.pad = true;
            c.pad = true;
        }
        self.pressed.clear();
        self.pad_pressed.clear();
        self.right_click = false;
        self.left_click = false;
        self.left_released = false;
        c
    }
}
