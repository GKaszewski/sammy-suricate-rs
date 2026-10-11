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
        app.init_resource::<player::JumpParams>()
            .init_resource::<player::PlayerTuning>()
            .init_resource::<GameState>()
            .init_resource::<crate::game::Rng>()
            .init_resource::<pickups::GemSprites>()
            .add_message::<crate::game::ResetScan>()
            .add_message::<wasp::WaspHit>()
            .add_message::<player::PlayerTouched>()
            .init_resource::<enemy::LionGlobals>()
            .init_resource::<enemy::AiClock>()
            .init_resource::<enemy::FriendCalls>()
            .add_message::<enemy::WarningAttack>()
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
                        gems::fly_gems,
                        gems::gem_lifetimes,
                        gems::gem_gravity,
                        gems::surprise_gems,
                        door::run_door_gems,
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
                        enemy::home_wasps,
                        wasp::fly_wasps,
                        wasp::update_weak_doors,
                        wasp::fly_planks,
                        wasp::run_hives.run_if(is_playing),
                        gems::run_boulders.run_if(is_playing),
                        wasp::fly_hive_wasps,
                        wasp::expire_pickups,
                    )
                        .chain(),
                    (
                        enemy::add_player_collider,
                        enemy::sync_player_collider,
                        enemy::advance_clock,
                        enemy::tick_enemies,
                        enemy::bump_clock,
                        enemy::add_enemy_colliders,
                        enemy::run_start_triggers,
                        enemy::sync_enemy_colliders,
                        enemy::fade_enemies,
                        enemy::place_enemies,
                    )
                        .chain(),
                    (
                        enemy::reset_calls,
                        fx::warning_light,
                        fx::spawn_shadows,
                        fx::place_shadows,
                        fx::run_eye,
                        fx::spawn_rings,
                        fx::update_rings,
                        fx::run_call_rings,
                    )
                        .chain(),
                    (health::run_death, pickups::reset_keys, health::reset_checkpoints, health::animate_hearts, health::spin_pickups),
                    title::update_titles,
                )
                    .chain(),
            )
            .add_systems(Update, decor::snap_sprites)
            .init_resource::<extro::Pov>()
            .init_resource::<extro::Scanners>()
            .init_resource::<extro::TriggerClock>()
            .add_systems(PreUpdate, (extro::update_pov, extro::trigger_clock))
            .add_systems(Update, (extro::run_triggers.after(enemy::tick_enemies), extro::scripted_anims, extro::fade_out))
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
    /// Sprites: snap to the floor below at `floor + half height + offset` (`InitArbre`, `OnGround`).
    pub ground: Option<f32>,
    /// Sprites: non-zero angles are drawn oriented (fixed in the world) instead of camera-facing.
    pub oriented: bool,
    /// Sprites: `facing` (screen-parallel).
    pub facing: bool,
    /// Opacity in percent (`ent_alphaset`), below 100 = blended.
    pub alpha: Option<f32>,
    /// Models: fixed frame (A4 1-based, fractions floored), no animation.
    pub frame: Option<f32>,
    /// Models: `overlay` colour key (palette index 0 / black is transparent).
    pub overlay: bool,
    /// Models: solid bounding box collider.
    pub solid_box: bool,
}

fn is_playing(current: Res<crate::flow::CurrentLevel>) -> bool {
    crate::flow::playing(&current)
}

/// Map entities the player walks through (pickups, triggers).
pub fn is_passable(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    ["initrestart", "initbonusvie", "initeyeoftruth", "initsmallheart", "inithive"].contains(&a.as_str())
}

/// Config and marker entities: their actions only read skills into globals
/// and hide the entity (mm1.wdl InitGame*, InitCamera*, ...).
fn is_marker(action: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "initgame", "initcamera", "initextraballes", "initnextraballes", "initpublic", "initinvisible",
        "initset", "initprotection", "initlionskills", "initrangemdl",
    ];
    // The extro's `InitSetPovSyn` / `InitSetAnimPlayer` / `InitSetFinExtro` are live triggers.
    let extro_trigger = ["initsetpovsyn", "initsetanimplayer", "initsetfinextro"].contains(&action);
    PREFIXES.iter().any(|p| action.starts_with(p)) && !extro_trigger
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
            let sk = obj.skills.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemy::LionGlobals>().apply_lion_skills1(&sk));
        }
        "initlionskills2" => {
            let sk = obj.skills.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<enemy::LionGlobals>().apply_lion_skills2(&sk));
        }
        "initgame5" => {
            let sk = obj.skills.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<player::PlayerTuning>().apply_game5(&sk));
        }
        "initgame4" => {
            let sk = obj.skills.clone();
            entity.commands().queue(move |w: &mut World| w.resource_mut::<player::PlayerTuning>().apply_game4(&sk));
        }
        "initgame6" => {
            let sk = obj.skills.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<enemy::LionGlobals>().apply_game6(&sk);
                w.resource_mut::<enemy::FriendCalls>().apply_game6(&sk);
            });
        }
        "initgame7" => {
            let sk = obj.skills.clone();
            let o = obj.clone();
            entity.commands().queue(move |w: &mut World| {
                w.resource_mut::<enemy::LionGlobals>().apply_game7(&sk);
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
