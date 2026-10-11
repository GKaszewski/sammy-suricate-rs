//! What the simulation asks the frontend to present: sounds, particle effects, popups,
//! saves. Drained once per frame with [`super::Sim::drain_events`].

/// A particle effect at a native position (docs/particles.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// `GemExplose`: a gem bursts (key colour; 0 = plain).
    GemExplode { at: [f32; 3], color: u32 },
    /// `Bonus3_Hit`: diamond pickup.
    DiamondBurst { at: [f32; 3] },
    /// `Key_Hit`: a key gem caught.
    GemCatch { at: [f32; 3], color: u32 },
    /// `AnimGemMix1`: two primaries mixed (stars of both colours at Sammy).
    GemMix { at: [f32; 3], colors: [u32; 2] },
    /// A small heart taken.
    SmallHeart { at: [f32; 3] },
    /// `particle_Getlife` after `delay` ticks.
    GetLife { at: [f32; 3], delay: f32 },
    /// A wasp bursts (`wasp_explode`).
    WaspExplode { at: [f32; 3], kind: u32, spread: f32 },
    /// An exhausted hive bursts.
    HiveBurst { at: [f32; 3] },
    /// Swipe hit stars.
    HitStars { at: [f32; 3], count: usize },
    /// The Eye of Truth taken.
    TakeEye { at: [f32; 3] },
}

/// One frame's output.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Sound(&'static str),
    Effect(Effect),
    /// Diamond score popup (`+50` / `+100`) at a pickup.
    Popup { at: [f32; 3], points: i32 },
    /// A lion called its friends: three expanding rings at this position.
    CallRings { at: [f32; 3] },
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
