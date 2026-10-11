//! Cameras: the game's type-3 follow camera (camera.wdl `HandleCamera3`,
//! simplified) and a free-fly debug camera. F1 toggles between them.

use crate::collision::Hull;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use formats::wmb::Object;

use crate::actions::player::{LevelCollision, Player, TICKS};
use crate::coords;
use crate::flow::{CurrentLevel, Phase};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<CameraParams>()
            .init_resource::<CameraRuntime>()
            .add_systems(Update, (camera_zones, reset_camera_on_respawn))
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
    /// Level-start camera origin and pan/tilt (`CameraPosOrigine`, `InitGame2` Skill2-6).
    pub origin: Option<([f32; 3], f32, f32)>,
    /// `CameraTypeStart` from `InitCamera1` flags (2 = `HandleCamera`, 3 = follow camera).
    pub start_type: u8,
    /// `CameraCanMoveInZ` (InitCamera1 Flag1).
    pub can_move_z: bool,
    /// Type-2 frictions (`Camera1Friction`/`Camera1VertFriction`, InitCamera2 Skill1/Skill4).
    pub fric1_h: f32,
    pub fric1_v: f32,
    /// `Camera3FrictionStart` (InitCamera3 Skill8): level-start sweep speed (both axes).
    pub start_fric: f32,
    /// Values restored on respawn (`HautCamPersoOrg`, `PersoDistCamOrg`, `Camera_PlusTiltOrg`).
    pub org: (f32, f32, f32),
    /// `camera.arc`: horizontal field of view in degrees (`CameraArcVoulu`, default 60).
    pub arc: f32,
    /// Emergency-camera zoom limits (`InitCamera4` Skill1; distance floor 50).
    pub zoom_min_height: f32,
    pub zoom_min_dist: f32,
}

impl Default for CameraParams {
    fn default() -> Self {
        // InitLvl.wdl defaults.
        Self {
            height: 800.0,
            dist: 400.0,
            back: 180.0,
            plus_tilt: 200.0,
            fric_h: 4.0,
            fric_v: 0.01,
            zoom_min_height: 105.0,
            zoom_min_dist: 50.0,
            arc: 60.0,
            origin: Some(([832.0, -316.0, 1550.0], 90.0, -65.0)),
            start_type: 2,
            can_move_z: false,
            fric1_h: 1.0,
            fric1_v: 0.1,
            start_fric: 0.5,
            org: (400.0, 800.0, 200.0),
        }
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
        self.org = (self.dist, self.height, self.plus_tilt);
        // Flags tested in order Flag2, Flag3, Flag4, each overriding; none = type 2.
        let f = |n: u32| obj.flags & (1 << (n - 1)) != 0;
        self.can_move_z = f(1);
        self.start_type = 2;
        if f(2) {
            self.start_type = 3;
        }
        if f(3) {
            self.start_type = 2; // type 1 is unused by shipped data; treated as type 2
        }
        if f(4) {
            self.start_type = 2;
        }
    }

    /// `InitCamera3` Skill8: start-sweep friction (0 → `Camera3FrictionOrg`).
    pub fn apply_init_camera3(&mut self, obj: &Object) {
        if obj.skills[7] != 0.0 {
            self.start_fric = obj.skills[7];
        }
    }

    pub fn apply_init_game2(&mut self, obj: &Object) {
        let s = &obj.skills;
        let (mut o, mut pan, mut tilt) = self.origin.unwrap_or(([832.0, -316.0, 1550.0], 90.0, -65.0));
        // Each skill overrides the default only when non-zero (mm1.wdl:1835-1846).
        for i in 0..3 {
            if s[1 + i] != 0.0 {
                o[i] = s[1 + i];
            }
        }
        if s[4] != 0.0 {
            pan = s[4];
        }
        if s[5] != 0.0 {
            tilt = if s[5] == 90.0 { -90.0 } else { s[5] }; // "erreur de Wed !"
        }
        self.origin = Some((o, pan, tilt));
    }

    pub fn apply_init_camera2(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[0] != 0.0 {
            self.fric1_h = s[0];
        }
        if s[3] != 0.0 {
            self.fric1_v = s[3];
        }
        if s[2] != 0.0 {
            self.fric_h = s[2];
        }
        if s[5] != 0.0 {
            self.fric_v = s[5];
        }
        // Original bug: tests Skill7 but assigns Skill6 (mm1.wdl:2818-2821).
        if s[6] != 0.0 {
            self.arc = s[5];
        }
    }
}

/// Camera globals that change during play (`CameraType`, `CameraBouge`, zone tween).
#[derive(Resource, Default)]
pub struct CameraRuntime {
    /// Current camera type (2 or 3); 0 = not yet chosen this level.
    pub cam_type: u8,
    /// A zone transition is running (`CameraBouge == 1`).
    pub bouge: bool,
    tween: Option<ZoneTween>,
    /// Ticks left of the 0.25 friction cap after a zone transition.
    fric_cap: f32,
    acc: f32,
}

struct ZoneTween {
    zone: Entity,
    steps: f32,
    k: f32,
    d: [f32; 4],
    target: [f32; 4],
}

/// A camera zone (`SetCamera`, camera.wdl:958-1173).
#[derive(Component)]
pub struct CameraZone {
    s: [f32; 8],
    pos: [f32; 3],
    armed: bool,
    pending: bool,
}

pub fn init_zone(obj: &Object, entity: &mut EntityCommands) {
    let mut s = [0.0; 8];
    s.copy_from_slice(&obj.skills[..8]);
    entity.insert(CameraZone { s, pos: obj.origin, armed: true, pending: false });
}

/// `SetCameraHit`: Sammy's per-tick id-25 scan (radius 500) arms a zone when the camera
/// differs and the zone sees Sammy; a tick later, within Skill7, it blends distance,
/// height, back angle and PlusTilt over Skill6+1 steps, snaps, and re-arms.
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
    rt.acc += time.delta_secs() * TICKS;
    while rt.acc >= 1.0 {
        rt.acc -= 1.0;
        if rt.fric_cap > 0.0 {
            rt.fric_cap -= 1.0;
        }
        if let Some(mut tw) = rt.tween.take() {
            if tw.steps > 0.0 {
                params.dist += tw.d[0] * tw.k;
                params.height += tw.d[1] * tw.k;
                params.back = angle_diff(params.back + tw.d[2] * tw.k);
                params.plus_tilt += tw.d[3] * tw.k;
                tw.steps -= 1.0;
                rt.tween = Some(tw);
            } else {
                // Snap to the zone's raw values, cap the type-3 frictions for 4 ticks, re-arm.
                [params.dist, params.height, params.back, params.plus_tilt] = tw.target;
                rt.bouge = false;
                rt.fric_cap = 4.0;
                if let Ok((_, mut z)) = zones.get_mut(tw.zone) {
                    z.armed = true;
                }
            }
            continue;
        }
        // Confirm a pending zone one tick later (distance ≤ Skill7), else drop it.
        let mut started = false;
        for (e, mut z) in &mut zones {
            if !z.pending {
                continue;
            }
            z.pending = false;
            let d = scanners.nearest(z.pos).map_or(f32::MAX, |(_, d)| d);
            if started || d > z.s[6] {
                continue;
            }
            started = true;
            z.armed = false;
            rt.bouge = true;
            rt.cam_type = if z.s[0] == 3.0 { 3 } else { 2 };
            let steps = z.s[5].max(1.0);
            rt.tween = Some(ZoneTween {
                zone: e,
                steps: steps + 1.0,
                k: 1.0 / steps,
                d: [z.s[1] - params.dist, z.s[2] - params.height, angle_diff(z.s[3] - params.back), z.s[4] - params.plus_tilt],
                target: [z.s[1], z.s[2], z.s[3], z.s[4]],
            });
        }
        if started || rt.bouge {
            continue;
        }
        // Synchronous part of SetCameraHit, during an id-25 scan (Sammy or a Flag5 enemy).
        for (_, mut z) in &mut zones {
            if !z.armed {
                continue;
            }
            let Some((sp, d)) = scanners.nearest(z.pos) else { continue };
            if d > crate::actions::extro::RANGE_SCAN_25 {
                continue;
            }
            let ty = if z.s[0] == 3.0 { 3 } else { 2 };
            if rt.cam_type == ty && params.dist == z.s[1] && params.height == z.s[2] && params.plus_tilt == z.s[4] {
                continue;
            }
            if level.0.trace(0, Hull::Point.index(), z.pos, sp).hit() {
                continue;
            }
            z.pending = true;
            break; // one zone per tick (the original's same-tick race is not replicated)
        }
    }
}

/// Respawn (`RestartSammy`): camera type back to the level's, Org values restored
/// (the back angle is not), running zone transitions interrupted.
pub fn reset_camera_on_respawn(
    mut reset: MessageReader<crate::game::ResetScan>,
    mut params: ResMut<CameraParams>,
    mut rt: ResMut<CameraRuntime>,
    mut cam: Query<&mut FollowState>,
) {
    if reset.read().count() == 0 {
        return;
    }
    (params.dist, params.height, params.plus_tilt) = params.org;
    rt.cam_type = params.start_type;
    rt.bouge = false;
    rt.tween = None;
    for mut f in &mut cam {
        f.set_pos = true;
    }
}

/// Follow-camera state in native coordinates.
#[derive(Component, Default)]
pub struct FollowState {
    /// `AngCamPerso`: smoothed copy of Sammy's pan.
    ang: f32,
    pos: Option<[f32; 3]>,
    /// Within 4 quants of the ideal position (ends the level-start sweep).
    pub converged: bool,
    /// Multiplier on `Camera_PlusTilt` (drops to 0 while Sammy dies, `Player_Kaputt`).
    pub tilt_scale: Option<f32>,
    /// Game over: orbit Sammy at 3°/tick instead of following his pan.
    pub orbit: bool,
    /// Type 2: place the camera with `SetPosCamera` on the next update.
    pub set_pos: bool,
    /// Type 2: smoothed look-at height (`Camera_TargetZ`) and eased tilt.
    target_z: f32,
    tilt: f32,
    /// Current (possibly zoomed-in) distance and height.
    dist: f32,
    height: f32,
}

impl FollowState {
    /// Places the camera proxy directly (respawn: `Camera_Syn` at Sammy + (32,32,150)).
    /// A fresh follow that eases in from `pos` (the intro starts at `CameraPosOrigine`).
    pub fn start_from(&mut self, pos: [f32; 3], ang: f32, params: &CameraParams) {
        *self = Self::default();
        self.pos = Some(pos);
        self.ang = ang;
        self.dist = params.dist;
        self.height = params.height;
    }

    pub fn snap_to(&mut self, pos: [f32; 3]) {
        self.pos = Some(pos);
        self.tilt_scale = None;
        self.orbit = false;
        self.set_pos = true;
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

fn angle_diff(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
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
    let t = time.delta_secs() * TICKS;
    if rt.cam_type == 0 {
        rt.cam_type = params.start_type;
    }
    if rt.cam_type == 2 {
        let Ok(p) = player.single() else { return };
        follow_type2(&params, &level, p, &mut st, &mut tf, t, time.delta_secs());
        st.converged = true; // no level-start sweep for type 2
        return;
    }

    // Type 3 follows `pov_Syn` (Sammy, or the extro's retargeted wasp/lion).
    let intro = current.phase == Phase::Intro;
    // 1. Yaw follows Sammy's pan (4/AverageFps per frame = 4 per second).
    if st.pos.is_none() {
        st.ang = pov.pan;
        // The level-start sweep flies in from `CameraPosOrigine`.
        if intro && let Some((o, ..)) = params.origin {
            st.pos = Some(o);
        }
    }
    if st.orbit {
        st.ang += 3.0 * t;
    } else {
        st.ang += angle_diff(pov.pan - st.ang) * (4.0 * time.delta_secs()).min(1.0);
    }

    // 2. Ideal position: `dist` behind at `height` above.
    if st.pos.is_none() {
        st.dist = params.dist;
        st.height = params.height;
    }
    let a = (st.ang + params.back).to_radians();
    let eye = [pov.pos[0], pov.pos[1], pov.pos[2] + params.plus_tilt * st.tilt_scale.unwrap_or(1.0)];
    let ideal_at = |d: f32, h: f32| [pov.pos[0] + a.cos() * d, pov.pos[1] + a.sin() * d, pov.pos[2] + h];
    let blocked = |to: [f32; 3]| level.as_ref().map(|l| l.0.trace(0, Hull::Narrow.index(), eye, to)).filter(|tr| tr.hit() && !tr.start_solid);

    // 4. Emergency camera (camera.wdl `Camera2_Syn`): while a wall hides Sammy, zoom in
    //    (distance ×(1-0.75·time) down to 50, height -50/tick down to 105); when the
    //    view clears, ease back out as long as the wider view stays clear.
    if blocked(ideal_at(st.dist, st.height)).is_some() {
        st.dist = (st.dist * (1.0 - 0.75 * t).max(0.0)).max(params.zoom_min_dist);
        st.height = (st.height - 50.0 * t).max(params.zoom_min_height);
    } else {
        let k = (0.1 * t).min(1.0);
        let (d, h) = (st.dist + (params.dist - st.dist) * k, st.height + (params.height - st.height) * k);
        if blocked(ideal_at(d, h)).is_none() {
            st.dist = d;
            st.height = h;
        }
    }
    let mut ideal = ideal_at(st.dist, st.height);
    // Never end up behind a wall: stop 32 quants in front of it.
    if let Some(tr) = blocked(ideal) {
        let len = ((ideal[0] - eye[0]).powi(2) + (ideal[1] - eye[1]).powi(2) + (ideal[2] - eye[2]).powi(2)).sqrt().max(1.0);
        ideal = [0, 1, 2].map(|i| tr.end[i] + (eye[i] - ideal[i]) * 32.0 / len);
    }

    // 3. Camera proxy eases toward the ideal (factor `fric × AdjustFpsCam3 × time`).
    //    Start sweep: `Camera3FrictionStart` on both axes with AdjustFpsCam3 0.1; after a
    //    zone transition both frictions are capped at 0.25 for 4 ticks.
    let pos = st.pos.get_or_insert(ideal);
    let (kh, kv) = if intro {
        let k = (params.start_fric * 0.1 * t).min(1.0);
        (k, k)
    } else {
        let cap = |f: f32| if rt.fric_cap > 0.0 { f.min(0.25) } else { f };
        ((cap(params.fric_h) * 0.5 * t).min(1.0), (cap(params.fric_v) * 0.5 * t).min(1.0))
    };
    let step = [(ideal[0] - pos[0]) * kh, (ideal[1] - pos[1]) * kh, (ideal[2] - pos[2]) * kv];
    for i in 0..3 {
        pos[i] += step[i];
    }
    let pos = *pos;
    // The sweep ends when the per-tick step (L1) falls to 4 quants (mm1.wdl:773-810).
    st.converged = (step[0].abs() + step[1].abs() + step[2].abs()) / t.max(1e-3) <= 4.0;

    // 5. Look at Sammy + PlusTilt.
    tf.translation = coords::pos(pos);
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

/// Camera type 2 (`HandleCamera`, camera.wdl:13-136) with `SetPosCamera` placement.
#[allow(clippy::too_many_arguments)]
fn follow_type2(
    params: &CameraParams,
    level: &Option<Res<LevelCollision>>,
    p: &Player,
    st: &mut FollowState,
    tf: &mut Transform,
    t: f32,
    dt: f32,
) {
    let me = p.pos;
    if st.pos.is_none() || st.set_pos {
        // SetPosCamera: camera at Sammy's back (pan − back), height HautCamPerso; the proxy
        // keeps its xy (from CameraPosOrigine) and only takes the z unless it can move in Z.
        let a = (p.pan - params.back).to_radians();
        let cam = [me[0] + params.org.0 * a.cos(), me[1] + params.org.0 * a.sin(), me[2] + params.org.1];
        let syn = match st.pos {
            Some(s) if st.set_pos => [s[0], s[1], if params.can_move_z { s[2] } else { cam[2] }],
            _ => {
                let o = params.origin.map_or(cam, |o| o.0);
                [o[0], o[1], if params.can_move_z { o[2] } else { cam[2] }]
            }
        };
        st.pos = Some(syn);
        st.ang = p.pan;
        st.target_z = me[2] + params.plus_tilt;
        st.set_pos = false;
    }
    st.ang += angle_diff(p.pan - st.ang) * (4.0 * dt).min(1.0);
    st.target_z += (me[2] + params.plus_tilt - st.target_z) / 4.0;
    let syn = st.pos.unwrap();
    let a = (st.ang - params.back).to_radians(); // NOTE minus: type 3 uses + back
    let gz = if params.can_move_z { syn[2] + (me[2] + params.height - syn[2]) * 0.25 } else { syn[2] };
    let goal = [me[0] + params.dist * a.cos(), me[1] + params.dist * a.sin(), gz];
    let v1 = (params.fric1_h * t).min(1.0);
    let v2 = (params.fric1_v * t).min(1.0);
    let delta = [(goal[0] - syn[0]) * v1, (goal[1] - syn[1]) * v1, (goal[2] - syn[2]) * v2];
    // The proxy flies through walls unless an obstacle sits in the last 20 % next to it.
    let mut pos = [syn[0] + delta[0], syn[1] + delta[1], syn[2] + delta[2]];
    if let Some(l) = level {
        let cam_dist = ((me[0] - syn[0]).powi(2) + (me[1] - syn[1]).powi(2) + (me[2] - syn[2]).powi(2)).sqrt();
        let los = l.0.trace(0, Hull::Point.index(), me, syn);
        let near_obstacle = los.hit() && los.fraction * cam_dist >= cam_dist * 0.8;
        if near_obstacle {
            let tr = l.0.trace(0, Hull::Narrow.index(), syn, pos);
            if !tr.start_solid {
                pos = tr.end;
            }
        }
        // Ground clearance: keep at least 80 above the floor.
        let floor = l.0.trace(0, Hull::Point.index(), pos, [pos[0], pos[1], pos[2] - 1000.0]);
        if floor.hit() && pos[2] - floor.end[2] < 80.0 {
            pos[2] += (floor.end[2] + 80.0 - pos[2]) * (0.1 * t).min(1.0);
        }
    }
    st.pos = Some(pos);
    // Look: pan snaps to Sammy, tilt eases 0.25 toward Camera_TargetZ.
    let d = [me[0] - pos[0], me[1] - pos[1], st.target_z - pos[2]];
    let pan = d[1].atan2(d[0]).to_degrees();
    let want_tilt = d[2].atan2((d[0] * d[0] + d[1] * d[1]).sqrt()).to_degrees();
    st.tilt += angle_diff(want_tilt - st.tilt) * (0.25 * t).min(1.0);
    tf.translation = coords::pos(pos);
    tf.rotation = coords::rotation([pan, st.tilt, 0.0]);
}
