//! Gem effects: trails, bursts, catches and mixes.

use super::*;
use sammy_core::space::Pos;
use sammy_core::keys::KeyColor;

// ---------------------------------------------------------------- gem colours

/// Gem colour index (Skill6): 1 red, 2 yellow, 3 orange, 4 blue, 5 magenta, 6 green.
pub(super) fn star_maps(color: u32) -> [&'static str; 2] {
    match color {
        1 => ["tistar1a.pcx", "tistar1b.pcx"],
        2 => ["tistar2a.pcx", "tistar2b.pcx"],
        3 => ["tistar3a.pcx", "tistar3b.pcx"],
        4 => ["tistar4a.pcx", "tistar4b.pcx"],
        5 => ["tistar5a.pcx", "tistar5b.pcx"],
        6 => ["tistar6a.pcx", "tistar6b.pcx"],
        _ => ["tistar0a.pcx", "tistar0b.pcx"],
    }
}

pub(super) fn star_map(rng: &mut ParticleRng, color: u32) -> &'static str {
    star_maps(color)[(rng.random(10.0) > 5.0) as usize]
}

/// Map-less dot colour (§2.1); some variants re-roll every frame.
pub(super) fn dot_color(rng: &mut ParticleRng, color: u32) -> [f32; 3] {
    match color {
        1 => [250.0, 150.0, 50.0],
        2 => [200.0 + rng.random(50.0), 230.0 + rng.random(20.0), rng.random(10.0)],
        3 => [200.0 + rng.random(50.0), 130.0 + rng.random(20.0), rng.random(10.0)],
        4 => [rng.random(50.0), 100.0 + rng.random(80.0), 230.0 + rng.random(20.0)],
        5 => [230.0 + rng.random(20.0), rng.random(20.0), 230.0 + rng.random(20.0)],
        _ => [rng.random(10.0), 230.0 + rng.random(20.0), rng.random(10.0)],
    }
}

/// `particle_<colour>` dots (§2.1): life 10, falling.
pub(super) fn init_dot(p: &mut Particle, rng: &mut ParticleRng, color: u32) {
    p.vel = [rng.random(2.0) - 1.0, rng.random(2.0) - 1.0, rng.random(1.0) - 4.0];
    p.size = if color == 1 { 200.0 } else { 100.0 + rng.random(50.0) };
    p.color = dot_color(rng, color);
    p.data[0] = color as f32;
    p.behaviour = dot;
}

pub(super) fn dot(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
    let c = p.data[0] as u32;
    if matches!(c, 2..=5) {
        p.color = dot_color(ctx.rng, c);
    }
    p.age < DOT_LIFE
}

/// `particle_<colour>Bmp` stars (§2.2): life 10.
pub(super) fn init_star(p: &mut Particle, rng: &mut ParticleRng, color: u32) {
    p.vel = [rng.random(6.0) - 3.0, rng.random(6.0) - 3.0, rng.random(1.0) - 4.0];
    p.size = rng.random(300.0) + 750.0;
    mapped(p, star_map(rng, color));
    p.behaviour = |p, _| p.age < STAR_LIFE;
}

/// `Particle_GemKeys` (§3.2): a flying gem's trail, 3 stars of its colour (255 = one of each).
pub fn gem_trail(ps: &mut Particles, pos: Pos, color: KeyColor) {
    let color = color.get();
    let pos = pos.get();
    if color == sammy_core::keys::MASTER_KEY.get() {
        for c in 1..=GEM_COLORS {
            ps.emit(1, pos, |p, rng| init_star(p, rng, c));
        }
    } else {
        ps.emit(3, pos, |p, rng| init_star(p, rng, color));
    }
}

/// `GemExplose` (§2.3, gems.wdl:364-391): ~15 falling shards dropping dots.
pub fn gem_explode(ps: &mut Particles, pos: Pos, color: KeyColor) {
    let color = color.get();
    let pos = pos.get();
    let color = if (1..=GEM_COLORS).contains(&color) { color } else { 1 + ps.random(GEM_COLORS as f32) as u32 };
    let v4 = ps.random(5.0) + 5.0;
    ps.emit(15, pos, |p, rng| {
        p.vel = [v4 * (rng.random(6.0) - 3.0), v4 * (rng.random(6.0) - 3.0), v4 + rng.random(30.0)];
        p.size = rng.random(200.0) + 2000.0;
        mapped(p, ["morgem1.pcx", "morgem2.pcx", "morgem3.pcx", "morgem4.pcx", "morgem5.pcx", "morgem6.pcx"][color as usize - 1]);
        p.data[0] = color as f32;
        p.behaviour = shard;
    });
}

pub(super) fn shard(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
    p.vel[2] -= 4.0;
    if age(p) % 2 == 0 {
        let mut d = Particle::new(p.pos, dot);
        init_dot(&mut d, ctx.rng, p.data[0] as u32);
        ctx.spawn.push(d);
    }
    p.age < SHARD_LIFE
}

/// `GemCatch` (§2.4, gems.wdl:395-449): 25 rising stars, in three waves.
pub fn gem_catch(ps: &mut Particles, pos: Pos, color: KeyColor) {
    let color = color.get();
    let pos = pos.get();
    let color = if (1..=GEM_COLORS).contains(&color) { color } else { 0 };
    let v4 = ps.random(2.0) + 1.0;
    for (n, delay) in [(5, 0.0), (10, 2.0), (10, 6.0)] {
        ps.emit_delayed(n, pos, delay, |p, rng| {
            p.vel = [v4 * (rng.random(2.0) - 1.0), v4 * (rng.random(2.0) - 1.0), v4 + rng.random(20.0)];
            p.size = rng.random(2000.0);
            mapped(p, star_map(rng, color));
            p.behaviour = |p, _| {
                p.vel[2] -= 1.0;
                if p.size > 100.0 {
                    p.size -= 50.0;
                }
                p.age < GEM_CATCH_LIFE
            };
        });
    }
}

/// Gem mix end burst (§2.5, gems.wdl:596-605): 70 stars of a primary colour.
pub fn gem_mix(ps: &mut Particles, pos: Pos, color: KeyColor) {
    let color = color.get();
    let pos = pos.get();
    let speed = ps.random(10.0) + 10.0;
    ps.emit(70, pos, |p, rng| {
        p.vel = rnd_dir(rng, 2.5, speed);
        p.size = 1000.0 + rng.random(if color == 1 { 100.0 } else { 50.0 });
        mapped(p, star_map(rng, color));
        p.behaviour = |p, _| {
            if p.size > 10.0 {
                p.size -= 2.0 * F;
            }
            p.vel[2] -= 1.5;
            p.age < GEM_MIX_LIFE
        };
    });
}

/// `particle_ExploseDiam` ×15 (§2.6): diamond picked up.
pub fn diamond_burst(ps: &mut Particles, pos: Pos) {
    let pos = pos.get();
    ps.emit(15, pos, |p, rng| {
        p.vel = rnd_dir(rng, 2.5, 10.0);
        p.size = 400.0 + rng.random(100.0);
        mapped(p, if rng.random(10.0) > 5.0 { "tistar0a.pcx" } else { "tistar0b.pcx" });
        p.behaviour = |p, _| {
            if p.size > 20.0 {
                p.size -= 30.0;
            }
            p.age < DIAMOND_LIFE
        };
    });
}
