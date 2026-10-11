//! WDL `random()`.

/// WDL `random(x)`: uniform in [0, x), from a small xorshift so a run is reproducible
/// from its seed. The frontend chooses the seed.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn random(&mut self, x: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32 * x
    }
}
