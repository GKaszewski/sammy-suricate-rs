//! Cameras: the game's type-3 follow camera (camera.wdl `HandleCamera3`,
//! simplified) and a free-fly debug camera. F1 toggles between them.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use formats::wmb::Object;

use crate::actions::player::{LevelCollision, Player, TICKS};
use crate::coords;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<CameraParams>()
            .add_systems(Update, (toggle_mode, fly.run_if(resource_equals(CameraMode::Fly))))
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

/// Renders sky domes; mirrors the main camera's transform.
#[derive(Component)]
pub struct SkyCamera;

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum CameraMode {
    #[default]
    Follow,
    Fly,
}

/// Per-level follow-camera settings (`InitCamera1`/`InitCamera2`, mm1.wdl:2719-2851).
#[derive(Resource, Clone, Copy)]
pub struct CameraParams {
    /// `HautCamPerso`: height above Sammy.
    pub height: f32,
    /// `PersoDistCam`: distance behind.
    pub dist: f32,
    /// `PersoAngArriere`: angle behind Sammy (180 = directly behind).
    pub back: f32,
    /// `Camera_PlusTilt`: look-at point above Sammy.
    pub plus_tilt: f32,
    /// Type-3 horizontal / vertical frictions.
    pub fric_h: f32,
    pub fric_v: f32,
}

impl Default for CameraParams {
    fn default() -> Self {
        // InitLvl.wdl defaults.
        Self { height: 800.0, dist: 400.0, back: 180.0, plus_tilt: 200.0, fric_h: 4.0, fric_v: 0.01 }
    }
}

impl CameraParams {
    pub fn apply_init_camera1(&mut self, obj: &Object) {
        let s = &obj.skills;
        let or = |v: f32, d: f32| if v == 0.0 { d } else { v };
        self.height = or(s[0], self.height);
        self.dist = or(s[3], self.dist);
        self.back = or(s[6], self.back);
        // Skill8 is "always applied, even 0" (docs/gameplay.md §6.1).
        self.plus_tilt = s[7];
    }

    pub fn apply_init_camera2(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[2] != 0.0 {
            self.fric_h = s[2];
        }
        if s[5] != 0.0 {
            self.fric_v = s[5];
        }
    }
}

/// Follow-camera state in native coordinates.
#[derive(Component, Default)]
pub struct FollowState {
    /// `AngCamPerso`: smoothed copy of Sammy's pan.
    ang: f32,
    pos: Option<[f32; 3]>,
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

fn angle_diff(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

fn follow(
    time: Res<Time>,
    params: Res<CameraParams>,
    level: Option<Res<LevelCollision>>,
    player: Query<&Player>,
    mut cam: Query<(&mut Transform, &mut FollowState), With<MainCamera>>,
) {
    let (Ok(p), Ok((mut tf, mut st))) = (player.single(), cam.single_mut()) else { return };
    let t = time.delta_secs() * TICKS;

    // 1. Yaw follows Sammy's pan (4/AverageFps per frame = 4 per second).
    if st.pos.is_none() {
        st.ang = p.pan;
    }
    st.ang += angle_diff(p.pan - st.ang) * (4.0 * time.delta_secs()).min(1.0);

    // 2. Ideal position: `dist` behind at `height` above.
    let a = (st.ang + params.back).to_radians();
    let mut ideal = [p.pos[0] + a.cos() * params.dist, p.pos[1] + a.sin() * params.dist, p.pos[2] + params.height];
    let eye = [p.pos[0], p.pos[1], p.pos[2] + params.plus_tilt];

    // 4. (simplified emergency camera) If a wall hides Sammy, stop in front of it.
    if let Some(level) = &level {
        let tr = level.0.trace(0, 1, eye, ideal);
        if tr.hit() && !tr.start_solid {
            let back = 32.0 / params.dist.max(1.0);
            ideal = [0, 1, 2].map(|i| tr.end[i] + (eye[i] - ideal[i]) * back);
        }
    }

    // 3. Camera proxy eases toward the ideal (factor `fric × AdjustFpsCam3(0.5) × time`).
    let pos = st.pos.get_or_insert(ideal);
    let kh = (params.fric_h * 0.5 * t).min(1.0);
    let kv = (params.fric_v * 0.5 * t).min(1.0);
    pos[0] += (ideal[0] - pos[0]) * kh;
    pos[1] += (ideal[1] - pos[1]) * kh;
    pos[2] += (ideal[2] - pos[2]) * kv;

    // 5. Look at Sammy + PlusTilt.
    tf.translation = coords::pos(*pos);
    tf.look_at(coords::pos(eye), Vec3::Y);
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
