//! A4 bitmap fonts and WDL strings (docs/hud-saves.md §2.1): 256-glyph sheets of 32
//! columns where glyph index = CP1250 byte. Strings are read from the game's own
//! scripts (`SamTexts.wdl`, `panels.wdl`) so the Polish text is byte-exact. [`layout`]
//! turns a text into glyph quads a frontend draws from the font's image.

use std::collections::HashMap;

/// The game's fonts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontKind {
    /// `standard_font` (ackfont.pcx 6×9).
    Standard,
    /// `FonteF_1216` (polish1017.pcx 10×17): tooltips, dialogs.
    Small,
    /// `Fnt_Help` (polish1320.pcx 13×20): help pages, credits.
    Help,
}

impl FontKind {
    pub const ALL: [FontKind; 3] = [FontKind::Standard, FontKind::Small, FontKind::Help];

    /// The font's sheet and glyph cell size.
    pub fn sheet(self) -> (&'static str, [f32; 2]) {
        match self {
            FontKind::Standard => ("ackfont.pcx", [6.0, 9.0]),
            FontKind::Small => ("polish1017.pcx", [10.0, 17.0]),
            FontKind::Help => ("polish1320.pcx", [13.0, 20.0]),
        }
    }
}

/// Font sheets: 256 glyph cells laid out 32 per row.
const GLYPH_COUNT: u32 = 256;
const GLYPH_COLS: u32 = 32;
/// Narrow text: an empty glyph is this fraction of the cell wide, and glyphs advance by
/// their ink plus this gap (px).
const EMPTY_GLYPH_WIDTH: f32 = 0.4;
const NARROW_GAP: f32 = 1.0;
/// The game's string scripts.
pub const STRING_FILES: &[&str] = &["SamTexts.wdl", "panels.wdl"];

/// A font's metrics: cell size and, per glyph, the first inked column and inked width
/// (for `Narrow` text).
#[derive(Clone, Debug)]
pub struct FontMetrics {
    pub cell: [f32; 2],
    ink: Vec<(f32, f32)>,
}

impl FontMetrics {
    /// From the sheet's pixels (RGBA, `width` wide); `blank(px)` says a pixel is
    /// transparent (the colour key).
    pub fn measure(kind: FontKind, rgba: &[u8], width: u32, blank: impl Fn(&[u8]) -> bool) -> Self {
        let (_, cell) = kind.sheet();
        let ink = (0..GLYPH_COUNT)
            .map(|g| {
                let (gx, gy) = ((g % GLYPH_COLS) * cell[0] as u32, (g / GLYPH_COLS) * cell[1] as u32);
                let cols: Vec<u32> = (0..cell[0] as u32)
                    .filter(|&x| {
                        (0..cell[1] as u32).any(|y| {
                            let i = (((gy + y) * width + gx + x) * 4) as usize;
                            rgba.get(i..i + 3).is_some_and(|p| !blank(p))
                        })
                    })
                    .collect();
                match (cols.first(), cols.last()) {
                    (Some(&a), Some(&b)) => (a as f32, (b - a + 1) as f32),
                    _ => (0.0, cell[0] * EMPTY_GLYPH_WIDTH),
                }
            })
            .collect();
        Self { cell, ink }
    }
}

/// One glyph to draw: `src` in the sheet and `dst` relative to the text's top-left
/// (both `[x, y, w, h]` in pixels).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub src: [f32; 4],
    pub dst: [f32; 4],
}

/// Lays out `bytes` (CP1250, `\n` breaks lines); `narrow` = A4 `NARROW` proportional
/// spacing from the glyph ink [U]. Returns the glyphs and the text's size.
pub fn layout(font: &FontMetrics, bytes: &[u8], narrow: bool) -> (Vec<Glyph>, [f32; 2]) {
    let [cw, ch] = font.cell;
    let (mut x, mut row, mut max_x) = (0.0f32, 0u32, 0.0f32);
    let mut out = Vec::new();
    for &b in bytes {
        if b == b'\n' {
            row += 1;
            x = 0.0;
            continue;
        }
        // Narrow: draw the inked columns only and advance by them + 1 px.
        let (ix, iw) = if narrow { font.ink[b as usize] } else { (0.0, cw) };
        if b != b' ' {
            let (gx, gy) = ((b as u32 % GLYPH_COLS) as f32 * cw + ix, (b as u32 / GLYPH_COLS) as f32 * ch);
            out.push(Glyph { src: [gx, gy, iw, ch], dst: [x, row as f32 * ch, iw, ch] });
        }
        x += if narrow { iw + NARROW_GAP } else { cw };
        max_x = max_x.max(x);
    }
    (out, [max_x, (row + 1) as f32 * ch])
}

/// `STRING name, "..."` declarations (lower-case names), CP1250 bytes, CRLF → `\n`.
#[derive(Clone, Debug, Default)]
pub struct WdlStrings(HashMap<String, Vec<u8>>);

impl WdlStrings {
    pub fn get(&self, name: &str) -> &[u8] {
        self.0.get(&name.to_ascii_lowercase()).map_or(&[], |v| v.as_slice())
    }

    pub fn parse(&mut self, src: &[u8]) {
        let mut i = 0;
        let lower: Vec<u8> = src.iter().map(|b| b.to_ascii_lowercase()).collect();
        while let Some(p) = find(&lower[i..], b"string") {
            let start = i + p;
            i = start + 6;
            // Must be a declaration at line start: `STRING  name, "text";`.
            let line_start = src[..start].iter().rposition(|&b| b == b'\n').map_or(0, |n| n + 1);
            if !src[line_start..start].iter().all(|b| b.is_ascii_whitespace()) {
                continue;
            }
            let Some(comma) = src[i..].iter().position(|&b| b == b',' || b == b';').map(|n| n + i) else { break };
            if src[comma] != b',' {
                continue;
            }
            let name = String::from_utf8_lossy(&src[i..comma]).trim().to_ascii_lowercase();
            let Some(q1) = src[comma..].iter().position(|&b| b == b'"').map(|n| n + comma) else { break };
            let Some(q2) = src[q1 + 1..].iter().position(|&b| b == b'"').map(|n| n + q1 + 1) else { break };
            let text: Vec<u8> = src[q1 + 1..q2].iter().copied().filter(|&b| b != b'\r').collect();
            self.0.entry(name).or_insert(text);
            i = q2 + 1;
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// UTF-8 → CP1250 (ASCII plus the Polish letters).
pub fn cp1250(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            'Ą' => 0xA5,
            'ą' => 0xB9,
            'Ć' => 0xC6,
            'ć' => 0xE6,
            'Ę' => 0xCA,
            'ę' => 0xEA,
            'Ł' => 0xA3,
            'ł' => 0xB3,
            'Ń' => 0xD1,
            'ń' => 0xF1,
            'Ó' => 0xD3,
            'ó' => 0xF3,
            'Ś' => 0x8C,
            'ś' => 0x9C,
            'Ź' => 0x8F,
            'ź' => 0x9F,
            'Ż' => 0xAF,
            'ż' => 0xBF,
            c if c.is_ascii() => c as u8,
            _ => b'?',
        })
        .collect()
}

#[cfg(test)]
mod tests;
