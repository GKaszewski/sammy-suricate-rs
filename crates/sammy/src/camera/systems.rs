#[allow(unused_imports)]
use super::*;

/// Fly camera: radians per pixel of mouse motion, pitch limit, speeds (units/s, Shift fast).
const FLY_LOOK: f32 = 0.003;
const FLY_PITCH_MAX: f32 = 1.54;
const FLY_SPEED: f32 = 10.0;
const FLY_SPEED_FAST: f32 = 40.0;
/// The original 4:3 screen aspect (wider windows see more at the sides).
const ASPECT_43: f32 = 4.0 / 3.0;

pub(crate) fn toggle_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>, mut q: Query<(&Transform, &mut FlyCamera)>) {
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

/// The main camera where the game's camera is.
pub fn place(game: Res<Game>, mut cam: Query<&mut Transform, With<MainCamera>>) {
    let (Some(pose), Ok(mut tf)) = (game.camera.pose, cam.single_mut()) else { return };
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

pub(crate) fn copy_to_sky(main: Query<&Transform, With<MainCamera>>, mut sky: Query<&mut Transform, (With<SkyCamera>, Without<MainCamera>)>) {
    let Ok(m) = main.single() else { return };
    for mut s in &mut sky {
        *s = *m;
    }
}

pub(crate) fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut FlyCamera)>,
) {
    for (mut tf, mut cam) in &mut q {
        if buttons.pressed(MouseButton::Right) {
            cam.yaw -= motion.delta.x * FLY_LOOK;
            cam.pitch = (cam.pitch - motion.delta.y * FLY_LOOK).clamp(-FLY_PITCH_MAX, FLY_PITCH_MAX);
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
        let speed = if keys.pressed(KeyCode::ShiftLeft) { FLY_SPEED_FAST } else { FLY_SPEED };
        tf.translation += v.normalize_or_zero() * speed * time.delta_secs();
    }
}

/// Acknex `arc` is horizontal for the original 4:3 screens; keep that vertical view and
/// let wider windows see more at the sides (Hor+).
pub(crate) fn apply_arc(game: Res<Game>, windows: Query<&Window>, mut q: Query<&mut Projection>) {
    let Ok(w) = windows.single() else { return };
    let aspect = (w.width() / w.height().max(1.0)).min(ASPECT_43);
    let fov = 2.0 * ((game.camera.params.arc.to_radians() / 2.0).tan() / aspect).atan();
    for mut proj in &mut q {
        if let Projection::Perspective(p) = &mut *proj
            && (p.fov - fov).abs() > 1e-4
        {
            p.fov = fov;
        }
    }
}
