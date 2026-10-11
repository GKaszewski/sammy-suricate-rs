//! Draws [`sammy_present::hud`]'s quads: the HUD images (black transparent, unfiltered)
//! scaled by the whole-number UI scale.

use std::collections::HashMap;

use glam::Mat4;
use glow::HasContext;
use sammy_core::game::GameState;
use sammy_platform::GameFiles;
use sammy_present::hud::{DIGIT_GLYPHS, Hud, IMAGES, ui_scale};

use crate::gfx::{Gfx, Layout, Mesh, Texture};

pub struct HudView {
    hud: Hud,
    images: HashMap<&'static str, Texture>,
    quad: Mesh,
}

/// A UI image from a PCX: near-black is transparent (`overlay`), sharp pixels.
pub fn load_ui_image(gfx: &Gfx, files: &GameFiles, name: &str) -> Option<Texture> {
    let mut img = formats::pcx::parse(&files.read(name).ok()?).ok()?.image;
    for px in img.data.chunks_exact_mut(4) {
        if formats::image::is_color_key(px) {
            px[3] = 0;
        }
    }
    Some(gfx.texture(&img, false, false))
}

impl HudView {
    pub fn new(gfx: &Gfx, files: &GameFiles) -> Self {
        let images = IMAGES.iter().filter_map(|n| load_ui_image(gfx, files, n).map(|t| (*n, t))).collect();
        let quad = Mesh::new(&gfx.gl, Layout::Ui, &[0.0; 16], &[0, 1, 2, 0, 2, 3], true);
        Self { hud: Hud::default(), images, quad }
    }

    pub fn tick(&mut self, game: &GameState, t: f32) {
        self.hud.tick(game, t);
    }

    /// Draws the HUD on a `width × height` pixel screen.
    pub fn draw(&self, gfx: &Gfx, game: &GameState, width: f32, height: f32, ui_size: u32) {
        let s = ui_scale(width, height, ui_size);
        let cell = |font: &str| self.images.get(font).map_or([0.0, 0.0], |t| [(t.width / DIGIT_GLYPHS) as f32, t.height as f32]);
        let quads = self.hud.quads(game, [width / s, height / s], cell);
        // HUD pixels → clip space (y down).
        let proj = Mat4::orthographic_rh_gl(0.0, width / s, height / s, 0.0, -1.0, 1.0);
        unsafe {
            gfx.gl.disable(glow::DEPTH_TEST);
            gfx.gl.disable(glow::CULL_FACE);
            gfx.gl.enable(glow::BLEND);
            gfx.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        for q in quads {
            let Some(tex) = self.images.get(q.image) else { continue };
            let [sx, sy, sw, sh] = q.src.unwrap_or([0.0, 0.0, tex.width as f32, tex.height as f32]);
            let (u0, v0, u1, v1) = (sx / tex.width as f32, sy / tex.height as f32, (sx + sw) / tex.width as f32, (sy + sh) / tex.height as f32);
            let [x, y, w, h] = q.dst;
            #[rustfmt::skip]
            let v = [x, y, u0, v0,  x + w, y, u1, v0,  x + w, y + h, u1, v1,  x, y + h, u0, v1];
            self.quad.update(&gfx.gl, &v);
            gfx.draw_ui(&self.quad, &proj, *tex, [1.0, 1.0, 1.0, q.alpha]);
        }
        unsafe { gfx.gl.disable(glow::BLEND) };
    }
}
