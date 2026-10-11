//! Drawing a frame: the world, the effects over it, the intro logo, then 2D.

use glow::HasContext;
use sammy_core::flow::Phase;
use sammy_present::options::DisplayInfo;

use super::App;
use super::frame::FrameInput;
use crate::{fx, menu, ui};

impl App {
    pub(super) fn render(&mut self, input: &FrameInput, display: Option<&DisplayInfo>) {
        let (w, h) = input.size;
        let gfx = &self.gfx;
        unsafe {
            gfx.gl.viewport(0, 0, w as i32, h as i32);
            gfx.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gfx.gl.depth_mask(true);
            gfx.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
        }
        let hud_shown = self.sim.flow.phase != Phase::Menu && self.draw.contains("hud");
        if let Some(cam) = self.last_cam.as_ref().filter(|_| self.draw.contains("scene")) {
            self.scene.draw(gfx, &self.sim, cam, &self.draw);
            self.fx.draw_world(gfx, &self.files, &mut self.scene, &self.sim, cam);
            self.scene.draw_near(gfx, &self.sim, cam);
            let screen = fx::HudScreen { window: input.window_px(), ui_scale: self.painter.scale, shown: hud_shown };
            self.fx.draw_over(gfx, &self.files, &mut self.scene, &self.sim, cam, &screen);
            self.trace("scene drawn");
        }
        // The intro: black fog over the view, the logo in front of it.
        let (logo, fog) = self.last_cam.as_ref().map_or((None, 0.0), |c| sammy_present::intro::overlay(&self.sim, c.pos.into(), c.axes.x_axis.into()));
        if fog > 0.0 {
            self.painter.begin(gfx);
            self.painter.rect(gfx, [0.0, 0.0, self.painter.size[0], self.painter.size[1]], [0.0, 0.0, 0.0, fog]);
        }
        if let Some(cam) = &self.last_cam {
            self.scene.draw_logo(gfx, &self.files, cam, logo.as_ref());
        }
        self.painter.begin(gfx);
        if hud_shown {
            self.hud.draw(gfx, &self.painter, &self.sim.game);
        }
        let canvas = ui::Canvas { gfx, ui: &self.painter, fonts: &self.fonts };
        let screen = menu::Screen { scene: &self.scene, cam: self.last_cam.as_ref(), size: input.window_px() };
        self.menu.draw(&canvas, &self.sim, &screen, &input.pointer);
        self.options.draw(&canvas, &self.sim, &input.pointer, input.controls.pad, display);
        self.painter.end(gfx);
        self.mouse.show_cursor(!self.menu.owns_cursor(&self.sim));
    }
}
