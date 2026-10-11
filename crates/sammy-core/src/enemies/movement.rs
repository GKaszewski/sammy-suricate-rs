//! Hovering, moving with glide, speed and bouncing off walls.

use super::*;

/// `HandleGravityEnnemy` (§2.4): hover at a constant height over the traced floor.
pub(super) fn gravity(e: &mut Enemy, ctx: &mut Ctx<'_>) {
    let g = ctx.g;
    let hit = ctx.space.trace(Hull::Point, e.pos, [e.pos[0], e.pos[1], e.pos[2] - 2000.0], Some(ctx.me));
    e.floor_entity = hit.entity.filter(|_| hit.trace.hit());
    if hit.trace.hit() {
        e.floor = hit.trace.end[2];
        e.vertical = 0.0;
    } else {
        e.vertical = 0.0;
        e.vspeed = 0.0;
    }
    if e.flag(8) {
        // Flyers: no gravity, only kept 64 above the floor (lion.wdl:1394-1403).
        if e.pos[2] < e.floor + 64.0 {
            e.vertical = (e.floor + 64.0) - e.pos[2];
            e.vspeed = 0.0;
        }
        return;
    }
    if e.vspeed == 0.0 {
        if e.pos[2] - e.floor > e.hover + 0.5 {
            let up = ctx.sammy.map_or(0.0, |(_, sp, _)| sp[2] - e.pos[2]);
            if e.pos[2] - e.floor > 200.0 && up > 100.0 && up < 400.0 {
                e.vspeed = 20.0;
                e.fric_vspeed = 3.0;
            } else {
                e.vspeed = g.gravite;
                e.fric_vspeed = g.acc_gravite;
            }
        } else {
            e.vertical = (e.floor + e.hover) - e.pos[2];
        }
    }
    if e.vspeed != 0.0 {
        e.vspeed -= e.fric_vspeed;
        if e.pos[2] < e.floor + e.hover {
            e.vertical = (e.floor + e.hover) - e.pos[2];
            e.vspeed = 0.0;
        } else {
            e.vertical = e.vspeed;
        }
    }
}

/// MOVE with glide; returns (moved distance, wall normal if a wall was hit, entity hit).
pub(super) fn move_enemy(e: &mut Enemy, ctx: &mut Ctx<'_>, mut delta: [f32; 3]) -> (f32, Option<[f32; 3]>, Option<Id>) {
    let start = e.pos;
    let mut wall = None;
    let mut ent = None;
    for _ in 0..3 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [0, 1, 2].map(|i| e.pos[i] + delta[i]);
        let hit = ctx.space.trace(HULL, e.pos, end, Some(ctx.me));
        if hit.trace.start_solid {
            break;
        }
        e.pos = hit.trace.end;
        if !hit.trace.hit() {
            break;
        }
        if hit.entity.is_some() {
            ent = hit.entity;
        } else if hit.trace.normal[2].abs() < 0.7 {
            wall = Some(hit.trace.normal);
        }
        let rest = delta.map(|d| d * (1.0 - hit.trace.fraction));
        let n = hit.trace.normal;
        let into = rest[0] * n[0] + rest[1] * n[1] + rest[2] * n[2];
        delta = [0, 1, 2].map(|i| rest[i] - n[i] * into);
    }
    let moved = (0..3).map(|i| (e.pos[i] - start[i]).powi(2)).sum::<f32>().sqrt();
    (moved, wall, ent)
}

/// `HandleSpeedLion` (§2.3).
pub(super) fn handle_speed(e: &mut Enemy, ctx: &mut Ctx<'_>) {
    let mut fwd = 0.0;
    if e.etat == 1 && e.delta_pan.abs() < 10.0 {
        fwd = e.speed * e.ball_accel;
    }
    if e.etat == 2 || e.etat == 13 {
        fwd = e.speed * e.ball_accel;
    }
    if e.etat == 10 {
        fwd = e.speed * e.ball_accel * 1.5;
    }
    if e.etat == 12 {
        fwd = e.speed * e.ball_accel * 1.2;
    }
    gravity(e, ctx);
    let dir = forward(e.pan, e.tilt);
    let before = e.pos;
    let (moved, wall, ent) = move_enemy(e, ctx, [dir[0] * fwd, dir[1] * fwd, dir[2] * fwd + e.vertical]);
    e.vel = if moved == 0.0 {
        let (s, c) = e.pan.to_radians().sin_cos();
        [c * 0.1, s * 0.1, e.vel[2]]
    } else {
        [0, 1, 2].map(|i| e.pos[i] - before[i])
    };
    if let Some(n) = wall
        && e.enable_block
    {
        on_block(e, ctx, n);
    }
    if let Some(other) = ent {
        on_entity(e, ctx, other);
    }

    // Muzzle wall probe (:1122-1162): 32 quants ahead of the head vertex.
    e.muzzle = None;
    if e.head_vertex != 0 && ctx.g.dist_muzzle != 0.0
        && let Some(head) = head_pos(e, ctx.head_local) {
            e.muzzle = Some(head);
            let f = forward(e.pan, e.tilt);
            let tip = [0, 1, 2].map(|i| head[i] + f[i] * ctx.g.dist_muzzle);
            let tr = ctx.space.trace_static(Hull::Point, head, tip);
            if tr.trace.start_solid {
                let a = if e.etat == 6 { e.pan } else { e.pan - 180.0 };
                let (s, c) = a.to_radians().sin_cos();
                e.bounce[0] = c * ctx.g.intensite_bounce;
                e.bounce[1] = s * ctx.g.intensite_bounce;
            } else if tr.trace.hit() && tr.entity.is_none() {
                on_block(e, ctx, tr.trace.normal);
            }
        }

    // Knockback channel (:1164-1204), decaying linearly (AverageFps reference 50).
    if e.bounce != [0.0; 3] {
        let b = e.bounce;
        move_enemy(e, ctx, b);
        let dec = ctx.g.amorti_bounce * 80.0 / 51.0;
        e.bounce = e.bounce.map(|v| if v.abs() <= dec { 0.0 } else { v - dec * v.signum() });
    }
}

/// Head vertex in world space (current frame).
pub(super) fn head_pos(e: &Enemy, local: Option<[f32; 3]>) -> Option<[f32; 3]> {
    let v = local?;
    let (s, c) = e.pan.to_radians().sin_cos();
    let (x, y, z) = (v[0] * e.scale, v[1] * e.scale, v[2] * e.scale);
    Some([e.pos[0] + x * c - y * s, e.pos[1] + x * s + y * c, e.pos[2] + z])
}

/// `EnnemyOnBlock` (§2.8): bounce off the wall and re-steer 25-45° off its normal.
pub(super) fn on_block(e: &mut Enemy, ctx: &mut Ctx<'_>, n: [f32; 3]) {
    let g = *ctx.g;
    if stuck_roar(e, ctx) {
        return;
    }
    e.guard_stuck += 3.0;
    let mut v = (e.vel[0].abs() + e.vel[1].abs()) * 0.1 * e.weight * g.intensite_bounce;
    if v == 0.0 {
        v = g.intensite_bounce;
    }
    e.bounce = n.map(|c| c * v);
    let coin = ctx.rng.random(1.0) > 0.5;
    let off = ctx.rng.random(20.0) + 25.0;
    let a = to_angle(n).0 + if coin { off } else { -off };
    if e.random_time <= 0.0 && e.delta_pan.abs() < g.delta_dest_pan {
        e.dest_pan = ang(a);
    }
    if e.flag(3) {
        if e.etat != 2 && e.random_time <= 0.0 {
            e.random_time = e.time_for_nothing * 1.5;
        }
        if e.etat == 2 || e.etat == 12 {
            if e.random_time <= 0.0 {
                e.random_time = e.time_for_nothing;
            }
            e.etat = 1;
            e.anim_speed = e.walk_speed();
        }
    }
}
