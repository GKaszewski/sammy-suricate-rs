//! Ports of the WDL entity actions (see docs/gameplay.md). Each level object's
//! action name is dispatched here when the level is spawned.

mod decor;
pub mod player;

use bevy::prelude::*;
use formats::wmb::Object;

use crate::camera::CameraParams;


pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player::move_player)
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
}

/// Config and marker entities: their actions only read skills into globals
/// and hide the entity (mm1.wdl InitGame*, InitCamera*, ...).
fn is_marker(action: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "initgame", "initcamera", "initextraballes", "initnextraballes", "initpublic", "initinvisible",
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
        "initcamera1" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<CameraParams>().apply_init_camera1(&o));
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
        "initdecor" => {
            decor::init_decor(obj);
            ActionInit::default()
        }
        _ => ActionInit::default(),
    }
}
