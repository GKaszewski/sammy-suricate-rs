#[allow(unused_imports)]
use super::*;

/// The level being played ([`sammy_core::flow::LevelFlow`]) and a counter of loads.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct CurrentLevel {
    #[deref]
    pub flow: sammy_core::flow::LevelFlow,
    /// Bumped by every level load (reloads of the same file included).
    pub loads: u32,
}

/// Request to (re)load a level file on the next frame.
#[derive(Resource, Default)]
pub struct PendingLoad(pub Option<String>);
