//! The game camera ([`crate::camera`]): per-level settings, zones, the follow camera,
//! and the sky domes that hang on it. Its pose is what the frontend renders from.

use super::{Dt, Sim};
use crate::camera::{CameraParams, CameraPose, CameraRuntime, FollowState, Subject};
use crate::flow::Phase;
use crate::id::Id;
use crate::time::TICKS_PER_SEC;
use crate::angle::Angles;
use crate::space::Pos;

/// The camera's state and this frame's pose.
#[derive(Clone, Debug, Default)]
pub struct Camera {
    pub params: CameraParams,
    pub runtime: CameraRuntime,
    pub follow: FollowState,
    /// Where the camera is (None until something placed it).
    pub pose: Option<CameraPose>,
}

/// Position and (pan, tilt) of a pose.
pub fn pan_tilt(pose: &CameraPose) -> ([f32; 3], f32, f32) {
    match *pose {
        CameraPose::LookAt { pos, target } => {
            let d = [target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]];
            (pos, d[1].atan2(d[0]).to_degrees(), d[2].atan2(d[0].hypot(d[1])).to_degrees())
        }
        CameraPose::Angles { pos, angles } => (pos, angles.pan.0, angles.tilt.0),
    }
}

/// Respawn (`RestartSammy`) resets the camera.
pub(super) fn reset_on_respawn(sim: &mut Sim, reset: bool) {
    if reset {
        let c = &mut sim.camera;
        crate::camera::reset_on_respawn(&mut c.params, &mut c.runtime, &mut c.follow);
    }
}

/// Zones react to id-25 scanners while playing (and during the intro's follow).
pub(super) fn zones(sim: &mut Sim, dt: Dt) {
    let Some(level) = sim.level.clone() else { return };
    if sim.flow.phase != Phase::Playing && !sim.intro.following() {
        return;
    }
    let mut refs: Vec<(Id, &mut crate::camera::CameraZone)> = sim.world.camera_zones.iter_mut().map(|(id, z)| (*id, z)).collect();
    crate::camera::camera_zones(dt.t, &level, &mut sim.camera.params, &mut sim.camera.runtime, &sim.scanners, &mut refs);
}

/// The follow camera (not in the menu, except while the intro follows its Sammy).
pub(super) fn follow(sim: &mut Sim, dt: Dt) {
    if sim.flow.phase == Phase::Menu && !sim.intro.following() {
        return;
    }
    if sim.pov.entity.is_none() {
        return;
    }
    let subject = Subject { pos: sim.pov.pos, pan: sim.pov.pan };
    let sammy = sim.world.player().map(|(_, p)| Subject { pos: p.pos, pan: p.pan.get() });
    let sweep = sim.flow.phase == Phase::Intro;
    let c = &mut sim.camera;
    if let Some(pose) = crate::camera::follow(&mut c.follow, &c.params, &mut c.runtime, sim.level.as_deref(), subject, sammy, sweep, dt.t, dt.secs) {
        c.pose = Some(pose);
    }
    if let (Some([dist, z, angle]), Some(s)) = (sim.debug.portrait, sammy) {
        let a = (s.pan + angle).to_radians();
        let target = [s.pos[0], s.pos[1], s.pos[2] + z];
        sim.camera.pose = Some(CameraPose::LookAt { pos: [target[0] + a.cos() * dist, target[1] + a.sin() * dist, target[2]], target });
    }
}

/// Sky domes stay centred on the camera at their height offset, spinning.
pub(super) fn sky(sim: &mut Sim, dt: Dt) {
    let Some(pose) = sim.camera.pose else { return };
    let (cam, ..) = pan_tilt(&pose);
    let ticks = dt.secs * TICKS_PER_SEC;
    let w = &mut sim.world;
    for (id, s) in w.sky_follow.iter_mut() {
        for (a, spin) in s.angles.iter_mut().zip(s.spin) {
            *a += spin * ticks;
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = Pos([cam[0], cam[1], cam[2] + s.dz]);
            b.angles = Angles::from_array(s.angles);
        }
    }
}
