//! In-game HUD: [`sammy_present::hud`]'s quads as UI image nodes (one pooled node per
//! quad), in HUD pixels scaled by [`UiScale`].

use std::collections::HashMap;

use bevy::prelude::*;
use sammy_present::hud::{DIGIT_GLYPHS, Hud, IMAGES};
use sammy_core::time::TICKS_PER_SEC as TICKS;

use crate::game::Game;
use crate::platform::vfs::GameFiles;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudVisible>().init_resource::<HudState>().add_systems(Startup, setup).add_systems(Update, update);
    }
}

/// Shown only during a game (`TypeJeu == 25`); the menu hides it.
#[derive(Resource)]
pub struct HudVisible(pub bool);

impl Default for HudVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// The HUD's clock.
#[derive(Resource, Default, Deref, DerefMut)]
struct HudState(Hud);

/// The HUD images and their sizes.
#[derive(Resource)]
struct HudImages(HashMap<&'static str, (Handle<Image>, UVec2)>);

/// Above the world, below menus and the options.
const Z_HUD: i32 = 5;

#[derive(Component)]
struct HudRoot;

/// Loads a PCX as a UI image: black is transparent (`overlay`), nearest filtering.
pub fn load_ui_image(files: &GameFiles, images: &mut Assets<Image>, name: &str) -> Option<(Handle<Image>, UVec2)> {
    let pcx = formats::pcx::parse(&files.read(name).ok()?).ok()?;
    let mut img = pcx.image;
    for px in img.data.chunks_exact_mut(4) {
        if crate::render::image::is_key(px) {
            px[3] = 0;
        }
    }
    let size = UVec2::new(img.width, img.height);
    Some((images.add(crate::render::image::to_bevy_filtered(&img, true, false, false)), size))
}

fn setup(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let loaded = IMAGES.iter().filter_map(|n| load_ui_image(&files, &mut images, n).map(|i| (*n, i))).collect();
    commands.insert_resource(HudImages(loaded));
    commands.spawn((HudRoot, Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }, GlobalZIndex(Z_HUD)));
}

/// Places one node per quad, reusing the root's children; extra ones are hidden.
#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<Game>,
    visible: Res<HudVisible>,
    images: Option<Res<HudImages>>,
    mut state: ResMut<HudState>,
    (windows, ui_scale): (Query<&Window>, Res<UiScale>),
    mut root: Query<(Entity, &mut Visibility, Option<&Children>), With<HudRoot>>,
    mut nodes: Query<(&mut Node, &mut ImageNode)>,
) {
    state.tick(&sim.game, time.delta_secs() * TICKS);
    let (Some(images), Ok((root, mut vis, children)), Ok(win)) = (images, root.single_mut(), windows.single()) else { return };
    *vis = if visible.0 { Visibility::Inherited } else { Visibility::Hidden };
    let size = [win.width() / ui_scale.0, win.height() / ui_scale.0];
    let cell = |font: &str| images.0.get(font).map_or([0.0, 0.0], |(_, s)| [(s.x / DIGIT_GLYPHS) as f32, s.y as f32]);
    let quads = state.quads(&sim.game, size, cell);
    let pool: Vec<Entity> = children.map(|c| c.iter().collect()).unwrap_or_default();
    for (n, q) in quads.iter().enumerate() {
        let Some((image, _)) = images.0.get(q.image) else { continue };
        let node = Node { position_type: PositionType::Absolute, left: px(q.dst[0]), top: px(q.dst[1]), width: px(q.dst[2]), height: px(q.dst[3]), ..default() };
        let shown = ImageNode {
            image: image.clone(),
            rect: q.src.map(|s| Rect::new(s[0], s[1], s[0] + s[2], s[1] + s[3])),
            color: Color::srgba(1.0, 1.0, 1.0, q.alpha),
            ..default()
        };
        match pool.get(n).and_then(|e| nodes.get_mut(*e).ok()) {
            Some((mut old_node, mut old_image)) => {
                *old_node = node;
                *old_image = shown;
            }
            None => {
                commands.entity(root).with_child((node, shown));
            }
        }
    }
    for e in pool.iter().skip(quads.len()) {
        if let Ok((mut node, _)) = nodes.get_mut(*e) {
            node.display = Display::None;
        }
    }
}
