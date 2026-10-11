//! Debug overrides a frontend may set.

/// Debug overrides a frontend may set (from the environment, a console ...).
#[derive(Clone, Debug, Default)]
pub struct DebugOptions {
    /// Sammy's spawn point and pan instead of the level's.
    pub start: Option<([f32; 3], Option<f32>)>,
    /// Starting key colour (255 = master).
    pub key: Option<u32>,
    pub lives: Option<i32>,
    pub hearts: Option<f32>,
    /// Eye of Truth for the whole level.
    pub eye: bool,
    /// Ammo (count, type 1..7).
    pub ammo: Option<(i32, u32)>,
    /// Hold forward.
    pub autowalk: bool,
    /// Fire whenever possible.
    pub autofire: bool,
    /// Level objects with these actions are not spawned (lower case).
    pub skip_actions: Vec<String>,
    /// No boot intro.
    pub no_intro: bool,
    /// Give up the level after this many seconds of play (counts down).
    pub give_up_after: Option<f32>,
    /// A portrait camera on Sammy: (distance, height above his origin, angle from his
    /// facing in degrees), looking at that height.
    pub portrait: Option<[f32; 3]>,
}
