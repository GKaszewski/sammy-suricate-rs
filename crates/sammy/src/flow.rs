//! Level flow (mm1.wdl:542-1156, Loads.wdl): the level table, loading and
//! unloading, the level-start camera sweep, and win/lose progression.

use bevy::prelude::*;

use crate::actions::player::{JumpParams, TICKS};
use crate::camera::{CameraParams, FollowState};
use crate::game::GameState;
use crate::level::Spawner;
use crate::material::LevelMaterial;
use crate::vfs::GameFiles;

pub use sammy_core::levels::{EXTRO_SLOT, LEVELS, file_for, next_slot, slot_for};

pub use sammy_core::flow::Phase;

/// The level being played ([`sammy_core::flow::LevelFlow`]) and a counter of loads.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct CurrentLevel {
    #[deref]
    pub flow: sammy_core::flow::LevelFlow,
    /// Bumped by every level load (reloads of the same file included).
    pub loads: u32,
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
            .add_systems(Update, run_flow.in_set(crate::schedule::GameSet::Flow));
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
    *current = CurrentLevel { flow: sammy_core::flow::LevelFlow { slot, file: file.clone(), phase, intro_ticks: 0.0 }, loads: current.loads + 1 };
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


fn run_flow(
    time: Res<Time>,
    controls: Res<crate::input::Controls>,
    game: Res<GameState>,
    mut save: ResMut<crate::save::SaveGame>,
    follow: Query<&FollowState>,
    mut current: ResMut<CurrentLevel>,
    mut pending: ResMut<PendingLoad>,
) {
    let input = sammy_core::flow::FlowInput {
        t: time.delta_secs() * TICKS,
        skip: controls.confirm || controls.options || controls.accept,
        any: controls.any,
        converged: follow.iter().any(|f| f.converged),
    };
    let step = current.flow.step(input, &game, &mut save);
    if step.saved {
        crate::save::write(&save);
    }
    if let Some(file) = step.load {
        pending.0 = Some(file);
    }
}

pub fn playing(current: &CurrentLevel) -> bool {
    current.flow.playing()
}
