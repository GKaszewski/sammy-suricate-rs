//! Collisions: other entities, the stuck roar, touching Sammy and wasp hits.

use super::*;

/// EVENT_ENTITY on the lion (§2.8 step 4) for non-Sammy entities; also the stuck roar.
pub(super) fn on_entity(e: &mut Enemy, ctx: &mut Ctx<'_>, other: Id) {
    let g = *ctx.g;
    e.guard_stuck += e.ball_accel;
    if stuck_roar(e, ctx) {
        return;
    }
    if ctx.sammy.is_some_and(|(s, _, _)| s == other) {
        e.random_time = 0.0;
        e.dest_pan = e.pan; // keep the heading into Sammy
        return;
    }
    // Bumped something else (another lion, a door): bounce and turn ±45°.
    let n = to_angle([-e.vel[0], -e.vel[1], 0.0]);
    let a = n.0 + if ctx.rng.random(1.0) > 0.5 { 45.0 } else { 315.0 };
    let (s, c) = n.0.to_radians().sin_cos();
    e.bounce = [c * g.intensite_bounce * e.weight, s * g.intensite_bounce * e.weight, 0.0];
    if e.delta_pan.abs() < g.delta_dest_pan {
        e.dest_pan = ang(a);
    }
    if e.flag(3) && e.etat != 2 && e.random_time <= 0.0 {
        e.random_time = e.time_for_nothing;
    }
}

/// Stuck roar (Lion_Hit :319-376), checked first on every collision event: a lion
/// running and stuck (`GuardStuck > 10`) with Sammy in clear view roars, and with
/// Flag7 calls its friends.
pub(super) fn stuck_roar(e: &mut Enemy, ctx: &mut Ctx<'_>) -> bool {
    if e.guard_stuck > 10.0 && matches!(e.etat, 2 | 12 | 13) {
        e.random_time = 0.0;
        e.guard_stuck = 0.0;
        // The original traces from Sammy+100 to the lion+100 against level geometry only.
        let clear = ctx.sammy.is_some_and(|(_, sp, _)| {
            !ctx.space.trace_static(Hull::Point, [sp[0], sp[1], sp[2] + 100.0], [e.pos[0], e.pos[1], e.pos[2] + 100.0]).trace.hit()
        });
        if clear {
            e.dest_pan = ang(e.pan - 180.0);
            if e.spec == 3 {
                ctx.sounds.push("ScorpionAttack_son");
            } else if e.flag(7) && ctx.calls.attack_call == 0.0 {
                ctx.sounds.push("tigre_son");
                ctx.sounds.push("LionAttack1_son");
            } else {
                ctx.sounds.push("tigre_son");
            }
            e.random_time = e.time_for_nothing * 2.0;
            e.set_state(3, 4.0);
            if e.flag(7) && e.dist < e.attack_range && ctx.calls.attack_call == 0.0 && ctx.calls.answer != 0 {
                // CallFriendsToHelp.
                ctx.calls.answer = 0;
                ctx.calls.attack_call = ctx.calls.tempo_call;
                ctx.calls.friend_call = e.spec as f32 + e.weight;
                ctx.call_rings.push(e.pos);
            }
            return true;
        }
    }
    false
}

/// Lion touching Sammy (§4.1/4.2): bump damage and swipe start, then the swipe hit.
/// Returns the number of hit stars to emit (`particle_Hit1`, lion.wdl:499,510).
pub(super) fn contact_sammy(e: &mut Enemy, p: &mut Player, game: &mut GameState, g: &LionGlobals, sounds: &mut Vec<&'static str>, rng: &mut Rng) -> usize {
    // Sammy is pushed by the lion's motion (`ObjetPoussant`): vel × 0.8 × BounceForce.
    p.knock[0] = e.vel[0] * 0.8 * e.push_force;
    p.knock[1] = e.vel[1] * 0.8 * e.push_force;
    if e.etat == 2 {
        game.hearts -= e.resistance * 0.25;
        e.set_state(10, e.anim_speed); // swipe at the inherited run speed
        e.swipe_pending = true;
        return 1;
    } else if e.etat == 10 && e.anim_step > 60.0 && e.swipe_pending {
        game.hearts -= e.resistance * g.hargne;
        e.swipe_pending = false;
        sounds.push(["Hit1", "Hit2", "Hit3"][(rng.random(3.0) as usize).min(2)]);
        return 10;
    }
    0
}

/// Wasp impact on a lion (§5.1, IMPACT path).
pub(super) fn wasp_hit(e: &mut Enemy, h: &Impact) {
    if e.class != LION || matches!(e.etat, 7) {
        return;
    }
    if e.etat == 0 || e.etat == 4 {
        take_attack_bis(e);
        second_react_range(e);
    }
    e.dest_pan = h.wasp_pan;
    e.bounce = [h.wasp_disp[0] * e.weight * h.bounce_force, h.wasp_disp[1] * e.weight * h.bounce_force, 0.0];
    if e.flag(3) && e.random_time <= 0.0 {
        e.random_time = e.time_for_nothing;
    }
    e.resistance -= h.poison;
    if e.resistance <= 0.0 {
        if e.etat != 6 {
            e.pan = ang(e.dest_pan - 180.0);
            e.set_state(6, 4.0);
            e.vspeed = 20.0;
            e.fric_vspeed = 3.5;
        }
    } else if e.etat != 11 && e.etat != 6 {
        e.vspeed = 20.0 * e.weight;
        e.fric_vspeed = 3.5;
        e.attacked += h.poison;
        if e.resistance < e.resistance_org * 0.95 {
            let s = (10.0 * e.resistance * 0.3).clamp(5.0, 20.0);
            e.set_state(11, s);
        } else {
            e.bounce[0] *= 2.5;
            e.bounce[1] *= 2.5;
        }
    }
}

/// Sammy's collision box from his model's first frame (scaled): (half extents, centre
/// height above his origin).
pub fn player_box(model: &Mdl, scale: f32) -> Option<([f32; 3], f32)> {
    let f = model.frames.first()?;
    let hx = (f.bbox_max[0] - f.bbox_min[0]).max(f.bbox_max[1] - f.bbox_min[1]) * 0.25 * scale;
    let hz = (f.bbox_max[2] - f.bbox_min[2]) * 0.5 * scale;
    let cz = (f.bbox_max[2] + f.bbox_min[2]) * 0.5 * scale;
    Some(([hx, hx, hz], cz))
}
