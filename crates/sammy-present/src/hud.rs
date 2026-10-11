//! The in-game HUD (`InitPanels`, panels.wdl; docs/hud-saves.md §1): the original PCX
//! plates, scroll windows and digit fonts at their 640×480 (mode 6) positions, anchored
//! to the same screen edges. [`Hud::quads`] turns the game state into image quads in
//! HUD pixels; a frontend draws them scaled by [`ui_scale`].

use sammy_core::game::{BOOST_MAX, GameState, MASTER_KEY, MAX_HEARTS, MAX_LIVES};

/// Mode-6 canvas.
pub const W: f32 = 640.0;
pub const H: f32 = 480.0;
/// Windows smaller than 640×480 still get at least this scale.
const MIN_FRACTIONAL_SCALE: f32 = 0.25;

/// The HUD's images (PCX; black is transparent, drawn unfiltered).
pub const IMAGES: &[&str] = &[
    "fondlife.pcx", "vbarhear.pcx", "fondscor.pcx", "fondgems.pcx", "hbargems.pcx", "Hbarboos.pcx", "panboost.pcx", "panboos2.pcx",
    "boink22.pcx", "boink24.pcx", "lucidd14.pcx", "boink20.pcx",
];
/// Digit fonts are one row of glyphs `0123456789:` (docs/hud-saves.md §2.2).
pub const DIGIT_GLYPHS: u32 = 11;

/// Screen edge an element keeps its mode-6 distance to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    TopCenter,
    BottomLeft,
    BottomRight,
}

/// A `w×h` element at mode-6 pixel (x, y).
#[derive(Clone, Copy, Debug)]
struct Slot {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Slot {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// Lives plate (panels.wdl `InitPanels`).
const LIVES_PLATE: Slot = Slot::new(12.0, 2.0, 88.0, 46.0);
/// Hearts window; shows `HEARTS_WINDOW_H` rows of the `vbarhear` strip.
const HEARTS: Slot = Slot::new(0.0, 2.0, 12.0, 48.0);
const HEARTS_STRIP_W: f32 = 12.0;
const HEARTS_WINDOW_H: f32 = 48.0;
/// Lives tens / ones digit positions (1 cell each).
const LIVES_TENS_POS: (f32, f32) = (46.0, 22.0);
const LIVES_ONES_POS: (f32, f32) = (62.0, 22.0);
/// Score plate, score / goal / flashing-goal digit positions and cell count.
const SCORE_PLATE: Slot = Slot::new(464.0, 4.0, 177.0, 51.0);
const SCORE_POS: (f32, f32) = (519.0, 7.0);
const GOAL_POS: (f32, f32) = (499.0, 36.0);
const GOAL_FLASH_POS: (f32, f32) = (499.0, 34.0);
const SCORE_DIGITS: u32 = 5;
/// `WarnScoreGoal`: the goal shows in boink20 while more than this many ticks remain.
const WARN_FLASH_TICKS: f32 = 4.0;
/// Key gem plate (half transparent, top-centre) and the window over the 8-slot strip.
const GEM_PLATE: Slot = Slot::new(268.0, 5.0, 103.0, 89.0);
const GEM_ICON: Slot = Slot::new(298.0, 29.0, 42.0, 42.0);
const GEM_CELL: f32 = 42.0;
/// Last slot of the key gem strip (7 = master key).
const GEM_LAST_SLOT: u32 = 7;
/// The key gem blinks with this half-period (ticks) once the goal is reached.
const GEM_BLINK_TICKS: f32 = 4.0;
/// Boost trails (half transparent), the bar window (`BOOST_MAX` columns) and the dino.
const BOOST_TRAIL_A: Slot = Slot::new(6.0, 400.0, 73.0, 40.0);
const BOOST_TRAIL_B: Slot = Slot::new(12.0, 400.0, 73.0, 40.0);
const BOOST_BAR: Slot = Slot::new(50.0, 438.0, BOOST_MAX, 8.0);
const BOOST_DINO: Slot = Slot::new(46.0, 400.0, 43.0, 40.0);
/// Ammo digit position (5 cells).
const AMMO_POS: (f32, f32) = (540.0, 433.0);
/// `transparent` panels [U: 50 %].
const HALF: f32 = 0.5;

/// An image drawn on the HUD: `src` in image pixels (None = whole image), `dst` in HUD
/// pixels from the screen's top-left (`[x, y, w, h]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub image: &'static str,
    pub src: Option<[f32; 4]>,
    pub dst: [f32; 4],
    pub alpha: f32,
}

/// HUD pixels per window pixel: whole multiples of the 640×480 art (sharp pixels);
/// `ui_size` 0 = largest, 1 = one less, ...; windows smaller than 640×480 get a
/// fractional fit.
pub fn ui_scale(width: f32, height: f32, ui_size: u32) -> f32 {
    let fit = (height / H).min(width / W);
    if fit < 1.0 { fit.max(MIN_FRACTIONAL_SCALE) } else { (fit.floor() - ui_size as f32).max(1.0) }
}

/// The HUD's own clock (the key gem blink).
#[derive(Clone, Debug, Default)]
pub struct Hud {
    blink: f32,
}

impl Hud {
    /// Advances by `t` ticks.
    pub fn tick(&mut self, game: &GameState, t: f32) {
        self.blink = if game.goal_reached { self.blink + t } else { 0.0 };
    }

    /// The quads for `game` on a screen of `size` HUD pixels; `cell(font)` gives a digit
    /// font's glyph size (from its image).
    pub fn quads(&self, game: &GameState, size: [f32; 2], cell: impl Fn(&str) -> [f32; 2]) -> Vec<Quad> {
        let mut out = Vec::new();
        let at = |anchor: Anchor, x: f32, y: f32, w: f32, h: f32| -> [f32; 4] {
            let (sx, sy) = match anchor {
                Anchor::TopLeft => (x, y),
                Anchor::TopRight => (size[0] - W + x, y),
                Anchor::TopCenter => (size[0] / 2.0 + x - W / 2.0, y),
                Anchor::BottomLeft => (x, size[1] - H + y),
                Anchor::BottomRight => (size[0] - W + x, size[1] - H + y),
            };
            [sx, sy, w, h]
        };
        let mut image = |name: &'static str, anchor: Anchor, s: Slot, src: Option<[f32; 4]>, alpha: f32| {
            out.push(Quad { image: name, src, dst: at(anchor, s.x, s.y, s.w, s.h), alpha });
        };
        // Lives plate, hearts window (rows [hearts, hearts + 48) of vbarhear).
        image("fondlife.pcx", Anchor::TopLeft, LIVES_PLATE, None, 1.0);
        let h = game.hearts.clamp(0.0, MAX_HEARTS).round();
        image("vbarhear.pcx", Anchor::TopLeft, HEARTS, Some([0.0, h, HEARTS_STRIP_W, HEARTS_WINDOW_H]), 1.0);
        image("fondscor.pcx", Anchor::TopRight, SCORE_PLATE, None, 1.0);
        // Key gem: slot = key bitmask (255 = master → slot 7); blinks once the goal is reached.
        let slot = if game.key == MASTER_KEY { GEM_LAST_SLOT } else { game.key.min(GEM_LAST_SLOT) };
        let blink_on = !game.goal_reached || (self.blink / GEM_BLINK_TICKS) as i32 % 2 == 0;
        if slot != 0 {
            image("fondgems.pcx", Anchor::TopCenter, GEM_PLATE, None, HALF);
            if blink_on {
                image("hbargems.pcx", Anchor::TopCenter, GEM_ICON, Some([slot as f32 * GEM_CELL, 0.0, GEM_CELL, GEM_CELL]), 1.0);
            }
        }
        // Boost: trails, the bar window (cols [boost, boost + 150)), the dino; shown while not empty.
        if game.boost < BOOST_MAX {
            image("panboos2.pcx", Anchor::BottomLeft, BOOST_TRAIL_A, None, HALF);
            image("panboos2.pcx", Anchor::BottomLeft, BOOST_TRAIL_B, None, HALF);
            let b = game.boost.clamp(0.0, BOOST_MAX);
            image("Hbarboos.pcx", Anchor::BottomLeft, BOOST_BAR, Some([b, 0.0, BOOST_MAX, BOOST_BAR.h]), 1.0);
            image("panboost.pcx", Anchor::BottomLeft, BOOST_DINO, None, 1.0);
        }
        // Digits: lives (tens, ones), score, goal (flashing in boink20), ammo.
        let lives = game.lives.clamp(0, MAX_LIVES) as u64;
        let flash = game.warn_score > WARN_FLASH_TICKS;
        let mut digits = |font: &'static str, anchor: Anchor, (x, y): (f32, f32), n: u32, value: u64| {
            let [cw, ch] = cell(font);
            for i in 0..n {
                let place = 10u64.pow(n - 1 - i);
                // Leading zeros are blank; the units cell always shows.
                if place > value && i + 1 < n {
                    continue;
                }
                let d = (value / place % 10) as f32;
                out.push(Quad { image: font, src: Some([d * cw, 0.0, cw, ch]), dst: at(anchor, x + i as f32 * cw, y, cw, ch), alpha: 1.0 });
            }
        };
        digits("boink22.pcx", Anchor::TopLeft, LIVES_TENS_POS, 1, lives / 10);
        digits("boink22.pcx", Anchor::TopLeft, LIVES_ONES_POS, 1, lives % 10);
        digits("boink24.pcx", Anchor::TopRight, SCORE_POS, SCORE_DIGITS, game.score.max(0) as u64);
        if flash {
            digits("boink20.pcx", Anchor::TopRight, GOAL_FLASH_POS, SCORE_DIGITS, game.goal.max(0) as u64);
        } else {
            digits("lucidd14.pcx", Anchor::TopRight, GOAL_POS, SCORE_DIGITS, game.goal.max(0) as u64);
        }
        if game.ammo > 0 {
            digits("boink22.pcx", Anchor::BottomRight, AMMO_POS, SCORE_DIGITS, game.ammo as u64);
        }
        out
    }
}

#[cfg(test)]
mod tests;
