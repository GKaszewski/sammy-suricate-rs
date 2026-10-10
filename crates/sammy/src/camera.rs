//! Free-fly debug camera: right mouse to look, WASD + Space/Ctrl to move, Shift = fast.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fly);
    }
}

/// The world-rendering camera (other cameras, e.g. the sky pass, copy it).
#[derive(Component)]
pub struct MainCamera;

#[derive(Component, Default)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
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
            (KeyCode::ControlLeft, Vec3::NEG_Y),
        ] {
            if keys.pressed(k) {
                v += d;
            }
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) { 40.0 } else { 10.0 };
        tf.translation += v.normalize_or_zero() * speed * time.delta_secs();
    }
}
