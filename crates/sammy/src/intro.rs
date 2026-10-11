//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): inside the menu scene a
//! scripted Sammy walks the path-node loop with the follow camera behind him while the
//! A5, publisher and Suricate logos fade in front of the camera. Node key 999, Enter,
//! Esc or the right mouse button end it: the camera glides to the menu pose.

use bevy::prelude::*;
use formats::wmb::Object;

use crate::actions::player::{BLOB_HAUT, TICKS};
use crate::camera::{CameraParams, FollowState, MainCamera};
use crate::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::level::{GameAssets, NEAR_LAYER};
use crate::vfs::GameFiles;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Intro>()
            .add_systems(Update, (start, tick).chain())
            .add_systems(PostUpdate, (glide, place_logo).chain().after(crate::camera::follow).before(TransformSystems::Propagate));
    }
}

/// `CameraPosOrigine` / `CameraAngleOrigine` of the menu (`InitGame2`).
const ORIGIN: ([f32; 3], f32, f32) = ([1.0, -369.0, 115.0], 90.0, 1.0);

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

/// `DisplayLogoA5` (Intro2.wdl:924-1181): one entity morphed through three logos.
#[derive(Default)]
struct Logo {
    step: usize,
    /// Ticks spent in the current step.
    t: f32,
    /// `Skill2`: distance ahead of the camera; `Skill6`: fade counter.
    dist: f32,
    s6: f32,
    alpha: f32,
    /// 0 none, 1 A5 sprite, 2 publisher, 3 Suricate.
    shown: u8,
}

/// The scripted Sammy (`InitPlayerPatrol` :231-329, `MoveSammyonPath` :412-456).
#[derive(Component)]
pub struct Walker {
    pub pos: [f32; 3],
    pub pan: f32,
    pub vel: [f32; 3],
    dest_pan: f32,
    /// Skill2 `SpeedBalle` (15) and Skill5 `BlobZoneScan` (600).
    speed: f32,
    range: f32,
    /// `PlayerPatrolEtat` and the scanned node (x, y, pan).
    etat: u8,
    target: Option<[f32; 3]>,
    /// `PlayerKey` of the last node scanned.
    pub key: f32,
    walking: bool,
    snapped: bool,
}

/// `InitPathPlayer` (:392-398): invisible node, Skill4 = key, pan = way to the next.
#[derive(Component)]
pub struct PathNode {
    pos: [f32; 3],
    pan: f32,
    key: f32,
}

pub fn init_walker(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    entity.insert((
        Walker {
            pos: obj.origin,
            pan: obj.angles[0],
            vel: [0.0; 3],
            dest_pan: obj.angles[0],
            speed: if s[1] != 0.0 { s[1] } else { 15.0 },
            range: if s[4] != 0.0 { s[4] } else { 600.0 },
            etat: 0,
            target: None,
            key: 1.0,
            walking: false,
            snapped: false,
        },
        crate::shadow::CastsShadow,
    ));
}

pub fn init_node(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode { pos: obj.origin, pan: obj.angles[0], key: obj.skills[3] });
}

fn ang(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

/// `SCAN` with a horizontal cone: the nearest node within `range` and `half` degrees of
/// `pan` (all hit nodes set the target; the nearest is assumed to win [U]).
fn scan<'a>(nodes: &[(&'a PathNode,)], from: [f32; 3], pan: f32, half: f32, range: f32) -> Option<&'a PathNode> {
    nodes
        .iter()
        .map(|(n,)| *n)
        .filter_map(|n| {
            let (dx, dy) = (n.pos[0] - from[0], n.pos[1] - from[1]);
            let d = dx.hypot(dy);
            (d <= range && (d < 1.0 || ang(dy.atan2(dx).to_degrees() - pan).abs() <= half)).then_some((n, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(n, _)| n)
}

impl Walker {
    fn hit(&mut self, n: &PathNode) {
        self.target = Some([n.pos[0], n.pos[1], n.pan]);
        self.etat = 1;
        self.key = n.key;
    }

    /// `MoveSammyonPath` over `t` ticks (per frame, rates × `time` as in the script).
    fn step(&mut self, nodes: &[(&PathNode,)], t: f32) {
        // PlayerPatrol.
        match self.target {
            None => {
                if let Some(n) = scan(nodes, self.pos, self.pan, 30.0, self.range) {
                    self.hit(n);
                }
            }
            Some([tx, ty, tpan]) => {
                let (dx, dy) = (tx - self.pos[0], ty - self.pos[1]);
                let dist = dx.hypot(dy);
                let to = dy.atan2(dx).to_degrees();
                let mut v = 0.0;
                if dist < 100.0 {
                    v = ang(tpan - self.dest_pan) * (0.7 * t).min(1.0);
                    self.etat = 2;
                } else if self.etat == 1 {
                    v = ang(to - self.dest_pan) * (0.5 * t).min(1.0);
                } else if dist > 600.0 && self.etat == 2 {
                    self.etat = 1;
                    v = ang(to - self.dest_pan) * (0.2 * t).min(1.0);
                }
                self.dest_pan += v;
                if dist < 50.0 || (self.etat == 2 && v < 5.0) {
                    self.etat = 0;
                    self.target = None;
                    if let Some(n) = scan(nodes, [tx, ty, self.pos[2]], tpan, 10.0, 600.0) {
                        self.hit(n);
                    }
                }
            }
        }
        // Heading, then full speed ahead (`VitesseRaq.X = SpeedBalle`).
        self.pan += ang(self.dest_pan - self.pan) * (0.25 * t).min(1.0);
        let a = self.pan.to_radians();
        self.vel = [a.cos() * self.speed, a.sin() * self.speed, 0.0];
        self.pos[0] += self.vel[0] * t;
        self.pos[1] += self.vel[1] * t;
        // The per-tick path scan (cone 60, range BlobZoneScan, id 30).
        if let Some(n) = scan(nodes, self.pos, self.pan, 30.0, self.range) {
            self.hit(n);
        }
    }
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
    params.plus_tilt = 50.0;
    params.arc = 70.0;
    if let Ok(mut st) = cam.single_mut() {
        st.start_from(ORIGIN.0, pan, &params);
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
    time: Res<Time>,
    controls: Res<crate::input::Controls>,
    mouse: Res<ButtonInput<MouseButton>>,
    world: crate::collision::CollisionWorld,
    mut intro: ResMut<Intro>,
    mut params: ResMut<CameraParams>,
    mut walkers: Query<(&mut Walker, &mut Transform, Option<&mut crate::model::ModelAnimator>)>,
    nodes: Query<(&PathNode,)>,
) {
    let nodes: Vec<_> = nodes.iter().collect();
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
        if intro.active && !intro.gliding && w.key == 999.0 {
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
        if intro.ticks == 12.0 {
            params.fric_h = 0.1;
            params.fric_v = 0.5;
        }
        logo_tick(&mut intro);
        if intro.gliding {
            intro.fog = (intro.fog - 5.0).max(0.0);
        }
    }
    // `Sonar`: keep the feet on the floor; walk cycle at 0.5 × speed % per tick.
    for (mut w, mut tf, anim) in &mut walkers {
        let [x, y, z] = w.pos;
        if world.ready() {
            let tr = world.trace_static(0, [x, y, z + 200.0], [x, y, z - 2000.0]).trace;
            if tr.hit() && !tr.start_solid {
                w.pos[2] = tr.end[2] + BLOB_HAUT;
                w.snapped = true;
            }
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, 0.0, 0.0]);
        if let Some(mut a) = anim {
            a.set_speed_pct(if w.walking { 0.5 * w.speed } else { 0.0 }, TICKS);
        }
    }
}

/// One tick of `DisplayLogoA5` (docs/menu.md §2.2). The logo also drives the fog: the
/// intro loop's own fog changes race with it in the original [U]; the logo's are kept.
fn logo_tick(intro: &mut Intro) {
    let l = &mut intro.logo;
    l.t += 1.0;
    let next = |l: &mut Logo| {
        l.step += 1;
        l.t = 0.0;
    };
    match l.step {
        // Morph through the three files (1 tick each), fog 90.
        0 => {
            (l.dist, l.s6, l.alpha, l.shown) = (50.0, 100.0, 0.0, 1);
            intro.fog = 90.0;
            if l.t >= 3.0 {
                next(l);
            }
        }
        // A5 fade-in: Skill6 100 → 3 by 4, slides out to 285.
        1 => {
            l.s6 -= 4.0;
            l.dist = (l.dist + 10.0).min(285.0);
            l.alpha = 100.0 - l.s6;
            intro.fog = 90.0 - l.alpha * 0.25;
            if l.s6 <= 3.0 {
                next(l);
            }
        }
        // Hold 24.
        2 => {
            l.alpha = 100.0;
            l.dist = (l.dist + 5.0).min(285.0);
            intro.fog -= intro.fog * 0.05;
            if l.t >= 24.0 {
                l.s6 = 31.0;
                next(l);
            }
        }
        // Fade-out 30.
        3 => {
            l.alpha = l.s6 * 3.125;
            l.s6 -= 1.0;
            intro.fog -= intro.fog * 0.25;
            if l.s6 < 1.0 {
                next(l);
            }
        }
        // Gap 8, invisible, fog off.
        4 => {
            (l.alpha, l.shown) = (0.0, 0);
            intro.fog = 0.0;
            if l.t >= 8.0 {
                l.shown = 2;
                next(l);
            }
        }
        // Publisher: in +10, hold 32 + 24, out -20.
        5 => {
            l.alpha = (l.alpha + 10.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        6 => {
            if l.t >= 56.0 {
                next(l);
            }
        }
        7 => {
            l.alpha -= 20.0;
            if l.alpha <= 4.0 || l.t >= 16.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        // Suricate: gap 32, in +5, hold 64 + 24, out -15, removed.
        8 => {
            if l.t >= 32.0 {
                l.shown = 3;
                next(l);
            }
        }
        9 => {
            l.alpha = (l.alpha + 5.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        10 => {
            if l.t >= 88.0 {
                next(l);
            }
        }
        11 => {
            l.alpha -= 15.0;
            if l.alpha <= 0.0 || l.t >= 32.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        _ => {}
    }
}

/// `MoveMenu` (interfac.wdl:625-653): position and angles ease 0.2 per tick to the
/// menu pose; done within 2 quants in X and Z.
fn glide(time: Res<Time>, mut intro: ResMut<Intro>, mut cam: Query<&mut Transform, With<MainCamera>>) {
    if !intro.gliding {
        return;
    }
    let Ok(mut tf) = cam.single_mut() else { return };
    let k = (0.2 * time.delta_secs() * TICKS).min(1.0);
    let mut pos = coords::native(tf.translation);
    let f = coords::native(*tf.forward()) ;
    let (mut pan, mut tilt) = (f[1].atan2(f[0]).to_degrees(), f[2].atan2(f[0].hypot(f[1])).to_degrees());
    for i in 0..3 {
        pos[i] += (ORIGIN.0[i] - pos[i]) * k;
    }
    pan += ang(ORIGIN.1 - pan) * k;
    tilt += (ORIGIN.2 - tilt) * k;
    if (pos[0] - ORIGIN.0[0]).abs() < 2.0 && (pos[2] - ORIGIN.0[2]).abs() < 2.0 {
        (pos, pan, tilt) = ORIGIN;
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
                let tex = images.add(crate::image::to_bevy(&img, true, false));
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
            img.data[i + 3] = if crate::image::is_key(&rgb) { 0 } else { 255 };
        }
    }
    Some(img)
}
