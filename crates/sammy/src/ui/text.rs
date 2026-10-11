//! A4 bitmap fonts and WDL strings (docs/hud-saves.md §2.1): 256-glyph sheets of 32
//! columns where glyph index = CP1250 byte. Strings are read from the game's own
//! scripts (`SamTexts.wdl`, `panels.wdl`) so the Polish text is byte-exact.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::ui::hud::load_ui_image;
use crate::platform::vfs::GameFiles;

pub struct UiTextPlugin;

impl Plugin for UiTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(PostUpdate, layout_text.before(bevy::ui::UiSystems::Layout));
    }
}

pub use sammy_present::text::{FontKind, WdlStrings, cp1250};

pub struct BitmapFont {
    image: Handle<Image>,
    metrics: sammy_present::text::FontMetrics,
}

#[derive(Resource)]
pub struct BitmapFonts(HashMap<FontKind, BitmapFont>);

/// The game's strings as a resource.
#[derive(Resource, Default, Deref)]
pub struct Strings(pub WdlStrings);

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
    for kind in FontKind::ALL {
        let (file, _) = kind.sheet();
        let Some(pcx) = files.read(file).ok().and_then(|d| formats::pcx::parse(&d).ok()) else { continue };
        let metrics = sammy_present::text::FontMetrics::measure(kind, &pcx.image.data, pcx.image.width, crate::render::image::is_key);
        if let Some((image, _)) = load_ui_image(&files, &mut images, file) {
            fonts.insert(kind, BitmapFont { image, metrics });
        }
    }
    commands.insert_resource(BitmapFonts(fonts));
    let mut strings = WdlStrings::default();
    for file in sammy_present::text::STRING_FILES {
        if let Ok(src) = files.read(file) {
            strings.parse(&src);
        }
    }
    commands.insert_resource(Strings(strings));
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
        let (glyphs, size) = sammy_present::text::layout(&font.metrics, &text.bytes, text.narrow);
        commands.entity(e).with_children(|p| {
            for g in glyphs {
                let [sx, sy, sw, sh] = g.src;
                let [dx, dy, dw, dh] = g.dst;
                p.spawn((
                    Glyph,
                    Node { position_type: PositionType::Absolute, left: px(dx), top: px(dy), width: px(dw), height: px(dh), ..default() },
                    ImageNode { image: font.image.clone(), rect: Some(Rect::new(sx, sy, sx + sw, sy + sh)), ..default() },
                ));
            }
        });
        node.width = px(size[0]);
        node.height = px(size[1]);
    }
}
