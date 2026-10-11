//! Level flow (mm1.wdl:542-1156, Loads.wdl): the level table, loading and
//! unloading, the level-start camera sweep, and win/lose progression.

use bevy::prelude::*;

use crate::actions::player::{JumpParams, TICKS};
use crate::camera::{CameraParams, FollowState};
use crate::game::GameState;
use crate::level::Spawner;
use crate::material::LevelMaterial;
use crate::vfs::GameFiles;

/// `InitLevelN` → WMB (Loads.wdl; docs/gameplay.md §3.1). Bonus slots 13-16,
/// 29-32 and 45-48 point at files that never shipped and are skipped.
pub const LEVELS: &[(u32, &str)] = &[
    (1, "10001.wmb"),
    (2, "10002.wmb"),
    (3, "10003.wmb"),
    (4, "10004.wmb"),
    (5, "10005.wmb"),
    (6, "10006.wmb"),
    (7, "10007.wmb"),
    (8, "10008.wmb"),
    (9, "10009.wmb"),
    (10, "10012.wmb"),
    (11, "10010.wmb"),
    (12, "10011.wmb"),
    (17, "101o_j.wmb"),
    (18, "104M_j.wmb"),
    (19, "103J_j.wmb"),
    (20, "112e_j.wmb"),
    (21, "107i_j.wmb"),
    (22, "174a_j07.wmb"),
    (23, "111r_j.wmb"),
    (24, "172c_j06.wmb"),
    (25, "175c_t.wmb"),
    (26, "177e_j.wmb"),
    (27, "108i_j.wmb"),
    (28, "179f_t.wmb"),
    (33, "106g_t.wmb"),
    (34, "171c_t04.wmb"),
    (35, "110n_t.wmb"),
    (36, "180c_t.wmb"),
    (37, "170b_t00.wmb"),
    (38, "102N_t.wmb"),
    (39, "173b_t05.wmb"),
    (40, "114t_t.wmb"),
    (41, "178d_t.wmb"),
    (42, "176g_t.wmb"),
    (43, "113q_t.wmb"),
    (44, "109o_t.wmb"),
    (49, "extro.wmb"),
];

pub fn file_for(slot: u32) -> Option<&'static str> {
    LEVELS.iter().find(|(s, _)| *s == slot).map(|(_, f)| *f)
}

pub fn slot_for(file: &str) -> Option<u32> {
    LEVELS.iter().find(|(_, f)| f.eq_ignore_ascii_case(file)).map(|(s, _)| *s)
}

/// `SearchNextLevel`: the next existing slot. The last level of each world
/// (12 / 28 / 44) returns to the menu in the original; with no menu yet we
/// continue with the next world, and 44 leads to the extro.
pub const EXTRO_SLOT: u32 = 49;

pub fn next_slot(slot: u32) -> u32 {
    match slot {
        12 => 17,
        28 => 33,
        44 => 49,
        49 => 1,
        s => LEVELS.iter().map(|(n, _)| *n).find(|&n| n > s).unwrap_or(1),
    }
}

/// `PartieEncours` / `PartieGagnee` reduced to the phases the port needs.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Phase {
    /// Level-start camera sweep (input disabled), with elapsed ticks.
    #[default]
    Intro,
    Playing,
    /// Win / lose screens, with elapsed ticks (320 or any key).
    Won(f32),
    Lost(f32),
    /// The 3D main menu scene (no game running).
    Menu,
}

#[derive(Resource, Default)]
pub struct CurrentLevel {
    pub slot: Option<u32>,
    pub file: String,
    pub phase: Phase,
    pub intro_ticks: f32,
}

/// Request to (re)load a level file on the next frame.
#[derive(Resource, Default)]
pub struct PendingLoad(pub Option<String>);

/// Root of everything spawned from the level file.
#[derive(Component)]
pub struct LevelRoot;

/// Runtime entities that belong to the current level (e.g. ejected keys).
#[derive(Component)]
pub struct LevelScoped;

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .init_resource::<PendingLoad>()
            .init_resource::<crate::level::GameAssets>()
            .add_systems(PreUpdate, load_pending)
            .add_systems(Update, run_flow);
    }
}

#[allow(clippy::too_many_arguments)]
fn load_pending(
    mut commands: Commands,
    mut pending: ResMut<PendingLoad>,
    mut current: ResMut<CurrentLevel>,
    files: Res<GameFiles>,
    old: Query<Entity, Or<(With<LevelRoot>, With<LevelScoped>)>>,
    mut follow: Query<&mut FollowState>,
    (mut game, mut jump, mut cam, mut assets, mut save): (
        ResMut<GameState>,
        ResMut<JumpParams>,
        ResMut<CameraParams>,
        ResMut<crate::level::GameAssets>,
        ResMut<crate::save::SaveGame>,
    ),
    (mut meshes, mut images, mut materials, mut level_materials): (
        ResMut<Assets<Mesh>>,
        ResMut<Assets<Image>>,
        ResMut<Assets<StandardMaterial>>,
        ResMut<Assets<LevelMaterial>>,
    ),
) {
    let Some(file) = pending.0.take() else { return };
    for e in &old {
        commands.entity(e).despawn();
    }
    // `InitLevelGlobal`: per-level globals back to their defaults; the level's
    // config entities then override them (queued by their actions).
    *game = GameState::default();
    game.apply_debug_env();
    *jump = JumpParams::default();
    *cam = CameraParams::default();
    commands.insert_resource(crate::actions::player::PlayerTuning::default());
    commands.insert_resource(crate::camera::CameraRuntime::default());
    commands.insert_resource(crate::actions::enemy::LionGlobals::default());
    commands.insert_resource(crate::actions::extro::Pov::default());
    commands.insert_resource(crate::particles::Particles::default());
    commands.remove_resource::<crate::actions::extro::FinExtro>();
    for mut f in &mut follow {
        *f = FollowState::default();
    }
    let slot = slot_for(&file);
    // `sun_angle`: pan 135 / tilt 85 in levels (mm1.wdl:618-620), pan 90 in the extro (Extro.wdl:81).
    commands.insert_resource(if slot == Some(EXTRO_SLOT) { crate::shadow::SunAngle(90.0, 85.0) } else { crate::shadow::SunAngle::default() });
    let phase = if file.eq_ignore_ascii_case(crate::menu::MENU_FILE) { Phase::Menu } else { Phase::Intro };
    *current = CurrentLevel { slot, file: file.clone(), phase, intro_ticks: 0.0 };
    let mut spawner = Spawner {
        commands: &mut commands,
        files: &files,
        meshes: &mut meshes,
        images: &mut images,
        materials: &mut materials,
        level_materials: &mut level_materials,
        assets: &mut assets,
    };
    if let Err(e) = spawner.spawn_level(&file) {
        error!("loading {file}: {e}");
        return;
    }
    if slot == Some(EXTRO_SLOT) && !save.extro_seen {
        // `ExtroVue = 1` (Extro.wdl:15): unlocks the Bonus menu entry.
        save.extro_seen = true;
        crate::save::write(&save);
    }
    // AfterLoadInitLevel: start at the checkpoint saved for this level (InitLvl.wdl:232-237).
    if let Some(cp) = slot.and_then(|s| save.checkpoints.get(s as usize).copied().flatten()) {
        commands.queue(move |w: &mut World| {
            let mut q = w.query::<&mut crate::actions::player::Player>();
            let mut pan = 0.0;
            for mut p in q.iter_mut(w) {
                pan = p.pan;
                p.teleport(cp, pan);
            }
            let mut g = w.resource_mut::<GameState>();
            g.start = Some((cp, pan));
            g.checkpoint = true; // [I] kept until the next game end stores it again
        });
    }
    // Level 10 hard-codes 4 lives (Loads.wdl:80).
    if slot == Some(10) {
        commands.queue(|w: &mut World| w.resource_mut::<GameState>().lives = 4);
    }
    // Debug overrides applied after the level's own config entities.
    commands.queue(|w: &mut World| {
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f32>().ok());
        let mut g = w.resource_mut::<GameState>();
        if let Some(l) = env("SAMMY_LIVES") {
            g.lives = l as i32;
        }
        if let Some(h) = env("SAMMY_HEARTS") {
            g.hearts = h;
        }
        if env("SAMMY_EYE").is_some() {
            g.eye_ticks = 1.0e9; // Eye of Truth for the whole level
        }
        // SAMMY_AMMO=count,type
        if let Some((n, k)) = std::env::var("SAMMY_AMMO").ok().and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.parse::<i32>().ok()?, b.parse::<u32>().ok()?))
        }) {
            g.ammo = n;
            g.ammo_type = k;
            g.ammo_poison = [3.0, 2.0, 1.0, 1.0, 4.0, 6.0, 1.0][(k.clamp(1, 7) - 1) as usize];
        }
    });
    info!("loaded level {file} (slot {slot:?})");
}

fn any_key(keys: &ButtonInput<KeyCode>) -> bool {
    keys.get_just_pressed().next().is_some()
}

fn run_flow(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<GameState>,
    mut save: ResMut<crate::save::SaveGame>,
    follow: Query<&FollowState>,
    mut current: ResMut<CurrentLevel>,
    mut pending: ResMut<PendingLoad>,
) {
    let t = time.delta_secs() * TICKS;
    current.phase = match current.phase {
        Phase::Intro => {
            current.intro_ticks += t;
            let converged = follow.iter().any(|f| f.converged);
            // Ends when the camera converges, after 128 ticks, or on Enter/Esc (mm1.wdl:744-813).
            let skip = keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Escape]);
            if (converged && current.intro_ticks > 8.0) || current.intro_ticks >= 128.0 || skip {
                Phase::Playing
            } else {
                Phase::Intro
            }
        }
        Phase::Playing if game.won || (game.lost && current.slot != Some(EXTRO_SLOT)) => {
            // End of a game: StoreCurrentSkills, best score, ValideLevel, Save_Info "BestSco"
            // (mm1.wdl:972-997).
            if let Some(slot) = current.slot {
                let checkpoint = game.checkpoint.then_some(game.start).flatten().map(|(p, _)| p);
                let next = game.won.then(|| next_slot(slot)).filter(|&n| n != slot);
                save.record_game(slot, game.score, checkpoint, game.won, next);
                if game.won && slot == EXTRO_SLOT {
                    save.extro_seen = true;
                }
                crate::save::write(&save);
            }
            if game.won { Phase::Won(0.0) } else { Phase::Lost(0.0) }
        }
        // Win: Sammy dances until a key (`AnimGagnee` blocks, mm1.wdl:947); lose: 320 ticks or a key.
        Phase::Won(ticks) | Phase::Lost(ticks)
            if (ticks > 16.0 && any_key(&keys)) || (matches!(current.phase, Phase::Lost(_)) && ticks >= 320.0) =>
        {
            let won = matches!(current.phase, Phase::Won(_));
            pending.0 = match current.slot {
                Some(slot) if won => Some(after_win(slot, &save).to_owned()),
                _ => Some(current.file.clone()), // lose: same level again
            };
            current.phase
        }
        Phase::Won(ticks) => Phase::Won(ticks + t),
        Phase::Lost(ticks) => Phase::Lost(ticks + t),
        p => p,
    };
}

/// After a win (mm1.wdl:988-1123): the last level of a world returns to the menu, unless
/// both 28 and 44 are reached, which starts the extro; otherwise the next level starts.
fn after_win(slot: u32, save: &crate::save::SaveGame) -> &'static str {
    use crate::save::UNLOCKED;
    let both = save.state(28) == UNLOCKED && save.state(44) == UNLOCKED;
    if matches!(slot, 28 | 44) && both {
        return file_for(EXTRO_SLOT).unwrap_or(crate::menu::MENU_FILE);
    }
    if matches!(slot, 12 | 28 | 44) || slot == EXTRO_SLOT {
        return crate::menu::MENU_FILE;
    }
    file_for(next_slot(slot)).unwrap_or(crate::menu::MENU_FILE)
}

pub fn playing(current: &CurrentLevel) -> bool {
    current.phase == Phase::Playing
}
