//! The type-2 camera (`HandleCamera`).

use super::*;

/// Camera type 2 (`HandleCamera`, camera.wdl:13-136) with `SetPosCamera` placement.
#[allow(clippy::too_many_arguments)]
pub(super) fn follow_type2(params: &CameraParams, level: Option<&Wmb>, p: Subject, st: &mut FollowState, t: f32, dt: f32) -> CameraPose {
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
        let los = l.trace(0, Hull::Point.index(), me, syn);
        let near_obstacle = los.hit() && los.fraction * cam_dist >= cam_dist * 0.8;
        if near_obstacle {
            let tr = l.trace(0, Hull::Narrow.index(), syn, pos);
            if !tr.start_solid {
                pos = tr.end;
            }
        }
        // Ground clearance: keep at least 80 above the floor.
        let floor = l.trace(0, Hull::Point.index(), pos, [pos[0], pos[1], pos[2] - 1000.0]);
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
    CameraPose::Angles { pos, angles: [pan, st.tilt, 0.0] }
}
