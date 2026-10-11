//! What becomes of a launched gem once it lands.

/// Skill1 of launched gems (and wasps): what becomes of them once they land.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fate {
    /// 0: decorative, shrinks and bursts, never collectable.
    Decorative,
    /// 1..999: collectable for that many ticks.
    Lifetime(f32),
    /// 1000: stays.
    Stays,
    /// 1001: stays and comes back after a respawn (like a level-placed key).
    Respawns,
}

impl Fate {
    pub fn from_skill(v: f32) -> Self {
        match v {
            v if v <= 0.0 => Self::Decorative,
            1000.0 => Self::Stays,
            1001.0 => Self::Respawns,
            v if v < 1000.0 => Self::Lifetime(v),
            _ => Self::Stays,
        }
    }
}
