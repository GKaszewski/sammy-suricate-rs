//! A new game with the debug overrides of the environment (`SAMMY_START`,
//! `SAMMY_KEY` ...), the same for every frontend.

use sammy_core::save::SaveGame;
use sammy_core::sim::{DebugOptions, Sim};

/// A game with `save`, seeded from `SAMMY_SEED` or the clock, and the environment's
/// debug overrides.
pub fn new_sim(save: SaveGame) -> Sim {
    let seed = std::env::var("SAMMY_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or_else(clock_seed);
    let mut sim = Sim::new(save, seed);
    sim.debug = options();
    sim.menu.debug_camera = std::env::var("SAMMY_MENU_CAM").ok().and_then(|v| {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (n.len() == 5).then(|| ([n[0], n[1], n[2]], n[3], n[4]))
    });
    sim
}

#[cfg(not(target_arch = "wasm32"))]
fn clock_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

#[cfg(target_arch = "wasm32")]
fn clock_seed() -> u64 {
    1
}

/// Debug overrides from the environment.
pub fn options() -> DebugOptions {
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
        // SAMMY_GIVE_UP=<secs> gives up the level (lose screen) after that much play.
        give_up_after: num("SAMMY_GIVE_UP"),
        // SAMMY_PORTRAIT=distance,height,angle looks at Sammy's face.
        portrait: var("SAMMY_PORTRAIT").and_then(|v| v.split(',').map(|x| x.trim().parse().ok()).collect::<Option<Vec<f32>>>()).and_then(|n| n.try_into().ok()),
        no_intro: ["SAMMY_MENU_VIEW", "SAMMY_MENU_CAM", "SAMMY_OPTIONS", "SAMMY_MENU_CHOICE", "SAMMY_NOINTRO"].iter().any(|v| var(v).is_some()),
    }
}

/// Debug overrides kept as `KEY=VALUE` lines in the storage's `env.txt`, for platforms
/// without an environment or command line (the PS Vita); real environment variables
/// win. Call before anything reads the environment.
pub fn load_env_file(storage: &crate::Storage) {
    let Some(text) = storage.read("env.txt").and_then(|d| String::from_utf8(d).ok()) else { return };
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some((k, v)) = line.split_once('=')
            && std::env::var_os(k.trim()).is_none()
        {
            // SAFETY: called at start-up, before any other thread exists.
            unsafe { std::env::set_var(k.trim(), v.trim()) };
        }
    }
}

/// `SAMMY_PAD_NAV=right,down,a,b,...`: scripted pad presses, the first after
/// `PAD_NAV_START` seconds, then one every `PAD_NAV_STEP`.
const PAD_NAV_START: f32 = 3.0;
const PAD_NAV_STEP: f32 = 0.5;

/// The scripted pad press due in the frame ending at `clock` seconds (`dt` long), and
/// whether the script is running (the pad counts as in use).
pub fn pad_nav(clock: f32, dt: f32) -> (Option<String>, bool) {
    let Ok(script) = std::env::var("SAMMY_PAD_NAV") else { return (None, false) };
    let t = clock - PAD_NAV_START;
    let step = (t / PAD_NAV_STEP).floor();
    let due = (t >= 0.0 && (t - dt) / PAD_NAV_STEP < step).then(|| script.split(',').nth(step as usize).map(str::to_owned)).flatten();
    (due, t >= 0.0)
}
