//! The order of a frame's game logic, explicit: every gameplay system has its own step,
//! and [`FRAME`] lists the steps and the command sync points in order (it mirrors the WDL
//! main loop as first ported). Feature plugins put their systems into their steps. Both
//! the order and the sync points are part of the game's behaviour: the replay test locks
//! them.

use bevy::ecs::schedule::ApplyDeferred;
use bevy::prelude::*;

/// One gameplay step (in `Update`).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Level flow (phases, loading), the extro fade, flying popups, the sun, sprites
    /// snapped onto the floor.
    Flow,
    // Engines and platforms move before everything else, so riders see this frame's
    // positions.
    LinkPlatforms,
    RunEngines,
    FollowEngines,
    RidePlatforms,
    // Animation clocks, then scripted animations.
    AnimateModels,
    ScriptedAnims,
    // Camera.
    CameraReset,
    CameraZones,
    FaceCamera,
    // Sammy's door scan; blob shadows follow last frame's movers.
    DoorScan,
    Shadows,
    Player,
    // Sammy's scans.
    TeleportEntry,
    Hazards,
    HealthPickups,
    Exits,
    Shrink,
    DoorGems,
    SurpriseGems,
    GemGravity,
    GemLifetimes,
    GemFlight,
    Collect,
    SnapPickups,
    Teleports,
    // Wasps.
    CollectAmmo,
    Fire,
    SpinAmmo,
    WaspHoming,
    WaspFlight,
    WeakDoors,
    Planks,
    Hives,
    Boulders,
    HiveWasps,
    ExpireAmmo,
    // Enemies.
    PlayerCollider,
    SyncPlayerCollider,
    AiClock,
    EnemyAi,
    ExtroTriggers,
    BumpClock,
    EnemyColliders,
    StartTriggers,
    SyncEnemyColliders,
    FadeEnemies,
    PlaceEnemies,
    CallReset,
    // Effects.
    WarningLight,
    SpawnShadows,
    PlaceShadows,
    Eye,
    SpawnRings,
    UpdateRings,
    CallRings,
    // Death and respawn.
    SpinHealth,
    AnimateHearts,
    ResetCheckpoints,
    KeyReset,
    Death,
    Titles,
}

/// A frame entry: a step, or a point where buffered commands are applied.
#[derive(Clone, Copy, Debug)]
pub enum Entry {
    Step(GameSet),
    Sync,
}

use Entry::{Step as S, Sync};
use GameSet::*;

/// The frame, in order.
pub const FRAME: &[Entry] = &[
    S(Flow),
    S(LinkPlatforms),
    S(RunEngines),
    S(FollowEngines),
    S(RidePlatforms),
    S(AnimateModels),
    S(ScriptedAnims),
    S(CameraReset),
    S(CameraZones),
    S(FaceCamera),
    Sync,
    S(DoorScan),
    S(Shadows),
    Sync,
    S(Player),
    Sync,
    S(TeleportEntry),
    S(Hazards),
    S(HealthPickups),
    S(Exits),
    S(Shrink),
    S(DoorGems),
    S(SurpriseGems),
    S(GemGravity),
    S(GemLifetimes),
    S(GemFlight),
    S(Collect),
    S(SnapPickups),
    Sync,
    S(Teleports),
    Sync,
    S(CollectAmmo),
    Sync,
    S(Fire),
    Sync,
    S(SpinAmmo),
    S(WaspHoming),
    S(WaspFlight),
    Sync,
    S(WeakDoors),
    Sync,
    S(Planks),
    Sync,
    S(Hives),
    Sync,
    S(Boulders),
    Sync,
    S(HiveWasps),
    Sync,
    S(ExpireAmmo),
    Sync,
    S(PlayerCollider),
    Sync,
    S(SyncPlayerCollider),
    S(AiClock),
    S(EnemyAi),
    Sync,
    S(ExtroTriggers),
    S(BumpClock),
    S(EnemyColliders),
    Sync,
    S(StartTriggers),
    S(SyncEnemyColliders),
    S(FadeEnemies),
    Sync,
    S(PlaceEnemies),
    S(CallReset),
    S(WarningLight),
    Sync,
    S(SpawnShadows),
    Sync,
    S(PlaceShadows),
    Sync,
    S(Eye),
    Sync,
    S(SpawnRings),
    Sync,
    S(UpdateRings),
    S(CallRings),
    Sync,
    S(SpinHealth),
    S(AnimateHearts),
    S(ResetCheckpoints),
    S(KeyReset),
    S(Death),
    Sync,
    S(Titles),
];

pub struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        // Consecutive steps are ordered without implicit syncs; the explicit ones apply
        // commands between the steps around them.
        let mut last: Option<GameSet> = None;
        let mut pending_sync = false;
        for entry in FRAME {
            match *entry {
                Entry::Sync => pending_sync = true,
                Entry::Step(set) => {
                    if let Some(prev) = last {
                        app.configure_sets(Update, set.after_ignore_deferred(prev));
                        if pending_sync {
                            app.add_systems(Update, ApplyDeferred.after_ignore_deferred(prev).before_ignore_deferred(set));
                        }
                    }
                    last = Some(set);
                    pending_sync = false;
                }
            }
        }
    }
}
