//! Ports of the WDL entity actions (see docs/gameplay.md). Each level object's
//! action name is dispatched here when the level is spawned.

pub mod decor;
pub mod extro;
pub mod door;
pub mod enemy;
pub mod fx;
pub mod gems;
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
        app.init_resource::<GameState>()
            .init_resource::<crate::game::Rng>()
            .add_message::<crate::game::ResetScan>()
            .add_plugins((
                platform::PlatformsPlugin,
                door::DoorsPlugin,
                player::PlayerPlugin,
                pickups::PickupsPlugin,
                health::HealthPlugin,
                wasp::WaspsPlugin,
                enemy::EnemiesPlugin,
                extro::ExtroPlugin,
                fx::EffectsPlugin,
                title::TitlesPlugin,
            ))
            .add_systems(Update, decor::snap_sprites.in_set(crate::schedule::GameSet::Flow))
            .add_systems(PostUpdate, decor::follow_sky.before(TransformSystems::Propagate));
    }
}

pub use sammy_core::level::Presentation as ActionInit;
pub use sammy_core::level::{is_marker, is_passable};

/// Run condition: the level is being played (`PartieEncours == 1`).
pub fn is_playing(current: Res<crate::flow::CurrentLevel>) -> bool {
    crate::flow::playing(&current)
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
        "initlionskills1" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemy::LionGlobals>().apply_lion_skills1(&o));
        }
        "initlionskills2" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemy::LionGlobals>().apply_lion_skills2(&o));
        }
        "initgame5" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<player::PlayerTuning>().apply_init_game5(&o));
        }
        "initgame4" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<player::PlayerTuning>().apply_init_game4(&o));
        }
        "initgame6" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<enemy::LionGlobals>().apply_game6(&o);
                w.resource_mut::<enemy::FriendCalls>().apply_game6(&o);
            });
        }
        "initgame7" => {
            let sk = obj.skills.clone();
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<enemy::LionGlobals>().apply_game7(&o);
                w.resource_mut::<GameState>().apply_init_game7(&o);
                if sk[0] != 0.0 {
                    w.resource_mut::<player::JumpParams>().bounce_on_enemy = sk[0];
                }
            });
        }
        "initcamera3" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<CameraParams>().apply_init_camera3(&o));
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
        "initboost" => {
            health::init_boost(obj, entity);
            ActionInit { anim: Some(("stand", 10.0)), ..default() }
        }
        "setcamera" => {
            crate::camera::init_zone(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initeyeoftruth" => {
            fx::init_eye(obj, entity);
            ActionInit::default()
        }
        "initlion" => {
            let skin = enemy::init_lion(obj, entity);
            ActionInit { skin, anim: Some(("stand", 3.0)), ..default() }
        }
        "initwaspennemy" => {
            let skin = enemy::init_wasp_ennemy(obj, entity, false);
            ActionInit { skin, anim: Some(("stand", 3.0)), ..default() }
        }
        "startextro" => {
            let skin = enemy::init_wasp_ennemy(obj, entity, true);
            extro::start_extro(entity);
            ActionInit { skin, anim: Some(("walk", 3.0)), ..default() }
        }
        "initsetpovsyn" | "initsetanimplayer" | "initsetfinextro" => {
            extro::init_trigger(&action, obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initmainmenu" => crate::menu::init_item(obj, false, entity),
        "initmaintitle" => crate::menu::init_item(obj, true, entity),
        "postutoriallevels" => crate::menu::init_marker(obj, 2, entity),
        "posjumplevels" => crate::menu::init_marker(obj, 3, entity),
        "posthinklevels" => crate::menu::init_marker(obj, 4, entity),
        "posbonuslevels" => crate::menu::init_marker(obj, 50, entity),
        "initplayerpatrol" => {
            crate::intro::init_walker(obj, entity);
            ActionInit { anim: Some(("walkB", 0.0)), ..default() }
        }
        "initpathplayer" => {
            crate::intro::init_node(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initstartennemy" => {
            enemy::init_start_ennemy(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initpath" => {
            enemy::init_path(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initammo1" => {
            let (skin, anim) = wasp::init_ammo(obj, entity);
            ActionInit { skin: Some(skin), anim: Some(("stand", anim)), ..default() }
        }
        "initboulder" => {
            gems::init_boulder(obj, entity);
            ActionInit::default()
        }
        "inithive" => {
            wasp::init_hive(obj, entity);
            ActionInit::default()
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
        "initarbre" => decor::init_arbre(obj),
        "onground" => decor::on_ground(obj),
        "initmdl" => decor::init_mdl(obj),
        _ => ActionInit::default(),
    }
}
