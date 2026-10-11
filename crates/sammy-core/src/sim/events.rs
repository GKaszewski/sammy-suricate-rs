//! What the simulation asks the frontend to present: sounds, particle effects, popups,
//! saves. Drained once per frame with [`super::Sim::drain_events`].

use crate::space::Pos;
use crate::keys::KeyColor;
use crate::points::Points;

/// A particle effect at a native position (docs/particles.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// `GemExplose`: a gem bursts (key colour; 0 = plain).
    GemExplode { at: Pos, color: KeyColor },
    /// `Bonus3_Hit`: diamond pickup.
    DiamondBurst { at: Pos },
    /// `Key_Hit`: a key gem caught.
    GemCatch { at: Pos, color: KeyColor },
    /// `AnimGemMix1`: two primaries mixed (stars of both colours at Sammy).
    GemMix { at: Pos, colors: [KeyColor; 2] },
    /// A small heart taken.
    SmallHeart { at: Pos },
    /// `particle_Getlife` after `delay` ticks.
    GetLife { at: Pos, delay: f32 },
    /// A wasp bursts (`wasp_explode`).
    WaspExplode { at: Pos, kind: u32, spread: f32 },
    /// An exhausted hive bursts.
    HiveBurst { at: Pos },
    /// Swipe hit stars.
    HitStars { at: Pos, count: usize },
    /// The Eye of Truth taken.
    TakeEye { at: Pos },
}

/// One frame's output.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Sound(&'static str),
    Effect(Effect),
    /// Diamond score popup (`+50` / `+100`) at a pickup.
    Popup { at: Pos, points: Points },
    /// A lion called its friends: three expanding rings at this position.
    CallRings { at: Pos },
    /// The save changed (write it out).
    Saved,
    /// A level file was loaded (its entities replace the previous level's).
    Loaded { file: String },
    /// The level file could not be loaded.
    LoadFailed { file: String },
    /// The menu asked for the options screen.
    OpenOptions,
    /// The menu asked to quit.
    Quit,
}
