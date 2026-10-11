#[allow(unused_imports)]
use super::*;

use sammy_present::menu::{CHOICE_BACK, CLICK_VOL, CURSOR_HOTSPOT, CURSOR_SIZE, MOUSE_RANGE, TOUCH_VOL, VERSION_POS, VERSION_TEXT};
/// Ray-box test: rays this parallel to a slab are treated as parallel, and boxes are
/// at least this thick (half extent).
const RAY_PARALLEL_EPS: f32 = 1e-8;
const MIN_HALF_EXTENT: f32 = 0.05;
/// UI stacking: version text, tooltips, cursor.
const Z_VERSION: i32 = 20;
const Z_TOOLTIP: i32 = 21;
const Z_CURSOR: i32 = 100;
pub fn in_menu(game: Res<Game>) -> bool {
    game.in_menu()
}

/// The menu items shown (code, locked, icon) by their mirrored entities.
type Items<'w, 's> = Query<'w, 's, (&'static Mirrored, &'static Visibility, &'static GlobalTransform, Option<&'static Aabb>, Option<&'static Children>)>;

/// Ray from the cursor against the items' mesh boxes (`Mouse_Range`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn pick(
    mut game: ResMut<Game>,
    mut state: ResMut<MenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    controls: Res<crate::platform::input::Controls>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    items: Items,
    parts: Query<(&GlobalTransform, &Aabb)>,
    mut sfx: Sfx,
    mut choices: MessageWriter<MenuChoice>,
    mut particles: ResMut<crate::render::particles::Particles>,
) {
    let (Ok(win), Ok((camera, cam_tf))) = (windows.single(), cam.single()) else { return };
    let ray = if state.modal { None } else { win.cursor_position().and_then(|c| camera.viewport_to_world(cam_tf, c).ok()) };
    let menu = &game.world.menu_items;
    let mut best: Option<(Id, f32)> = None;
    if let Some(ray) = ray {
        for (m, vis, gt, aabb, children) in &items {
            if menu.get(&m.0).is_none_or(|i| i.code == 0) || *vis == Visibility::Hidden {
                continue;
            }
            let own = aabb.map(|a| (*gt, *a));
            let kids = children.into_iter().flatten().filter_map(|c| parts.get(*c).ok().map(|(g, a)| (*g, *a)));
            for (g, a) in own.into_iter().chain(kids) {
                if let Some(d) = ray_obb(ray, &g, &a)
                    && d * coords::QUANTS_PER_UNIT < MOUSE_RANGE
                    && best.is_none_or(|(_, b)| d < b)
                {
                    best = Some((m.0, d));
                }
            }
        }
    }
    let mut now = best.map(|(id, _)| id);
    // A pad press only activates an item that was already focused.
    let before = game.menu.hovered;
    if controls.pad && !state.modal {
        now = pad_focus(&controls, before, camera, cam_tf, &items, &game);
        // B backs out of a level grid.
        if controls.back && game.menu.view != 0 {
            choices.write(MenuChoice(CHOICE_BACK));
        }
    }
    if now != before {
        // Touch: `Toc2` at 25 (the game lights the item).
        if now.is_some() {
            sfx.play_vol("Toc2", TOUCH_VOL);
        }
        state.sparkle = [0.0; 2];
        game.menu.hovered = now;
    }
    let hovered = game.menu.hovered.and_then(|h| game.world.menu_items.get(&h).map(|i| (h, i.clone())));
    // Sparkles orbit a hovered icon (`particle_interface1`, 64 quants out).
    if let Some((h, item)) = &hovered
        && item.is_icon()
        && !item.locked
        && let Some((_, _, gt, ..)) = items.iter().find(|(m, ..)| m.0 == *h)
    {
        for _ in 0..particles.ticks {
            let off = sammy_present::menu::sparkle(&mut state.sparkle);
            let c = coords::native(gt.translation());
            crate::render::effects::menu_sparkle(&mut particles, [c[0] + off[0], c[1] + off[1], c[2] + off[2]]);
        }
    }
    if (mouse.just_pressed(MouseButton::Left) || (controls.pad && controls.accept && before == game.menu.hovered))
        && !state.modal
        && let Some((_, item)) = hovered
        && !item.locked
    {
        sfx.play_vol("Ding1", CLICK_VOL);
        state.clicked_once = true;
        choices.write(MenuChoice(item.code));
    }
}

/// Slab test of a world ray against a transformed box; returns the distance.
/// Port: the level-grid rooms are built only about 4:3 wide, so wide screens saw the
/// void beside them. Each room's back wall (the farthest face looking back at its camera
/// marker) is continued sideways and vertically by a quad just behind it, with the same
/// texture and texture alignment.
pub(crate) fn extend_grid_walls(
    mut commands: Commands,
    game: Res<Game>,
    mut events: MessageReader<GameEvent>,
    old: Query<Entity, With<GridWall>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !events.read().any(|e| matches!(e.0, Event::Loaded { .. })) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(level) = game.level.clone() else { return };
    let w = &*level;
    for q in sammy_present::menu::grid_walls(w, game.world.menu_markers.values()) {
        let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, q.corners.iter().map(|&p| coords::pos(p).to_array()).collect::<Vec<_>>());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![coords::pos([0.0, -1.0, 0.0]).normalize().to_array(); 4]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, q.uvs.to_vec());
        mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let image = images.add(crate::render::image::to_bevy(&w.textures[q.texture].image, true, true));
        let l = sammy_present::menu::WALL_LIGHT;
        let mat = materials.add(StandardMaterial { base_color_texture: Some(image), base_color: Color::srgb(l, l, l), unlit: true, cull_mode: None, ..default() });
        commands.spawn((GridWall, Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat), Transform::default()));
    }
}



/// Gamepad focus over the menu items on screen: the D-pad / stick moves to the nearest
/// item with another code in that direction; the first press picks the one nearest the
/// screen centre.
#[allow(clippy::type_complexity)]
pub(crate) fn pad_focus(
    controls: &crate::platform::input::Controls,
    current: Option<Id>,
    camera: &Camera,
    cam_tf: &GlobalTransform,
    items: &Items,
    game: &Game,
) -> Option<Id> {
    let size = camera.logical_viewport_size()?;
    let on_screen: Vec<(Id, u32, [f32; 2])> = items
        .iter()
        .filter(|(_, vis, ..)| **vis != Visibility::Hidden)
        .filter_map(|(m, _, gt, ..)| {
            let code = game.world.menu_items.get(&m.0).map(|i| i.code).filter(|c| *c != 0)?;
            let p = camera.world_to_viewport(cam_tf, gt.translation()).ok()?;
            (p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y).then_some((m.0, code, p.to_array()))
        })
        .collect();
    sammy_present::menu::pad_focus(&on_screen, current, controls.nav, controls.accept, size.to_array())
}

pub(crate) fn ray_obb(ray: Ray3d, gt: &GlobalTransform, aabb: &Aabb) -> Option<f32> {
    let inv = gt.affine().inverse();
    let o = inv.transform_point3(ray.origin);
    let d = inv.transform_vector3(ray.direction.as_vec3());
    let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents).max(Vec3::splat(MIN_HALF_EXTENT)));
    let (mut t0, mut t1) = (f32::MIN, f32::MAX);
    for i in 0..3 {
        if d[i].abs() < RAY_PARALLEL_EPS {
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

/// `MainMenuSammy` (interfac.wdl:403-624): the game acts on the choice.
pub(crate) fn act(mut choices: MessageReader<MenuChoice>, mut game: ResMut<Game>) {
    for MenuChoice(code) in choices.read().copied() {
        game.menu_choice(code);
    }
}

/// Debug: `SAMMY_MENU_VIEW=3` starts on a level grid, `SAMMY_OPTIONS=1` opens the options,
/// `SAMMY_MENU_CHOICE=n` clicks item n.
pub(crate) fn debug_start(mut game: ResMut<Game>, mut req: ResMut<crate::ui::options::OptionsRequest>, mut choices: MessageWriter<MenuChoice>, mut done: Local<bool>) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    if let Some(v) = std::env::var("SAMMY_MENU_VIEW").ok().and_then(|v| v.parse().ok()) {
        game.menu.view = v;
    }
    // SAMMY_MENU_CHOICE=105: as if that item had been clicked.
    if let Some(c) = std::env::var("SAMMY_MENU_CHOICE").ok().and_then(|v| v.parse().ok()) {
        choices.write(MenuChoice(c));
    }
    if std::env::var_os("SAMMY_OPTIONS").is_some() {
        req.open = Some(crate::ui::options::OptionsMode::Menu);
    }
}

pub(crate) fn setup_overlay(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let abs = |x: f32, y: f32| Node { position_type: PositionType::Absolute, left: px(x), top: px(y), ..default() };
    commands.spawn((VersionText, abs(VERSION_POS[0], VERSION_POS[1]), BitmapText::new(FontKind::Standard, VERSION_TEXT.to_vec()), GlobalZIndex(Z_VERSION), Visibility::Hidden));
    commands.spawn((TooltipTitle, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    commands.spawn((TooltipMessage, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    if let Some((image, _)) = load_ui_image(&files, &mut images, sammy_present::menu::CURSOR_FILE) {
        commands.spawn((Cursor, Node { position_type: PositionType::Absolute, width: px(CURSOR_SIZE), height: px(CURSOR_SIZE), ..default() }, ImageNode::new(image), GlobalZIndex(Z_CURSOR), Visibility::Hidden));
    }
}

/// Version text and the level tooltips that follow the cursor (interfac.wdl:840-902, :1016-1112).
#[allow(clippy::type_complexity)]
pub(crate) fn overlay(
    state: Res<MenuState>,
    game: Res<Game>,
    strings: Option<Res<Strings>>,
    ui_scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut version: Query<&mut Visibility, (With<VersionText>, Without<TooltipTitle>, Without<TooltipMessage>)>,
    mut title: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipTitle>, Without<TooltipMessage>)>,
    mut message: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipMessage>, Without<TooltipTitle>)>,
) {
    let Some(strings) = strings else { return };
    if let Ok(mut v) = version.single_mut() {
        *v = if state.clicked_once || state.modal { Visibility::Hidden } else { Visibility::Inherited };
    }
    let cursor = windows.single().ok().and_then(|w| w.cursor_position()).map(|c| c / ui_scale.0).unwrap_or_default();
    let tip = game.menu.hovered.and_then(|h| game.world.menu_items.get(&h)).and_then(|i| sammy_present::menu::tooltip(i, &game.save, &strings, cursor.to_array()));
    let (Ok((mut tt, mut tn, mut tv)), Ok((mut mt, mut mn, mut mv))) = (title.single_mut(), message.single_mut()) else { return };
    let Some(tip) = tip else {
        *tv = Visibility::Hidden;
        *mv = Visibility::Hidden;
        return;
    };
    if tt.bytes != tip.title {
        tt.bytes = tip.title;
    }
    *tv = Visibility::Inherited;
    (tn.left, tn.top) = (px(tip.title_at[0]), px(tip.title_at[1]));
    match tip.message {
        Some((bytes, at)) => {
            if mt.bytes != bytes {
                mt.bytes = bytes;
            }
            (mn.left, mn.top) = (px(at[0]), px(at[1]));
            *mv = Visibility::Inherited;
        }
        None => *mv = Visibility::Hidden,
    }
}

/// The diamond cursor (`arrow.pcx`, hot spot 8,8) replaces the system one in the menu
/// and in modal overlays.
pub(crate) fn show_cursor(
    game: Res<Game>,
    state: Res<MenuState>,
    controls: Res<crate::platform::input::Controls>,
    ui_scale: Res<UiScale>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor: Query<(&mut Node, &mut Visibility), With<Cursor>>,
) {
    let Ok((win, mut opts)) = windows.single_mut() else { return };
    let on = game.in_menu() || state.modal;
    if opts.visible == on {
        opts.visible = !on;
    }
    let Ok((mut n, mut v)) = cursor.single_mut() else { return };
    match win.cursor_position().filter(|_| on && !controls.pad) {
        Some(c) => {
            let c = c / ui_scale.0;
            (n.left, n.top) = (px(c.x - CURSOR_HOTSPOT), px(c.y - CURSOR_HOTSPOT));
            *v = Visibility::Inherited;
        }
        None => *v = Visibility::Hidden,
    }
}

/// The HUD only shows in a game; the menu's texts only in the menu.
#[allow(clippy::type_complexity)]
pub(crate) fn hide_hud(
    game: Res<Game>,
    mut hud: ResMut<crate::ui::hud::HudVisible>,
    mut texts: Query<&mut Visibility, Or<(With<VersionText>, With<TooltipTitle>, With<TooltipMessage>)>>,
) {
    let show = game.flow.phase != Phase::Menu;
    if hud.0 != show {
        hud.0 = show;
    }
    // `Texte_Version` only appears once `MainMenuSammy` starts.
    if show || game.intro.active {
        for mut v in &mut texts {
            *v = Visibility::Hidden;
        }
    }
}
