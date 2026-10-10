//! Ports of the WDL entity actions (see docs/gameplay.md). Each level object's
//! action name is dispatched here when the level is spawned.

mod decor;
pub mod door;
pub mod pickups;
pub mod player;
pub mod title;

use bevy::prelude::*;
use formats::wmb::Object;

use crate::camera::CameraParams;
use crate::game::GameState;


pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<player::JumpParams>()
            .init_resource::<GameState>()
            .init_resource::<pickups::GemSprites>()
            .add_systems(
                Update,
                (
                    // Sammy's scans (doors, pickups, exit) only run while playing.
                    door::run_doors.run_if(is_playing),
                    player::move_player,
                    (
                        pickups::snap_pickups,
                        pickups::collect.run_if(is_playing),
                        pickups::bounce_keys,
                        pickups::shrink,
                        door::run_exits.run_if(is_playing),
                    ),
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
}

fn is_playing(current: Res<crate::flow::CurrentLevel>) -> bool {
    crate::flow::playing(&current)
}

/// Map entities the player walks through (pickups, triggers).
pub fn is_passable(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    ["initteleport", "initrestart", "initbonusvie", "initeyeoftruth", "initsmallheart"].contains(&a.as_str())
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
