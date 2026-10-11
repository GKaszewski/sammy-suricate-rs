#[allow(unused_imports)]
use super::*;

/// Sammy ([`sammy_core::player::Player`]).
#[derive(Component, Deref, DerefMut)]
pub struct Player(pub sammy_core::player::Player);

impl Player {
    /// The entity he stands on, if any (None = level geometry).
    pub fn ground(&self) -> Option<Entity> {
        self.ground_entity.map(entity)
    }
}
