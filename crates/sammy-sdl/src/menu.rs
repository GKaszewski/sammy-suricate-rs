//! The 3D main menu's interaction: mouse picking of the game's menu items, pad focus,
//! clicks, the version text, level tooltips and the cursor ([`sammy_present::menu`]).
//! The menu itself (items, views, camera, choices) is the game's.

use sammy_core::id::Id;
use sammy_core::input::Controls;
use sammy_core::sim::Sim;
use sammy_platform::GameFiles;
use sammy_present::menu::{CHOICE_BACK, CLICK_VOL, CURSOR_FILE, CURSOR_HOTSPOT, CURSOR_SIZE, MOUSE_RANGE, TOUCH_VOL, VERSION_POS, VERSION_TEXT};
use sammy_present::text::FontKind;

use crate::audio::Audio;
use crate::gfx::{Gfx, Texture};
use crate::input::Pointer;
use crate::scene::{Camera, Scene};
use crate::text::Style;
use crate::ui::Canvas;

const WHITE: [f32; 4] = [1.0; 4];
const VERSION_STYLE: Style = Style::new(FontKind::Standard);
const TOOLTIP_STYLE: Style = Style::new(FontKind::Small);

/// What the frame shows: the scene, its camera (last frame's, for picking) and the
/// window size in pixels.
pub struct Screen<'a> {
    pub scene: &'a Scene,
    pub cam: Option<&'a Camera>,
    pub size: [f32; 2],
}

pub struct MenuView {
    cursor: Option<Texture>,
    /// `Texte_Version` is hidden by the first click (interfac.wdl:442).
    clicked_once: bool,
    /// Clicked codes, applied after the frame's tick (a chosen level loads next frame).
    choices: Vec<u32>,
    /// A modal overlay (options, Yes/No) owns the input.
    pub modal: bool,
    debug_done: bool,
}

impl MenuView {
    pub fn new(gfx: &Gfx, files: &GameFiles) -> Self {
        Self { cursor: crate::hud::load_ui_image(gfx, files, CURSOR_FILE), clicked_once: false, choices: Vec::new(), modal: false, debug_done: false }
    }

    /// The menu items on screen: id, code, window-pixel rect and distance.
    fn items(sim: &Sim, scene: &Scene, cam: &Camera, size: [f32; 2]) -> Vec<(Id, u32, [f32; 4], f32)> {
        let w = &sim.world;
        w.menu_items
            .iter()
            .filter(|(id, i)| i.code != 0 && w.bodies.get(id).is_some_and(|b| !b.hidden))
            .filter_map(|(id, i)| scene.screen_rect(sim, *id, cam, size).map(|(r, d)| (*id, i.code, r, d)))
            .filter(|(_, _, r, _)| r[2] >= 0.0 && r[3] >= 0.0 && r[0] <= size[0] && r[1] <= size[1])
            .collect()
    }

    /// Hover and clicks, before the frame's tick.
    pub fn pick(&mut self, sim: &mut Sim, screen: &Screen, c: &Controls, pointer: &Pointer, audio: &mut Audio) {
        let (scene, cam, size) = (screen.scene, screen.cam, screen.size);
        if !sim.in_menu() {
            return;
        }
        // Debug: SAMMY_MENU_VIEW=3 starts on a level grid, SAMMY_MENU_CHOICE=n clicks item n.
        if !std::mem::replace(&mut self.debug_done, true) {
            if let Some(v) = std::env::var("SAMMY_MENU_VIEW").ok().and_then(|v| v.parse().ok()) {
                sim.menu.view = v;
            }
            if let Some(c) = std::env::var("SAMMY_MENU_CHOICE").ok().and_then(|v| v.parse().ok()) {
                self.choices.push(c);
            }
        }
        let Some(cam) = cam else { return };
        let items = Self::items(sim, scene, cam, size);
        let before = sim.menu.hovered;
        let mut now = None;
        if !self.modal {
            if c.pad {
                let centres: Vec<(Id, u32, [f32; 2])> = items.iter().map(|(id, code, r, _)| (*id, *code, [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0])).collect();
                now = sammy_present::menu::pad_focus(&centres, before, c.nav, c.accept, size);
                // B backs out of a level grid.
                if c.back && sim.menu.view != 0 {
                    self.choices.push(CHOICE_BACK);
                }
            } else if let Some(p) = pointer.pos {
                now = items
                    .iter()
                    .filter(|(_, _, r, d)| p[0] >= r[0] && p[0] <= r[2] && p[1] >= r[1] && p[1] <= r[3] && *d < MOUSE_RANGE)
                    .min_by(|a, b| a.3.total_cmp(&b.3))
                    .map(|i| i.0);
            }
        }
        if now != before {
            // Touch: `Toc2` at 25 (the game lights the item).
            if now.is_some() {
                audio.play("Toc2", TOUCH_VOL);
            }
            sim.menu.hovered = now;
        }
        let clicked = (pointer.clicked && !c.pad) || (c.pad && c.accept && before == sim.menu.hovered);
        if clicked
            && !self.modal
            && let Some(item) = sim.menu.hovered.and_then(|h| sim.world.menu_items.get(&h))
            && !item.locked
        {
            audio.play("Ding1", CLICK_VOL);
            self.clicked_once = true;
            self.choices.push(item.code);
        }
    }

    /// The frame's choices (`ChoixMenu`), after the tick.
    pub fn act(&mut self, sim: &mut Sim) {
        for code in self.choices.drain(..) {
            sim.menu_choice(code);
        }
    }

    /// Version text, tooltip and cursor (the painter's pass is begun).
    pub fn draw(&self, c: &Canvas, sim: &Sim, screen: &Screen, pointer: &Pointer) {
        let (gfx, ui, fonts) = (c.gfx, c.ui, c.fonts);
        let (scene, cam, window) = (screen.scene, screen.cam, screen.size);
        let s = ui.scale;
        if sim.in_menu() {
            if !self.clicked_once && !self.modal {
                fonts.draw(gfx, ui, VERSION_STYLE, VERSION_TEXT, VERSION_POS);
            }
            // The tooltip follows the pointer, or sits at the focused icon on a pad.
            let at = pointer.pos.map(|p| [p[0] / s, p[1] / s]).or_else(|| {
                let (id, cam) = (sim.menu.hovered?, cam?);
                scene.screen_rect(sim, id, cam, window).map(|(r, _)| [r[2] / s, r[1] / s])
            });
            if let (Some(at), Some(item)) = (at, sim.menu.hovered.and_then(|h| sim.world.menu_items.get(&h)))
                && let Some(tip) = sammy_present::menu::tooltip(item, &sim.save, &fonts.strings, at)
            {
                // Kept on screen (a pad's focus can be at the edge).
                let fit = |bytes: &[u8], at: [f32; 2]| {
                    let size = fonts.measure(TOOLTIP_STYLE, bytes);
                    [at[0].min(ui.size[0] - size[0]).max(0.0), at[1].min(ui.size[1] - size[1]).max(0.0)]
                };
                fonts.draw(gfx, ui, TOOLTIP_STYLE, &tip.title, fit(&tip.title, tip.title_at));
                if let Some((msg, at)) = &tip.message {
                    fonts.draw(gfx, ui, TOOLTIP_STYLE, msg, fit(msg, *at));
                }
            }
        }
        // The diamond cursor replaces the system one in the menu and in overlays.
        if (sim.in_menu() || self.modal)
            && let (Some(p), Some(tex)) = (pointer.pos, self.cursor)
        {
            ui.image(gfx, tex, None, [p[0] / s - CURSOR_HOTSPOT, p[1] / s - CURSOR_HOTSPOT, CURSOR_SIZE, CURSOR_SIZE], WHITE);
        }
    }

    /// Whether the game draws its own cursor now (the system one is hidden then).
    pub fn owns_cursor(&self, sim: &Sim) -> bool {
        sim.in_menu() || self.modal
    }
}
