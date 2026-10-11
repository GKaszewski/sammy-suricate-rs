//! The follow camera's state and its frame.

use super::*;
use crate::angle::Degrees;
use crate::time::{Secs, Ticks};

/// Follow-camera state in native coordinates.
#[derive(Clone, Debug, Default)]
pub struct FollowState {
    /// `AngCamPerso`: smoothed copy of Sammy's pan.
    pub(super) ang: f32,
    pub(super) pos: Option<[f32; 3]>,
    /// Within 4 quants of the ideal position (ends the level-start sweep).
    pub converged: bool,
    /// Multiplier on `Camera_PlusTilt` (drops to 0 while Sammy dies, `Player_Kaputt`).
    pub tilt_scale: Option<f32>,
    /// Game over: orbit Sammy at 3°/tick instead of following his pan.
    pub orbit: bool,
    /// Type 2: place the camera with `SetPosCamera` on the next update.
    pub set_pos: bool,
    /// Type 2: smoothed look-at height (`Camera_TargetZ`) and eased tilt.
    pub(super) target_z: f32,
    pub(super) tilt: f32,
    /// Current (possibly zoomed-in) distance and height.
    pub(super) dist: f32,
    pub(super) height: f32,
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

/// Where the camera is and how it looks.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CameraPose {
    /// Looking at a point (type 3).
    LookAt { pos: [f32; 3], target: [f32; 3] },
    /// Pan and tilt (type 2).
    Angles { pos: [f32; 3], angles: crate::angle::Angles },
}

/// The camera's subject this frame: `pov_Syn` for type 3, Sammy for type 2.
#[derive(Clone, Copy, Debug)]
pub struct Subject {
    pub pos: [f32; 3],
    pub pan: Degrees,
}

/// One frame of the follow camera (`t` ticks, `dt` seconds). `sweep`: the level-start
/// fly-in is running. `sammy` is needed by the type-2 camera.
#[allow(clippy::too_many_arguments)]
pub fn follow(st: &mut FollowState, params: &CameraParams, rt: &mut CameraRuntime, level: Option<&Wmb>, pov: Subject, sammy: Option<Subject>, sweep: bool, t: Ticks, dt: Secs) -> Option<CameraPose> {
    let dt = dt.get();
    if rt.cam_type == 0 {
        rt.cam_type = params.start_type;
    }
    if rt.cam_type == 2 {
        let p = sammy?;
        let pose = follow_type2(params, level, p, st, t, dt);
        st.converged = true; // no level-start sweep for type 2
        return Some(pose);
    }

    // Type 3 follows `pov_Syn` (Sammy, or the extro's retargeted wasp/lion).
    let intro = sweep;
    // 1. Yaw follows Sammy's pan (4/AverageFps per frame = 4 per second).
    if st.pos.is_none() {
        st.ang = pov.pan.get();
        // The level-start sweep flies in from `CameraPosOrigine`.
        if intro && let Some((o, ..)) = params.origin {
            st.pos = Some(o);
        }
    }
    if st.orbit {
        st.ang += 3.0 * t;
    } else {
        st.ang += angle_diff((pov.pan - Degrees(st.ang)).get()) * (4.0 * dt).min(1.0);
    }

    // 2. Ideal position: `dist` behind at `height` above.
    if st.pos.is_none() {
        st.dist = params.dist;
        st.height = params.height;
    }
    let a = (st.ang + params.back).to_radians();
    let eye = [pov.pos[0], pov.pos[1], pov.pos[2] + params.plus_tilt * st.tilt_scale.unwrap_or(1.0)];
    let ideal_at = |d: f32, h: f32| [pov.pos[0] + a.cos() * d, pov.pos[1] + a.sin() * d, pov.pos[2] + h];
    let blocked = |to: [f32; 3]| level.map(|l| l.trace(0, Hull::Narrow.index(), eye, to)).filter(|tr| tr.hit() && !tr.start_solid);

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
    st.converged = (step[0].abs() + step[1].abs() + step[2].abs()) / t.get().max(1e-3) <= 4.0;

    // 5. Look at Sammy + PlusTilt.
    Some(CameraPose::LookAt { pos, target: eye })
}
