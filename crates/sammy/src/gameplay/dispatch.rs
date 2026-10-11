//! Level objects by WDL action name: each action's synchronous initialisation (the code
//! before its first `wait`) adds the components of its feature and says how the object
//! is drawn ([`ActionInit`]).

use bevy::prelude::*;
use formats::wmb::Object;

use super::*;
use crate::camera::CameraParams;

/// Stand-animation speeds: the boost pickup, and lions / wasp enemies (also the extro
/// wasp's walk).
const BOOST_ANIM_SPEED: f32 = 10.0;
const ENEMY_ANIM_SPEED: f32 = 3.0;
/// Menu level-grid markers (`ChoixMenu` view codes): tutorial, jump and think worlds, bonus.
const MARKER_TUTORIAL: u32 = 2;
const MARKER_JUMP: u32 = 3;
const MARKER_THINK: u32 = 4;
const MARKER_BONUS: u32 = 50;

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
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemies::LionGlobals>().apply_lion_skills1(&o));
        }
        "initlionskills2" => {
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemies::LionGlobals>().apply_lion_skills2(&o));
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
                w.resource_mut::<enemies::LionGlobals>().apply_game6(&o);
                w.resource_mut::<enemies::FriendCalls>().apply_game6(&o);
            });
        }
        "initgame7" => {
            let sk = obj.skills.clone();
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<enemies::LionGlobals>().apply_game7(&o);
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
            platforms::init_engine(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initplatform" => {
            platforms::init_platform(obj, false, entity);
            ActionInit::default()
        }
        "initbodyplatform" => {
            platforms::init_platform(obj, true, entity);
            ActionInit::default()
        }
        "initteleport" => {
            platforms::init_teleport(obj, entity);
            ActionInit::default()
        }
        "initboost" => {
            health::init_boost(obj, entity);
            ActionInit { anim: Some(("stand", BOOST_ANIM_SPEED)), ..default() }
        }
        "setcamera" => {
            crate::camera::init_zone(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initeyeoftruth" => {
            indicators::init_eye(obj, entity);
            ActionInit::default()
        }
        "initlion" => {
            let skin = enemies::init_lion(obj, entity);
            ActionInit { skin, anim: Some(("stand", ENEMY_ANIM_SPEED)), ..default() }
        }
        "initwaspennemy" => {
            let skin = enemies::init_wasp_ennemy(obj, entity, false);
            ActionInit { skin, anim: Some(("stand", ENEMY_ANIM_SPEED)), ..default() }
        }
        "startextro" => {
            let skin = enemies::init_wasp_ennemy(obj, entity, true);
            extro::start_extro(entity);
            ActionInit { skin, anim: Some(("walk", ENEMY_ANIM_SPEED)), ..default() }
        }
        "initsetpovsyn" | "initsetanimplayer" | "initsetfinextro" => {
            extro::init_trigger(&action, obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initmainmenu" => crate::ui::menu::init_item(obj, false, entity),
        "initmaintitle" => crate::ui::menu::init_item(obj, true, entity),
        "postutoriallevels" => crate::ui::menu::init_marker(obj, MARKER_TUTORIAL, entity),
        "posjumplevels" => crate::ui::menu::init_marker(obj, MARKER_JUMP, entity),
        "posthinklevels" => crate::ui::menu::init_marker(obj, MARKER_THINK, entity),
        "posbonuslevels" => crate::ui::menu::init_marker(obj, MARKER_BONUS, entity),
        "initplayerpatrol" => {
            crate::ui::intro::init_walker(obj, entity);
            ActionInit { anim: Some(("walkB", 0.0)), ..default() }
        }
        "initpathplayer" => {
            crate::ui::intro::init_node(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initstartennemy" => {
            enemies::init_start_ennemy(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initpath" => {
            enemies::init_path(obj, entity);
            ActionInit { hidden_but_alive: true, ..default() }
        }
        "initammo1" => {
            let (skin, anim) = wasps::init_ammo(obj, entity);
            ActionInit { skin: Some(skin), anim: Some(("stand", anim)), ..default() }
        }
        "initboulder" => {
            gems::init_boulder(obj, entity);
            ActionInit::default()
        }
        "inithive" => {
            wasps::init_hive(obj, entity);
            ActionInit::default()
        }
        "initdoorweak" => {
            wasps::init_door_weak(obj, entity);
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
            doors::init_door(obj, entity);
            ActionInit::default()
        }
        "initexit" => {
            doors::init_exit(entity);
            ActionInit::default()
        }
        "inittitle" => {
            titles::init_title(obj, entity);
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
