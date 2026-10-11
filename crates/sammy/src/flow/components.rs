#[allow(unused_imports)]
use super::*;

/// Root of everything spawned from the level file.
#[derive(Component)]
pub struct LevelRoot;

/// Runtime entities that belong to the current level (e.g. ejected keys).
#[derive(Component)]
pub struct LevelScoped;
