//! The game's files ([`sammy_platform::GameFiles`]) as a Bevy resource.

use bevy::prelude::*;

#[derive(Resource, Deref)]
pub struct GameFiles(pub sammy_platform::GameFiles);

impl sammy_core::sim::Files for GameFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.0.read(name).ok()
    }
}
