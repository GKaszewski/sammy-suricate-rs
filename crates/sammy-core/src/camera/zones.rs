//! Camera zones (`SetCamera`) and the respawn reset.

use super::*;

/// A camera zone (`SetCamera`, camera.wdl:958-1173).
#[derive(Clone, Debug)]
pub struct CameraZone {
    pub(super) s: [f32; 8],
    pub(super) pos: [f32; 3],
    pub(super) armed: bool,
    pub(super) pending: bool,
}

impl CameraZone {
    pub fn from_object(obj: &Object) -> Self {
        let mut s = [0.0; 8];
        s.copy_from_slice(&obj.skills[..8]);
        Self { s, pos: obj.origin, armed: true, pending: false }
    }
}

/// `SetCameraHit`: Sammy's per-tick id-25 scan (radius 500) arms a zone when the camera
/// differs and the zone sees Sammy; a tick later, within Skill7, it blends distance,
/// height, back angle and PlusTilt over Skill6+1 steps, snaps, and re-arms.
pub fn camera_zones(t: f32, level: &Wmb, params: &mut CameraParams, rt: &mut CameraRuntime, scanners: &Scanners, zones: &mut [(Id, &mut CameraZone)]) {
    rt.acc += t;
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
                if let Some((_, z)) = zones.iter_mut().find(|(id, _)| *id == tw.zone) {
                    z.armed = true;
                }
            }
            continue;
        }
        // Confirm a pending zone one tick later (distance ≤ Skill7), else drop it.
        let mut started = false;
        for (e, z) in zones.iter_mut() {
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
                zone: *e,
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
        for (_, z) in zones.iter_mut() {
            if !z.armed {
                continue;
            }
            let Some((sp, d)) = scanners.nearest(z.pos) else { continue };
            if d > RANGE_SCAN_25 {
                continue;
            }
            let ty = if z.s[0] == 3.0 { 3 } else { 2 };
            if rt.cam_type == ty && params.dist == z.s[1] && params.height == z.s[2] && params.plus_tilt == z.s[4] {
                continue;
            }
            if level.trace(0, Hull::Point.index(), z.pos, sp).hit() {
                continue;
            }
            z.pending = true;
            break; // one zone per tick (the original's same-tick race is not replicated)
        }
    }
}

/// Respawn (`RestartSammy`): camera type back to the level's, Org values restored
/// (the back angle is not), running zone transitions interrupted; the follow camera is
/// placed again.
pub fn reset_on_respawn(params: &mut CameraParams, rt: &mut CameraRuntime, follow: &mut FollowState) {
    (params.dist, params.height, params.plus_tilt) = params.org;
    rt.cam_type = params.start_type;
    rt.bouge = false;
    rt.tween = None;
    follow.set_pos = true;
}
