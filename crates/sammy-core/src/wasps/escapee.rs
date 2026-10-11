//! Wasps escaping when Sammy drops his ammo.

use super::*;

/// Escaping wasps on a type switch: at most this many, shown at half size, for this many
/// ticks, turning `ESCAPE_SPIN` degrees per tick (bullets.wdl:749-921).
pub const ESCAPE_MAX: i32 = 15;
pub const ESCAPE_SCALE: f32 = 0.5;
pub(super) const ESCAPE_TICKS: f32 = 80.0;
pub(super) const ESCAPE_SPIN: f32 = 20.0;
/// Pan `random(360)`, tilt `60 + random(40)`, speed `20 + random(10)`.
pub(super) const ESCAPE_TILT: (f32, f32) = (60.0, 40.0);
pub(super) const ESCAPE_SPEED: (f32, f32) = (20.0, 10.0);

/// A wasp of the old type escaping when Sammy takes another type (`WaspEscape`,
/// bullets.wdl:847-921): it spirals upward and vanishes. Purely cosmetic (passable).
#[derive(Clone, Debug)]
pub struct Escapee {
    pub kind: u32,
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
    pub(super) speed: f32,
    pub(super) timer: f32,
}

impl Escapee {
    pub fn new(kind: u32, pos: [f32; 3], rng: &mut Rng) -> Self {
        let pan = rng.random(360.0);
        let tilt = ESCAPE_TILT.0 + rng.random(ESCAPE_TILT.1);
        let speed = ESCAPE_SPEED.0 + rng.random(ESCAPE_SPEED.1);
        Self { kind, pos, pan, tilt, speed, timer: 0.0 }
    }

    /// One frame; true when it is gone.
    pub fn step(&mut self, t: f32) -> bool {
        let dir = forward(self.pan, self.tilt);
        for (p, d) in self.pos.iter_mut().zip(dir) {
            *p += d * self.speed * t;
        }
        self.pan += ESCAPE_SPIN * t;
        self.timer += t;
        self.timer >= ESCAPE_TICKS
    }
}
