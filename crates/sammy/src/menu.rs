//! The 3D main menu (`menu_greenpepper.WMB`, interfac.wdl; docs/menu.md §3): title models
//! and level icons picked with the mouse, hard camera cuts to the level grids, tooltips.

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use formats::wmb::Object;

use crate::actions::ActionInit;
use crate::actions::player::TICKS;
use crate::audio::Sfx;
use crate::camera::{CameraParams, MainCamera};
use crate::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::hud::load_ui_image;
use crate::save::{SaveGame, ABSENT, LOCKED};
use crate::ui_text::{BitmapText, FontKind, WdlStrings};
use crate::vfs::GameFiles;

pub const MENU_FILE: &str = "menu_greenpepper.wmb";

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_message::<MenuChoice>()
            .add_systems(Startup, setup_overlay)
            .add_systems(Update, (debug_start, style_items, animate_items, pick, act, overlay).chain().run_if(in_menu))
            .add_systems(PostUpdate, place_camera.run_if(in_menu).before(TransformSystems::Propagate))
            .add_systems(Update, (show_cursor, hide_hud));
    }
}

pub fn in_menu(current: Res<CurrentLevel>) -> bool {
    current.phase == Phase::Menu
}

/// `InitMainMenu` / `InitMainTitle` entity (interfac.wdl:23-199, :950-1014).
#[derive(Component)]
pub struct MenuItem {
    /// Skill1 = `ChoixMenu` code (0 decor, 1 Play, 2-4 worlds, 5 Options, 6 Quit, 20 Back,
    /// 50 Bonus, 100+N level icon).
    pub code: u32,
    org: [f32; 3],
    angles: [f32; 3],
    /// Skill2-4 spin per tick, Skill7 hover amplitude (int) / damping (frac).
    spin: [f32; 3],
    hover: f32,
    damping: f32,
    z: f32,
    target: f32,
    dir: f32,
    /// `InitMainTitle` flips at 8 quants instead of 4.
    title: bool,
    sprite: bool,
    alpha: f32,
    scale: Vec3,
    styled: bool,
    /// Icon of a locked / absent level (`LevelNotReady`).
    locked: bool,
    sparkle: [f32; 2],
}

/// `Pos*Levels` camera markers (interfac.wdl:903-923).
#[derive(Component)]
pub struct MenuMarker {
    code: u32,
    pos: [f32; 3],
    pan: f32,
    tilt: f32,
}

pub fn init_item(obj: &Object, title: bool, entity: &mut EntityCommands) -> ActionInit {
    let s = &obj.skills;
    let sprite = obj.file.to_lowercase().ends_with(".pcx");
    entity.insert(MenuItem {
        code: s[0] as u32,
        org: obj.origin,
        angles: obj.angles,
        spin: [s[1], s[2], s[3]],
        hover: s[6].trunc(),
        damping: s[6] - s[6].trunc(),
        z: obj.origin[2],
        target: obj.origin[2] + s[6].trunc(),
        dir: 1.0,
        title,
        sprite,
        alpha: if obj.flags & (1 << 2) != 0 { 80.0 } else { 100.0 },
        scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs(),
        styled: false,
        locked: false,
        sparkle: [0.0; 2],
    });
    // `facing = 1`: sprites are screen-parallel.
    ActionInit { facing: sprite, ..default() }
}

pub fn init_marker(obj: &Object, code: u32, entity: &mut EntityCommands) -> ActionInit {
    entity.insert(MenuMarker { code, pos: obj.origin, pan: obj.angles[0], tilt: obj.angles[1] });
    ActionInit { hidden_but_alive: true, ..default() }
}

/// What the menu shows and what is under the cursor.
#[derive(Resource, Default)]
pub struct MenuState {
    /// 0 = root view, else the code of the level grid shown (2, 3, 4, 50).
    pub view: u32,
    hovered: Option<Entity>,
    /// `Texte_Version` is hidden by the first click (interfac.wdl:442).
    clicked_once: bool,
    /// A modal overlay (options, Yes/No) owns the mouse.
    pub modal: bool,
}

/// A click on a menu item (`ChoixMenu`).
#[derive(Message, Clone, Copy)]
pub struct MenuChoice(pub u32);

/// Icon looks from `LevelsInfo` (interfac.wdl:94-175): absent bonus slots vanish,
/// locked levels are 50 % ghosts (absent ones also darkened).
fn style_items(
    save: Res<SaveGame>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut MenuItem, &mut Visibility, Option<&mut MeshMaterial3d<StandardMaterial>>)>,
) {
    for (mut item, mut vis, mat) in &mut q {
        if item.styled {
            continue;
        }
        item.styled = true;
        let hidden = match item.code {
            1 => save.games_started == 0, // Play: hidden until the first level start ever
            50 => save.bonus_state == 0 || (save.bonus_state == 1 && !save.extro_seen),
            c if c > 100 => {
                let n = c - 100;
                let in_world = matches!(n, 1..=12 | 17..=28 | 33..=44);
                save.state(n) == ABSENT && !in_world
            }
            _ => false,
        };
        if hidden {
            *vis = Visibility::Hidden;
            item.code = 0;
            continue;
        }
        let (mut alpha, mut dark) = (item.alpha, false);
        if item.code > 100 && !save.playable(item.code - 100) {
            item.locked = true;
            alpha = 50.0;
            dark = save.state(item.code - 100) != LOCKED;
        }
        if let Some(mut mat) = mat
            && (alpha < 100.0 || dark)
        {
            let mut m = materials.get(&mat.0).cloned().unwrap_or_default();
            m.alpha_mode = AlphaMode::Blend;
            m.base_color = if dark { Color::srgba(0.35, 0.35, 0.35, alpha / 100.0) } else { Color::srgba(1.0, 1.0, 1.0, alpha / 100.0) };
            mat.0 = materials.add(m);
        }
    }
}

/// Hover bob and spin (`InitMainMenu :74-198`, `InitMainTitle :977-1014`).
fn animate_items(time: Res<Time>, state: Res<MenuState>, mut q: Query<(Entity, &mut MenuItem, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut item, mut tf) in &mut q {
        if item.hover != 0.0 && item.code != 0 {
            item.z += (item.target - item.z) * item.damping * t;
            for i in 0..3 {
                item.angles[i] += item.spin[i] * t;
            }
            let threshold = if item.title { 8.0 } else { 4.0 };
            if (item.z - item.target).abs() < threshold {
                item.dir = -item.dir;
                item.target = item.org[2] + item.hover * item.dir;
            }
            tf.translation = coords::pos([item.org[0], item.org[1], item.z]);
            if !item.sprite {
                tf.rotation = coords::rotation(item.angles);
            }
        }
        // Icons grow ×1.2 while hovered.
        let k = if state.hovered == Some(e) && item.code > 100 && !item.locked { 1.2 } else { 1.0 };
        tf.scale = item.scale * k;
    }
}

/// Root view: `CameraPosOrigine`; level grids: hard cut to the marker (arc 70).
fn place_camera(
    state: Res<MenuState>,
    mut params: ResMut<CameraParams>,
    markers: Query<&MenuMarker>,
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    if params.arc != 70.0 {
        params.arc = 70.0;
    }
    let (mut pos, mut pan, mut tilt) = match markers.iter().find(|m| m.code == state.view && state.view != 0) {
        Some(m) => (m.pos, m.pan, m.tilt),
        None => params.origin.unwrap_or(([1.0, -369.0, 115.0], 90.0, 1.0)),
    };
    // Debug: SAMMY_MENU_CAM=x,y,z,pan,tilt.
    if let Some(v) = std::env::var("SAMMY_MENU_CAM").ok().map(|v| v.split(',').filter_map(|n| n.parse::<f32>().ok()).collect::<Vec<_>>())
        && v.len() == 5
    {
        (pos, pan, tilt) = ([v[0], v[1], v[2]], v[3], v[4]);
    }
    for mut tf in &mut cam {
        tf.translation = coords::pos(pos);
        tf.rotation = coords::rotation([pan, tilt, 0.0]);
    }
}

/// Ray from the cursor against the items' mesh boxes (`Mouse_Range` 6000).
#[allow(clippy::too_many_arguments)]
fn pick(
    mut state: ResMut<MenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut items: Query<(Entity, &mut MenuItem, &Visibility, &GlobalTransform, Option<&Aabb>, Option<&Children>)>,
    parts: Query<(&GlobalTransform, &Aabb)>,
    mut sfx: Sfx,
    mut commands: Commands,
    mut choices: MessageWriter<MenuChoice>,
    mut particles: ResMut<crate::particles::Particles>,
) {
    let (Ok(win), Ok((camera, cam_tf))) = (windows.single(), cam.single()) else { return };
    let ray = if state.modal { None } else { win.cursor_position().and_then(|c| camera.viewport_to_world(cam_tf, c).ok()) };
    let mut best: Option<(Entity, f32)> = None;
    if let Some(ray) = ray {
        for (e, item, vis, gt, aabb, children) in &items {
            if item.code == 0 || *vis == Visibility::Hidden {
                continue;
            }
            let own = aabb.map(|a| (*gt, *a));
            let kids = children.into_iter().flatten().filter_map(|c| parts.get(*c).ok().map(|(g, a)| (*g, *a)));
            for (g, a) in own.into_iter().chain(kids) {
                if let Some(d) = ray_obb(ray, &g, &a)
                    && d * coords::QUANTS_PER_UNIT < 6000.0
                    && best.is_none_or(|(_, b)| d < b)
                {
                    best = Some((e, d));
                }
            }
        }
    }
    let now = best.map(|(e, _)| e);
    if now != state.hovered {
        // Release: light off. Touch: `Toc2` at 25 and an orange light (darker for locked icons).
        if let Some(old) = state.hovered {
            commands.entity(old).remove::<crate::lights::DynamicLight>();
        }
        if let Some(new) = now
            && let Ok((_, item, ..)) = items.get(new)
        {
            sfx.play_vol("Toc2", 25.0);
            let color = if item.locked { [255.0, 100.0, 20.0] } else { [255.0, 180.0, 50.0] };
            commands.entity(new).insert(crate::lights::DynamicLight { color, range: 300.0 });
        }
        state.hovered = now;
    }
    // Sparkles orbit a hovered icon (`particle_interface1`, 64 quants out).
    if let Some(h) = state.hovered
        && let Ok((_, mut item, _, gt, ..)) = items.get_mut(h)
        && item.code > 100
        && !item.locked
    {
        for _ in 0..particles.ticks {
            let (a, b) = (item.sparkle[0].to_radians(), item.sparkle[1].to_radians());
            let off = [64.0 * a.cos() * b.cos() - 5.0 * a.sin(), 64.0 * a.sin() * b.cos() + 5.0 * a.cos(), 64.0 * b.sin()];
            let c = coords::native(gt.translation());
            crate::effects::menu_sparkle(&mut particles, [c[0] + off[0], c[1] + off[1], c[2] + off[2]]);
            item.sparkle[0] += 40.0;
            item.sparkle[1] += 15.0;
        }
    }
    if mouse.just_pressed(MouseButton::Left)
        && !state.modal
        && let Some(h) = state.hovered
        && let Ok((_, item, ..)) = items.get(h)
        && !item.locked
    {
        sfx.play_vol("Ding1", 50.0);
        state.clicked_once = true;
        choices.write(MenuChoice(item.code));
    }
}

/// Slab test of a world ray against a transformed box; returns the distance.
fn ray_obb(ray: Ray3d, gt: &GlobalTransform, aabb: &Aabb) -> Option<f32> {
    let inv = gt.affine().inverse();
    let o = inv.transform_point3(ray.origin);
    let d = inv.transform_vector3(ray.direction.as_vec3());
    let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents).max(Vec3::splat(0.05)));
    let (mut t0, mut t1) = (f32::MIN, f32::MAX);
    for i in 0..3 {
        if d[i].abs() < 1e-8 {
            if (o[i] - c[i]).abs() > h[i] {
                return None;
            }
            continue;
        }
        let (a, b) = ((c[i] - h[i] - o[i]) / d[i], (c[i] + h[i] - o[i]) / d[i]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    if t1 < t0.max(0.0) {
        return None;
    }
    // Back to world distance.
    Some((gt.transform_point(o + d * t0.max(0.0)) - ray.origin).length())
}

/// `MainMenuSammy` (interfac.wdl:403-624).
fn act(
    mut choices: MessageReader<MenuChoice>,
    mut state: ResMut<MenuState>,
    mut save: ResMut<SaveGame>,
    mut pending: ResMut<crate::flow::PendingLoad>,
    mut options: ResMut<crate::options::OptionsRequest>,
) {
    for MenuChoice(code) in choices.read().copied() {
        match code {
            1 => {
                let last = save.last_level;
                start_level(&mut save, &mut pending, last);
            }
            2 | 3 | 4 | 50 => state.view = code,
            20 => state.view = 0,
            5 => options.open = Some(crate::options::OptionsMode::Menu),
            6 => options.quit = true,
            c if c > 100 => start_level(&mut save, &mut pending, c - 100),
            _ => {}
        }
    }
}

/// `GestionPartie2` (mm1.wdl:531-540): `NeverPlayed += 1`, then `DebutPartie`.
/// Debug: `SAMMY_MENU_VIEW=3` starts on a level grid, `SAMMY_OPTIONS=1` opens the options,
/// `SAMMY_MENU_CHOICE=n` clicks item n.
fn debug_start(mut state: ResMut<MenuState>, mut req: ResMut<crate::options::OptionsRequest>, mut choices: MessageWriter<MenuChoice>, mut done: Local<bool>) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    if let Some(v) = std::env::var("SAMMY_MENU_VIEW").ok().and_then(|v| v.parse().ok()) {
        state.view = v;
    }
    // SAMMY_MENU_CHOICE=105: as if that item had been clicked.
    if let Some(c) = std::env::var("SAMMY_MENU_CHOICE").ok().and_then(|v| v.parse().ok()) {
        choices.write(MenuChoice(c));
    }
    if std::env::var_os("SAMMY_OPTIONS").is_some() {
        req.open = Some(crate::options::OptionsMode::Menu);
    }
}

pub fn start_level(save: &mut SaveGame, pending: &mut crate::flow::PendingLoad, slot: u32) {
    let Some(file) = crate::flow::file_for(slot) else { return };
    save.games_started += 1;
    save.last_level = slot;
    crate::save::write(save);
    pending.0 = Some(file.to_owned());
}

// ---------------------------------------------------------------- overlay

#[derive(Component)]
struct VersionText;
#[derive(Component)]
struct TooltipTitle;
#[derive(Component)]
struct TooltipMessage;
#[derive(Component)]
struct Cursor;

fn setup_overlay(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let abs = |x: f32, y: f32| Node { position_type: PositionType::Absolute, left: px(x), top: px(y), ..default() };
    commands.spawn((VersionText, abs(2.0, 4.0), BitmapText::new(FontKind::Standard, b"Sammy Suricate V1.42".to_vec()), GlobalZIndex(20), Visibility::Hidden));
    commands.spawn((TooltipTitle, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(21), Visibility::Hidden));
    commands.spawn((TooltipMessage, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(21), Visibility::Hidden));
    if let Some((image, _)) = load_ui_image(&files, &mut images, "arrow.pcx") {
        commands.spawn((Cursor, Node { position_type: PositionType::Absolute, width: px(16.0), height: px(16.0), ..default() }, ImageNode::new(image), GlobalZIndex(100), Visibility::Hidden));
    }
}

/// Version text and the level tooltips that follow the cursor (interfac.wdl:840-902, :1016-1112).
#[allow(clippy::type_complexity)]
fn overlay(
    state: Res<MenuState>,
    strings: Option<Res<WdlStrings>>,
    save: Res<SaveGame>,
    ui_scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    items: Query<&MenuItem>,
    mut version: Query<&mut Visibility, (With<VersionText>, Without<TooltipTitle>, Without<TooltipMessage>)>,
    mut title: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipTitle>, Without<TooltipMessage>)>,
    mut message: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipMessage>, Without<TooltipTitle>)>,
) {
    let Some(strings) = strings else { return };
    if let Ok(mut v) = version.single_mut() {
        *v = if state.clicked_once || state.modal { Visibility::Hidden } else { Visibility::Inherited };
    }
    let cursor = windows.single().ok().and_then(|w| w.cursor_position()).map(|c| c / ui_scale.0).unwrap_or_default();
    let icon = state.hovered.and_then(|h| items.get(h).ok()).filter(|i| i.code > 100);
    let (Ok((mut tt, mut tn, mut tv)), Ok((mut mt, mut mn, mut mv))) = (title.single_mut(), message.single_mut()) else { return };
    let Some(item) = icon else {
        *tv = Visibility::Hidden;
        *mv = Visibility::Hidden;
        return;
    };
    let n = item.code - 100;
    let text = strings.get(&format!("level{n}_Str")).to_vec();
    if tt.bytes != text {
        tt.bytes = text;
    }
    *tv = Visibility::Inherited;
    let dx = if item.locked { 32.0 } else { 20.0 };
    (tn.left, tn.top) = (px(cursor.x + dx), px(cursor.y));
    if item.locked {
        let msg = if save.state(n) == LOCKED { "mustbewin_Str" } else { "LevelNothere_Str" };
        let bytes = strings.get(msg).to_vec();
        if mt.bytes != bytes {
            mt.bytes = bytes;
        }
        (mn.left, mn.top) = (px(cursor.x - 64.0), px(cursor.y + 26.0));
        *mv = Visibility::Inherited;
    } else {
        *mv = Visibility::Hidden;
    }
}

/// The diamond cursor (`arrow.pcx`, hot spot 8,8) replaces the system one in the menu
/// and in modal overlays.
fn show_cursor(
    current: Res<CurrentLevel>,
    state: Res<MenuState>,
    ui_scale: Res<UiScale>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor: Query<(&mut Node, &mut Visibility), With<Cursor>>,
) {
    let Ok((win, mut opts)) = windows.single_mut() else { return };
    let on = current.phase == Phase::Menu || state.modal;
    if opts.visible == on {
        opts.visible = !on;
    }
    let Ok((mut n, mut v)) = cursor.single_mut() else { return };
    match win.cursor_position().filter(|_| on) {
        Some(c) => {
            let c = c / ui_scale.0;
            (n.left, n.top) = (px(c.x - 8.0), px(c.y - 8.0));
            *v = Visibility::Inherited;
        }
        None => *v = Visibility::Hidden,
    }
}

/// The HUD only shows in a game; the menu's texts only in the menu.
#[allow(clippy::type_complexity)]
fn hide_hud(current: Res<CurrentLevel>, mut hud: ResMut<crate::hud::HudVisible>, mut texts: Query<&mut Visibility, Or<(With<VersionText>, With<TooltipTitle>, With<TooltipMessage>)>>) {
    let show = current.phase != Phase::Menu;
    if hud.0 != show {
        hud.0 = show;
    }
    if show {
        for mut v in &mut texts {
            *v = Visibility::Hidden;
        }
    }
}
