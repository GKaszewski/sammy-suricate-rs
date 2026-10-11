//! The player's storage ([`sammy_platform::Storage`]) as a Bevy resource.

use bevy::prelude::*;

pub use sammy_platform::Settings;

#[derive(Resource, Clone, Debug, Deref)]
pub struct Storage(pub sammy_platform::Storage);

impl Storage {
    pub fn user() -> Self {
        Self(sammy_platform::Storage::user())
    }
}
