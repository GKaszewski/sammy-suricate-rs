//! Ports of the WDL entity actions (see docs/gameplay.md). Each level object's
//! action name is dispatched here when the level is spawned.

mod decor;
pub mod door;
pub mod health;
pub mod pickups;
pub mod platform;
pub mod player;
pub mod title;
pub mod wasp;

use bevy::prelude::*;
use formats::wmb::Object;

use crate::camera::CameraParams;
use crate::game::GameState;


pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<player::JumpParams>()
            .init_resource::<GameState>()
            .init_resource::<crate::game::Rng>()
            .init_resource::<pickups::GemSprites>()
            .add_message::<crate::game::ResetScan>()
            .add_message::<wasp::WaspHit>()
            .add_systems(
                Update,
                (
                    // Platforms move first so riders and the player see this frame's positions.
                    (platform::link_platforms, platform::run_engines.run_if(is_playing), platform::follow_engines, platform::ride_platforms)
                        .chain(),
                    // Sammy's scans (doors, pickups, exit) only run while playing.
                    door::run_doors.run_if(is_playing),
                    player::move_player,
                    (
                        pickups::snap_pickups,
                        pickups::collect.run_if(is_playing),
                        pickups::bounce_keys,
                        pickups::shrink,
                        door::run_exits.run_if(is_playing),
                        health::collect_health.run_if(is_playing),
                        health::detect_deaths.run_if(is_playing),
                        platform::enter_teleports.run_if(is_playing),
                    ),
                    platform::run_teleports,
                    (
                        wasp::collect_ammo.run_if(is_playing),
                        wasp::fire,
                        wasp::spin_pickups,
                        wasp::fly_wasps,
                        wasp::update_weak_doors,
                        wasp::fly_planks,
                    )
                        .chain(),
                    (health::run_death, pickups::reset_keys, health::reset_checkpoints, health::animate_hearts, health::spin_pickups),
                    title::update_titles,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, decor::follow_sky.before(TransformSystems::Propagate));
    }
}

/// What the level spawner needs from an action's initialisation.
#[derive(Default)]
pub struct ActionInit {
    /// Render unlit, behind the world (sky domes).
    pub sky: bool,
    /// Script sets `my.invisible` immediately (config/marker entities).
    pub hidden: bool,
    /// Help sign: own fading material, drawn on top, screen-aligned.
    pub title: bool,
    /// Invisible logic entity that still runs (platform engines).
    pub hidden_but_alive: bool,
    /// Model skin (A4 1-based `my.skin`).
    pub skin: Option<u32>,
    /// Starting animation scene and speed (% of the scene per tick).
    pub anim: Option<(&'static str, f32)>,
}

fn is_playing(current: Res<crate::flow::CurrentLevel>) -> bool {
    crate::flow::playing(&current)
}

/// Map entities the player walks through (pickups, triggers).
pub fn is_passable(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    ["initrestart", "initbonusvie", "initeyeoftruth", "initsmallheart"].contains(&a.as_str())
}

/// Config and marker entities: their actions only read skills into globals
/// and hide the entity (mm1.wdl InitGame*, InitCamera*, ...).
fn is_marker(action: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "initgame", "initcamera", "setcamera", "initextraballes", "initnextraballes", "initpublic", "initinvisible",
        "initset", "initpath", "initprotection", "initlionskills", "initrangemdl",
    ];
    PREFIXES.iter().any(|p| action.starts_with(p))
}

/// Runs the synchronous part of an entity action (the code before its first
/// `wait`). `pov_z` is the Acknex Z of the initial camera/player position.
pub fn init(obj: &mut Object, pov_z: f32, entity: &mut EntityCommands) -> ActionInit {
    let action = obj.action.to_ascii_lowercase();
    // Camera config entities feed the follow camera before being hidden.
    match action.as_str() {
        "initgame" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<GameState>().apply_init_game(&o));
        }
        "initgame2" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<GameState>().apply_init_game2(&o);
                w.resource_mut::<CameraParams>().apply_init_game2(&o);
            });
        }
        "initcamera1" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<CameraParams>().apply_init_camera1(&o));
        }
        "initgame3" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<player::JumpParams>().apply_init_game3(&o));
        }
        "initcamera2" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<CameraParams>().apply_init_camera2(&o));
        }
        _ => {}
    }
    if is_marker(&action) {
        return ActionInit { hidden: true, ..default() };
    }
    match action.as_str() {
        "initskymenu" => decor::init_sky_menu(obj, pov_z, entity),
        "initmarche" => {
            player::init_player(obj, entity);
            health::init_start(obj.origin, obj.angles[0], entity);
            ActionInit::default()
        }
        "inittiledeath" => {
            health::init_tile_death(entity);
            ActionInit::default()
        }
        "initlava" => {
            health::init_lava(entity);
            ActionInit::default()
        }
        "initsmallheart" => {
            health::init_small_heart(obj, entity);
            ActionInit::default()
        }
        "initbonusvie" | "initbonuslife" => {
            health::init_bonus_vie(obj, entity);
            ActionInit::default()
        }
        "initengineplatform" => {
            platform::init_engine(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initplatform" => {
            platform::init_platform(obj, false, entity);
            ActionInit::default()
        }
        "initbodyplatform" => {
            platform::init_platform(obj, true, entity);
            ActionInit::default()
        }
        "initteleport" => {
            platform::init_teleport(obj, entity);
            ActionInit::default()
        }
        "initammo1" => {
            let (skin, anim) = wasp::init_ammo(obj, entity);
            ActionInit { skin: Some(skin), anim: Some(("stand", anim)), ..default() }
        }
        "initdoorweak" => {
            wasp::init_door_weak(obj, entity);
            ActionInit::default()
        }
        "initrestart" => {
            health::init_restart(obj, entity);
            ActionInit::default()
        }
        "initbonus3" => {
            pickups::init_bonus3(obj, entity);
            ActionInit::default()
        }
        "initkey" => {
            pickups::init_key(obj, entity);
            ActionInit::default()
        }
        "initdoor" => {
            door::init_door(obj, entity);
            ActionInit::default()
        }
        "initexit" => {
            door::init_exit(entity);
            ActionInit::default()
        }
        "inittitle" => {
            title::init_title(obj, entity);
            ActionInit { title: true, ..default() }
        }
        "initdecor" => {
            decor::init_decor(obj);
            ActionInit::default()
        }
        _ => ActionInit::default(),
    }
}
