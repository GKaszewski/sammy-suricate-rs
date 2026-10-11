//! Camera-space HUD models and screens (docs/hud-saves.md §1.6-1.7, §3): the spinning
//! ammo wasp, the Eye of Truth plate, the +50/+100 popups and the lose/win titles.
//! Models sit on the near layer (drawn over the level); their screen position is kept
//! next to the 2D HUD by unprojecting the mode-6 pixel at the original depth.

use bevy::prelude::*;

use crate::actions::player::TICKS;
use crate::camera::MainCamera;
use crate::coords;
use crate::flow::{CurrentLevel, Phase};
use crate::game::GameState;
use crate::hud::{H, HudVisible, W};
use crate::level::{Billboard, GameAssets, NEAR_LAYER, SpriteAnim};

pub struct Hud3dPlugin;

impl Plugin for Hud3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_hud_models, end_titles, fly_popups.in_set(crate::schedule::GameSet::Flow)))
            .add_systems(PostUpdate, place_hud_models.after(TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
    }
}

/// `PanWasp` scale per wasp type (`InitCurrentPanWasp`, panels.wdl:1411-1461).
const WASP_SCALE: [f32; 7] = [0.3, 0.23, 0.2, 0.2, 0.25, 0.3, 0.2];

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

fn spawn_hud_models(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(), Or<(With<HudWasp>, With<HudEye>)>>,
) {
    let Some(assets) = assets else { return };
    if !existing.is_empty() {
        return;
    }
    if let Some(m) = assets.model("panwasp.mdl") {
        // Camera space (100,-45,-35): screen ≈ (569, 434) at 640×480.
        let skins: Vec<_> = (1..=7).map(|k| unlit(&mut materials, &m.skin(k), false)).collect();
        commands.spawn((
            HudWasp { pan: 90.0, kind: 1 },
            CameraSpace { px: Vec2::new(569.0 - W, 434.0), depth: 100.0, scale: 0.3, angles: [90.0, 0.0, 0.0] },
            HudSkins(skins.clone()),
            Mesh3d(meshes.add(m.build_mesh(0))),
            MeshMaterial3d(skins[0].clone()),
            Transform::default(),
            Visibility::Hidden,
            NEAR_LAYER,
        ));
    }
    if let Some(m) = assets.model("paneye.mdl") {
        // Camera space (200,-45,-72): screen ≈ (445, 440); additive (`Flare`).
        let mat = unlit(&mut materials, &m.skin(1), true);
        commands.spawn((
            HudEye,
            CameraSpace { px: Vec2::new(445.0, 440.0), depth: 200.0, scale: 1.0, angles: [0.0; 3] },
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
    game: Res<GameState>,
    visible: Res<HudVisible>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    cam: Query<(&Camera, &GlobalTransform, &Projection), With<MainCamera>>,
    mut q: Query<
        (&mut CameraSpace, &mut Transform, &mut GlobalTransform, &mut Visibility, Option<&mut HudWasp>, Option<&HudSkins>, &mut MeshMaterial3d<StandardMaterial>, Has<HudEye>),
        Without<MainCamera>,
    >,
) {
    let (Ok(win), Ok((camera, cam_tf, proj))) = (windows.single(), cam.single()) else { return };
    let Projection::Perspective(p) = proj else { return };
    // Mode-6 pixel size in window pixels, as for the 2D HUD (whole-number UI scale).
    let s = ui_scale.0;
    // Same size relative to the 2D HUD as at 4:3 with the original 60° arc.
    let vfov_43 = 2.0 * ((30f32).to_radians().tan() / (4.0 / 3.0)).atan();
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
            w.pan += 2.0 * time.delta_secs() * TICKS;
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
        let pos = ray.origin + ray.direction.as_vec3() * (depth / ray.direction.dot(fwd).max(0.1));
        let (_, cam_rot, _) = cam_tf.to_scale_rotation_translation();
        *tf = Transform { translation: pos, rotation: cam_rot * coords::rotation(cs.angles), scale: Vec3::splat(cs.scale * size_k) };
        *gtf = GlobalTransform::from(*tf);
    }
}

// ---------------------------------------------------------------- popups

/// `FlyingPCX` (mm1.wdl:1333-1384): a score sprite flying toward the camera and up.
#[derive(Component)]
pub struct FlyingPopup {
    pos: [f32; 3],
    dir: [f32; 3],
    speed: f32,
    life: f32,
}

/// Diamond `+50` / `+100` popup at a pickup (`Bonus3_Hit`, mm1.wdl:1252-1275).
pub fn spawn_popup(commands: &mut Commands, assets: &GameAssets, materials: &mut Assets<StandardMaterial>, at: [f32; 3], cam: [f32; 3], points: i32) {
    let (file, life) = match points {
        50 => ("plus50+6.pcx", 32.0),
        100 => ("plu100+2.pcx", 48.0),
        _ => return,
    };
    let Some((mesh, mat, frames)) = assets.sprite(file) else { return };
    let pos = [at[0], at[1], at[2] + 32.0];
    // Pan toward the camera, tilt +20° (`InvDirCamera`).
    let d = [cam[0] - pos[0], cam[1] - pos[1]];
    let pan = d[1].atan2(d[0]);
    let tilt = 20f32.to_radians();
    let dir = [pan.cos() * tilt.cos(), pan.sin() * tilt.cos(), tilt.sin()];
    let m = materials.get(&mat).cloned().unwrap_or_default();
    commands.spawn((
        FlyingPopup { pos, dir, speed: 40.0, life },
        SpriteAnim { frames, frame: 0.0, fps: 0.2 * TICKS },
        Billboard,
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(m)),
        Transform::from_translation(coords::pos(pos)),
        crate::flow::LevelScoped,
    ));
}

fn fly_popups(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut FlyingPopup, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut p, mut tf) in &mut q {
        for i in 0..3 {
            p.pos[i] += p.dir[i] * p.speed * t;
        }
        p.life -= t;
        tf.translation = coords::pos(p.pos);
        if p.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

// ---------------------------------------------------------------- lose title

/// `InitTitleLoose` (mm1.wdl:5314-5393): "Przegrałeś" flies in to depth 137.6 at 20
/// above the view axis, with an amber light and smoke from the logo.
#[derive(Component)]
struct LoseTitle {
    depth: f32,
    speed: f32,
}

#[allow(clippy::too_many_arguments)]
fn end_titles(
    time: Res<Time>,
    mut commands: Commands,
    current: Res<CurrentLevel>,
    assets: Option<Res<GameAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particles: ResMut<crate::particles::Particles>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mut titles: Query<(&mut LoseTitle, &mut Transform)>,
) {
    let Ok(cam) = cam.single() else { return };
    let lost = matches!(current.phase, Phase::Lost(_));
    if lost && titles.is_empty() {
        if let Some(m) = assets.as_ref().and_then(|a| a.model("lose.mdl")) {
            let mat = unlit(&mut materials, &m.skin(1), false);
            commands.spawn((
                LoseTitle { depth: 20.0, speed: 20.0 },
                Mesh3d(meshes.add(m.build_mesh(0))),
                MeshMaterial3d(mat),
                Transform::default(),
                crate::lights::DynamicLight { color: [250.0, 200.0, 50.0], range: 500.0 },
                NEAR_LAYER,
                crate::flow::LevelScoped,
            ));
        }
        return;
    }
    let t = time.delta_secs() * TICKS;
    for (mut title, mut tf) in &mut titles {
        if title.depth < 128.0 {
            title.depth += title.speed * t;
            title.speed = (title.speed - 0.8 * t).max(2.0);
        }
        // Camera-relative (depth, 0, 20), facing the camera; scale 0.32.
        let (_, rot, pos) = cam.to_scale_rotation_translation();
        let local = Vec3::new(0.0, 20.0, -title.depth) / coords::QUANTS_PER_UNIT;
        tf.translation = pos + rot * local;
        tf.rotation = rot * coords::rotation([180.0, 0.0, 0.0]);
        tf.scale = Vec3::splat(0.32);
        for _ in 0..particles.ticks {
            let at = coords::native(tf.translation);
            crate::effects::tile_smoke(&mut particles, at);
        }
    }
}

/// [`spawn_popup`] from a system that only has `Commands`.
pub fn queue_popup(commands: &mut Commands, at: [f32; 3], points: i32) {
    commands.queue(move |w: &mut World| {
        let cam = w.query_filtered::<&GlobalTransform, With<MainCamera>>().iter(w).next().map(|g| coords::native(g.translation()));
        let Some(cam) = cam else { return };
        w.resource_scope(|w, assets: Mut<GameAssets>| {
            w.resource_scope(|w, mut materials: Mut<Assets<StandardMaterial>>| {
                spawn_popup(&mut w.commands(), &assets, &mut materials, at, cam, points);
            });
        });
        w.flush();
    });
}
