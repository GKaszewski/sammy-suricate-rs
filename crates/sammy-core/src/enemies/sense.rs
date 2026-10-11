//! Finding Sammy: distance, line of sight and reaction ranges.

use super::*;

/// `TestDistLionPlayer` (§2.6): returns (pan, tilt, dist) to Sammy.
pub(super) fn test_dist(e: &mut Enemy, ctx: &mut Ctx<'_>, g: &LionGlobals) -> (f32, f32, f32) {
    let Some((_, sp, _)) = ctx.sammy else { return (0.0, 0.0, f32::MAX) };
    if e.etat == 12 {
        return (0.0, 0.0, 1.0);
    }
    let mut d = to_angle([sp[0] - e.pos[0], sp[1] - e.pos[1], sp[2] + 15.0 - e.pos[2]]);
    e.dist = d.2;
    if ctx.calls.attack_call != 0.0 {
        // Answering a friend call: same species+subtype within RangeCall (exact float compare).
        if e.spec as f32 + e.weight == ctx.calls.friend_call && d.2 < ctx.calls.range_call && d.2 > 128.0 {
            if matches!(e.etat, 0 | 1 | 4 | 8 | 9) {
                if d.2 > e.attack_range {
                    ctx.calls.answer += 1;
                }
                e.etat_bis = 0;
                e.dest_pan = d.0;
                let r = e.run_speed();
                e.set_state(2, r);
            }
            d.2 = 64.0;
        }
        return d; // during a call nobody reverts to idle
    }
    if d.2 > e.react_range * 1.1 && (e.etat == 1 || e.etat == 2) {
        e.etat = if ctx.rng.random(100.0) > g.random_attack { 4 } else { 0 };
        e.anim_speed = 3.0;
        e.anim_step = 0.0;
    }
    d
}

/// `LionShootPlayer` (§2.6): line of sight from the lion origin to Sammy.
pub(super) fn sees_sammy(e: &mut Enemy, ctx: &Ctx<'_>) -> bool {
    let Some((s, sp, _)) = ctx.sammy else { return false };
    let hit = ctx.space.trace(Hull::Point, e.pos, [sp[0], sp[1], sp[2] + 15.0], Some(ctx.me));
    let seen = !hit.trace.hit() || hit.entity == Some(s);
    if hit.trace.hit() {
        e.avu = seen;
    }
    seen
}

pub(super) fn second_react_range(e: &mut Enemy) {
    let v = (frc(e.react_range) * 10000.0 + 0.5).trunc();
    if v != 0.0 {
        e.react_range = v;
    }
}

pub(super) fn take_attack_bis(e: &mut Enemy) {
    if e.attack_range_bis != 0.0 {
        e.attack_range = e.attack_range_bis;
        e.attack_range_bis = 0.0;
    }
}
