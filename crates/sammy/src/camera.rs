//! Cameras: the game's follow camera ([`sammy_core::camera`]) on Bevy's main camera,
//! the sky/near cameras that mirror it, the field of view, and a free-fly debug camera
//! (F1 toggles).

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::camera::{CameraPose, Subject};

use crate::actions::player::{LevelCollision, Player, TICKS};
use crate::collision::id;
use crate::coords;
use crate::flow::{CurrentLevel, Phase};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<CameraParams>()
            .init_resource::<CameraRuntime>()
            .add_systems(Update, (reset_camera_on_respawn.in_set(crate::schedule::GameSet::CameraReset), camera_zones.in_set(crate::schedule::GameSet::CameraZones)))
            .add_systems(Update, (toggle_mode, fly.run_if(resource_equals(CameraMode::Fly)), apply_arc))
            .add_systems(
                PostUpdate,
                (follow.run_if(resource_equals(CameraMode::Follow)), copy_to_sky)
                    .chain()
                    .before(TransformSystems::Propagate),
            );
    }
}

/// The world-rendering camera.
#[derive(Component)]
pub struct MainCamera;

/// Secondary cameras (sky pass, near pass) that mirror the main camera's transform.
#[derive(Component)]
pub struct SkyCamera;

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum CameraMode {
    #[default]
    Follow,
    Fly,
}

/// Per-level camera settings ([`sammy_core::camera::CameraParams`]).
#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
pub struct CameraParams(pub sammy_core::camera::CameraParams);

/// Camera globals that change during play ([`sammy_core::camera::CameraRuntime`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct CameraRuntime(pub sammy_core::camera::CameraRuntime);

/// A camera zone ([`sammy_core::camera::CameraZone`]).
#[derive(Component, Deref, DerefMut)]
pub struct CameraZone(sammy_core::camera::CameraZone);

pub fn init_zone(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(CameraZone(sammy_core::camera::CameraZone::from_object(obj)));
}

/// Follow-camera state ([`sammy_core::camera::FollowState`]).
#[derive(Component, Default, Deref, DerefMut)]
pub struct FollowState(pub sammy_core::camera::FollowState);

/// Zones react to id-25 scanners while playing (and during the intro's follow).
pub fn camera_zones(
    time: Res<Time>,
    current: Res<CurrentLevel>,
    level: Option<Res<LevelCollision>>,
    mut params: ResMut<CameraParams>,
    mut rt: ResMut<CameraRuntime>,
    scanners: Res<crate::actions::extro::Scanners>,
    mut zones: Query<(Entity, &mut CameraZone)>,
    intro: Res<crate::intro::Intro>,
) {
    let Some(level) = level else { return };
    if current.phase != Phase::Playing && !intro.following() {
        return;
    }
    let mut list: Vec<(Entity, Mut<CameraZone>)> = zones.iter_mut().collect();
    let mut refs: Vec<(sammy_core::id::Id, &mut sammy_core::camera::CameraZone)> = list.iter_mut().map(|(e, z)| (id(*e), &mut z.0)).collect();
    sammy_core::camera::camera_zones(time.delta_secs() * TICKS, &level.0, &mut params, &mut rt, &scanners, &mut refs);
}

/// Respawn (`RestartSammy`) resets the camera.
pub fn reset_camera_on_respawn(
    mut reset: MessageReader<crate::game::ResetScan>,
    mut params: ResMut<CameraParams>,
    mut rt: ResMut<CameraRuntime>,
    mut cam: Query<&mut FollowState>,
) {
    if reset.read().count() == 0 {
        return;
    }
    for mut f in &mut cam {
        sammy_core::camera::reset_on_respawn(&mut params, &mut rt, &mut f);
    }
}

#[derive(Component, Default)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
}

fn toggle_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>, mut q: Query<(&Transform, &mut FlyCamera)>) {
    if keys.just_pressed(KeyCode::F1) {
        *mode = match *mode {
            CameraMode::Follow => CameraMode::Fly,
            CameraMode::Fly => CameraMode::Follow,
        };
        // Start flying from wherever the follow camera was.
        for (tf, mut fly) in &mut q {
            let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
            fly.yaw = yaw;
            fly.pitch = pitch;
        }
    }
}

pub fn follow(
    time: Res<Time>,
    intro: Res<crate::intro::Intro>,
    params: Res<CameraParams>,
    mut rt: ResMut<CameraRuntime>,
    current: Res<CurrentLevel>,
    level: Option<Res<LevelCollision>>,
    player: Query<&Player>,
    pov: Res<crate::actions::extro::Pov>,
    mut cam: Query<(&mut Transform, &mut FollowState), With<MainCamera>>,
) {
    // The menu has a fixed camera, except while the intro follows the scripted Sammy.
    if current.phase == Phase::Menu && !intro.following() {
        return;
    }
    let Ok((mut tf, mut st)) = cam.single_mut() else { return };
    if pov.entity.is_none() {
        return;
    }
    let subject = Subject { pos: pov.pos, pan: pov.pan };
    let sammy = player.single().ok().map(|p| Subject { pos: p.pos, pan: p.pan });
    let level = level.as_ref().map(|l| &*l.0);
    let sweep = current.phase == Phase::Intro;
    let Some(pose) = sammy_core::camera::follow(&mut st, &params, &mut rt, level, subject, sammy, sweep, time.delta_secs() * TICKS, time.delta_secs()) else { return };
    match pose {
        CameraPose::LookAt { pos, target } => {
            tf.translation = coords::pos(pos);
            tf.look_at(coords::pos(target), Vec3::Y);
        }
        CameraPose::Angles { pos, angles } => {
            tf.translation = coords::pos(pos);
            tf.rotation = coords::rotation(angles);
        }
    }
}

fn copy_to_sky(main: Query<&Transform, With<MainCamera>>, mut sky: Query<&mut Transform, (With<SkyCamera>, Without<MainCamera>)>) {
    let Ok(m) = main.single() else { return };
    for mut s in &mut sky {
        *s = *m;
    }
}

fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut FlyCamera)>,
) {
    for (mut tf, mut cam) in &mut q {
        if buttons.pressed(MouseButton::Right) {
            cam.yaw -= motion.delta.x * 0.003;
            cam.pitch = (cam.pitch - motion.delta.y * 0.003).clamp(-1.54, 1.54);
        }
        tf.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
        let mut v = Vec3::ZERO;
        for (k, d) in [
            (KeyCode::KeyW, tf.forward().as_vec3()),
            (KeyCode::KeyS, tf.back().as_vec3()),
            (KeyCode::KeyA, tf.left().as_vec3()),
            (KeyCode::KeyD, tf.right().as_vec3()),
            (KeyCode::Space, Vec3::Y),
            (KeyCode::KeyQ, Vec3::NEG_Y),
        ] {
            if keys.pressed(k) {
                v += d;
            }
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) { 40.0 } else { 10.0 };
        tf.translation += v.normalize_or_zero() * speed * time.delta_secs();
    }
}

/// Acknex `arc` is horizontal for the original 4:3 screens; keep that vertical view and
/// let wider windows see more at the sides (Hor+).
fn apply_arc(params: Res<CameraParams>, windows: Query<&Window>, mut q: Query<&mut Projection>) {
    let Ok(w) = windows.single() else { return };
    let aspect = (w.width() / w.height().max(1.0)).min(4.0 / 3.0);
    let fov = 2.0 * ((params.arc.to_radians() / 2.0).tan() / aspect).atan();
    for mut proj in &mut q {
        if let Projection::Perspective(p) = &mut *proj
            && (p.fov - fov).abs() > 1e-4
        {
            p.fov = fov;
        }
    }
}

