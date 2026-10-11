//! The game itself ([`sammy_core::sim::Sim`]) as a Bevy resource, ticked once per frame
//! with the frame's (virtual, pausable) time and input. Its events become sounds, save
//! writes and [`GameEvent`] messages for the presenting plugins.

use bevy::prelude::*;
use sammy_core::sim::{Event, Sim};

use crate::platform::audio::Sfx;
use crate::platform::input::Controls;
use crate::platform::vfs::GameFiles;
use sammy_core::time::Secs;

/// The game.
#[derive(Resource, Deref, DerefMut)]
pub struct Game(pub Sim);

/// A [`Sim`] event for the presenting plugins (sounds and saves are handled here).
#[derive(Message, Clone, Debug)]
pub struct GameEvent(pub Event);

/// The game's frame: everything that feeds it runs before [`GameSystems::Tick`],
/// everything that shows it after.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GameSystems {
    Tick,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameEvent>().add_systems(Update, tick.in_set(GameSystems::Tick));
    }
}

/// A new game with the player's save and the environment's debug overrides
/// ([`sammy_platform::debug`]); without the `intro` feature it boots into the menu.
pub fn new_game(save: sammy_core::save::SaveGame) -> Game {
    let mut sim = sammy_platform::debug::new_sim(save);
    sim.debug.no_intro |= !cfg!(feature = "intro");
    Game(sim)
}

fn tick(time: Res<Time>, controls: Res<Controls>, files: Res<GameFiles>, storage: Res<crate::platform::storage::Storage>, mut game: ResMut<Game>, mut sfx: Sfx, mut events: MessageWriter<GameEvent>) {
    game.tick(Secs(time.delta_secs()), &controls.0, &*files);
    let drained: Vec<Event> = game.drain_events().collect();
    for e in drained {
        match e {
            Event::Sound(name) => sfx.play(name),
            Event::Saved => crate::platform::save::write(&storage, &game.save),
            Event::LoadFailed { ref file } => error!("could not load level {file}"),
            Event::Loaded { ref file } => info!("loaded level {file} (slot {:?})", game.flow.slot),
            _ => {}
        }
        events.write(GameEvent(e));
    }
}
