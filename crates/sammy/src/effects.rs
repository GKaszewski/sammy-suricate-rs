//! The game's particle effects (`particul.wdl`, docs/particles.md §2) and their emitters
//! (§3). Speeds are quants/tick; per-frame constants of the original are scaled by
//! `F` frames per tick (§1.6); colour is ignored when a bitmap is set (§1.5).

use crate::particles::{Particle, ParticleCtx, ParticleRng, Particles};

/// Frames per tick the original was tuned at (≈ 40 fps, docs/particles.md §1.6).
const F: f32 = 2.5;

/// Every particle bitmap, preloaded with the level.
pub const PARTICLE_MAPS: &[&str] = &[
    "tistar0a.pcx", "tistar0b.pcx", "tistar1a.pcx", "tistar1b.pcx", "tistar2a.pcx", "tistar2b.pcx", "tistar3a.pcx", "tistar3b.pcx",
    "tistar4a.pcx", "tistar4b.pcx", "tistar5a.pcx", "tistar5b.pcx", "tistar6a.pcx", "tistar6b.pcx", "morgem1.pcx", "morgem2.pcx",
    "morgem3.pcx", "morgem4.pcx", "morgem5.pcx", "morgem6.pcx", "starhit1.pcx", "starhit2.pcx", "starhit3.pcx", "starhit4.pcx",
    "starhit5.pcx", "greenpr1.pcx", "greenpr2.pcx", "phearth1.pcx", "phearth2.pcx", "phearth3.pcx", "phearth4.pcx", "phearth5.pcx",
    "phearth6.pcx", "phearth7.pcx", "peye1.pcx", "peye2.pcx", "fumeec5.pcx", "fumeec7.pcx", "fumeec9.pcx", "fumeeb1.pcx",
    "fumeeb3.pcx", "fumeeb5.pcx", "fumee3.pcx", "fumee5.pcx", "fumee6.pcx", "fumee7.pcx", "fumee8.pcx", "fumee9.pcx",
    "waspefr0.pcx", "waspefr2.pcx", "waspefr4.pcx", "waspefr6.pcx", "waspefy0.pcx", "waspefy2.pcx", "waspefy4.pcx", "waspefy6.pcx",
    "waspefb0.pcx", "waspefb2.pcx", "waspefb4.pcx", "waspefb6.pcx", "waspefg0.pcx", "waspefg2.pcx", "waspefg4.pcx", "waspefg6.pcx",
    "waspefm0.pcx", "waspefm2.pcx", "waspefm4.pcx", "waspefm6.pcx", "waspefx0.pcx", "waspefx2.pcx", "waspefx4.pcx", "waspefx6.pcx",
    "waspefw0.pcx", "waspefw2.pcx", "waspefw4.pcx", "waspefw6.pcx",
];

/// `NORMALIZE v, len` of a random cube vector.
fn rnd_dir(rng: &mut ParticleRng, half: f32, len: f32) -> [f32; 3] {
    let v = [rng.random(half * 2.0) - half, rng.random(half * 2.0) - half, rng.random(half * 2.0) - half];
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
    v.map(|c| c / l * len)
}

fn mapped(p: &mut Particle, map: &'static str) {
    p.map = Some(map);
    p.flare = true;
    p.color = [255.0; 3];
}

fn age(p: &Particle) -> i32 {
    p.age as i32
}

// ---------------------------------------------------------------- gem colours

/// Gem colour index (Skill6): 1 red, 2 yellow, 3 orange, 4 blue, 5 magenta, 6 green.
fn star_maps(color: u32) -> [&'static str; 2] {
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

fn star_map(rng: &mut ParticleRng, color: u32) -> &'static str {
    star_maps(color)[(rng.random(10.0) > 5.0) as usize]
}

/// Map-less dot colour (§2.1); some variants re-roll every frame.
fn dot_color(rng: &mut ParticleRng, color: u32) -> [f32; 3] {
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
fn init_dot(p: &mut Particle, rng: &mut ParticleRng, color: u32) {
    p.vel = [rng.random(2.0) - 1.0, rng.random(2.0) - 1.0, rng.random(1.0) - 4.0];
    p.size = if color == 1 { 200.0 } else { 100.0 + rng.random(50.0) };
    p.color = dot_color(rng, color);
    p.data[0] = color as f32;
    p.behaviour = dot;
}

fn dot(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
    let c = p.data[0] as u32;
    if matches!(c, 2..=5) {
        p.color = dot_color(ctx.rng, c);
    }
    p.age < 10.0
}

/// `particle_<colour>Bmp` stars (§2.2): life 10.
fn init_star(p: &mut Particle, rng: &mut ParticleRng, color: u32) {
    p.vel = [rng.random(6.0) - 3.0, rng.random(6.0) - 3.0, rng.random(1.0) - 4.0];
    p.size = rng.random(300.0) + 750.0;
    mapped(p, star_map(rng, color));
    p.behaviour = |p, _| p.age < 10.0;
}

/// `Particle_GemKeys` (§3.2): a flying gem's trail, 3 stars of its colour (255 = one of each).
pub fn gem_trail(ps: &mut Particles, pos: [f32; 3], color: u32) {
    if color == crate::game::MASTER_KEY {
        for c in 1..=6 {
            ps.emit(1, pos, |p, rng| init_star(p, rng, c));
        }
    } else {
        ps.emit(3, pos, |p, rng| init_star(p, rng, color));
    }
}

/// `GemExplose` (§2.3, gems.wdl:364-391): ~15 falling shards dropping dots.
pub fn gem_explode(ps: &mut Particles, pos: [f32; 3], color: u32) {
    let color = if (1..=6).contains(&color) { color } else { 1 + ps.random(6.0) as u32 };
    let v4 = ps.random(5.0) + 5.0;
    ps.emit(15, pos, |p, rng| {
        p.vel = [v4 * (rng.random(6.0) - 3.0), v4 * (rng.random(6.0) - 3.0), v4 + rng.random(30.0)];
        p.size = rng.random(200.0) + 2000.0;
        mapped(p, ["morgem1.pcx", "morgem2.pcx", "morgem3.pcx", "morgem4.pcx", "morgem5.pcx", "morgem6.pcx"][color as usize - 1]);
        p.data[0] = color as f32;
        p.behaviour = shard;
    });
}

fn shard(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
    p.vel[2] -= 4.0;
    if age(p) % 2 == 0 {
        let mut d = Particle::new(p.pos, dot);
        init_dot(&mut d, ctx.rng, p.data[0] as u32);
        ctx.spawn.push(d);
    }
    p.age < 30.0
}

/// `GemCatch` (§2.4, gems.wdl:395-449): 25 rising stars, in three waves.
pub fn gem_catch(ps: &mut Particles, pos: [f32; 3], color: u32) {
    let color = if (1..=6).contains(&color) { color } else { 0 };
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
                p.age < 30.0
            };
        });
    }
}

/// Gem mix end burst (§2.5, gems.wdl:596-605): 70 stars of a primary colour.
pub fn gem_mix(ps: &mut Particles, pos: [f32; 3], color: u32) {
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
            p.age < 30.0
        };
    });
}

/// `particle_ExploseDiam` ×15 (§2.6): diamond picked up.
pub fn diamond_burst(ps: &mut Particles, pos: [f32; 3]) {
    ps.emit(15, pos, |p, rng| {
        p.vel = rnd_dir(rng, 2.5, 10.0);
        p.size = 400.0 + rng.random(100.0);
        mapped(p, if rng.random(10.0) > 5.0 { "tistar0a.pcx" } else { "tistar0b.pcx" });
        p.behaviour = |p, _| {
            if p.size > 20.0 {
                p.size -= 30.0;
            }
            p.age < 8.0
        };
    });
}

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
            p.age < 40.0
        };
    });
}

fn random_starhit(rng: &mut ParticleRng) -> &'static str {
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
            p.age < 16.0
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
            p.age < 24.0
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

fn sparkle(p: &mut Particle, ctx: &mut ParticleCtx, every: i32, damp: f32, rise: f32, life: f32) -> bool {
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
            p.age < 30.0
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
            p.age < 22.0
        };
    });
}

// ---------------------------------------------------------------- wasps

/// Wasp effect bitmap families (`WaspEf<letter>{0,2,4,6}`), by `IdWaspSpec` (§3.1):
/// 1 red+yellow, 2 red, 3 blue, 4 green, 5 magenta, 6 gradient, 7 white.
const WASP_LETTERS: [[&str; 4]; 7] = [
    ["waspefr0.pcx", "waspefr2.pcx", "waspefr4.pcx", "waspefr6.pcx"],
    ["waspefy0.pcx", "waspefy2.pcx", "waspefy4.pcx", "waspefy6.pcx"],
    ["waspefb0.pcx", "waspefb2.pcx", "waspefb4.pcx", "waspefb6.pcx"],
    ["waspefg0.pcx", "waspefg2.pcx", "waspefg4.pcx", "waspefg6.pcx"],
    ["waspefm0.pcx", "waspefm2.pcx", "waspefm4.pcx", "waspefm6.pcx"],
    ["waspefx0.pcx", "waspefx2.pcx", "waspefx4.pcx", "waspefx6.pcx"],
    ["waspefw0.pcx", "waspefw2.pcx", "waspefw4.pcx", "waspefw6.pcx"],
];
const RED: usize = 0;
const YELLOW: usize = 1;

/// Bitmap family of wasp type 2-7 (types 1 use red + yellow).
fn wasp_family(kind: u32) -> usize {
    match kind {
        2 => RED,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => 6,
        _ => YELLOW,
    }
}

fn wasp_disc(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
    // Frame swaps by age: 1 → 3 → 5 → 7 → 1 ... (§2.10).
    let frame = match age(p) {
        2 | 6 | 12 => Some(1),
        3 | 7 | 14 => Some(2),
        4 | 8 | 16 => Some(3),
        5 | 10 => Some(0),
        _ => None,
    };
    let _ = ctx;
    if let Some(f) = frame {
        p.map = Some(WASP_LETTERS[p.data[0] as usize][f]);
    }
    if p.size > 100.0 {
        p.size += (100.0 - p.size) * 0.2;
    }
    p.age < 18.0
}

fn init_disc(p: &mut Particle, family: usize, size: f32) {
    p.size = size;
    mapped(p, WASP_LETTERS[family][0]);
    p.data[0] = family as f32;
    p.behaviour = wasp_disc;
}

/// One tick of a flying wasp's trail (`Wasp`, bullets.wdl:410-446; top fps tier
/// (2,4,2) → red 1/2, yellow 1/4, other colours 1/2 per tick). `scale` 0.5 for escaping wasps.
pub fn wasp_trail(ps: &mut Particles, pos: [f32; 3], kind: u32, scale: f32) {
    if kind <= 1 {
        if ps.random(2.0) < 1.0 {
            ps.emit(1, pos, |p, _| init_disc(p, RED, 3000.0 * scale));
        }
        if ps.random(4.0) < 1.0 {
            ps.emit(1, pos, |p, _| init_disc(p, YELLOW, 3000.0 * scale));
        }
    } else if ps.random(2.0) < 1.0 {
        let f = wasp_family(kind);
        ps.emit(1, pos, |p, _| init_disc(p, f, 3000.0 * scale));
    }
}

/// Wasp death burst (`particle_WaspExplode*`, §2.11): 1 particle per tick for 4 ticks,
/// with a per-tick random spread vector `(rand(2s)-s)³ × rand(5)` (s = 2; 10 for hives).
pub fn wasp_explode(ps: &mut Particles, pos: [f32; 3], kind: u32, spread: f32) {
    let family = wasp_family(kind);
    for tick in 0..4 {
        let v = [ps.random(spread * 2.0) - spread, ps.random(spread * 2.0) - spread, ps.random(spread * 2.0) - spread];
        ps.emit_delayed(1, pos, tick as f32, |p, rng| {
            p.vel = v.map(|c| c * rng.random(5.0));
            init_disc(p, family, 1500.0);
        });
    }
}

/// Hive exhausted (bullets.wdl:1255-1272): 12 bursts 3 ticks apart of 6 colours.
pub fn hive_burst(ps: &mut Particles, pos: [f32; 3]) {
    for burst in 0..12 {
        for family in [YELLOW, RED, 2, 3, 4, 5] {
            let v = [ps.random(20.0) - 10.0, ps.random(20.0) - 10.0, ps.random(20.0) - 10.0];
            ps.emit_delayed(1, pos, burst as f32 * 3.0, |p, rng| {
                p.vel = v.map(|c| c * rng.random(5.0));
                init_disc(p, family, 1500.0);
            });
        }
    }
}

/// `Particle_WaspHive` (§3.2): trail of a hive wasp while it rises, by its type.
pub fn hive_wasp_trail(ps: &mut Particles, pos: [f32; 3], kind: u32) {
    let colors: &[(u32, usize)] = match kind {
        1 => &[(2, 1), (3, 2)],
        2 => &[(1, 3)],
        3 => &[(4, 3)],
        4 => &[(6, 3)],
        5 => &[(5, 3)],
        _ => &[(6, 1), (1, 1), (2, 1)],
    };
    for &(c, n) in colors {
        ps.emit(n, pos, |p, rng| init_star(p, rng, c));
    }
}

/// `Particle_Win` (§3.3): 1 star of each colour, every 3 frames (≈ 1/3 per tick per caller).
pub fn win_sparkle(ps: &mut Particles, pos: [f32; 3]) {
    if ps.random(3.0) < 1.0 {
        for c in 1..=6 {
            ps.emit(1, pos, |p, rng| init_star(p, rng, c));
        }
    }
}

// ---------------------------------------------------------------- pickups

fn heart_frames(p: &mut Particle) {
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
fn init_life(p: &mut Particle, rng: &mut ParticleRng) {
    p.vel = [rng.random(4.0) - 2.0, rng.random(4.0) - 2.0, -8.0];
    p.size = rng.random(500.0) + 500.0;
    mapped(p, "phearth1.pcx");
    p.behaviour = |p, _| {
        heart_frames(p);
        p.age < 10.0
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
            p.age < 20.0
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
            p.age < 12.0
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
            p.age < 16.0
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

/// Runs `f` on the particle list at the end of the frame's command flush (for systems
/// that only have `Commands`).
pub fn queue(commands: &mut bevy::prelude::Commands, f: impl FnOnce(&mut Particles) + Send + 'static) {
    commands.queue(move |w: &mut bevy::prelude::World| f(&mut w.resource_mut::<Particles>()));
}

// ---------------------------------------------------------------- per-tick emitters

use bevy::prelude::*;

use crate::actions::enemy::Enemy;
use crate::actions::gems::{DoorGem, FlyingGem};
use crate::actions::health::{BoostPickup, Checkpoint, DeathTile, LavaTile};
use crate::actions::pickups::{Collected, KeyGem, PickupPos};
use crate::actions::platform::{Teleport, Teleporting};
use crate::actions::player::{Player, State};
use crate::actions::wasp::{HiveWasp, Wasp};
use crate::level::LevelObject;

/// Idle emitters further than this from the camera target are skipped (the original
/// relied on its frame budget instead).
const IDLE_RANGE: f32 = 3000.0;

#[allow(clippy::type_complexity)]
pub fn emit_trails(
    mut ps: ResMut<Particles>,
    pov: Res<crate::actions::extro::Pov>,
    wasps: Query<&Wasp>,
    hive_wasps: Query<&HiveWasp>,
    enemies: Query<&Enemy>,
    gems: Query<(&Transform, &KeyGem), Or<(With<FlyingGem>,)>>,
    door_gems: Query<(&Transform, &DoorGem)>,
    (tiles, pads, checkpoints, boosts): (
        Query<(&Transform, &LevelObject, Has<LavaTile>), Or<(With<DeathTile>, With<LavaTile>)>>,
        Query<(&Transform, &LevelObject), With<Teleport>>,
        Query<&PickupPos, (With<Checkpoint>, Without<Collected>)>,
        Query<(&PickupPos, &Visibility), With<BoostPickup>>,
    ),
) {
    let near = |p: [f32; 3]| (0..3).map(|i| (p[i] - pov.pos[i]).powi(2)).sum::<f32>() < IDLE_RANGE * IDLE_RANGE;
    for _ in 0..ps.ticks {
        for w in &wasps {
            wasp_trail(&mut ps, w.pos, w.kind, 1.0);
        }
        for w in &hive_wasps {
            if let Some((kind, pos)) = w.trail() {
                hive_wasp_trail(&mut ps, pos, kind);
            }
        }
        for e in &enemies {
            if e.gone() {
                continue;
            }
            if e.class() == 99 {
                // "You Win" wasp: a burst from a random vertex (bullets.wdl:1810-1821).
                let at = [e.pos[0] + ps.random(120.0) - 60.0, e.pos[1] + ps.random(120.0) - 60.0, e.pos[2] + ps.random(80.0) - 40.0];
                win_sparkle(&mut ps, at);
            } else if (101..=107).contains(&e.wasp_spec) {
                wasp_trail(&mut ps, e.pos, e.wasp_spec - 100, 1.0);
            }
        }
        for (tf, gem) in &gems {
            gem_trail(&mut ps, crate::coords::native(tf.translation), gem.color);
        }
        for (tf, gem) in &door_gems {
            gem_trail(&mut ps, crate::coords::native(tf.translation), gem.color);
        }
        // Death tiles every 2 ticks, lava 1/9 of that (mm1.wdl:4033,4878); Flag3 = no smoke.
        for (tf, obj, lava) in &tiles {
            let at = crate::coords::native(tf.translation);
            if obj.0.flags & (1 << 2) != 0 || !near(at) {
                continue;
            }
            if ps.random(if lava { 18.0 } else { 2.0 }) < 1.0 {
                tile_smoke(&mut ps, at);
            }
        }
        // Idle teleport pads (Flag3 != 1), 1/4 per tick (mm1.wdl:4109).
        for (tf, obj) in &pads {
            let at = crate::coords::native(tf.translation);
            if obj.0.flags & (1 << 2) == 0 && near(at) && ps.random(4.0) < 1.0 {
                teleport_sparkle(&mut ps, at, false, 1);
            }
        }
        for pos in &checkpoints {
            if near(pos.0) && ps.random(4.0) < 1.0 {
                restart_sparkle(&mut ps, pos.0);
            }
        }
        // Boost arrows: 2 stars at mesh vertices 3 and 11, 1/6 per tick (mm1.wdl:4360-4362).
        for (pos, vis) in &boosts {
            if *vis != Visibility::Hidden && near(pos.0) && ps.random(6.0) < 1.0 {
                for side in [-1.0, 1.0] {
                    let at = [pos.0[0] + side * 12.0, pos.0[1] + ps.random(16.0) - 8.0, pos.0[2] + 8.0];
                    boost_sparkle(&mut ps, at);
                }
            }
        }
    }
}

/// Sammy's dust, boost stars and teleport sparkle (docs/particles.md §3.5).
pub fn emit_sammy(mut ps: ResMut<Particles>, player: Query<(&Player, Has<Teleporting>)>, mut prev: Local<Option<(State, bool)>>) {
    let Ok((p, teleporting)) = player.single() else { return };
    let ground = [p.pos[0], p.pos[1], p.floor];
    let fwd = (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt();
    let (was, was_tp) = prev.unwrap_or((p.state, teleporting));
    // Landing: one big puff on a hard landing, a smaller one landing on the move.
    if p.state == State::Land && was != State::Land {
        if p.hard_landing {
            dust(&mut ps, ground, 5000.0, fwd);
        } else if fwd > 1.0 && ps.random(2.0) < 1.0 {
            dust(&mut ps, ground, 3000.0, fwd);
        }
    }
    if teleporting && !was_tp {
        teleport_sparkle(&mut ps, [p.pos[0], p.pos[1], p.pos[2] + 20.0], true, 5);
    }
    *prev = Some((p.state, teleporting));
    for _ in 0..ps.ticks {
        // `ticks1 % TicksPoussiere` sampled per frame: walk 4 → ≈ f/4, turn 8 → ≈ f/8 per tick.
        let (chance, target) = match p.state {
            State::Walk => (F / 4.0, 3000.0),
            State::Turn => (F / 8.0, 2000.0),
            State::Land => (1.0, 3000.0),
            _ => (0.0, 0.0),
        };
        if chance > 0.0 && ps.random(1.0) < chance {
            dust(&mut ps, ground, target, fwd);
        }
        // `LessBoost`: every 2 frames while boosting.
        if p.boosting {
            let n = (F / 2.0) as usize + (ps.random(1.0) < (F / 2.0).fract()) as usize;
            for _ in 0..n {
                speedy(&mut ps, p.pos);
            }
        }
        if teleporting {
            teleport_sparkle(&mut ps, [p.pos[0], p.pos[1], p.pos[2] + 20.0], true, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every effect spawns something and dies out within its documented lifetime.
    #[test]
    fn every_effect_spawns_and_expires() {
        let o = [0.0; 3];
        let effects: Vec<(&str, Box<dyn Fn(&mut Particles)>)> = vec![
            ("gem_trail", Box::new(move |ps| gem_trail(ps, o, 3))),
            ("gem_trail_master", Box::new(move |ps| gem_trail(ps, o, crate::game::MASTER_KEY))),
            ("gem_explode", Box::new(move |ps| gem_explode(ps, o, 4))),
            ("gem_catch", Box::new(move |ps| gem_catch(ps, o, 1))),
            ("gem_mix", Box::new(move |ps| gem_mix(ps, o, 2))),
            ("diamond", Box::new(move |ps| diamond_burst(ps, o))),
            ("dust", Box::new(move |ps| dust(ps, o, 3000.0, 10.0))),
            ("hit", Box::new(move |ps| hit_stars(ps, o, 10))),
            ("speedy", Box::new(move |ps| speedy(ps, o))),
            ("boost", Box::new(move |ps| boost_sparkle(ps, o))),
            ("teleport", Box::new(move |ps| teleport_sparkle(ps, o, false, 3))),
            ("teleport2", Box::new(move |ps| teleport_sparkle(ps, o, true, 3))),
            ("restart", Box::new(move |ps| restart_sparkle(ps, o))),
            ("smoke", Box::new(move |ps| tile_smoke(ps, o))),
            ("wasp_explode", Box::new(move |ps| wasp_explode(ps, o, 3, 2.0))),
            ("hive_burst", Box::new(move |ps| hive_burst(ps, o))),
            ("hive_trail", Box::new(move |ps| hive_wasp_trail(ps, o, 1))),
            ("get_life", Box::new(move |ps| get_life(ps, o, 2.0))),
            ("small_heart", Box::new(move |ps| small_heart(ps, o))),
            ("take_eye", Box::new(move |ps| take_eye(ps, o))),
        ];
        for (name, f) in effects {
            let mut ps = Particles::default();
            f(&mut ps);
            assert!(!ps.list.is_empty(), "{name} spawned nothing");
            for _ in 0..80 {
                ps.step();
            }
            assert!(ps.list.is_empty(), "{name} still has {} particles after 80 ticks", ps.list.len());
        }
        // Probabilistic emitters fire over enough ticks.
        let mut ps = Particles::default();
        for _ in 0..64 {
            wasp_trail(&mut ps, o, 1, 1.0);
            eye_idle(&mut ps, o);
            win_sparkle(&mut ps, o);
        }
        assert!(ps.list.len() > 40);
    }
}
