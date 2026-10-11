//! Effects around Sammy: dust, hit stars, boosts, teleports and respawns.

use super::*;

// ---------------------------------------------------------------- Sammy

/// `DustunderSammy` / `particle_Poussiere1` (§2.7): one puff growing to `target`
/// (`TaillePoussiere1`) at the ground under Sammy; `fwd` = his forward speed.
pub fn dust(ps: &mut Particles, ground: [f32; 3], target: f32, fwd: f32) {
    let pos = [ground[0] + ps.random(12.0) - 6.0, ground[1] + ps.random(12.0) - 6.0, ground[2]];
    ps.emit(1, pos, |p, rng| {
        p.vel = [rng.random(6.0) - 3.0, rng.random(6.0) - 3.0, rng.random(2.0) + 1.0 + fwd * 0.05];
        p.size = rng.random(100.0) + 400.0;
        mapped(p, "fumeec5.pcx");
        p.data[0] = target;
        p.behaviour = |p, _| {
            match age(p) {
                20 => p.map = Some("fumeec7.pcx"),
                30 => p.map = Some("fumeec9.pcx"),
                _ => {}
            }
            if p.size < p.data[0] - 10.0 {
                p.size += (p.data[0] - p.size) * 0.2;
            }
            p.vel[2] -= 0.08 * F;
            p.age < DUST_LIFE
        };
    });
}

pub(super) fn random_starhit(rng: &mut ParticleRng) -> &'static str {
    ["starhit1.pcx", "starhit2.pcx", "starhit3.pcx", "starhit4.pcx", "starhit5.pcx"][(rng.random(5.0) as usize).min(4)]
}

/// `particle_hit1` (§2.12): the lion's swipe connects (10) or contact (1).
pub fn hit_stars(ps: &mut Particles, pos: [f32; 3], n: usize) {
    ps.emit(n, pos, |p, rng| {
        p.vel = rnd_dir(rng, 4.0, 10.0);
        p.size = 1500.0 + rng.random(500.0);
        mapped(p, "starhit1.pcx");
        p.behaviour = |p, ctx| {
            if age(p) % 4 == 0 {
                p.map = Some(random_starhit(ctx.rng));
            }
            if p.size > 50.0 {
                p.size += (50.0 - p.size) * 0.1;
            }
            p.age < HIT_LIFE
        };
    });
}

/// `particle_SpeedySammy` (§2.14): boosting, around Sammy.
pub fn speedy(ps: &mut Particles, pos: [f32; 3]) {
    let at = [pos[0] + ps.random(32.0) - 16.0, pos[1] + ps.random(32.0) - 16.0, pos[2] + ps.random(100.0)];
    ps.emit(1, at, |p, rng| {
        p.vel = [rng.random(2.0) - 1.0, rng.random(2.0) - 1.0, rng.random(2.0) - 4.0];
        p.size = 2000.0 + rng.random(500.0);
        mapped(p, "starhit1.pcx");
        p.behaviour = |p, ctx| {
            if age(p) % 6 == 0 {
                p.map = Some(random_starhit(ctx.rng));
            }
            if p.size > 50.0 {
                p.size += (50.0 - p.size) * 0.2;
            }
            p.age < SPEEDY_LIFE
        };
    });
}

/// `particle_Boost1` (§2.13): boost pickup idle sparkle.
pub fn boost_sparkle(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(1, pos, |p, rng| {
        p.vel = [rng.random(2.0) - 1.0, rng.random(2.0) - 1.0, rng.random(4.0) + 2.0];
        p.size = rng.random(250.0) + 600.0;
        mapped(p, "tistar1a.pcx");
        p.behaviour = |p, _| {
            p.size -= 60.0;
            p.size > 100.0
        };
    });
}

/// `particle_Teleport` (idle pad) and `particle_Teleport2` (around Sammy) (§2.15).
pub fn teleport_sparkle(ps: &mut Particles, pos: [f32; 3], around_sammy: bool, n: usize) {
    ps.emit(n, pos, |p, rng| {
        if around_sammy {
            p.vel = [rng.random(32.0) - 16.0, rng.random(32.0) - 16.0, rng.random(5.0) + 3.0];
            p.size = rng.random(1500.0) + 1000.0;
            p.behaviour = |p, ctx| sparkle(p, ctx, 6, 0.85, 1.1, 16.0);
        } else {
            p.vel = [rng.random(64.0) - 32.0, rng.random(64.0) - 32.0, rng.random(10.0) + 3.0];
            p.size = rng.random(1500.0) + 1500.0;
            p.behaviour = |p, ctx| sparkle(p, ctx, 5, 0.9, 1.2, 20.0);
        }
        mapped(p, "starhit1.pcx");
    });
}

pub(super) fn sparkle(p: &mut Particle, ctx: &mut ParticleCtx, every: i32, damp: f32, rise: f32, life: f32) -> bool {
    if age(p) % every == 0 {
        p.map = Some(random_starhit(ctx.rng));
    }
    let k = damp.powf(F);
    p.vel[0] *= k;
    p.vel[1] *= k;
    p.vel[2] += rise;
    if p.size <= 100.0 {
        return false;
    }
    p.size += (100.0 - p.size) * 0.1;
    p.age < life
}

/// `particle_GemRestart` (§2.8): idle restart point, 100 below it.
pub fn restart_sparkle(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(1, [pos[0], pos[1], pos[2] - 100.0], |p, rng| {
        p.vel = [rng.random(40.0) - 20.0, rng.random(40.0) - 20.0, rng.random(10.0) + 3.0];
        p.size = rng.random(2000.0);
        mapped(p, if rng.random(10.0) > 5.0 { "greenpr1.pcx" } else { "greenpr2.pcx" });
        p.behaviour = |p, _| {
            let k = 0.9f32.powf(F);
            p.vel[0] *= k;
            p.vel[1] *= k;
            p.vel[2] += 1.0;
            if p.size > 100.0 {
                p.size -= 50.0;
            }
            p.age < RESTART_LIFE
        };
    });
}

/// `particle_TileDeath` (§2.9): smoke from death tiles and lava.
pub fn tile_smoke(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(1, pos, |p, rng| {
        p.vel = [rng.random(64.0) - 32.0, rng.random(64.0) - 32.0, rng.random(10.0) + 3.0];
        p.size = rng.random(1500.0);
        mapped(p, "fumeeb1.pcx");
        p.behaviour = |p, _| {
            let map = match age(p) {
                4 => Some("fumeeb3.pcx"),
                6 => Some("fumeeb5.pcx"),
                7 => Some("fumee3.pcx"),
                10 => Some("fumee5.pcx"),
                12 => Some("fumee6.pcx"),
                15 => Some("fumee7.pcx"),
                17 => Some("fumee8.pcx"),
                19 => Some("fumee9.pcx"),
                _ => None,
            };
            if map.is_some() {
                p.map = map;
            }
            let k = 0.9f32.powf(F);
            p.vel[0] *= k;
            p.vel[1] *= k;
            p.vel[2] += 1.0;
            if p.size < 2500.0 {
                p.size += 50.0;
            }
            p.age < SMOKE_LIFE
        };
    });
}
