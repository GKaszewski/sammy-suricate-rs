//! The game's bitmap fonts and strings ([`sammy_present::text`]) drawn with the 2D
//! painter.

use std::collections::HashMap;

use sammy_platform::GameFiles;
use sammy_present::text::{FontKind, FontMetrics, WdlStrings};

use crate::gfx::{Gfx, Texture};
use crate::ui::Painter;

/// How a text is drawn.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub font: FontKind,
    /// A4 `NARROW`: proportional spacing.
    pub narrow: bool,
    pub color: [f32; 4],
}

impl Style {
    pub const fn new(font: FontKind) -> Self {
        Self { font, narrow: false, color: [1.0; 4] }
    }

    pub const fn narrow(mut self) -> Self {
        self.narrow = true;
        self
    }
}

pub struct Fonts {
    fonts: HashMap<FontKind, (Texture, FontMetrics)>,
    pub strings: WdlStrings,
}

impl Fonts {
    pub fn new(gfx: &Gfx, files: &GameFiles) -> Self {
        let mut fonts = HashMap::new();
        for kind in FontKind::ALL {
            let (file, _) = kind.sheet();
            let Some(pcx) = files.read(file).ok().and_then(|d| formats::pcx::parse(&d).ok()) else { continue };
            let metrics = FontMetrics::measure(kind, &pcx.image.data, pcx.image.width, formats::image::is_color_key);
            if let Some(tex) = crate::hud::load_ui_image(gfx, files, file) {
                fonts.insert(kind, (tex, metrics));
            }
        }
        let mut strings = WdlStrings::default();
        for file in sammy_present::text::STRING_FILES {
            if let Ok(src) = files.read(file) {
                strings.parse(&src);
            }
        }
        Self { fonts, strings }
    }

    /// The size of `bytes` in `style` (HUD pixels).
    pub fn measure(&self, style: Style, bytes: &[u8]) -> [f32; 2] {
        self.fonts.get(&style.font).map_or([0.0; 2], |(_, m)| sammy_present::text::layout(m, bytes, style.narrow).1)
    }

    /// Draws `bytes` with its top-left at `at` (HUD pixels).
    pub fn draw(&self, gfx: &Gfx, ui: &Painter, style: Style, bytes: &[u8], at: [f32; 2]) {
        let Some((tex, metrics)) = self.fonts.get(&style.font) else { return };
        let (glyphs, _) = sammy_present::text::layout(metrics, bytes, style.narrow);
        for g in glyphs {
            let [x, y, w, h] = g.dst;
            ui.image(gfx, *tex, Some(g.src), [at[0] + x, at[1] + y, w, h], style.color);
        }
    }
}
