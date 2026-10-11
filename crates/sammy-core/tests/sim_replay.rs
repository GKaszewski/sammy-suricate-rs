//! Behaviour lock for the core game: plays scenarios on [`Sim`] alone (no engine) with
//! a fixed 60 Hz frame, a seeded RNG and scripted input, and compares a per-half-second
//! digest against `tests/golden/*.txt`.
//!
//! Needs the original game files (`SAMMY_DIR` or the default install); skipped
//! without them. `UPDATE_GOLDEN=1 cargo test --release --test sim_replay` rewrites the
//! golden files (only after a change that is *meant* to alter behaviour).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sammy_core::input::Controls;
use sammy_core::levels::{LEVELS, MENU_FILE, file_for};
use sammy_core::save::{LevelState, SaveGame};
use sammy_core::sim::{Files, Sim};

const FPS: f32 = 60.0;
/// Frames between digests (0.5 s).
const SAMPLE: u64 = 30;
const DEFAULT_GAME_DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

/// Loose files over `*.wrs` archive entries, names case-insensitive.
struct GameFiles {
    loose: HashMap<String, PathBuf>,
    archived: HashMap<String, Vec<u8>>,
}

impl GameFiles {
    fn open(dir: &Path) -> Option<Self> {
        let mut loose = HashMap::new();
        let mut archived = HashMap::new();
        for entry in std::fs::read_dir(dir).ok()? {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_string_lossy().to_lowercase();
            if name.ends_with(".wrs") {
                for e in formats::wrs::parse(&std::fs::read(&path).ok()?).ok()? {
                    archived.insert(e.name.to_lowercase(), e.data());
                }
            } else if path.is_file() {
                loose.insert(name, path);
            }
        }
        Some(Self { loose, archived })
    }
}

impl Files for GameFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let key = name.to_lowercase();
        match self.loose.get(&key) {
            Some(p) => std::fs::read(p).ok(),
            None => self.archived.get(&key).cloned(),
        }
    }
}

fn files() -> Option<&'static GameFiles> {
    static FILES: std::sync::OnceLock<Option<GameFiles>> = std::sync::OnceLock::new();
    FILES
        .get_or_init(|| {
            let dir = std::env::var_os("SAMMY_DIR").map(PathBuf::from).unwrap_or_else(|| DEFAULT_GAME_DIR.into());
            GameFiles::open(&dir)
        })
        .as_ref()
}

/// FNV-1a (stable across Rust versions, unlike `DefaultHasher`).
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// Rounded to 1/100 (of 32 quants for positions); -0 printed as 0.
fn r(v: f32) -> f32 {
    let v = (v * 100.0).round() / 100.0;
    if v == 0.0 { 0.0 } else { v }
}

/// Game state, Sammy, and a hash over every shown entity's body.
fn digest(sim: &Sim) -> String {
    let g = &sim.game;
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
    for p in sim.world.players.values() {
        let _ = write!(out, " | sammy {} {} {} pan {}", r(p.pos[0] / 32.0), r(p.pos[1] / 32.0), r(p.pos[2] / 32.0), r(p.pan));
    }
    let lines: Vec<String> = sim
        .world
        .bodies
        .values()
        .map(|b| {
            let [x, y, z] = b.pos.map(|v| r(v / 32.0));
            let [p, t, ro] = b.angles.map(r);
            format!("{} {x} {y} {z} {p} {t} {ro} {} {}", b.name, b.hidden, r(b.scale[0]))
        })
        .collect();
    let _ = write!(out, " | {:?} | entities {} {:016x}", sim.flow.phase, lines.len(), fnv(&lines.join("\n")));
    out
}

fn new_sim() -> Sim {
    // Every level open.
    let mut save = SaveGame { readonly: true, ..Default::default() };
    for s in save.level_state.iter_mut().skip(1) {
        if *s != LevelState::Absent {
            *s = LevelState::Unlocked;
        }
    }
    save.games_started = 1;
    Sim::new(save, 42)
}

/// Plays `level` (a slot, or `menu`) for `seconds` and returns the digest lines.
fn play(level: &str, seconds: f32, mut script: impl FnMut(u64) -> Controls) -> Option<String> {
    let files = files()?;
    let mut sim = new_sim();
    let file = match level.parse::<u32>() {
        Ok(slot) => file_for(slot)?.to_owned(),
        Err(_) => MENU_FILE.to_owned(),
    };
    sim.request_load(file);
    let mut got = String::new();
    for f in 0..(seconds * FPS) as u64 {
        // The first frame has no elapsed time (as on an engine's first update).
        let dt = if f == 0 { 0.0 } else { 1.0 / FPS };
        sim.tick(dt, &script(f), files);
        sim.drain_events().for_each(drop);
        if f % SAMPLE == 0 {
            let _ = writeln!(got, "{:5.1}s {}", f as f32 / FPS, digest(&sim));
        }
    }
    Some(got)
}

fn run(name: &str, level: &str, seconds: f32, script: impl FnMut(u64) -> Controls) {
    let Some(got) = play(level, seconds, script) else {
        eprintln!("skipped: no game data");
        return;
    };
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.txt"));
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

fn walk(turn: f32) -> Controls {
    Controls { forward: 1.0, turn, ..Default::default() }
}

fn level01(f: u64) -> Controls {
    let mut c = walk(if (120..170).contains(&(f % 600)) { 1.0 } else { 0.0 });
    c.jump = (200..215).contains(&(f % 400));
    c.jump_hit = f % 400 == 200;
    c.boost = (300..360).contains(&(f % 900));
    c.uturn_hit = f == 1500;
    c
}

fn level04(f: u64) -> Controls {
    let mut c = walk(if (60..100).contains(&(f % 480)) { -1.0 } else { 0.0 });
    c.fire_hit = f % 90 == 45;
    c.jump_hit = f % 300 == 150;
    c.jump = (150..165).contains(&(f % 300));
    c
}

fn level20(f: u64) -> Controls {
    let mut c = walk(if (200..260).contains(&(f % 700)) { 1.0 } else { 0.0 });
    c.jump_hit = f % 120 == 60;
    c.jump = (60..80).contains(&(f % 120));
    c
}

fn level36(f: u64) -> Controls {
    let mut c = walk(if (100..140).contains(&(f % 360)) { -1.0 } else { 0.0 });
    c.fire_hit = f % 200 == 100;
    c
}

fn sweep(f: u64) -> Controls {
    let mut c = walk(if (90..130).contains(&(f % 400)) { 1.0 } else if (250..270).contains(&(f % 400)) { -1.0 } else { 0.0 });
    c.jump_hit = f % 150 == 75;
    c.jump = (75..95).contains(&(f % 150));
    c.fire_hit = f % 110 == 55;
    c.boost = (200..240).contains(&(f % 500));
    c.uturn_hit = f % 700 == 350;
    c
}

#[test]
fn level01_walk_jump_boost() {
    run("level01", "1", 40.0, level01);
}

#[test]
fn level04_lions_fire() {
    run("level04", "4", 40.0, level04);
}

#[test]
fn level20_jump_world() {
    run("level20", "20", 40.0, level20);
}

#[test]
fn level36_think_world() {
    run("level36", "36", 40.0, level36);
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
    for (slot, _) in LEVELS {
        if *slot == 49 {
            continue; // the extro has its own scenario
        }
        run(&format!("sweep{slot:02}"), &slot.to_string(), 20.0, sweep);
    }
}

/// Debug: `COMPARE=1` prints where the game state and Sammy first differ from the
/// engine port's goldens (`crates/sammy/tests/golden`), scenario by scenario.
#[test]
fn compare_with_engine_port() {
    if std::env::var_os("COMPARE").is_none() {
        return;
    }
    let old = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sammy/tests/golden");
    type Case = (String, String, f32, fn(u64) -> Controls);
    let mut cases: Vec<Case> = vec![
        ("level01".into(), "1".into(), 40.0, level01),
        ("level04".into(), "4".into(), 40.0, level04),
        ("level20".into(), "20".into(), 40.0, level20),
        ("level36".into(), "36".into(), 40.0, level36),
        ("extro".into(), "49".into(), 60.0, |_| Controls::default()),
        ("intro".into(), "menu".into(), 50.0, |_| Controls::default()),
    ];
    for (slot, _) in LEVELS.iter().filter(|(s, _)| *s != 49) {
        cases.push((format!("sweep{slot:02}"), slot.to_string(), 20.0, sweep));
    }
    for (name, level, secs, script) in cases {
        let Some(got) = play(&level, secs, script) else { return };
        let Ok(want) = std::fs::read_to_string(old.join(format!("{name}.txt"))) else { continue };
        let state = |l: &str| l.split(" | entities").next().unwrap_or("").to_owned();
        let strip = |l: &str| {
            // Our lines add the phase before the entity hash.
            let s = state(l);
            s.rsplit_once(" | ").filter(|(_, p)| !p.starts_with("sammy")).map_or(s.clone(), |(a, _)| a.to_owned())
        };
        match want.lines().zip(got.lines()).enumerate().find(|(_, (w, g))| strip(w) != strip(g)) {
            None => println!("SAME {name}"),
            Some((i, (w, g))) => println!("DIFF {name} line {}\n  old: {}\n  new: {}", i + 1, strip(w), strip(g)),
        }
    }
}

