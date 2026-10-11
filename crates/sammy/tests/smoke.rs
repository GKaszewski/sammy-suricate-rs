//! The Bevy frontend headless (no window, no GPU): every scene loads, the game ticks,
//! and the scene mirrors the game's entities. Game behaviour itself is locked by
//! `sammy-core`'s `sim_replay`.
//!
//! Needs the original game files (`SAMMY_DIR`, or the folder the game remembered);
//! skipped without them.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use sammy::game::Game;
use sammy::platform::input::ScriptedInput;
use sammy::scene::{Mirror, Mirrored};
use sammy_core::input::Controls;
use sammy_core::save::{LevelState, SaveGame};

const FPS: f64 = 60.0;

/// A headless game on `level` driven by `script`, or `None` without game data.
fn headless(level: &str, script: impl FnMut(u64) -> Controls + Send + Sync + 'static) -> Option<App> {
    // The game folder of this machine: `SAMMY_DIR`, else the one the game remembered.
    let storage = sammy::platform::storage::Storage::user();
    let Some(files) = sammy::platform::locate::known(&storage).and_then(|d| sammy::platform::vfs::GameFiles::open(&d).ok()).filter(|f| f.is_game()) else {
        eprintln!("skipped: no game data (set SAMMY_DIR)");
        return None;
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::winit::WinitPlugin>()
            .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, close_when_requested: false, ..default() })
            .set(RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })), ..default() }),
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / FPS)));
    // Every level open; nothing ever written to the player's save.
    let mut save = SaveGame { readonly: true, ..Default::default() };
    for s in save.level_state.iter_mut().skip(1) {
        if *s != LevelState::Absent {
            *s = LevelState::Unlocked;
        }
    }
    save.games_started = 1;
    app.insert_resource(Game(sammy_core::sim::Sim::new(save, 42)));
    app.insert_resource(ScriptedInput { frame: 0, script: Box::new(script) });
    sammy::add_game(&mut app, files, storage, sammy::Args { model: None, level: level.into() });
    app.finish();
    app.cleanup();
    Some(app)
}

/// Plays `seconds` and checks the scene against the game every second.
fn play(level: &str, seconds: f64, script: impl FnMut(u64) -> Controls + Send + Sync + 'static) {
    let Some(mut app) = headless(level, script) else { return };
    for f in 0..(seconds * FPS) as u64 {
        app.update();
        if f % FPS as u64 != FPS as u64 - 1 {
            continue;
        }
        let w = app.world_mut();
        let looks: Vec<_> = w.resource::<Game>().world.looks.keys().copied().collect();
        assert!(!looks.is_empty(), "{level}: nothing to show at frame {f}");
        let mirror = w.resource::<Mirror>();
        let missing = looks.iter().filter(|id| mirror.entity(**id).is_none()).count();
        assert_eq!(missing, 0, "{level}: {missing} game entities not mirrored at frame {f}");
        let shown = w.query::<&Mirrored>().iter(w).count();
        assert_eq!(shown, looks.len(), "{level}: mirrored entities without a game entity at frame {f}");
    }
}

fn walk_jump_fire(f: u64) -> Controls {
    Controls { forward: 1.0, turn: if (90..130).contains(&(f % 400)) { 1.0 } else { 0.0 }, jump_hit: f % 150 == 75, jump: (75..95).contains(&(f % 150)), fire_hit: f % 110 == 55, ..Default::default() }
}

#[test]
fn menu_and_intro() {
    play("menu", 12.0, |_| Controls::default());
}

#[test]
fn tutorial_level() {
    play("4", 15.0, walk_jump_fire);
}

#[test]
fn think_level() {
    play("36", 15.0, walk_jump_fire);
}

#[test]
fn extro() {
    play("49", 15.0, |_| Controls::default());
}
