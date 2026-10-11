//! The game's particle effects (`particul.wdl`, docs/particles.md §2) and their emitters
//! (§3). Speeds are quants/tick; per-frame constants of the original are scaled by
//! `F` frames per tick (§1.6); colour is ignored when a bitmap is set (§1.5).

use crate::particles::{Particle, ParticleCtx, ParticleRng, Particles};

mod game;
mod gems;
mod pickups;
mod sammy;
mod wasps;

pub use self::{game::*, gems::*, pickups::*, sammy::*, wasps::*};

/// Frames per tick the original was tuned at (≈ 40 fps, docs/particles.md §1.6).
const F: f32 = 2.5;

/// Particle colour channel maximum (RGB is 0..255).
const COLOR_MAX: f32 = 255.0;
/// Number of gem colours (Skill6 1..=6).
const GEM_COLORS: u32 = 6;

// Lifetimes in ticks (`p.age < LIFE`), per effect.
/// `particle_<colour>` dots (§2.1).
const DOT_LIFE: f32 = 10.0;
/// `particle_<colour>Bmp` stars (§2.2).
const STAR_LIFE: f32 = 10.0;
/// `GemExplose` shards (§2.3).
const SHARD_LIFE: f32 = 30.0;
/// `GemCatch` stars (§2.4).
const GEM_CATCH_LIFE: f32 = 30.0;
/// Gem mix end burst stars (§2.5).
const GEM_MIX_LIFE: f32 = 30.0;
/// Diamond burst (§2.6).
const DIAMOND_LIFE: f32 = 8.0;
/// `Poussiere1` dust puff (§2.7).
const DUST_LIFE: f32 = 40.0;
/// `particle_hit1` stars (§2.12).
const HIT_LIFE: f32 = 16.0;
/// `particle_SpeedySammy` (§2.14).
const SPEEDY_LIFE: f32 = 24.0;
/// `particle_GemRestart` (§2.8).
const RESTART_LIFE: f32 = 30.0;
/// `particle_TileDeath` smoke (§2.9).
const SMOKE_LIFE: f32 = 22.0;
/// Wasp effect discs (§2.10).
const WASP_DISC_LIFE: f32 = 18.0;
/// `particle_life` heart (§2.16).
const HEART_LIFE: f32 = 10.0;
/// `particle_Getlife` (§2.16).
const GET_LIFE_LIFE: f32 = 20.0;
/// `particle_SmallHeart` (§2.16).
const SMALL_HEART_LIFE: f32 = 12.0;
/// `particle_TakeEyeTruth` (§2.17).
const TAKE_EYE_LIFE: f32 = 16.0;

/// Start size of a wasp trail disc (scaled by the wasp), `Wasp` trail.
const WASP_TRAIL_SIZE: f32 = 3000.0;
/// Start size of a wasp death / hive burst disc.
const WASP_BURST_SIZE: f32 = 1500.0;
/// Dust puff target size after a hard landing.
const DUST_HARD_LANDING: f32 = 5000.0;
/// Dust puff target size when walking, landing on the move, or landing.
const DUST_WALK: f32 = 3000.0;
/// Dust puff target size when turning.
const DUST_TURN: f32 = 2000.0;
/// Editor Flag3 bit (index 2) of a level object: disables the tile smoke / idle teleport sparkle.
const FLAG3: u32 = 1 << 2;
/// Enemy class of the "You Win" wasp.
const WIN_WASP_CLASS: u32 = 99;
/// `wasp_spec` of an enemy wasp is `WASP_SPEC_BASE + wasp type` (types 1..=7).
const WASP_SPEC_BASE: u32 = 100;

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
    p.color = [COLOR_MAX; 3];
}

fn age(p: &Particle) -> i32 {
    p.age as i32
}

#[cfg(test)]
mod tests;
