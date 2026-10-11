//! Behaviour lock for refactoring: plays levels headlessly (no window, no GPU) with a
//! fixed 60 Hz timestep, a seeded RNG, scripted input and a single-threaded schedule,
//! and compares a per-half-second digest of the game against `tests/golden/*.txt`.
//!
//! Needs the original game files (`SAMMY_DIR` or the default install); skipped
//! without them. `UPDATE_GOLDEN=1 cargo test --release --test replay` rewrites the
//! golden files (only after a change that is *meant* to alter behaviour).

use std::fmt::Write as _;
use std::time::Duration;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use sammy::input::{Controls, ScriptedInput};

const FPS: f64 = 60.0;
/// Frames between digests (0.5 s).
const SAMPLE: u64 = 30;

/// A headless game on `level` driven by `script`, or `None` without game data.
fn headless(level: &str, script: impl FnMut(u64) -> Controls + Send + Sync + 'static) -> Option<App> {
    let dir = sammy::game_dir();
    if !dir.exists() {
        eprintln!("skipped: no game data at {}", dir.display());
        return None;
    }
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
    let mut save = sammy::save::SaveGame::default();
    save.readonly = true;
    for s in save.level_state.iter_mut().skip(1) {
        if *s != sammy::save::LevelState::Absent {
            *s = sammy::save::LevelState::Unlocked;
        }
    }
    save.games_started = 1;
    app.insert_resource(save);
    app.insert_resource(sammy::game::Rng::seeded(42));
    app.insert_resource(ScriptedInput { frame: 0, script: Box::new(script) });
    sammy::add_game(&mut app, &dir, sammy::Args { model: None, level: level.into() }).ok()?;
    // Deterministic system order.
    fn single<L: ScheduleLabel + Clone>(app: &mut App, l: L) {
        app.edit_schedule(l, |s| {
            s.set_executor(SingleThreadedExecutor::new());
        });
    }
    single(&mut app, PreStartup);
    single(&mut app, Startup);
    single(&mut app, PostStartup);
    single(&mut app, First);
    single(&mut app, PreUpdate);
    single(&mut app, Update);
    single(&mut app, PostUpdate);
    single(&mut app, Last);
    app.finish();
    app.cleanup();
    Some(app)
}

/// FNV-1a (stable across Rust versions, unlike `DefaultHasher`).
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

fn r(v: f32) -> f32 {
    // Rounded to 1/100 of a Bevy unit (0.32 quants); -0 printed as 0.
    let v = (v * 100.0).round() / 100.0;
    if v == 0.0 { 0.0 } else { v }
}

/// Game state, Sammy, and a hash over every named entity's transform (sorted, so the
/// spawn order does not matter).
fn digest(world: &mut World) -> String {
    let g = world.resource::<sammy::game::GameState>();
    let mut out = format!(
        "score {} goal {} lives {} hearts {} key {} ammo {}x{} boost {} reached {} won {} lost {}",
        g.score,
        g.goal,
        g.lives,
        r(g.hearts),
        g.key,
        g.ammo,
        g.ammo_type,
        r(g.boost),
        g.goal_reached,
        g.won,
        g.lost
    );
    let mut q = world.query::<(&sammy::actions::player::Player, &Transform)>();
    for (p, _) in q.iter(world) {
        let _ = write!(out, " | sammy {} {} {} pan {}", r(p.pos[0] / 32.0), r(p.pos[1] / 32.0), r(p.pos[2] / 32.0), r(p.pan));
    }
    let mut lines: Vec<String> = world
        .query::<(Option<&Name>, &GlobalTransform)>()
        .iter(world)
        .map(|(n, t)| {
            let (_, rot, pos) = t.to_scale_rotation_translation();
            let (y, x, z) = rot.to_euler(EulerRot::YXZ);
            format!("{} {} {} {} {} {} {}", n.map_or("?", |n| n.as_str()), r(pos.x), r(pos.y), r(pos.z), r(y), r(x), r(z))
        })
        .collect();
    lines.sort();
    let _ = write!(out, " | entities {} {:016x}", lines.len(), fnv(&lines.join("\n")));
    out
}

fn run(name: &str, level: &str, seconds: f64, script: impl FnMut(u64) -> Controls + Send + Sync + 'static) {
    let Some(mut app) = headless(level, script) else { return };
    let frames = (seconds * FPS) as u64;
    let mut got = String::new();
    for f in 0..frames {
        app.update();
        if f % SAMPLE == 0 {
            let _ = writeln!(got, "{:5.1}s {}", f as f64 / FPS, digest(app.world_mut()));
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &got).unwrap();
        eprintln!("wrote {}", path.display());
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap();
    if let Some((i, (w, g))) = want.lines().zip(got.lines()).enumerate().find(|(_, (w, g))| w != g) {
        let actual = path.with_extension("actual.txt");
        std::fs::write(&actual, &got).unwrap();
        panic!("{name}: behaviour changed at line {}\n want: {w}\n  got: {g}\n(full run in {})", i + 1, actual.display());
    }
    assert_eq!(want.lines().count(), got.lines().count(), "{name}: run length changed");
}

/// Helpers for scripts.
fn walk(turn: f32) -> Controls {
    Controls { forward: 1.0, turn, ..default() }
}

#[test]
fn level01_walk_jump_boost() {
    run("level01", "1", 40.0, |f| {
        let mut c = walk(if (120..170).contains(&(f % 600)) { 1.0 } else { 0.0 });
        c.jump = (200..215).contains(&(f % 400));
        c.jump_hit = f % 400 == 200;
        c.boost = (300..360).contains(&(f % 900));
        c.uturn_hit = f == 1500;
        c
    });
}

#[test]
fn level04_lions_fire() {
    run("level04", "4", 40.0, |f| {
        let mut c = walk(if (60..100).contains(&(f % 480)) { -1.0 } else { 0.0 });
        c.fire_hit = f % 90 == 45;
        c.jump_hit = f % 300 == 150;
        c.jump = (150..165).contains(&(f % 300));
        c
    });
}

#[test]
fn level20_jump_world() {
    run("level20", "20", 40.0, |f| {
        let mut c = walk(if (200..260).contains(&(f % 700)) { 1.0 } else { 0.0 });
        c.jump_hit = f % 120 == 60;
        c.jump = (60..80).contains(&(f % 120));
        c
    });
}

#[test]
fn level36_think_world() {
    run("level36", "36", 40.0, |f| {
        let mut c = walk(if (100..140).contains(&(f % 360)) { -1.0 } else { 0.0 });
        c.fire_hit = f % 200 == 100;
        c
    });
}

#[test]
fn extro_ride() {
    run("extro", "49", 60.0, |_| Controls::default());
}

#[test]
fn menu_intro() {
    run("intro", "menu", 50.0, |_| Controls::default());
}

/// Every level for 20 s with a generic walk/turn/jump/fire/boost script.
#[test]
fn all_levels_sweep() {
    for (slot, _) in sammy::flow::LEVELS {
        if *slot == 49 {
            continue; // the extro has its own scenario
        }
        run(&format!("sweep{slot:02}"), &slot.to_string(), 20.0, |f| {
            let mut c = walk(if (90..130).contains(&(f % 400)) { 1.0 } else if (250..270).contains(&(f % 400)) { -1.0 } else { 0.0 });
            c.jump_hit = f % 150 == 75;
            c.jump = (75..95).contains(&(f % 150));
            c.fire_hit = f % 110 == 55;
            c.boost = (200..240).contains(&(f % 500));
            c.uturn_hit = f % 700 == 350;
            c
        });
    }
}
