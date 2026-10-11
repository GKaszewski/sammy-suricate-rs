//! Patrolling paths and random wandering.

use super::*;

/// Patrol step (§2.5). Returns the rotation rate override.
pub(super) fn patrol(e: &mut Enemy, ctx: &mut Ctx<'_>, g: &LionGlobals) -> Option<f32> {
    let nodes = ctx.nodes;
    let scan = |e: &mut Enemy, from: [f32; 3], pan: f32, tilt: f32, h: f32, range: f32| -> bool {
        // PathScanned: the first node with level line of sight wins; failing that,
        // the last node scanned still overwrites the target.
        let old = e.target;
        let mut accepted = false;
        let path_id = e.path_id;
        for n in nodes.iter().filter(|n| n.id == path_id) {
            if accepted || (n.pos[0] == e.target[0] && n.pos[1] == e.target[1]) {
                continue;
            }
            if !in_cone(from, pan, tilt, h, 120.0, range, n.pos) {
                continue;
            }
            if n.next != 0.0 {
                e.path_id = n.next;
            }
            e.target = n.pos;
            e.target_pan = n.pan;
            e.target_tilt = ang(n.tilt);
            let tr = ctx.space.trace_static(Hull::Point, n.pos, e.pos).trace;
            if !tr.hit() {
                accepted = true;
            }
        }
        e.target[0] != old[0] || e.target[1] != old[1]
    };

    e.time_without_patrol += 1.0;
    if e.time_without_patrol > g.limit_time_without_patrol {
        e.etat_bis = 0;
    }
    if e.etat_bis == 0 {
        let (pos, pan, tilt, range) = (e.pos, e.pan, e.tilt, e.path_range);
        if scan(e, pos, pan, tilt, 90.0, range) {
            e.etat_bis = 1;
            e.time_without_patrol = 0.0;
        } else {
            let d = to_angle([e.target[0] - e.pos[0], e.target[1] - e.pos[1], 0.0]);
            if e.target[0] != f32::MAX && d.2 > range * 1.8 {
                e.etat_bis = 0;
                if !scan(e, pos, pan, tilt, 100.0, range * 4.0) {
                    scan(e, pos, ang(pan - 180.0), 0.0, 100.0, range * 3.0);
                    e.time_without_patrol = 0.0;
                }
            }
        }
        return None;
    }
    let d = to_angle([e.target[0] - e.pos[0], e.target[1] - e.pos[1], 0.0]);
    let mut v = e.dest_pan;
    let mut rot = None;
    let pull = (g.love_patrol * 0.5).min(1.0);
    if d.2 < g.far_target_path && (e.etat_bis == 1 || e.etat_bis == 2) {
        v = e.target_pan;
        rot = Some(g.rotation_near);
        e.etat_bis = 2;
        e.bounce[0] = (e.target[0] - e.pos[0]) * pull;
        e.bounce[1] = (e.target[1] - e.pos[1]) * pull;
        e.tilt += ang((e.target_tilt - e.tilt) * 0.5);
    }
    if d.2 >= g.far_target_path && e.etat_bis == 1 {
        v = d.0;
        rot = Some(g.rotation_far);
    }
    if d.2 > e.path_range && e.etat_bis == 2 {
        e.etat_bis = 1;
        v = d.0;
        rot = Some(g.rotation_far);
    }
    e.dest_pan = if e.random_time <= 0.0 { ang(v) } else { ang(e.dest_pan + (v - e.dest_pan) * g.love_patrol) };
    if d.2 < g.near_target_path {
        e.etat_bis = 0;
        let (tp, tt, range) = (e.target, (e.target_pan, e.target_tilt), e.path_range);
        if scan(e, tp, tt.0, tt.1, 30.0, range) {
            e.etat_bis = 1;
            e.time_without_patrol = 0.0;
        }
    }
    rot
}

/// `RandomizeLion` (§2.7).
pub(super) fn randomize(e: &mut Enemy, ctx: &mut Ctx<'_>) {
    if e.random_time > 0.0 {
        return;
    }
    if e.random_time2 > 0.0 {
        e.random_time2 -= 1.0;
        return;
    }
    let v = ctx.rng.random(3.0) as i32 + 1;
    if v == 1 && e.etat != 8 {
        e.set_state(8, 3.0);
        e.random_time = e.random_org * 0.5;
        e.random_time2 = ctx.rng.random(e.random_org2) * 0.5;
        return;
    }
    if v == 2 {
        e.dest_pan = ctx.rng.random(360.0);
        if e.etat != 1 {
            e.anim_step = 0.0;
        }
        e.etat = 1;
        e.anim_speed = e.walk_speed();
        e.random_time = e.random_org;
        e.random_time2 = ctx.rng.random(e.random_org2) * 0.5;
        return;
    }
    if e.etat != 2 {
        e.anim_step = 0.0;
    }
    e.dest_pan = ctx.rng.random(360.0);
    e.etat = 13;
    e.anim_speed = e.run_speed();
    e.random_time = e.random_org;
    e.random_time2 = ctx.rng.random(e.random_org2);
}
