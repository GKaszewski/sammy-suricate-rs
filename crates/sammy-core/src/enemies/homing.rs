//! Wasps homing onto enemies.

use super::*;
use crate::time::Ticks;

/// Wasp homing and fear tagging (`Wasp_Hit` EVENT_DETECT, docs/wasps.md §4.4): the
/// wasp steers toward every living target in its cone, nearest first.
pub fn home_wasp(w: &mut crate::wasps::Wasp, enemies: &mut [&mut Enemy], t: Ticks) {
    if !w.can_home() {
        return;
    }
    let aim_of = |e: &Enemy| e.muzzle.map(|m| [m[0], m[1], m[2] - 32.0]).unwrap_or([e.pos[0], e.pos[1], e.pos[2] + 32.0]);
    let mut targets: Vec<(f32, &mut Enemy)> = enemies
        .iter_mut()
        .filter(|e| e.alive() && e.is_class_target())
        .filter_map(|e| {
            let aim = aim_of(e);
            in_cone(w.pos, w.pan, w.tilt, 120.0, 120.0, w.homing_range, aim).then(|| (to_angle([aim[0] - w.pos[0], aim[1] - w.pos[1], aim[2] - w.pos[2]]).2, &mut **e))
        })
        .collect();
    targets.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, e) in targets {
        if e.attacked <= 0.0 && w.find_cd <= 0.0 {
            w.find_cd = 16.0;
            e.attacked += w.poison;
        }
        let aim = aim_of(e);
        let (dp, dt, _) = to_angle([aim[0] - w.pos[0], aim[1] - w.pos[1], aim[2] - w.pos[2]]);
        w.pan += ang(dp - w.pan) * 0.4 * t;
        let gain = w.tilt_gain;
        w.tilt += ang(dt - w.tilt) * gain * t;
    }
}
