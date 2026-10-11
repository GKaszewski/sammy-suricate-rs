#[allow(unused_imports)]
use super::*;

/// Intro fog and logo alpha are percentages; the fog starts fully black.
const FULL_PERCENT: f32 = 100.0;
/// `Sonar`: the floor trace starts this far above and ends this far below the feet.
const SONAR_UP: f32 = 200.0;
const SONAR_DOWN: f32 = 2000.0;
/// Model logos (`publogo`, `surilogo`) are shown at this scale; the sprite logo at 1.
const LOGO_MODEL_SCALE: f32 = 1.5;
/// The fog quad's edge, and its distance from the camera, in quants.
const FOG_QUAD_QUANTS: f32 = 400.0;
/// Uncompressed TGA: header length, image type 2 (true colour) and the descriptor's
/// top-down bit.
const TGA_HEADER_LEN: usize = 18;
const TGA_TRUECOLOR: u8 = 2;
const TGA_TOP_DOWN_BIT: u8 = 0x20;

pub(crate) fn start(
    current: Res<CurrentLevel>,
    mut intro: ResMut<Intro>,
    mut params: ResMut<CameraParams>,
    mut walkers: Query<&mut Walker>,
    mut cam: Query<&mut FollowState, With<MainCamera>>,
) {
    if current.loads == intro.loads {
        return;
    }
    if current.phase == Phase::Menu && walkers.is_empty() && !intro.played {
        return; // the scene's entities spawn next frame
    }
    intro.loads = current.loads;
    intro.active = false;
    intro.gliding = false;
    let debug = ["SAMMY_MENU_VIEW", "SAMMY_MENU_CAM", "SAMMY_OPTIONS", "SAMMY_MENU_CHOICE", "SAMMY_NOINTRO"].iter().any(|v| std::env::var(v).is_ok());
    // Only the boot runs the intro; later menu loads leave Sammy standing (docs §2.6).
    let boot = !intro.played;
    intro.played = true;
    if !boot || current.phase != Phase::Menu || debug {
        return;
    }
    *intro = Intro { active: true, fog: FULL_PERCENT, played: true, loads: current.loads, ..default() };
    let mut pan = 0.0;
    for mut w in &mut walkers {
        w.walking = true; // `EtapesIntro = 1`
        pan = w.pan;
    }
    // `Camera_PlusTilt = 50`, arc 70, start at `CameraPosOrigine`.
    params.plus_tilt = PLUS_TILT;
    params.arc = crate::ui::menu::MENU_ARC;
    if let Ok(mut st) = cam.single_mut() {
        st.start_from(ORIGIN.0, pan, &params);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn tick(
    time: Res<Time>,
    controls: Res<crate::platform::input::Controls>,
    mouse: Res<ButtonInput<MouseButton>>,
    world: crate::gameplay::collision::CollisionWorld,
    mut intro: ResMut<Intro>,
    mut params: ResMut<CameraParams>,
    mut walkers: Query<(&mut Walker, &mut Transform, Option<&mut crate::render::model::ModelAnimator>)>,
    nodes: Query<&PathNode>,
) {
    let nodes: Vec<sammy_core::intro::PathNode> = nodes.iter().map(|n| n.0.clone()).collect();
    let skip = controls.confirm || controls.options || controls.accept || controls.back || mouse.just_pressed(MouseButton::Right);
    if intro.active && !intro.gliding && skip {
        intro.gliding = true;
    }
    let t = time.delta_secs() * TICKS;
    for (mut w, _, _) in &mut walkers {
        if !w.walking || t <= 0.0 {
            continue;
        }
        // Sub-step long frames so the node logic keeps its 1-tick resolution.
        let n = t.ceil().max(1.0);
        for _ in 0..n as u32 {
            w.step(&nodes, t / n);
        }
        if intro.active && !intro.gliding && w.key == END_KEY {
            intro.gliding = true;
        }
    }
    intro.acc += t;
    while intro.acc >= 1.0 {
        intro.acc -= 1.0;
        if !intro.active {
            continue;
        }
        intro.ticks += 1.0;
        // After the first 8 (+4) ticks: `Camera3Friction = 0.1`, vertical 0.5.
        if intro.ticks == FRICTION_AT {
            (params.fric_h, params.fric_v) = FRICTION;
        }
        let Intro { logo, fog, .. } = &mut *intro;
        sammy_core::intro::logo_tick(logo, fog);
        if intro.gliding {
            intro.fog = (intro.fog - GLIDE_FOG_CLEAR).max(0.0);
        }
    }
    // `Sonar`: keep the feet on the floor; walk cycle at 0.5 × speed % per tick.
    for (mut w, mut tf, anim) in &mut walkers {
        let [x, y, z] = w.pos;
        if world.ready() {
            let tr = world.trace_static(Hull::Point, [x, y, z + SONAR_UP], [x, y, z - SONAR_DOWN]).trace;
            if tr.hit() && !tr.start_solid {
                w.pos[2] = tr.end[2] + BLOB_HAUT;
            }
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, 0.0, 0.0]);
        if let Some(mut a) = anim {
            a.set_speed_pct(if w.walking { 0.5 * w.speed() } else { 0.0 }, TICKS);
        }
    }
}

/// `MoveMenu` (interfac.wdl:625-653): position and angles ease 0.2 per tick to the
/// menu pose; done within 2 quants in X and Z.
pub(crate) fn glide(time: Res<Time>, mut intro: ResMut<Intro>, mut cam: Query<&mut Transform, With<MainCamera>>) {
    if !intro.gliding {
        return;
    }
    let Ok(mut tf) = cam.single_mut() else { return };
    let mut pos = coords::native(tf.translation);
    let f = coords::native(*tf.forward());
    let (mut pan, mut tilt) = (f[1].atan2(f[0]).to_degrees(), f[2].atan2(f[0].hypot(f[1])).to_degrees());
    if sammy_core::intro::glide(&mut pos, &mut pan, &mut tilt, ORIGIN, time.delta_secs() * TICKS) {
        intro.active = false;
        intro.gliding = false;
        intro.fog = 0.0;
    }
    tf.translation = coords::pos(pos);
    tf.rotation = coords::rotation([pan, tilt, 0.0]);
}

/// Logo `Skill2` quants ahead of the camera at its height, facing it (`PlaceTitle`), and
/// the black fog as a near-layer quad behind it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn place_logo(
    mut commands: Commands,
    intro: Res<Intro>,
    files: Res<GameFiles>,
    assets: Option<Res<GameAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cam: Query<&Transform, (With<MainCamera>, Without<LogoView>, Without<FogQuad>)>,
    mut logos: Query<(Entity, &LogoView, &mut Transform, &MeshMaterial3d<StandardMaterial>), Without<FogQuad>>,
    mut fog: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>, &mut Visibility), With<FogQuad>>,
) {
    let Ok(cam) = cam.single() else { return };
    let shown = if intro.active { intro.logo.shown } else { 0 };
    // Respawn the entity when the logo changes.
    for (e, v, ..) in &logos {
        if v.0 != shown {
            commands.entity(e).despawn();
        }
    }
    if shown != 0 && !logos.iter().any(|(_, v, ..)| v.0 == shown) {
        let unlit = |materials: &mut Assets<StandardMaterial>, base: StandardMaterial| {
            materials.add(StandardMaterial { unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..base })
        };
        let spawned = match shown {
            1 => load_tga(&files, "slogoa5.tga").map(|img| {
                let tex = images.add(crate::render::image::to_bevy(&img, true, false));
                let size = img.width as f32 / coords::QUANTS_PER_UNIT;
                (meshes.add(Rectangle::new(size, size)), unlit(&mut materials, StandardMaterial { base_color_texture: Some(tex), ..default() }), 1.0)
            }),
            n => assets.as_ref().and_then(|a| a.model(if n == 2 { "publogo.mdl" } else { "surilogo.mdl" })).map(|m| {
                let base = materials.get(&m.skin(1)).cloned().unwrap_or_default();
                (meshes.add(m.build_mesh(0)), unlit(&mut materials, base), LOGO_MODEL_SCALE)
            }),
        };
        if let Some((mesh, mat, scale)) = spawned {
            commands.spawn((LogoView(shown), Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_scale(Vec3::splat(scale)), NEAR_LAYER, crate::flow::LevelScoped));
        }
    }
    let fwd = {
        let f = coords::native(*cam.forward());
        let a = f[1].atan2(f[0]);
        [a.cos(), a.sin(), 0.0]
    };
    let at = |d: f32| {
        let p = coords::native(cam.translation);
        coords::pos([p[0] + fwd[0] * d, p[1] + fwd[1] * d, p[2]])
    };
    for (_, v, mut tf, mat) in &mut logos {
        tf.translation = at(intro.logo.dist);
        // Sprites face the camera; models are turned to face it (like the lose title).
        tf.rotation = if v.0 == 1 { cam.rotation } else { cam.rotation * coords::rotation([0.0, 0.0, 0.0]) };
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color = m.base_color.with_alpha(intro.logo.alpha / FULL_PERCENT);
        }
    }
    // Fog quad.
    if fog.is_empty() {
        let mat = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        commands.spawn((FogQuad, Mesh3d(meshes.add(Rectangle::new(FOG_QUAD_QUANTS, FOG_QUAD_QUANTS))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden, NEAR_LAYER));
        return;
    }
    for (mut tf, mat, mut vis) in &mut fog {
        let f = if intro.active { intro.fog } else { 0.0 };
        *vis = if f > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        tf.translation = cam.translation + *cam.forward() * (FOG_QUAD_QUANTS / coords::QUANTS_PER_UNIT);
        tf.rotation = cam.rotation;
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color = Color::BLACK.with_alpha(f / FULL_PERCENT);
        }
    }
}

/// Uncompressed 24/32-bit TGA (bottom-up unless bit 5 of the descriptor); black is the
/// colour key.
pub(crate) fn load_tga(files: &GameFiles, name: &str) -> Option<formats::image::RgbaImage> {
    let d = files.read(name).ok()?;
    if d.len() < TGA_HEADER_LEN || d[2] != TGA_TRUECOLOR {
        return None;
    }
    let (w, h) = (u16::from_le_bytes([d[12], d[13]]) as u32, u16::from_le_bytes([d[14], d[15]]) as u32);
    let bpp = d[16] as usize / 8;
    let top_down = d[17] & TGA_TOP_DOWN_BIT != 0;
    let data = d.get(TGA_HEADER_LEN + d[0] as usize..)?;
    let mut img = formats::image::RgbaImage::new(w, h);
    for y in 0..h {
        let sy = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let s = ((sy * w + x) as usize) * bpp;
            let p = data.get(s..s + bpp)?;
            let rgb = [p[2], p[1], p[0]];
            let i = ((y * w + x) * 4) as usize;
            img.data[i..i + 3].copy_from_slice(&rgb);
            img.data[i + 3] = if crate::render::image::is_key(&rgb) { 0 } else { 255 };
        }
    }
    Some(img)
}
