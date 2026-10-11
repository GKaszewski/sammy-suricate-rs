//! The options overlay and Yes/No dialog ([`sammy_present::options`]) on screen: opened
//! with Esc / Start in a level or by the menu's Options and Quit items; its effects are
//! applied here (pause, abandon, restart, quit, settings).

use std::collections::HashMap;

use sammy_core::flow::Phase;
use sammy_core::input::Controls;
use sammy_core::sim::Sim;
use sammy_platform::GameFiles;
use sammy_present::options::{DisplayInfo, Effect, Element, FRAME_BORDER, FRAME_FILL_ALPHA, FRAME_PAD, FRAME_RGB, H, Input, Mode, OVERLAY_ALPHA, Options, Tab, W};

use crate::gfx::{Gfx, Texture};
use crate::input::Pointer;
use crate::text::Style;
use crate::ui::{Canvas, Painter};

pub struct OptionsView {
    pub model: Options,
    images: HashMap<String, Texture>,
    /// The overlay opened this frame (its opening key must not close it at once).
    just_opened: bool,
}

impl OptionsView {
    pub fn new(gfx: &Gfx, files: &GameFiles) -> Self {
        let images = Options::images().into_iter().filter_map(|n| crate::hud::load_ui_image(gfx, files, &n).map(|t| (n, t))).collect();
        Self { model: Options::default(), images, just_opened: false }
    }

    /// Opens the overlay in `mode` (debug `SAMMY_OPTIONS=audio` opens the audio tab).
    pub fn show(&mut self, mode: Mode) -> Vec<Effect> {
        self.just_opened = true;
        let tab = std::env::var("SAMMY_OPTIONS").is_ok_and(|v| v == "audio").then_some(Tab::Audio);
        self.model.show(mode, tab)
    }

    /// Esc / Start open the overlay in a level or the menu (not during the intro, where
    /// it skips instead).
    pub fn open_on_key(&mut self, sim: &Sim, c: &Controls) -> Vec<Effect> {
        if !c.options || self.model.open() || sim.intro.active {
            return Vec::new();
        }
        let mode = match sim.flow.phase {
            Phase::Menu => Mode::Menu,
            Phase::Playing if sim.flow.slot == Some(sammy_core::levels::EXTRO_SLOT) => Mode::Extro,
            Phase::Playing => Mode::Level,
            _ => return Vec::new(),
        };
        self.show(mode)
    }

    /// The overlay's frame: `pointer` in window pixels, `scale` HUD pixels per window
    /// pixel; settings changes go into the save.
    pub fn update(&mut self, sim: &mut Sim, c: &Controls, pointer: &Pointer, ui: &Painter, display: Option<&DisplayInfo>) -> Vec<Effect> {
        if std::mem::take(&mut self.just_opened) {
            return Vec::new();
        }
        let origin = Self::origin(ui);
        let input = Input {
            pointer: pointer.pos.map(|p| [p[0] / ui.scale - origin[0], p[1] / ui.scale - origin[1]]),
            pressed: pointer.clicked,
            held: pointer.held,
            released: pointer.released,
            nav: c.nav,
            accept: c.accept,
            // Esc / Enter / Start / B.
            close: c.options || c.confirm || c.back,
            yes: pointer.yes,
            no: pointer.no,
            pad: c.pad,
        };
        self.model.update(&input, &mut sim.save, display)
    }

    /// The plate's top-left in HUD pixels (centred).
    fn origin(ui: &Painter) -> [f32; 2] {
        [(ui.size[0] - W) / 2.0, (ui.size[1] - H) / 2.0]
    }

    /// Draws the overlay (the painter's pass is begun).
    pub fn draw(&self, c: &Canvas, sim: &Sim, pointer: &Pointer, pad: bool, display: Option<&DisplayInfo>) {
        let (gfx, ui, fonts) = (c.gfx, c.ui, c.fonts);
        if !self.model.open() {
            return;
        }
        if self.model.mode.is_some() {
            ui.rect(gfx, [0.0, 0.0, ui.size[0], ui.size[1]], [0.0, 0.0, 0.0, OVERLAY_ALPHA]);
        }
        let o = Self::origin(ui);
        let at = |a: [f32; 4]| [o[0] + a[0], o[1] + a[1], a[2], a[3]];
        let p = pointer.pos.map(|p| [p[0] / ui.scale - o[0], p[1] / ui.scale - o[1]]);
        for item in self.model.elements(&fonts.strings, &sim.save, display, p, pad) {
            match item.element {
                Element::Image { image, src, area } => {
                    if let Some(tex) = self.images.get(&image) {
                        ui.image(gfx, *tex, src, at(area), [1.0; 4]);
                    }
                }
                Element::Text { font, narrow, bytes, at: pos, centre_in } => {
                    let style = if narrow { Style::new(font).narrow() } else { Style::new(font) };
                    let pos = match centre_in {
                        Some(a) => {
                            let s = fonts.measure(style, &bytes);
                            [a[0] + (a[2] - s[0]) / 2.0, a[1] + (a[3] - s[1]) / 2.0]
                        }
                        None => pos,
                    };
                    fonts.draw(gfx, ui, style, &bytes, [o[0] + pos[0], o[1] + pos[1]]);
                }
            }
        }
        // A frame around the focused control while the pad is in use.
        if pad && let Some(a) = self.model.focus_area(display) {
            let [x, y, w, h] = at([a[0] - FRAME_PAD / 2.0, a[1] - FRAME_PAD / 2.0, a[2] + FRAME_PAD, a[3] + FRAME_PAD]);
            let [r, g, b] = FRAME_RGB;
            ui.rect(gfx, [x, y, w, h], [r, g, b, FRAME_FILL_ALPHA]);
            let line = [r, g, b, 1.0];
            ui.rect(gfx, [x, y, w, FRAME_BORDER], line);
            ui.rect(gfx, [x, y + h - FRAME_BORDER, w, FRAME_BORDER], line);
            ui.rect(gfx, [x, y, FRAME_BORDER, h], line);
            ui.rect(gfx, [x + w - FRAME_BORDER, y, FRAME_BORDER, h], line);
        }
    }
}
