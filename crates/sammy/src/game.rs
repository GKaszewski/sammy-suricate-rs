//! The game itself ([`sammy_core::sim::Sim`]) as a Bevy resource, ticked once per frame
//! with the frame's (virtual, pausable) time and input. Its events become sounds, save
//! writes and [`GameEvent`] messages for the presenting plugins.

use bevy::prelude::*;
use sammy_core::sim::{DebugOptions, Event, Sim};

use crate::platform::audio::Sfx;
use crate::platform::input::Controls;
use crate::platform::vfs::GameFiles;

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

/// A new game with the player's save, seeded from `SAMMY_SEED` or the clock, and the
/// debug overrides of the environment.
pub fn new_game(save: sammy_core::save::SaveGame) -> Game {
    let seed = std::env::var("SAMMY_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
    });
    let mut sim = Sim::new(save, seed);
    sim.debug = debug_options();
    sim.menu.debug_camera = std::env::var("SAMMY_MENU_CAM").ok().and_then(|v| {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (n.len() == 5).then(|| ([n[0], n[1], n[2]], n[3], n[4]))
    });
    Game(sim)
}

/// Debug overrides from the environment (`SAMMY_START`, `SAMMY_KEY` ...).
fn debug_options() -> DebugOptions {
    let var = |k: &str| std::env::var(k).ok();
    let num = |k: &str| var(k).and_then(|v| v.parse::<f32>().ok());
    DebugOptions {
        // SAMMY_START="x,y,z[,pan]" overrides the spawn point.
        start: var("SAMMY_START").and_then(|v| {
            let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (n.len() >= 3).then(|| ([n[0], n[1], n[2]], n.get(3).copied()))
        }),
        // SAMMY_KEY=<colour> starts with a key (255 = master).
        key: var("SAMMY_KEY").and_then(|v| v.parse().ok()),
        lives: num("SAMMY_LIVES").map(|l| l as i32),
        hearts: num("SAMMY_HEARTS"),
        eye: var("SAMMY_EYE").is_some(),
        // SAMMY_AMMO=count,type
        ammo: var("SAMMY_AMMO").and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        }),
        autowalk: var("SAMMY_AUTOWALK").is_some(),
        autofire: var("SAMMY_AUTOFIRE").is_some(),
        // SAMMY_SKIP=action1,action2 leaves out objects with those actions.
        skip_actions: var("SAMMY_SKIP").map(|v| v.split(',').map(|a| a.trim().to_lowercase()).collect()).unwrap_or_default(),
        // Without the `intro` feature the game boots straight into the menu.
        no_intro: !cfg!(feature = "intro") || ["SAMMY_MENU_VIEW", "SAMMY_MENU_CAM", "SAMMY_OPTIONS", "SAMMY_MENU_CHOICE", "SAMMY_NOINTRO"].iter().any(|v| var(v).is_some()),
    }
}

fn tick(time: Res<Time>, controls: Res<Controls>, files: Res<GameFiles>, mut game: ResMut<Game>, mut sfx: Sfx, mut events: MessageWriter<GameEvent>) {
    game.tick(time.delta_secs(), &controls.0, &*files);
    let drained: Vec<Event> = game.drain_events().collect();
    for e in drained {
        match e {
            Event::Sound(name) => sfx.play(name),
            Event::Saved => crate::platform::save::write(&game.save),
            Event::LoadFailed { ref file } => error!("could not load level {file}"),
            Event::Loaded { ref file } => info!("loaded level {file} (slot {:?})", game.flow.slot),
            _ => {}
        }
        events.write(GameEvent(e));
    }
}
