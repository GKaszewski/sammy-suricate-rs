#[allow(unused_imports)]
use super::*;

/// Icon alpha is a percentage; 100 is opaque.
const ALPHA_OPAQUE: f32 = 100.0;
/// Grey tint of a darkened (absent) level icon.
const DARK_TINT: f32 = 0.35;
/// Icons grow ×1.2 while hovered.
const HOVER_SCALE: f32 = 1.2;
/// Level icons have `ChoixMenu` codes above this (`LEVEL_ICON + slot`).
const ICON_CODE_BASE: u32 = sammy_core::menu::LEVEL_ICON;
/// `ChoixMenu` code 20: back to the root view.
const CHOICE_BACK: u32 = 20;
/// Camera `arc` in the menu (degrees).
pub(crate) const MENU_ARC: f32 = 70.0;
/// `SAMMY_MENU_CAM` carries x,y,z,pan,tilt.
const MENU_CAM_FIELDS: usize = 5;
/// `Mouse_Range`: picking reach in quants.
const MOUSE_RANGE: f32 = 6000.0;
/// `Toc2` touch / `Ding1` click volumes.
const TOUCH_VOL: f32 = 25.0;
const CLICK_VOL: f32 = 50.0;
/// Hover light colours (orange; darker for locked icons) and range.
const LIGHT_COLOR: [f32; 3] = [255.0, 180.0, 50.0];
const LIGHT_COLOR_LOCKED: [f32; 3] = [255.0, 100.0, 20.0];
const LIGHT_RANGE: f32 = 300.0;
/// `particle_interface1`: sparkle orbit radius (quants), sideways wobble and per-tick
/// angle steps (degrees).
const SPARKLE_RADIUS: f32 = 64.0;
const SPARKLE_WOBBLE: f32 = 5.0;
const SPARKLE_STEP_A: f32 = 40.0;
const SPARKLE_STEP_B: f32 = 15.0;
/// A face is a room's back wall when its normal points this far along -Y.
const WALL_NORMAL_Y: f32 = -0.99;
/// Back-wall search window around the camera marker: depth ahead, half-width, half-height.
const WALL_DEPTH: f32 = 1500.0;
const WALL_HALF_X: f32 = 700.0;
const WALL_HALF_Z: f32 = 600.0;
/// Faces within this of the farthest one belong to the back wall.
const WALL_THICKNESS: f32 = 1.0;
/// The extension quad sits this far behind the wall, and reaches this far sideways / vertically.
const WALL_QUAD_OFFSET: f32 = 2.0;
const WALL_QUAD_SIDE: f32 = 8000.0;
const WALL_QUAD_VERT: f32 = 1200.0;
/// Pad focus: minimum progress along the direction, and the weight of the sideways miss.
pub(crate) const PAD_MIN_ALONG: f32 = 1.0;
pub(crate) const PAD_CROSS_WEIGHT: f32 = 2.0;
/// Ray-box test: rays this parallel to a slab are treated as parallel, and boxes are
/// at least this thick (half extent).
const RAY_PARALLEL_EPS: f32 = 1e-8;
const MIN_HALF_EXTENT: f32 = 0.05;
/// Version text.
const VERSION_TEXT: &[u8] = b"Sammy Suricate V1.42";
const VERSION_POS: (f32, f32) = (2.0, 4.0);
/// UI stacking: version text, tooltips, cursor.
const Z_VERSION: i32 = 20;
const Z_TOOLTIP: i32 = 21;
const Z_CURSOR: i32 = 100;
/// Cursor sprite size; its hot spot is the centre.
const CURSOR_SIZE: f32 = 16.0;
const CURSOR_HOTSPOT: f32 = 8.0;
/// Tooltip offsets from the cursor: title x (unlocked / locked), and the locked message.
const TOOLTIP_DX: f32 = 20.0;
const TOOLTIP_DX_LOCKED: f32 = 32.0;
const TOOLTIP_MSG_OFFSET: (f32, f32) = (-64.0, 26.0);

pub fn in_menu(current: Res<CurrentLevel>, intro: Res<crate::ui::intro::Intro>) -> bool {
    current.phase == Phase::Menu && !intro.active
}

/// Icon looks from `LevelsInfo` (interfac.wdl:94-175): absent bonus slots vanish,
/// locked levels are 50 % ghosts (absent ones also darkened).
pub(crate) fn style_items(
    save: Res<SaveGame>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut MenuItem, &mut Visibility, Option<&mut MeshMaterial3d<StandardMaterial>>)>,
) {
    for (mut item, mut vis, mat) in &mut q {
        if item.styled {
            continue;
        }
        item.styled = true;
        let (alpha, dark) = match sammy_core::menu::look(Choice::from_code(item.code), item.alpha, &save) {
            Look::Hidden => {
                *vis = Visibility::Hidden;
                item.code = 0;
                continue;
            }
            Look::Shown { alpha } => (alpha, false),
            Look::Locked { alpha, dark } => {
                item.locked = true;
                (alpha, dark)
            }
        };
        if let Some(mut mat) = mat
            && (alpha < ALPHA_OPAQUE || dark)
        {
            let mut m = materials.get(&mat.0).cloned().unwrap_or_default();
            m.alpha_mode = AlphaMode::Blend;
            m.base_color = if dark { Color::srgba(DARK_TINT, DARK_TINT, DARK_TINT, alpha / ALPHA_OPAQUE) } else { Color::srgba(1.0, 1.0, 1.0, alpha / ALPHA_OPAQUE) };
            mat.0 = materials.add(m);
        }
    }
}

/// Hover bob and spin (`InitMainMenu :74-198`, `InitMainTitle :977-1014`).
pub(crate) fn animate_items(time: Res<Time>, state: Res<MenuState>, mut q: Query<(Entity, &mut MenuItem, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut item, mut tf) in &mut q {
        if item.bob.moves() && item.code != 0 {
            let pos = item.bob.step(t);
            tf.translation = coords::pos(pos);
            if !item.sprite {
                tf.rotation = coords::rotation(item.bob.angles);
            }
        }
        let k = if state.hovered == Some(e) && item.code > ICON_CODE_BASE && !item.locked { HOVER_SCALE } else { 1.0 };
        tf.scale = item.scale * k;
    }
}

/// Root view: `CameraPosOrigine`; level grids: hard cut to the marker (arc 70).
pub(crate) fn place_camera(
    state: Res<MenuState>,
    mut params: ResMut<CameraParams>,
    markers: Query<&MenuMarker>,
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    if params.arc != MENU_ARC {
        params.arc = MENU_ARC;
    }
    let (mut pos, mut pan, mut tilt) = match markers.iter().find(|m| m.code == state.view && state.view != 0) {
        Some(m) => (m.pos, m.pan, m.tilt),
        None => params.origin.unwrap_or(sammy_core::intro::MENU_ORIGIN),
    };
    // Debug: SAMMY_MENU_CAM=x,y,z,pan,tilt.
    if let Some(v) = std::env::var("SAMMY_MENU_CAM").ok().map(|v| v.split(',').filter_map(|n| n.parse::<f32>().ok()).collect::<Vec<_>>())
        && v.len() == MENU_CAM_FIELDS
    {
        (pos, pan, tilt) = ([v[0], v[1], v[2]], v[3], v[4]);
    }
    for mut tf in &mut cam {
        tf.translation = coords::pos(pos);
        tf.rotation = coords::rotation([pan, tilt, 0.0]);
    }
}

/// Ray from the cursor against the items' mesh boxes (`Mouse_Range`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn pick(
    mut state: ResMut<MenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    controls: Res<crate::platform::input::Controls>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut items: Query<(Entity, &mut MenuItem, &Visibility, &GlobalTransform, Option<&Aabb>, Option<&Children>)>,
    parts: Query<(&GlobalTransform, &Aabb)>,
    mut sfx: Sfx,
    mut commands: Commands,
    mut choices: MessageWriter<MenuChoice>,
    mut particles: ResMut<crate::render::particles::Particles>,
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
                    && d * coords::QUANTS_PER_UNIT < MOUSE_RANGE
                    && best.is_none_or(|(_, b)| d < b)
                {
                    best = Some((e, d));
                }
            }
        }
    }
    let mut now = best.map(|(e, _)| e);
    // A pad press only activates an item that was already focused.
    let before = state.hovered;
    if controls.pad && !state.modal {
        now = pad_focus(&controls, state.hovered, camera, cam_tf, &items);
        // B backs out of a level grid.
        if controls.back && state.view != 0 {
            choices.write(MenuChoice(CHOICE_BACK));
        }
    }
    if now != state.hovered {
        // Release: light off. Touch: `Toc2` at 25 and an orange light (darker for locked icons).
        if let Some(old) = state.hovered {
            commands.entity(old).remove::<crate::render::lights::DynamicLight>();
        }
        if let Some(new) = now
            && let Ok((_, item, ..)) = items.get(new)
        {
            sfx.play_vol("Toc2", TOUCH_VOL);
            let color = if item.locked { LIGHT_COLOR_LOCKED } else { LIGHT_COLOR };
            commands.entity(new).insert(crate::render::lights::DynamicLight { color, range: LIGHT_RANGE });
        }
        state.hovered = now;
    }
    // Sparkles orbit a hovered icon (`particle_interface1`, 64 quants out).
    if let Some(h) = state.hovered
        && let Ok((_, mut item, _, gt, ..)) = items.get_mut(h)
        && item.code > ICON_CODE_BASE
        && !item.locked
    {
        for _ in 0..particles.ticks {
            let (a, b) = (item.sparkle[0].to_radians(), item.sparkle[1].to_radians());
            let off = [
                SPARKLE_RADIUS * a.cos() * b.cos() - SPARKLE_WOBBLE * a.sin(),
                SPARKLE_RADIUS * a.sin() * b.cos() + SPARKLE_WOBBLE * a.cos(),
                SPARKLE_RADIUS * b.sin(),
            ];
            let c = coords::native(gt.translation());
            crate::render::effects::menu_sparkle(&mut particles, [c[0] + off[0], c[1] + off[1], c[2] + off[2]]);
            item.sparkle[0] += SPARKLE_STEP_A;
            item.sparkle[1] += SPARKLE_STEP_B;
        }
    }
    if (mouse.just_pressed(MouseButton::Left) || (controls.pad && controls.accept && before == state.hovered))
        && !state.modal
        && let Some(h) = state.hovered
        && let Ok((_, item, ..)) = items.get(h)
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
    level: Option<Res<crate::gameplay::player::LevelCollision>>,
    markers: Query<&MenuMarker, Added<MenuMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(level) = level else { return };
    let w = &level.0;
    for m in &markers {
        let [mx, my, mz] = m.pos;
        let faces: Vec<(usize, [f32; 3], [f32; 3])> = (0..w.faces.len())
            .filter(|&f| w.face_visible(f) && w.face_normal(f)[1] < WALL_NORMAL_Y)
            .map(|f| {
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for v in w.face_vertices(f) {
                    for i in 0..3 {
                        lo[i] = lo[i].min(v[i]);
                        hi[i] = hi[i].max(v[i]);
                    }
                }
                (f, lo, hi)
            })
            .filter(|(_, lo, hi)| lo[1] > my && lo[1] < my + WALL_DEPTH && hi[0] > mx - WALL_HALF_X && lo[0] < mx + WALL_HALF_X && hi[2] > mz - WALL_HALF_Z && lo[2] < mz + WALL_HALF_Z)
            .collect();
        let Some(wall_y) = faces.iter().map(|f| f.1[1]).reduce(f32::max) else { continue };
        let wall: Vec<_> = faces.iter().filter(|f| (f.1[1] - wall_y).abs() < WALL_THICKNESS).collect();
        let Some(&&(face, ..)) = wall.first() else { continue };
        let (Some(tex), ti) = (w.face_texture(face), &w.texinfos[w.faces[face].texinfo as usize]) else { continue };
        let (lo_x, hi_x) = wall.iter().fold((f32::MAX, f32::MIN), |a, f| (a.0.min(f.1[0]), a.1.max(f.2[0])));
        let (lo_z, hi_z) = wall.iter().fold((f32::MAX, f32::MIN), |a, f| (a.0.min(f.1[2]), a.1.max(f.2[2])));
        let t = &w.textures[tex];
        let (tw, th) = (t.width as f32, t.height as f32);
        let y = wall_y + WALL_QUAD_OFFSET;
        // Vertically only a little: the rooms lie under the menu desert.
        let corners = [
            [lo_x - WALL_QUAD_SIDE, y, lo_z - WALL_QUAD_VERT],
            [hi_x + WALL_QUAD_SIDE, y, lo_z - WALL_QUAD_VERT],
            [hi_x + WALL_QUAD_SIDE, y, hi_z + WALL_QUAD_VERT],
            [lo_x - WALL_QUAD_SIDE, y, hi_z + WALL_QUAD_VERT],
        ];
        let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners.iter().map(|&p| coords::pos(p).to_array()).collect::<Vec<_>>());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![coords::pos([0.0, -1.0, 0.0]).normalize().to_array(); 4]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, corners.iter().map(|&p| { let st = ti.st(p); [st[0] / tw, st[1] / th] }).collect::<Vec<_>>());
        mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let image = images.add(crate::render::image::to_bevy(&t.image, true, true));
        // Unlit, dimmed to roughly the wall's baked light [I].
        let mat = materials.add(StandardMaterial { base_color_texture: Some(image), base_color: Color::srgb(WALL_LIGHT, WALL_LIGHT, WALL_LIGHT), unlit: true, cull_mode: None, ..default() });
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat), Transform::default(), crate::flow::LevelScoped));
    }
}

pub(crate) const WALL_LIGHT: f32 = 0.8;

/// Gamepad focus over the menu items on screen: the D-pad / stick moves to the nearest
/// item with another code in that direction; the first press picks the one nearest the
/// screen centre.
#[allow(clippy::type_complexity)]
pub(crate) fn pad_focus(
    controls: &crate::platform::input::Controls,
    current: Option<Entity>,
    camera: &Camera,
    cam_tf: &GlobalTransform,
    items: &Query<(Entity, &mut MenuItem, &Visibility, &GlobalTransform, Option<&Aabb>, Option<&Children>)>,
) -> Option<Entity> {
    let size = camera.logical_viewport_size()?;
    let on_screen: Vec<(Entity, u32, Vec2)> = items
        .iter()
        .filter(|(_, item, vis, ..)| item.code != 0 && **vis != Visibility::Hidden)
        .filter_map(|(e, item, _, gt, ..)| {
            let p = camera.world_to_viewport(cam_tf, gt.translation()).ok()?;
            (p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y).then_some((e, item.code, p))
        })
        .collect();
    let cur = current.and_then(|c| on_screen.iter().find(|i| i.0 == c));
    let Some(&(_, code, at)) = cur else {
        if controls.nav.is_some() || controls.accept {
            return on_screen.iter().min_by(|a, b| a.2.distance(size / 2.0).total_cmp(&b.2.distance(size / 2.0))).map(|i| i.0);
        }
        return None;
    };
    let Some(d) = controls.nav_dir() else { return current };
    let dir = Vec2::new(d.x as f32, -d.y as f32);
    on_screen
        .iter()
        .filter(|i| i.1 != code)
        .filter_map(|i| {
            let v = i.2 - at;
            let along = v.dot(dir);
            (along > PAD_MIN_ALONG).then(|| (i.0, along + PAD_CROSS_WEIGHT * (v - dir * along).length()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|i| i.0)
        .or(current)
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

/// `MainMenuSammy` (interfac.wdl:403-624).
pub(crate) fn act(
    mut choices: MessageReader<MenuChoice>,
    mut state: ResMut<MenuState>,
    mut save: ResMut<SaveGame>,
    mut pending: ResMut<crate::flow::PendingLoad>,
    mut options: ResMut<crate::ui::options::OptionsRequest>,
) {
    for MenuChoice(code) in choices.read().copied() {
        match sammy_core::menu::act(Choice::from_code(code), &mut save) {
            MenuAction::StartLevel(file) => {
                crate::platform::save::write(&save);
                pending.0 = Some(file.to_owned());
            }
            MenuAction::ShowView(v) => state.view = v.code(),
            MenuAction::OpenOptions => options.open = Some(crate::ui::options::OptionsMode::Menu),
            MenuAction::Quit => options.quit = true,
            MenuAction::Nothing => {}
        }
    }
}

/// `GestionPartie2` (mm1.wdl:531-540): `NeverPlayed += 1`, then `DebutPartie`.
/// Debug: `SAMMY_MENU_VIEW=3` starts on a level grid, `SAMMY_OPTIONS=1` opens the options,
/// `SAMMY_MENU_CHOICE=n` clicks item n.
pub(crate) fn debug_start(mut state: ResMut<MenuState>, mut req: ResMut<crate::ui::options::OptionsRequest>, mut choices: MessageWriter<MenuChoice>, mut done: Local<bool>) {
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
        req.open = Some(crate::ui::options::OptionsMode::Menu);
    }
}

pub(crate) fn setup_overlay(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let abs = |x: f32, y: f32| Node { position_type: PositionType::Absolute, left: px(x), top: px(y), ..default() };
    commands.spawn((VersionText, abs(VERSION_POS.0, VERSION_POS.1), BitmapText::new(FontKind::Standard, VERSION_TEXT.to_vec()), GlobalZIndex(Z_VERSION), Visibility::Hidden));
    commands.spawn((TooltipTitle, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    commands.spawn((TooltipMessage, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    if let Some((image, _)) = load_ui_image(&files, &mut images, "arrow.pcx") {
        commands.spawn((Cursor, Node { position_type: PositionType::Absolute, width: px(CURSOR_SIZE), height: px(CURSOR_SIZE), ..default() }, ImageNode::new(image), GlobalZIndex(Z_CURSOR), Visibility::Hidden));
    }
}

/// Version text and the level tooltips that follow the cursor (interfac.wdl:840-902, :1016-1112).
#[allow(clippy::type_complexity)]
pub(crate) fn overlay(
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
    let icon = state.hovered.and_then(|h| items.get(h).ok()).filter(|i| i.code > ICON_CODE_BASE);
    let (Ok((mut tt, mut tn, mut tv)), Ok((mut mt, mut mn, mut mv))) = (title.single_mut(), message.single_mut()) else { return };
    let Some(item) = icon else {
        *tv = Visibility::Hidden;
        *mv = Visibility::Hidden;
        return;
    };
    let n = item.code - ICON_CODE_BASE;
    let text = strings.get(&format!("level{n}_Str")).to_vec();
    if tt.bytes != text {
        tt.bytes = text;
    }
    *tv = Visibility::Inherited;
    let dx = if item.locked { TOOLTIP_DX_LOCKED } else { TOOLTIP_DX };
    (tn.left, tn.top) = (px(cursor.x + dx), px(cursor.y));
    if item.locked {
        let msg = if save.state(n) == LevelState::Locked { "mustbewin_Str" } else { "LevelNothere_Str" };
        let bytes = strings.get(msg).to_vec();
        if mt.bytes != bytes {
            mt.bytes = bytes;
        }
        (mn.left, mn.top) = (px(cursor.x + TOOLTIP_MSG_OFFSET.0), px(cursor.y + TOOLTIP_MSG_OFFSET.1));
        *mv = Visibility::Inherited;
    } else {
        *mv = Visibility::Hidden;
    }
}

/// The diamond cursor (`arrow.pcx`, hot spot 8,8) replaces the system one in the menu
/// and in modal overlays.
pub(crate) fn show_cursor(
    current: Res<CurrentLevel>,
    state: Res<MenuState>,
    intro: Res<crate::ui::intro::Intro>,
    controls: Res<crate::platform::input::Controls>,
    ui_scale: Res<UiScale>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor: Query<(&mut Node, &mut Visibility), With<Cursor>>,
) {
    let Ok((win, mut opts)) = windows.single_mut() else { return };
    let on = (current.phase == Phase::Menu && !intro.active) || state.modal;
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
    current: Res<CurrentLevel>,
    intro: Res<crate::ui::intro::Intro>,
    mut hud: ResMut<crate::ui::hud::HudVisible>,
    mut texts: Query<&mut Visibility, Or<(With<VersionText>, With<TooltipTitle>, With<TooltipMessage>)>>,
) {
    let show = current.phase != Phase::Menu;
    if hud.0 != show {
        hud.0 = show;
    }
    // `Texte_Version` only appears once `MainMenuSammy` starts.
    if show || intro.active {
        for mut v in &mut texts {
            *v = Visibility::Hidden;
        }
    }
}
