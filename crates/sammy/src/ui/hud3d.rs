//! Camera-space HUD models and screens (docs/hud-saves.md §1.6-1.7, §3): the spinning
//! ammo wasp, the Eye of Truth plate, the +50/+100 popups and the lose/win titles.
//! Models sit on the near layer (drawn over the level); their screen position is kept
//! next to the 2D HUD by unprojecting the mode-6 pixel at the original depth.

use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::game::{Game, GameEvent, GameSystems};
use crate::render::coords;
use crate::scene::{Billboard, Loader, NEAR_LAYER};
use crate::ui::hud::{H, HudVisible, W};
use sammy_core::flow::Phase;
use sammy_core::sim::Event;
use sammy_core::time::TICKS_PER_SEC as TICKS;

pub struct Hud3dPlugin;

impl Plugin for Hud3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_hud_models, end_titles, spawn_popups, fly_popups).after(GameSystems::Tick))
            .add_systems(PostUpdate, place_hud_models.after(TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
    }
}

/// `PanWasp` scale per wasp type (`InitCurrentPanWasp`, panels.wdl:1411-1461).
const WASP_SCALE: [f32; 7] = [0.3, 0.23, 0.2, 0.2, 0.25, 0.3, 0.2];
/// HUD wasp: mode-6 screen pixel (camera space (100,-45,-35)), depth in quants, and the
/// starting pan (also its pitch angle) in degrees.
const WASP_SCREEN: (f32, f32) = (569.0, 434.0);
const WASP_DEPTH: f32 = 100.0;
const WASP_PAN_START: f32 = 90.0;
/// `panwasp.pan += 2*time`: spin per tick (degrees).
const WASP_SPIN: f32 = 2.0;
/// Eye of Truth plate: mode-6 screen pixel (camera space (200,-45,-72)) and depth in quants.
const EYE_SCREEN: (f32, f32) = (445.0, 440.0);
const EYE_DEPTH: f32 = 200.0;
/// The original screen's vertical arc basis: half of the 60° arc, at 4:3.
const ARC_HALF_DEG: f32 = 30.0;
const ASPECT_43: f32 = 4.0 / 3.0;
/// Rays at a grazing angle to the view axis are clamped to this cosine.
const MIN_RAY_COS: f32 = 0.1;

#[derive(Component)]
struct HudWasp {
    pan: f32,
    kind: u32,
}

#[derive(Component)]
struct HudEye;

/// Glued to the camera: mode-6 screen pixel (x from the right edge if negative), depth
/// along the view in quants, uniform scale, and A4 angles relative to the camera.
#[derive(Component)]
struct CameraSpace {
    px: Vec2,
    depth: f32,
    scale: f32,
    angles: [f32; 3],
}

fn unlit(materials: &mut Assets<StandardMaterial>, base: &Handle<StandardMaterial>, additive: bool) -> Handle<StandardMaterial> {
    let mut m = materials.get(base).cloned().unwrap_or_default();
    m.unlit = true;
    m.cull_mode = None;
    if additive {
        m.alpha_mode = AlphaMode::Add;
    }
    materials.add(m)
}

fn spawn_hud_models(mut commands: Commands, mut loader: Loader, existing: Query<(), Or<(With<HudWasp>, With<HudEye>)>>) {
    if !existing.is_empty() {
        return;
    }
    if let Some(m) = loader.model("panwasp.mdl") {
        let (meshes, materials) = (&mut loader.meshes, &mut loader.materials);
        // Camera space (100,-45,-35): screen ≈ (569, 434) at 640×480.
        let skins: Vec<_> = (1..=7).map(|k| unlit(materials, &m.skin(k), false)).collect();
        commands.spawn((
            HudWasp { pan: WASP_PAN_START, kind: 1 },
            CameraSpace { px: Vec2::new(WASP_SCREEN.0 - W, WASP_SCREEN.1), depth: WASP_DEPTH, scale: WASP_SCALE[0], angles: [WASP_PAN_START, 0.0, 0.0] },
            HudSkins(skins.clone()),
            Mesh3d(meshes.add(m.build_mesh(0))),
            MeshMaterial3d(skins[0].clone()),
            Transform::default(),
            Visibility::Hidden,
            NEAR_LAYER,
        ));
    }
    if let Some(m) = loader.model("paneye.mdl") {
        // Camera space (200,-45,-72): screen ≈ (445, 440); additive (`Flare`).
        let (meshes, materials) = (&mut loader.meshes, &mut loader.materials);
        let mat = unlit(materials, &m.skin(1), true);
        commands.spawn((
            HudEye,
            CameraSpace { px: Vec2::new(EYE_SCREEN.0, EYE_SCREEN.1), depth: EYE_DEPTH, scale: 1.0, angles: [0.0; 3] },
            Mesh3d(meshes.add(m.build_mesh(0))),
            MeshMaterial3d(mat),
            Transform::default(),
            Visibility::Hidden,
            NEAR_LAYER,
        ));
    }
}

#[derive(Component)]
struct HudSkins(Vec<Handle<StandardMaterial>>);

#[allow(clippy::type_complexity)]
fn place_hud_models(
    time: Res<Time>,
    sim: Res<Game>,
    visible: Res<HudVisible>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    cam: Query<(&Camera, &GlobalTransform, &Projection), With<MainCamera>>,
    mut q: Query<
        (&mut CameraSpace, &mut Transform, &mut GlobalTransform, &mut Visibility, Option<&mut HudWasp>, Option<&HudSkins>, &mut MeshMaterial3d<StandardMaterial>, Has<HudEye>),
        Without<MainCamera>,
    >,
) {
    let game = &sim.game;
    let (Ok(win), Ok((camera, cam_tf, proj))) = (windows.single(), cam.single()) else { return };
    let Projection::Perspective(p) = proj else { return };
    // Mode-6 pixel size in window pixels, as for the 2D HUD (whole-number UI scale).
    let s = ui_scale.0;
    // Same size relative to the 2D HUD as at 4:3 with the original 60° arc.
    let vfov_43 = 2.0 * (ARC_HALF_DEG.to_radians().tan() / ASPECT_43).atan();
    let size_k = (p.fov / 2.0).tan() / (vfov_43 / 2.0).tan() * s / (win.height() / H);
    for (mut cs, mut tf, mut gtf, mut vis, wasp, skins, mut mat, eye) in &mut q {
        let shown = visible.0
            && match (&wasp, eye) {
                (Some(_), _) => game.ammo > 0,
                (_, true) => game.eye_ticks > 0.0,
                _ => false,
            };
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(mut w) = wasp {
            // `panwasp.pan += 2*time`; skin and scale follow the ammo type.
            w.pan += WASP_SPIN * time.delta_secs() * TICKS;
            cs.angles[0] = w.pan;
            let k = game.ammo_type.clamp(1, 7);
            if k != w.kind
                && let Some(sk) = skins
            {
                w.kind = k;
                mat.0 = sk.0[k as usize - 1].clone();
            }
            cs.scale = WASP_SCALE[k as usize - 1];
        }
        // Mode-6 pixel → window pixel (negative x anchors to the right edge, y to the bottom).
        let x = if cs.px.x < 0.0 { win.width() + cs.px.x * s } else { win.width() / 2.0 + (cs.px.x - W / 2.0) * s };
        let y = win.height() - (H - cs.px.y) * s;
        let Ok(ray) = camera.viewport_to_world(cam_tf, Vec2::new(x, y)) else { continue };
        let fwd = cam_tf.forward().as_vec3();
        let depth = cs.depth / coords::QUANTS_PER_UNIT;
        let pos = ray.origin + ray.direction.as_vec3() * (depth / ray.direction.dot(fwd).max(MIN_RAY_COS));
        let (_, cam_rot, _) = cam_tf.to_scale_rotation_translation();
        *tf = Transform { translation: pos, rotation: cam_rot * coords::rotation(cs.angles), scale: Vec3::splat(cs.scale * size_k) };
        *gtf = GlobalTransform::from(*tf);
    }
}

// ---------------------------------------------------------------- popups

/// `Bonus3_Hit` popups: points → (sprite, life in ticks).
const POPUP_SMALL_POINTS: i32 = 50;
const POPUP_SMALL: (&str, f32) = ("plus50+6.pcx", 32.0);
const POPUP_BIG_POINTS: i32 = 100;
const POPUP_BIG: (&str, f32) = ("plu100+2.pcx", 48.0);
/// The popup starts this far above the pickup, flies at this speed per tick, tilted up
/// by `POPUP_TILT_DEG` (`InvDirCamera`), and animates at this many frames per tick.
const POPUP_LIFT: f32 = 32.0;
const POPUP_SPEED: f32 = 40.0;
const POPUP_TILT_DEG: f32 = 20.0;
const POPUP_FRAMES_PER_TICK: f32 = 0.2;

/// `FlyingPCX` (mm1.wdl:1333-1384): a score sprite flying toward the camera and up,
/// playing its strip.
#[derive(Component)]
pub struct FlyingPopup {
    pos: [f32; 3],
    dir: [f32; 3],
    speed: f32,
    life: f32,
    frame: f32,
    frames: u32,
}

/// Diamond `+50` / `+100` popups at pickups (`Bonus3_Hit`, mm1.wdl:1252-1275).
fn spawn_popups(mut commands: Commands, mut events: MessageReader<GameEvent>, mut loader: Loader, cam: Query<&GlobalTransform, With<MainCamera>>) {
    let cam = cam.single().map(|g| coords::native(g.translation())).unwrap_or_default();
    for ev in events.read() {
        let GameEvent(Event::Popup { at, points }) = ev else { continue };
        let (file, life) = match *points {
            POPUP_SMALL_POINTS => POPUP_SMALL,
            POPUP_BIG_POINTS => POPUP_BIG,
            _ => continue,
        };
        let Some(sprite) = loader.sprite(file) else { continue };
        let pos = [at[0], at[1], at[2] + POPUP_LIFT];
        // Pan toward the camera, tilt +20° (`InvDirCamera`).
        let d = [cam[0] - pos[0], cam[1] - pos[1]];
        let pan = d[1].atan2(d[0]);
        let tilt = POPUP_TILT_DEG.to_radians();
        let dir = [pan.cos() * tilt.cos(), pan.sin() * tilt.cos(), tilt.sin()];
        let m = loader.materials.get(&sprite.material).cloned().unwrap_or_default();
        commands.spawn((
            FlyingPopup { pos, dir, speed: POPUP_SPEED, life, frame: 0.0, frames: sprite.frames },
            Billboard,
            Mesh3d(sprite.mesh),
            MeshMaterial3d(loader.materials.add(m)),
            Transform::from_translation(coords::pos(pos)),
        ));
    }
}

fn fly_popups(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut FlyingPopup, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut p, mut tf, mat) in &mut q {
        let (dir, speed) = (p.dir, p.speed);
        for (c, d) in p.pos.iter_mut().zip(dir) {
            *c += d * speed * t;
        }
        p.life -= t;
        p.frame = (p.frame + POPUP_FRAMES_PER_TICK * t).rem_euclid(p.frames as f32);
        tf.translation = coords::pos(p.pos);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.uv_transform = bevy::math::Affine2::from_translation(Vec2::new(p.frame.floor() / p.frames as f32, 0.0));
        }
        if p.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

// ---------------------------------------------------------------- lose title

/// `InitTitleLoose` (mm1.wdl:5314-5393): "Przegrałeś" flies in to depth 137.6 at 20
/// above the view axis, with an amber light and smoke from the logo.
/// The title starts at `LOSE_START_DEPTH` moving `LOSE_START_SPEED` per tick, slows by
/// `LOSE_DECEL` (floor `LOSE_MIN_SPEED`) and stops past `LOSE_STOP_DEPTH`; it sits
/// `LOSE_RAISE` above the view axis, turned `LOSE_PAN` to face the camera, at scale
/// `LOSE_SCALE`, lit amber.
const LOSE_START_DEPTH: f32 = 20.0;
const LOSE_START_SPEED: f32 = 20.0;
const LOSE_DECEL: f32 = 0.8;
const LOSE_MIN_SPEED: f32 = 2.0;
const LOSE_STOP_DEPTH: f32 = 128.0;
const LOSE_RAISE: f32 = 20.0;
const LOSE_PAN: f32 = 180.0;
const LOSE_SCALE: f32 = 0.32;
const LOSE_LIGHT_COLOR: [f32; 3] = [250.0, 200.0, 50.0];
const LOSE_LIGHT_RANGE: f32 = 500.0;
#[derive(Component)]
struct LoseTitle {
    depth: f32,
    speed: f32,
}

#[allow(clippy::too_many_arguments)]
fn end_titles(
    time: Res<Time>,
    mut commands: Commands,
    game: Res<Game>,
    mut events: MessageReader<GameEvent>,
    mut loader: Loader,
    mut particles: ResMut<crate::render::particles::Particles>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mut titles: Query<(Entity, &mut LoseTitle, &mut Transform)>,
) {
    let Ok(cam) = cam.single() else { return };
    // A new level takes the title away.
    if events.read().any(|e| matches!(e.0, Event::Loaded { .. })) {
        for (e, ..) in &titles {
            commands.entity(e).despawn();
        }
        return;
    }
    let lost = matches!(game.flow.phase, Phase::Lost(_));
    if lost && titles.is_empty() {
        if let Some(m) = loader.model("lose.mdl") {
            let mat = unlit(&mut loader.materials, &m.skin(1), false);
            commands.spawn((
                LoseTitle { depth: LOSE_START_DEPTH, speed: LOSE_START_SPEED },
                Mesh3d(loader.meshes.add(m.build_mesh(0))),
                MeshMaterial3d(mat),
                Transform::default(),
                crate::render::lights::DynamicLight { color: LOSE_LIGHT_COLOR, range: LOSE_LIGHT_RANGE },
                NEAR_LAYER,
            ));
        }
        return;
    }
    let t = time.delta_secs() * TICKS;
    for (_, mut title, mut tf) in &mut titles {
        if title.depth < LOSE_STOP_DEPTH {
            title.depth += title.speed * t;
            title.speed = (title.speed - LOSE_DECEL * t).max(LOSE_MIN_SPEED);
        }
        // Camera-relative (depth, 0, 20), facing the camera; scale 0.32.
        let (_, rot, pos) = cam.to_scale_rotation_translation();
        let local = Vec3::new(0.0, LOSE_RAISE, -title.depth) / coords::QUANTS_PER_UNIT;
        tf.translation = pos + rot * local;
        tf.rotation = rot * coords::rotation([LOSE_PAN, 0.0, 0.0]);
        tf.scale = Vec3::splat(LOSE_SCALE);
        for _ in 0..particles.ticks {
            let at = coords::native(tf.translation);
            sammy_present::effects::tile_smoke(&mut particles, at);
        }
    }
}
