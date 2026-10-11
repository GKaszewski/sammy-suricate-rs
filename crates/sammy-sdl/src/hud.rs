//! Draws [`sammy_present::hud`]'s quads: the HUD images (black transparent, unfiltered)
//! scaled by the whole-number UI scale.

use std::collections::HashMap;

use sammy_core::game::GameState;
use sammy_platform::GameFiles;
use sammy_present::hud::{DIGIT_GLYPHS, Hud, IMAGES};

use crate::gfx::{Gfx, Texture};
use crate::ui::Painter;

pub struct HudView {
    hud: Hud,
    images: HashMap<&'static str, Texture>,
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
        Self { hud: Hud::default(), images }
    }

    pub fn tick(&mut self, game: &GameState, t: sammy_core::time::Ticks) {
        self.hud.tick(game, t);
    }

    /// Draws the HUD (the painter's pass is begun).
    pub fn draw(&self, gfx: &Gfx, ui: &Painter, game: &GameState) {
        let cell = |font: &str| self.images.get(font).map_or([0.0, 0.0], |t| [(t.width / DIGIT_GLYPHS) as f32, t.height as f32]);
        for q in self.hud.quads(game, ui.size, cell) {
            if let Some(tex) = self.images.get(q.image) {
                ui.image(gfx, *tex, q.src, q.dst, [1.0, 1.0, 1.0, q.alpha]);
            }
        }
    }
}
