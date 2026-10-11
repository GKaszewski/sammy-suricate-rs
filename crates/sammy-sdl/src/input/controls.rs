//! The frame's game controls from keys, mouse and pads.

use sammy_core::time::Secs;

use super::*;

impl Input {

    /// This frame's controls; clears the frame's presses.
    pub fn controls(&mut self, dt: Secs) -> Controls {
        let dt = dt.get();
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
