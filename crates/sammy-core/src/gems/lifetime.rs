//! Runtime gems' lifetime, gravity and surprise colours.

use super::*;
use crate::time::Ticks;
use crate::keys::KeyColor;

/// Remaining lifetime (Skill9) of a runtime gem; counts down once it rests.
#[derive(Clone, Debug)]
pub struct GemLifetime {
    pub ticks: Ticks,
    /// Blink: shown small during the second half of each animation cycle.
    pub small: bool,
    /// Blink animation speed (frames per tick), rising each blink.
    pub speed: f32,
}

impl GemLifetime {
    pub fn new(ticks: f32) -> Self {
        Self { ticks: Ticks(ticks), small: false, speed: BLINK_START_SPEED }
    }

    /// Counts down; true when the gem bursts (`GemExplose`).
    pub fn tick(&mut self, t: Ticks) -> bool {
        self.ticks -= t;
        self.ticks <= Ticks::ZERO
    }
}

/// Flag7 gem: falls at `Gravite` per tick down to its rest height, rides platforms.
#[derive(Clone, Copy, Debug)]
pub struct GemGravity {
    pub rest: f32,
}

impl GemGravity {
    /// New height for a gem at `z` above `floor`.
    pub fn fall(&self, z: f32, floor: f32, jump: &JumpParams, t: Ticks) -> f32 {
        if z > floor + self.rest { (z + jump.gravity * t).max(floor + self.rest) } else { floor + self.rest }
    }
}

/// Surprise gem (Skill5): not collectable for 1-8 ticks, then its colour climbs.
#[derive(Clone, Debug)]
pub struct Surprise {
    pub add: f32,
    pub(super) delay: Ticks,
}

/// Highest colour a surprise gem climbs to (green).
pub(super) const SURPRISE_MAX_COLOR: u32 = 6;

impl Surprise {
    /// Armed with a 1-8 tick delay (derived from the gem until the reset scan re-rolls it).
    pub fn new(add: f32, seed: f32) -> Self {
        Self { add, delay: Ticks((seed.abs() as u32 % 8) as f32 + 1.0) }
    }

    /// Re-roll on respawn (the colour keeps climbing, original quirk).
    pub fn rearm(&mut self, rng: &mut Rng) {
        self.delay = Ticks(rng.random(8.0).trunc() + 1.0);
    }

    /// Counts down; returns the new colour when it becomes collectable.
    pub fn step(&mut self, color: KeyColor, rng: &mut Rng, t: Ticks) -> Option<KeyColor> {
        if self.delay <= Ticks::ZERO {
            return None;
        }
        self.delay -= t;
        (self.delay <= Ticks::ZERO).then(|| KeyColor((color.get() + rng.random(self.add) as u32 + 1).min(SURPRISE_MAX_COLOR)))
    }
}
