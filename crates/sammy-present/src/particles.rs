//! A4 particles (`EMIT n, pos, function`, docs/particles.md): a CPU list of point
//! sprites, moved by their speed every tick and updated by their particle function.
//! Frontends draw the live ones as camera-facing quads ([`Particles::quads`]), one
//! batch per bitmap and blend mode.

use std::collections::BTreeMap;

/// Particle function, called once per tick after the move; returns false to remove it.
/// Particles it emits (nested `EMIT`) go to `spawn`.
pub type Behaviour = fn(&mut Particle, &mut ParticleCtx) -> bool;

pub struct ParticleCtx<'a> {
    pub rng: &'a mut ParticleRng,
    pub spawn: &'a mut Vec<Particle>,
}

/// A4 `MY_SIZE` to quad width in quants (docs/particles.md §1.5: size/100, uncalibrated).
pub const SIZE_SCALE: f32 = 0.01;

/// Default `MY_SIZE` of a new particle.
const DEFAULT_SIZE: f32 = 100.0;
/// Particle colour channel maximum (RGB is 0..255).
const COLOR_MAX: f32 = 255.0;
/// Particle alpha maximum (alpha is 0..100, percent).
const ALPHA_MAX: f32 = 100.0;

#[derive(Clone, Debug)]
pub struct Particle {
    /// Native position (quants).
    pub pos: [f32; 3],
    /// Quants per tick.
    pub vel: [f32; 3],
    /// A4 `MY_SIZE` (quad width = size × SIZE_SCALE quants).
    pub size: f32,
    /// RGB 0..255.
    pub color: [f32; 3],
    /// 0..100.
    pub alpha: f32,
    /// Bitmap (`my_map`); None = plain round dot in `color`.
    pub map: Option<&'static str>,
    /// `my_flare`: additive blending.
    pub flare: bool,
    /// Ticks since emission (`my_age`).
    pub age: f32,
    pub behaviour: Behaviour,
    /// Free per-particle state for behaviours.
    pub data: [f32; 4],
    /// Ticks before it appears (delayed bursts, `waitt n; EMIT`).
    pub delay: f32,
}

impl Particle {
    pub fn new(pos: [f32; 3], behaviour: Behaviour) -> Self {
        Self { pos, vel: [0.0; 3], size: DEFAULT_SIZE, color: [COLOR_MAX; 3], alpha: ALPHA_MAX, map: None, flare: false, age: 0.0, behaviour, data: [0.0; 4], delay: 0.0 }
    }
}

/// Small xorshift for particle randomness (keeps the game `Rng` sequence untouched).
pub struct ParticleRng(u32);

/// Initial xorshift state of the particle RNG.
const RNG_SEED: u32 = 0x1234_5678;

impl ParticleRng {
    /// WDL `random(n)`: uniform in [0, n).
    pub fn random(&mut self, n: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32 * n
    }
}

pub struct Particles {
    pub list: Vec<Particle>,
    pub rng: ParticleRng,
    /// Fractional tick accumulator.
    acc: f32,
    /// Whole 16 Hz ticks elapsing this frame (emitters run their per-tick code this often).
    pub ticks: u32,
}

impl Default for Particles {
    fn default() -> Self {
        Self { list: Vec::new(), rng: ParticleRng(RNG_SEED), acc: 0.0, ticks: 0 }
    }
}

/// Hard cap so a runaway emitter can't stall the frame.
const MAX_PARTICLES: usize = 4000;
/// Most ticks run in one frame, so a hitch can't snowball.
const MAX_TICKS_PER_FRAME: u32 = 4;

impl Particles {
    /// One 16 Hz tick: move, age and run every particle function.
    pub fn step(&mut self) {
        let mut spawn = Vec::new();
        let rng = &mut self.rng;
        self.list.retain_mut(|p| {
            if p.delay > 0.0 {
                p.delay -= 1.0;
                return true;
            }
            for i in 0..3 {
                p.pos[i] += p.vel[i];
            }
            p.age += 1.0;
            let mut ctx = ParticleCtx { rng, spawn: &mut spawn };
            (p.behaviour)(p, &mut ctx) && p.size > 0.0 && p.alpha > 0.0
        });
        let room = MAX_PARTICLES.saturating_sub(self.list.len());
        self.list.extend(spawn.into_iter().take(room));
    }

    /// `EMIT count, pos, init`: `init` sets up each new particle (the creation call of the
    /// particle function).
    pub fn emit(&mut self, count: usize, pos: [f32; 3], init: impl Fn(&mut Particle, &mut ParticleRng)) {
        self.emit_delayed(count, pos, 0.0, init);
    }

    /// `EMIT` after `delay` ticks.
    pub fn emit_delayed(&mut self, count: usize, pos: [f32; 3], delay: f32, init: impl Fn(&mut Particle, &mut ParticleRng)) {
        for _ in 0..count {
            if self.list.len() >= MAX_PARTICLES {
                return;
            }
            let mut p = Particle::new(pos, |_, _| true);
            init(&mut p, &mut self.rng);
            p.delay = delay;
            self.list.push(p);
        }
    }

    pub fn random(&mut self, n: f32) -> f32 {
        self.rng.random(n)
    }

    /// Advances the clock by `secs` of game time: the whole 16 Hz ticks elapsing go to
    /// [`Particles::ticks`] (emitters run their per-tick code this often).
    pub fn advance(&mut self, secs: f32) {
        self.acc += secs * sammy_core::time::TICKS_PER_SEC;
        self.ticks = (self.acc.floor() as u32).min(MAX_TICKS_PER_FRAME);
        self.acc = (self.acc - self.ticks as f32).min(1.0);
    }

    /// Runs this frame's ticks (after the emitters).
    pub fn update(&mut self) {
        for _ in 0..self.ticks {
            self.step();
        }
    }

    /// The live particles as camera-facing quads (native coordinates; `right` and `up`
    /// are the camera's unit axes), one batch per (bitmap, additive). Additive batches
    /// fade by darkening, blended ones by alpha.
    pub fn quads(&self, right: [f32; 3], up: [f32; 3]) -> BTreeMap<BatchKey, Vec<Vertex>> {
        let mut out: BTreeMap<BatchKey, Vec<Vertex>> = BTreeMap::new();
        for p in self.list.iter().filter(|p| p.delay <= 0.0) {
            let h = p.size * SIZE_SCALE * 0.5;
            let a = (p.alpha / ALPHA_MAX).clamp(0.0, 1.0);
            let k = if p.flare { a } else { 1.0 };
            let color = [p.color[0] / COLOR_MAX * k, p.color[1] / COLOR_MAX * k, p.color[2] / COLOR_MAX * k, a];
            let batch = out.entry((p.map, p.flare)).or_default();
            for (sr, su, uv) in [(-1.0, -1.0, [0.0, 1.0]), (1.0, -1.0, [1.0, 1.0]), (1.0, 1.0, [1.0, 0.0]), (-1.0, 1.0, [0.0, 0.0])] {
                let pos = [0, 1, 2].map(|i| p.pos[i] + (right[i] * sr + up[i] * su) * h);
                batch.push(Vertex { pos, uv, color });
            }
        }
        out
    }
}

/// A particle batch: its bitmap (None = the round [`dot_image`]) and whether it is
/// additive (`my_flare`).
pub type BatchKey = (Option<&'static str>, bool);

/// A quad corner; every 4 make a quad (corners 0-1-2, 0-2-3).
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    /// RGBA 0..1.
    pub color: [f32; 4],
}

/// Soft round dot for bitmap-less particles.
pub fn dot_image() -> formats::image::RgbaImage {
    /// Dot texture side in pixels.
    const N: u32 = 16;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 + 0.5 - N as f32 / 2.0).hypot(y as f32 + 0.5 - N as f32 / 2.0) / (N as f32 / 2.0)).min(1.0);
            let a = ((1.0 - d) * 2.0).min(1.0);
            data.extend([255, 255, 255, (a * 255.0) as u8]);
        }
    }
    formats::image::RgbaImage { width: N, height: N, data }
}
