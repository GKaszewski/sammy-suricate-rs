//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): inside the menu scene a
//! scripted Sammy walks the path-node loop with the follow camera behind him while the
//! A5, publisher and Suricate logos fade in front of the camera. Node key 999, Enter,
//! Esc or the right mouse button end it: the camera glides to the menu pose.

use crate::gameplay::collision::Hull;
use bevy::prelude::*;
use formats::wmb::Object;

use crate::gameplay::player::{BLOB_HAUT, TICKS};
use crate::camera::{CameraParams, FollowState, MainCamera};
use crate::render::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::level::{GameAssets, NEAR_LAYER};
use crate::platform::vfs::GameFiles;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Intro>()
            .add_systems(Update, (start, tick).chain())
            .add_systems(PostUpdate, (glide, place_logo).chain().after(crate::camera::follow).before(TransformSystems::Propagate));
    }
}

use sammy_core::intro::{END_KEY, FRICTION, FRICTION_AT, GLIDE_FOG_CLEAR, Logo, MENU_ORIGIN as ORIGIN, PLUS_TILT};

#[derive(Resource, Default)]
pub struct Intro {
    /// The intro owns the camera and the menu is not interactive yet.
    pub active: bool,
    /// `MoveMenu`: the camera glides back to the menu pose.
    gliding: bool,
    acc: f32,
    ticks: f32,
    /// `Camera.fog` (0..100, black).
    fog: f32,
    logo: Logo,
    /// The boot load happened (only a menu at boot plays the intro).
    played: bool,
    loads: u32,
}

impl Intro {
    /// The follow camera drives the view (not while gliding).
    pub fn following(&self) -> bool {
        self.active && !self.gliding
    }
}

/// The scripted Sammy ([`sammy_core::intro::Walker`]).
#[derive(Component, Deref, DerefMut)]
pub struct Walker(pub sammy_core::intro::Walker);

/// A path node of the intro walk ([`sammy_core::intro::PathNode`]).
#[derive(Component, Deref)]
pub struct PathNode(sammy_core::intro::PathNode);

pub fn init_walker(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((Walker(sammy_core::intro::Walker::from_object(obj)), crate::render::shadow::CastsShadow));
}

pub fn init_node(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode(sammy_core::intro::PathNode::from_object(obj)));
}

fn start(
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
    *intro = Intro { active: true, fog: 100.0, played: true, loads: current.loads, ..default() };
    let mut pan = 0.0;
    for mut w in &mut walkers {
        w.walking = true; // `EtapesIntro = 1`
        pan = w.pan;
    }
    // `Camera_PlusTilt = 50`, arc 70, start at `CameraPosOrigine`.
    params.plus_tilt = PLUS_TILT;
    params.arc = 70.0;
    if let Ok(mut st) = cam.single_mut() {
        st.start_from(ORIGIN.0, pan, &params);
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
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
            let tr = world.trace_static(Hull::Point, [x, y, z + 200.0], [x, y, z - 2000.0]).trace;
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
fn glide(time: Res<Time>, mut intro: ResMut<Intro>, mut cam: Query<&mut Transform, With<MainCamera>>) {
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

#[derive(Component)]
struct LogoView(u8);

#[derive(Component)]
struct FogQuad;

/// Logo `Skill2` quants ahead of the camera at its height, facing it (`PlaceTitle`), and
/// the black fog as a near-layer quad behind it.
#[allow(clippy::too_many_arguments)]
fn place_logo(
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
                (meshes.add(m.build_mesh(0)), unlit(&mut materials, base), 1.5)
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
            m.base_color = m.base_color.with_alpha(intro.logo.alpha / 100.0);
        }
    }
    // Fog quad.
    if fog.is_empty() {
        let mat = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        commands.spawn((FogQuad, Mesh3d(meshes.add(Rectangle::new(400.0, 400.0))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden, NEAR_LAYER));
        return;
    }
    for (mut tf, mat, mut vis) in &mut fog {
        let f = if intro.active { intro.fog } else { 0.0 };
        *vis = if f > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        tf.translation = cam.translation + *cam.forward() * (400.0 / coords::QUANTS_PER_UNIT);
        tf.rotation = cam.rotation;
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color = Color::BLACK.with_alpha(f / 100.0);
        }
    }
}

/// Uncompressed 24/32-bit TGA (bottom-up unless bit 5 of the descriptor); black is the
/// colour key.
fn load_tga(files: &GameFiles, name: &str) -> Option<formats::image::RgbaImage> {
    let d = files.read(name).ok()?;
    if d.len() < 18 || d[2] != 2 {
        return None;
    }
    let (w, h) = (u16::from_le_bytes([d[12], d[13]]) as u32, u16::from_le_bytes([d[14], d[15]]) as u32);
    let bpp = d[16] as usize / 8;
    let top_down = d[17] & 0x20 != 0;
    let data = d.get(18 + d[0] as usize..)?;
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
