//! In-game HUD (`InitPanels`, panels.wdl; docs/hud-saves.md §1): the original PCX
//! plates, scroll windows and digit fonts at their 640×480 (mode 6) positions, scaled
//! uniformly to the window height and anchored to the same screen edges.

use bevy::prelude::*;

use crate::gameplay::player::TICKS;
use crate::gameplay::state::{GameState, MASTER_KEY};
use crate::platform::vfs::GameFiles;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudVisible>().add_systems(Startup, setup).add_systems(Update, update);
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

/// Mode-6 canvas.
pub const W: f32 = 640.0;
pub const H: f32 = 480.0;

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

/// Screen edge a HUD element keeps its mode-6 distance to.
#[derive(Clone, Copy)]
pub enum Anchor {
    TopLeft,
    TopRight,
    TopCenter,
    BottomLeft,
    BottomRight,
}

/// Node for a `w×h` element drawn at mode-6 pixel (x, y).
pub fn place(anchor: Anchor, x: f32, y: f32, w: f32, h: f32) -> Node {
    let mut n = Node { position_type: PositionType::Absolute, width: px(w), height: px(h), ..default() };
    match anchor {
        Anchor::TopLeft => (n.left, n.top) = (px(x), px(y)),
        Anchor::TopRight => (n.right, n.top) = (px(W - x - w), px(y)),
        Anchor::TopCenter => {
            n.left = percent(50);
            n.margin.left = px(x - W / 2.0);
            n.top = px(y);
        }
        Anchor::BottomLeft => (n.left, n.bottom) = (px(x), px(H - y - h)),
        Anchor::BottomRight => (n.right, n.bottom) = (px(W - x - w), px(H - y - h)),
    }
    n
}

/// One-row digit font: glyphs `0123456789:` (docs/hud-saves.md §2.2).
#[derive(Clone)]
pub struct DigitFont {
    pub image: Handle<Image>,
    pub cell: Vec2,
}

impl DigitFont {
    fn load(files: &GameFiles, images: &mut Assets<Image>, name: &str) -> Option<Self> {
        let (image, size) = load_ui_image(files, images, name)?;
        Some(Self { image, cell: Vec2::new(size.x as f32 / 11.0, size.y as f32) })
    }

    fn glyph(&self, d: u32) -> Rect {
        Rect::new(d as f32 * self.cell.x, 0.0, (d + 1) as f32 * self.cell.x, self.cell.y)
    }
}

/// `DIGITS x,y,n,font`: n right-aligned cells without leading zeros (seen in the
/// original: score "0", goal "540"), low n digits kept.
#[derive(Component)]
pub struct Digits {
    pub n: u32,
    pub value: i64,
}

#[derive(Component)]
struct DigitCell(u32);

pub fn spawn_digits(parent: &mut ChildSpawnerCommands, font: &DigitFont, anchor: Anchor, x: f32, y: f32, n: u32, marker: impl Bundle) {
    let w = font.cell.x * n as f32;
    parent
        .spawn((place(anchor, x, y, w, font.cell.y), Digits { n, value: 0 }, marker))
        .with_children(|row| {
            for i in 0..n {
                row.spawn((
                    Node { position_type: PositionType::Absolute, left: px(i as f32 * font.cell.x), width: px(font.cell.x), height: px(font.cell.y), ..default() },
                    ImageNode { image: font.image.clone(), rect: Some(font.glyph(0)), ..default() },
                    DigitCell(i),
                ));
            }
        });
}

fn update_digits<F: bevy::ecs::query::QueryFilter>(digits: &Query<(&Digits, &Children)>, cells: &mut Query<(&DigitCell, &mut ImageNode), F>, fonts: &HudFonts) {
    for (d, children) in digits.iter() {
        for c in children.iter() {
            let Ok((cell, mut img)) = cells.get_mut(c) else { continue };
            let place = 10u64.pow(d.n - 1 - cell.0);
            let value = d.value.max(0) as u64;
            let digit = (value / place % 10) as u32;
            // Leading zeros are blank; the units cell always shows.
            let blank = place > value && cell.0 + 1 < d.n;
            img.color = if blank { Color::NONE } else { Color::WHITE };
            let font = fonts.by_cell(img.image.id());
            if let Some(f) = font {
                img.rect = Some(f.glyph(digit));
            }
        }
    }
}

#[derive(Resource)]
struct HudFonts(Vec<DigitFont>);

impl HudFonts {
    fn by_cell(&self, id: AssetId<Image>) -> Option<&DigitFont> {
        self.0.iter().find(|f| f.image.id() == id)
    }
}

#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct Hearts;
#[derive(Component)]
struct LivesTens;
#[derive(Component)]
struct LivesOnes;
#[derive(Component)]
struct Score;
#[derive(Component)]
struct Goal;
#[derive(Component)]
struct GoalFlash;
#[derive(Component)]
struct GemPlate;
#[derive(Component)]
struct GemIcon;
#[derive(Component)]
struct BoostPart;
#[derive(Component)]
struct BoostBar;
#[derive(Component)]
struct Ammo;

fn setup(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let mut img = |name: &str| load_ui_image(&files, &mut images, name).map(|(h, _)| h).unwrap_or_default();
    let (fondlife, vbarhear, fondscor, fondgems, hbargems, hbarboos, panboost, panboos2) = (
        img("fondlife.pcx"),
        img("vbarhear.pcx"),
        img("fondscor.pcx"),
        img("fondgems.pcx"),
        img("hbargems.pcx"),
        img("Hbarboos.pcx"),
        img("panboost.pcx"),
        img("panboos2.pcx"),
    );
    let font = |name: &str, images: &mut Assets<Image>| DigitFont::load(&files, images, name);
    let (Some(boink22), Some(boink24), Some(lucidd14), Some(boink20)) =
        (font("boink22.pcx", &mut images), font("boink24.pcx", &mut images), font("lucidd14.pcx", &mut images), font("boink20.pcx", &mut images))
    else {
        warn!("HUD fonts missing");
        return;
    };
    let half = Color::srgba(1.0, 1.0, 1.0, 0.5); // `transparent` panels [U: 50 %]
    commands
        .spawn((HudRoot, Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }, GlobalZIndex(5)))
        .with_children(|root| {
            // Lives plate, hearts window (rows [hearts, hearts+48) of vbarhear) and lives digits.
            root.spawn((place(Anchor::TopLeft, 12.0, 2.0, 88.0, 46.0), ImageNode::new(fondlife)));
            root.spawn((
                place(Anchor::TopLeft, 0.0, 2.0, 12.0, 48.0),
                ImageNode { image: vbarhear, rect: Some(Rect::new(0.0, 48.0, 12.0, 96.0)), ..default() },
                Hearts,
            ));
            spawn_digits(root, &boink22, Anchor::TopLeft, 46.0, 22.0, 1, LivesTens);
            spawn_digits(root, &boink22, Anchor::TopLeft, 62.0, 22.0, 1, LivesOnes);
            // Score plate, score and goal digits; the goal flashes in boink20 (WarnScoreGoal).
            root.spawn((place(Anchor::TopRight, 464.0, 4.0, 177.0, 51.0), ImageNode::new(fondscor)));
            spawn_digits(root, &boink24, Anchor::TopRight, 519.0, 7.0, 5, Score);
            spawn_digits(root, &lucidd14, Anchor::TopRight, 499.0, 36.0, 5, Goal);
            spawn_digits(root, &boink20, Anchor::TopRight, 499.0, 34.0, 5, GoalFlash);
            // Key gem: plate (transparent) and a 42×42 window over the 8-slot strip.
            root.spawn((place(Anchor::TopCenter, 268.0, 5.0, 103.0, 89.0), ImageNode { image: fondgems, color: half, ..default() }, GemPlate));
            root.spawn((place(Anchor::TopCenter, 298.0, 29.0, 42.0, 42.0), ImageNode { image: hbargems, rect: Some(Rect::new(0.0, 0.0, 42.0, 42.0)), ..default() }, GemIcon));
            // Boost: trails (transparent), the bar window, then the dino on top.
            root.spawn((place(Anchor::BottomLeft, 6.0, 400.0, 73.0, 40.0), ImageNode { image: panboos2.clone(), color: half, ..default() }, BoostPart));
            root.spawn((place(Anchor::BottomLeft, 12.0, 400.0, 73.0, 40.0), ImageNode { image: panboos2, color: half, ..default() }, BoostPart));
            root.spawn((
                place(Anchor::BottomLeft, 50.0, 438.0, 150.0, 8.0),
                ImageNode { image: hbarboos, rect: Some(Rect::new(0.0, 0.0, 150.0, 8.0)), ..default() },
                BoostPart,
                BoostBar,
            ));
            root.spawn((place(Anchor::BottomLeft, 46.0, 400.0, 43.0, 40.0), ImageNode::new(panboost), BoostPart));
            spawn_digits(root, &boink22, Anchor::BottomRight, 540.0, 433.0, 5, Ammo);
        });
    commands.insert_resource(HudFonts(vec![boink22, boink24, lucidd14, boink20]));
}

#[allow(clippy::type_complexity)]
fn update(
    time: Res<Time>,
    mut game: ResMut<GameState>,
    visible: Res<HudVisible>,
    fonts: Option<Res<HudFonts>>,
    mut blink: Local<f32>,
    mut root: Query<&mut Visibility, With<HudRoot>>,
    mut digits: ParamSet<(
        Query<&mut Digits, With<LivesTens>>,
        Query<&mut Digits, With<LivesOnes>>,
        Query<&mut Digits, With<Score>>,
        Query<(&mut Digits, &mut Node), With<Goal>>,
        Query<(&mut Digits, &mut Node), With<GoalFlash>>,
        Query<(&mut Digits, &mut Node), With<Ammo>>,
        Query<(&Digits, &Children)>,
    )>,
    mut cells: Query<(&DigitCell, &mut ImageNode), (Without<Hearts>, Without<GemIcon>, Without<BoostBar>)>,
    mut images: ParamSet<(
        Query<&mut ImageNode, With<Hearts>>,
        Query<(&mut ImageNode, &mut Node), (With<GemIcon>, Without<Digits>)>,
        Query<&mut Node, (With<GemPlate>, Without<Digits>)>,
        Query<&mut Node, (With<BoostPart>, Without<Digits>)>,
        Query<&mut ImageNode, With<BoostBar>>,
    )>,
) {
    let t = time.delta_secs() * TICKS;
    game.warn_score = (game.warn_score - t).max(0.0);
    if let Ok(mut v) = root.single_mut() {
        *v = if visible.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
    let show = |n: &mut Node, on: bool| n.display = if on { Display::Flex } else { Display::None };

    // Lives: tens and ones digit (`DIGITS` factor 0.051 and 1).
    let lives = game.lives.clamp(0, 19) as i64;
    if let Ok(mut d) = digits.p0().single_mut() {
        d.value = lives / 10;
    }
    if let Ok(mut d) = digits.p1().single_mut() {
        d.value = lives % 10;
    }
    if let Ok(mut d) = digits.p2().single_mut() {
        d.value = game.score as i64;
    }
    // WarnScoreGoal: 4 ticks in boink20, 4 ticks normal.
    let flash = game.warn_score > 4.0;
    if let Ok((mut d, mut n)) = digits.p3().single_mut() {
        d.value = game.goal as i64;
        show(&mut n, !flash);
    }
    if let Ok((mut d, mut n)) = digits.p4().single_mut() {
        d.value = game.goal as i64;
        show(&mut n, flash);
    }
    if let Ok((mut d, mut n)) = digits.p5().single_mut() {
        d.value = game.ammo as i64;
        show(&mut n, game.ammo > 0);
    }

    // Hearts window: rows [hearts, hearts + 48).
    if let Ok(mut img) = images.p0().single_mut() {
        let h = game.hearts.clamp(0.0, 48.0).round();
        img.rect = Some(Rect::new(0.0, h, 12.0, h + 48.0));
    }
    // Key gem: slot = key bitmask (255 = master → slot 7); blinks every ~4 ticks once the goal is reached.
    // `WindowKeyPlayer1`: the key actually held (7 = master).
    let slot = if game.key == MASTER_KEY { 7 } else { game.key.min(7) };
    *blink = if game.goal_reached { *blink + t } else { 0.0 };
    let blink_on = !game.goal_reached || (*blink / 4.0) as i32 % 2 == 0;
    if let Ok((mut img, mut n)) = images.p1().single_mut() {
        img.rect = Some(Rect::new(slot as f32 * 42.0, 0.0, slot as f32 * 42.0 + 42.0, 42.0));
        show(&mut n, slot != 0 && blink_on);
    }
    if let Ok(mut n) = images.p2().single_mut() {
        show(&mut n, slot != 0);
    }
    // Boost: visible while not empty; the window shows cols [boost, boost + 150).
    let boost_on = game.boost < 150.0;
    for mut n in &mut images.p3() {
        show(&mut n, boost_on);
    }
    if let Ok(mut img) = images.p4().single_mut() {
        let b = game.boost.clamp(0.0, 150.0);
        img.rect = Some(Rect::new(b, 0.0, b + 150.0, 8.0));
    }
    if let Some(fonts) = fonts {
        update_digits(&digits.p6(), &mut cells, &fonts);
    }
}
