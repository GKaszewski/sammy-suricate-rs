//! A4 bitmap fonts and WDL strings (docs/hud-saves.md §2.1): 256-glyph sheets of 32
//! columns where glyph index = CP1250 byte. Strings are read from the game's own
//! scripts (`SamTexts.wdl`, `panels.wdl`) so the Polish text is byte-exact.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::hud::load_ui_image;
use crate::vfs::GameFiles;

pub struct UiTextPlugin;

impl Plugin for UiTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(PostUpdate, layout_text.before(bevy::ui::UiSystems::Layout));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontKind {
    /// `standard_font` (ackfont.pcx 6×9).
    Standard,
    /// `FonteF_1216` (polish1017.pcx 10×17): tooltips, dialogs.
    Small,
    /// `Fnt_Help` (polish1320.pcx 13×20): help pages, credits.
    Help,
}

pub struct BitmapFont {
    image: Handle<Image>,
    cell: Vec2,
    /// Per glyph: first inked column and inked width (for `Narrow` text).
    ink: Vec<(f32, f32)>,
}

#[derive(Resource)]
pub struct BitmapFonts(HashMap<FontKind, BitmapFont>);

/// `STRING name, "..."` declarations (lower-case names), CP1250 bytes, CRLF → `\n`.
#[derive(Resource, Default)]
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
#[cfg_attr(not(test), allow(dead_code))]
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

/// A text drawn with a bitmap font; glyph children are rebuilt when it changes.
#[derive(Component, Clone)]
pub struct BitmapText {
    pub font: FontKind,
    pub bytes: Vec<u8>,
    /// A4 `NARROW`: proportional spacing from the glyph ink (help pages, credits) [U].
    pub narrow: bool,
}

impl BitmapText {
    pub fn new(font: FontKind, bytes: impl Into<Vec<u8>>) -> Self {
        Self { font, bytes: bytes.into(), narrow: false }
    }

    pub fn narrow(mut self) -> Self {
        self.narrow = true;
        self
    }
}

#[derive(Component)]
struct Glyph;

fn setup(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let mut fonts = HashMap::new();
    for (kind, file, cell) in [
        (FontKind::Standard, "ackfont.pcx", Vec2::new(6.0, 9.0)),
        (FontKind::Small, "polish1017.pcx", Vec2::new(10.0, 17.0)),
        (FontKind::Help, "polish1320.pcx", Vec2::new(13.0, 20.0)),
    ] {
        let Some(pcx) = files.read(file).ok().and_then(|d| formats::pcx::parse(&d).ok()) else { continue };
        let ink = (0..256u32)
            .map(|g| {
                let (gx, gy) = ((g % 32) * cell.x as u32, (g / 32) * cell.y as u32);
                let cols: Vec<u32> = (0..cell.x as u32)
                    .filter(|&x| {
                        (0..cell.y as u32).any(|y| {
                            let i = (((gy + y) * pcx.image.width + gx + x) * 4) as usize;
                            pcx.image.data.get(i..i + 3).is_some_and(|p| p != [0, 0, 0])
                        })
                    })
                    .collect();
                match (cols.first(), cols.last()) {
                    (Some(&a), Some(&b)) => (a as f32, (b - a + 1) as f32),
                    _ => (0.0, cell.x * 0.4),
                }
            })
            .collect();
        if let Some((image, _)) = load_ui_image(&files, &mut images, file) {
            fonts.insert(kind, BitmapFont { image, cell, ink });
        }
    }
    commands.insert_resource(BitmapFonts(fonts));
    let mut strings = WdlStrings::default();
    for file in ["SamTexts.wdl", "panels.wdl"] {
        if let Ok(src) = files.read(file) {
            strings.parse(&src);
        }
    }
    commands.insert_resource(strings);
}

fn layout_text(mut commands: Commands, fonts: Option<Res<BitmapFonts>>, mut q: Query<(Entity, &BitmapText, &mut Node, Option<&Children>), Changed<BitmapText>>, glyphs: Query<(), With<Glyph>>) {
    let Some(fonts) = fonts else { return };
    for (e, text, mut node, children) in &mut q {
        for c in children.into_iter().flatten() {
            if glyphs.contains(*c) {
                commands.entity(*c).despawn();
            }
        }
        let Some(font) = fonts.0.get(&text.font) else { continue };
        let (cw, ch) = (font.cell.x, font.cell.y);
        let (mut x, mut row, mut max_x) = (0.0f32, 0u32, 0.0f32);
        commands.entity(e).with_children(|p| {
            for &b in &text.bytes {
                if b == b'\n' {
                    row += 1;
                    x = 0.0;
                    continue;
                }
                // Narrow: draw the inked columns only and advance by them + 1 px.
                let (ix, iw) = if text.narrow { font.ink[b as usize] } else { (0.0, cw) };
                if b != b' ' {
                    let (gx, gy) = ((b % 32) as f32 * cw + ix, (b / 32) as f32 * ch);
                    p.spawn((
                        Glyph,
                        Node { position_type: PositionType::Absolute, left: px(x), top: px(row as f32 * ch), width: px(iw), height: px(ch), ..default() },
                        ImageNode { image: font.image.clone(), rect: Some(Rect::new(gx, gy, gx + iw, gy + ch)), ..default() },
                    ));
                }
                x += if text.narrow { iw + 1.0 } else { cw };
                max_x = max_x.max(x);
            }
        });
        node.width = px(max_x);
        node.height = px((row + 1) as f32 * ch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wdl_strings() {
        let mut s = WdlStrings::default();
        s.parse(b"STRING  QuitYesNo_Str,   \"Wyj\x9Ccie?\";\r\n// STRING fake, \"x\";\r\nSTRING  Two,\r\n\"a\r\nb\";\r\n");
        assert_eq!(s.get("quityesno_str"), b"Wyj\x9Ccie?");
        assert_eq!(s.get("Two"), b"a\nb");
        assert!(s.get("fake").is_empty());
        assert_eq!(cp1250("Wyjście?"), b"Wyj\x9Ccie?".to_vec());
    }
}
