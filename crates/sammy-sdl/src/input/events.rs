//! SDL events: keys, mouse and gamepads.

use super::*;

impl Input {

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
}
