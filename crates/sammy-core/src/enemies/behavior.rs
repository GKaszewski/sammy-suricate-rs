//! The state machine (`EnnemyBehavior`).

use super::*;

/// `EnnemyBehavior` (§2.2), one tick.
pub(super) fn enemy_behavior(e: &mut Enemy, ctx: &mut Ctx<'_>) {
    let g = *ctx.g;
    if e.etat == 7 {
        return;
    }
    if e.etat == 6 || e.etat == 3 {
        handle_speed(e, ctx);
        e.ball_accel += (1.0 - e.ball_accel) * 0.1;
        return;
    }
    if e.etat == 8 || e.etat == 9 {
        let d = test_dist(e, ctx, &g);
        if d.2 < e.attack_range && e.anim_step > 50.0 && sees_sammy(e, ctx) {
            take_attack_bis(e);
            e.guard_stuck = 0.0;
            let w = e.walk_speed();
            e.set_state(1, w);
        }
        e.guard_stuck = 6.0;
        handle_speed(e, ctx);
        e.random_time -= 1.0;
        randomize(e, ctx);
        e.guard_stuck -= 1.0;
        return;
    }
    if e.etat == 0 || e.etat == 4 {
        let d = test_dist(e, ctx, &g);
        e.guard_stuck = 12.0;
        if d.2 < e.react_range {
            // React by distance only, no line of sight (lion.wdl:1760).
            second_react_range(e);
            e.guard_stuck = 0.0;
            let w = e.walk_speed();
            e.set_state(1, w);
        } else {
            gravity(e, ctx);
            move_enemy(e, ctx, [0.0, 0.0, e.vertical]);
            e.guard_stuck -= 1.0;
        }
        return;
    }

    // Active states: 1, 2, 10, 11, 12, 13.
    let mut rotation = g.rotation_all;
    if e.etat == 1 && e.flag(3) {
        rotation = patrol(e, ctx, &g).unwrap_or(rotation);
    }
    handle_speed(e, ctx);

    if !e.flag(4) {
        let d = test_dist(e, ctx, &g);
        let period = match (e.spec == 3, e.etat == 2) {
            (false, false) => 10,
            (false, true) => 4,
            (true, false) => 16,
            (true, true) => 8,
        };
        let vu = ctx.ticks1.is_multiple_of(period) && sees_sammy(e, ctx);
        if vu && ((e.etat == 1 && ctx.rng.random(100.0) > g.random_attack) || e.etat == 9) {
            let random_mode = e.random_org2 != 0.0;
            if d.2 <= e.attack_range && d.1.abs() < 45.0 && ((e.random_time <= 0.0 && !random_mode) || random_mode) {
                if e.spec == 1 && ctx.rng.random(100.0) > g.random_attack {
                    ctx.sounds.push("LionAttack1_son");
                }
                if e.spec == 3 && ctx.rng.random(100.0) > g.random_attack {
                    ctx.sounds.push("ScorpionAttack_son");
                }
                *ctx.warn = true; // Warning_Attack
                e.etat_bis = 0;
                e.dest_pan = d.0;
                let r = e.run_speed();
                e.set_state(2, r);
                take_attack_bis(e);
            }
        }
        if e.etat == 2 || e.etat == 10 {
            if d.2 > e.attack_range + 128.0 {
                let w = e.walk_speed();
                e.set_state(1, w);
                if e.spec == 3 {
                    ctx.sounds.push("ScorpionWalk_son");
                }
            } else {
                e.ball_accel = e.accel_attack;
                if e.random_time <= 0.0 && e.guard_stuck < 9.0 {
                    e.dest_pan = d.0; // homing
                }
            }
            if d.2 < 16.0 {
                e.dest_pan = ang(d.0 - 180.0);
                e.random_time = e.time_for_nothing * 2.0;
            }
        } else {
            if e.etat == 13 {
                e.ball_accel = e.accel_attack;
                if e.guard_stuck > 9.0 {
                    e.dest_pan = ang(e.dest_pan + ctx.rng.random(20.0) - 10.0);
                }
            }
            if !e.flag(3) && e.random_org2 != 0.0 {
                randomize(e, ctx);
            }
        }
        // Fear (§2.2 :2128).
        if e.attacked > 0.0 {
            let v = (e.resistance * e.weight * g.lachete).abs();
            e.attacked -= g.frightened * v;
            if e.attacked > v {
                if e.etat <= 1 {
                    e.etat_bis = 0;
                    e.dest_pan = ctx.rng.random(360.0);
                    let r = e.run_speed();
                    e.set_state(12, r);
                    e.random_time = e.time_for_nothing;
                }
            } else if e.etat == 12 {
                let w = e.walk_speed();
                e.set_state(1, w);
            }
        }
        if e.etat == 12 {
            e.ball_accel = e.accel_attack;
        }
    }

    // Suite0 (:2172): rotation toward the destination heading.
    e.random_time -= 1.0;
    e.delta_pan = ang(e.dest_pan - e.pan) * rotation;
    if e.flag(8) {
        // Flyers turn anywhere and pitch toward the patrol target height (:2176-2198).
        e.pan = ang(e.pan + e.delta_pan);
        if e.flag(3) && e.target[2] != f32::MAX {
            let tz = e.target[2];
            if (e.pos[2] - tz).abs() < 16.0 {
                e.tilt -= e.tilt * 0.25;
            } else if e.pos[2] > tz {
                e.tilt -= rotation;
            } else {
                e.tilt += rotation;
            }
        }
    } else if e.pos[2] - e.floor < 128.0 {
        e.pan = ang(e.pan + e.delta_pan);
    }
    e.enable_block = e.delta_pan.abs() <= 6.0;
    if e.guard_stuck > 0.0 {
        e.guard_stuck -= 0.25;
        if e.guard_stuck > 16.0 {
            if !e.flag(3) {
                e.dest_pan = ang(e.pan - 180.0);
            } else {
                e.etat_bis = 0;
            }
            e.guard_stuck = 0.0;
        }
    }
    e.ball_accel += (1.0 - e.ball_accel) * 0.1;
}
