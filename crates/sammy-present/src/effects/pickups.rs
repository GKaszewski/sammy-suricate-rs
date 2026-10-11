//! Pickup effects: hearts, the Eye of Truth and menu sparkles.

use super::*;

// ---------------------------------------------------------------- pickups

pub(super) fn heart_frames(p: &mut Particle) {
    const H: [&str; 7] = ["phearth1.pcx", "phearth2.pcx", "phearth3.pcx", "phearth4.pcx", "phearth5.pcx", "phearth6.pcx", "phearth7.pcx"];
    let f = match age(p) {
        2..=7 => Some(age(p) as usize - 1),
        8 => Some(3),
        9 => Some(0),
        _ => None,
    };
    if let Some(f) = f {
        p.map = Some(H[f]);
    }
}

/// `particle_life` (§2.16): one falling heart that cycles red → green.
pub(super) fn init_life(p: &mut Particle, rng: &mut ParticleRng) {
    p.vel = [rng.random(4.0) - 2.0, rng.random(4.0) - 2.0, -8.0];
    p.size = rng.random(500.0) + 500.0;
    mapped(p, "phearth1.pcx");
    p.behaviour = |p, _| {
        heart_frames(p);
        p.age < HEART_LIFE
    };
}

/// `particle_Getlife` (§2.16): the life bonus; each one sheds a heart every tick.
pub fn get_life(ps: &mut Particles, pos: [f32; 3], delay: f32) {
    ps.emit_delayed(1, pos, delay, |p, rng| {
        p.vel = [rng.random(30.0) - 15.0, rng.random(30.0) - 15.0, rng.random(15.0) - 5.0];
        p.size = rng.random(1000.0) + 500.0;
        mapped(p, "phearth1.pcx");
        p.behaviour = |p, ctx| {
            if p.size > 50.0 {
                p.size -= 5.0;
            }
            let mut h = Particle::new(p.pos, |_, _| true);
            init_life(&mut h, ctx.rng);
            ctx.spawn.push(h);
            p.age < GET_LIFE_LIFE
        };
    });
}

/// `particle_SmallHeart` ×30 (§2.16): small heart picked up.
pub fn small_heart(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(30, pos, |p, rng| {
        p.vel = rnd_dir(rng, 4.0, 10.0);
        p.size = 400.0 + rng.random(400.0);
        mapped(p, "phearth1.pcx");
        p.behaviour = |p, _| {
            if p.size > 50.0 {
                p.size -= 30.0;
            }
            p.age < SMALL_HEART_LIFE
        };
    });
}

/// `particle_TakeEyeTruth` ×30 (§2.17): Eye of Truth picked up.
pub fn take_eye(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(30, pos, |p, rng| {
        p.vel = rnd_dir(rng, 4.0, 10.0);
        p.size = 400.0 + rng.random(600.0);
        mapped(p, "peye1.pcx");
        p.behaviour = |p, ctx| {
            p.map = Some(if ctx.rng.random(2.0) < 1.0 { "peye1.pcx" } else { "peye2.pcx" });
            if p.size > 50.0 {
                p.size -= 20.0;
            }
            p.age < TAKE_EYE_LIFE
        };
    });
}

/// `particle_interface1` (particul.wdl:1173-1206): menu icon sparkle.
pub fn menu_sparkle(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(1, pos, |p, rng| {
        p.vel = [rng.random(4.0) - 2.0, rng.random(4.0) - 2.0, rng.random(4.0) - 2.0];
        p.size = 200.0 + rng.random(50.0);
        mapped(p, ["tistar0a.pcx", "tistar2a.pcx", "tistar4a.pcx"][(rng.random(3.0) as usize).min(2)]);
        p.behaviour = |p, _| {
            p.size -= 10.0;
            p.vel[2] += 0.1;
            p.size > 1.0
        };
    });
}

/// `InitEyeOfTruth` idle (mm1.wdl:5002-5006): p = 1/8 per tick, `RangeLion1/2`.
pub fn eye_idle(ps: &mut Particles, pos: [f32; 3]) {
    if ps.random(8.0) >= 1.0 {
        return;
    }
    let at = [pos[0] + ps.random(50.0) - 25.0, pos[1] + ps.random(50.0) - 25.0, pos[2] + ps.random(50.0) - 25.0];
    let first = ps.random(10.0) > 3.0;
    ps.emit(1, at, |p, _| {
        if first {
            p.size = 500.0;
            mapped(p, "peye1.pcx");
            p.behaviour = |p, _| {
                p.size -= 20.0 * F;
                p.age < 8.0
            };
        } else {
            p.size = 700.0;
            mapped(p, "peye2.pcx");
            p.behaviour = |p, _| {
                p.size -= 30.0 * F;
                p.age < 12.0
            };
        }
    });
}
