//! Wasp effects: trails, bursts and hives.

use super::*;

// ---------------------------------------------------------------- wasps

/// Wasp effect bitmap families (`WaspEf<letter>{0,2,4,6}`), by `IdWaspSpec` (§3.1):
/// 1 red+yellow, 2 red, 3 blue, 4 green, 5 magenta, 6 gradient, 7 white.
pub(super) const WASP_LETTERS: [[&str; 4]; 7] = [
    ["waspefr0.pcx", "waspefr2.pcx", "waspefr4.pcx", "waspefr6.pcx"],
    ["waspefy0.pcx", "waspefy2.pcx", "waspefy4.pcx", "waspefy6.pcx"],
    ["waspefb0.pcx", "waspefb2.pcx", "waspefb4.pcx", "waspefb6.pcx"],
    ["waspefg0.pcx", "waspefg2.pcx", "waspefg4.pcx", "waspefg6.pcx"],
    ["waspefm0.pcx", "waspefm2.pcx", "waspefm4.pcx", "waspefm6.pcx"],
    ["waspefx0.pcx", "waspefx2.pcx", "waspefx4.pcx", "waspefx6.pcx"],
    ["waspefw0.pcx", "waspefw2.pcx", "waspefw4.pcx", "waspefw6.pcx"],
];
pub(super) const RED: usize = 0;
pub(super) const YELLOW: usize = 1;
pub(super) const BLUE: usize = 2;
pub(super) const GREEN: usize = 3;
pub(super) const MAGENTA: usize = 4;
pub(super) const GRADIENT: usize = 5;
pub(super) const WHITE: usize = 6;

/// Bitmap family of wasp type 2-7 (types 1 use red + yellow).
pub(super) fn wasp_family(kind: u32) -> usize {
    match kind {
        2 => RED,
        3 => BLUE,
        4 => GREEN,
        5 => MAGENTA,
        6 => GRADIENT,
        7 => WHITE,
        _ => YELLOW,
    }
}

pub(super) fn wasp_disc(p: &mut Particle, ctx: &mut ParticleCtx) -> bool {
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
    p.age < WASP_DISC_LIFE
}

pub(super) fn init_disc(p: &mut Particle, family: usize, size: f32) {
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
            ps.emit(1, pos, |p, _| init_disc(p, RED, WASP_TRAIL_SIZE * scale));
        }
        if ps.random(4.0) < 1.0 {
            ps.emit(1, pos, |p, _| init_disc(p, YELLOW, WASP_TRAIL_SIZE * scale));
        }
    } else if ps.random(2.0) < 1.0 {
        let f = wasp_family(kind);
        ps.emit(1, pos, |p, _| init_disc(p, f, WASP_TRAIL_SIZE * scale));
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
            init_disc(p, family, WASP_BURST_SIZE);
        });
    }
}

/// Hive exhausted (bullets.wdl:1255-1272): 12 bursts 3 ticks apart of 6 colours.
pub fn hive_burst(ps: &mut Particles, pos: [f32; 3]) {
    for burst in 0..12 {
        for family in [YELLOW, RED, BLUE, GREEN, MAGENTA, GRADIENT] {
            let v = [ps.random(20.0) - 10.0, ps.random(20.0) - 10.0, ps.random(20.0) - 10.0];
            ps.emit_delayed(1, pos, burst as f32 * 3.0, |p, rng| {
                p.vel = v.map(|c| c * rng.random(5.0));
                init_disc(p, family, WASP_BURST_SIZE);
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
