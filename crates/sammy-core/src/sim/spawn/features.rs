//! Gameplay objects: each action's feature state, and how the object is shown.

use formats::wmb::Object;

use super::*;
use crate::level::is_marker;

impl Sim {
    /// The action's initialisation: config entities set globals, the others get their
    /// feature state; returns how the object is shown.
    pub(super) fn init(&mut self, id: Id, obj: &mut Object, pov_z: f32) -> Presentation {
        let action = obj.action.to_ascii_lowercase();
        self.apply_config(&action, obj);
        if is_marker(&action) {
            return Presentation { hidden: true, ..Default::default() };
        }
        let w = &mut self.world;
        let alive = Presentation { hidden_but_alive: true, ..Default::default() };
        match action.as_str() {
            "initskymenu" => {
                let (look, follow) = crate::decor::init_sky_menu(obj, pov_z);
                if let Some(f) = follow {
                    w.sky_follow.insert(id, f);
                }
                look
            }
            "initmarche" => {
                crate::sim::player::init(self, id, obj);
                self.game.start = Some((obj.origin, obj.angles[0]));
                Presentation::default()
            }
            "inittiledeath" => {
                w.death_tiles.insert(id);
                Presentation::default()
            }
            "initlava" => {
                w.lava_tiles.insert(id);
                Presentation::default()
            }
            "initsmallheart" => {
                crate::sim::health::init_small_heart(w, id, obj);
                Presentation::default()
            }
            "initbonusvie" | "initbonuslife" => {
                w.big_hearts.insert(id, crate::health::BigHeart::from_object(obj));
                Presentation::default()
            }
            "initengineplatform" => {
                w.engines.insert(id, crate::platforms::Engine::from_object(obj));
                alive
            }
            "initplatform" | "initbodyplatform" => {
                w.platforms.insert(id, crate::sim::platforms::Linked { platform: crate::platforms::Platform::from_object(obj, action == "initbodyplatform"), engine: None });
                Presentation::default()
            }
            "initteleport" => {
                w.teleports.insert(id, crate::platforms::Teleport::from_object(obj));
                Presentation::default()
            }
            "initboost" => {
                w.boosts.insert(id, crate::sim::health::Scaled { item: crate::health::BoostPickup::from_object(obj), scale: obj.scale.map(f32::abs) });
                Presentation { anim: Some(("stand", BOOST_ANIM_SPEED)), ..Default::default() }
            }
            "setcamera" => {
                w.camera_zones.insert(id, crate::camera::CameraZone::from_object(obj));
                alive
            }
            "initeyeoftruth" => {
                crate::sim::health::init_eye(w, id, obj);
                Presentation::default()
            }
            "initlion" => {
                let (e, skin) = crate::enemies::init_lion(obj);
                w.enemies.insert(id, e);
                Presentation { skin, anim: Some(("stand", ENEMY_ANIM_SPEED)), ..Default::default() }
            }
            "initwaspennemy" | "startextro" => {
                let extro = action == "startextro";
                let (e, skin, scale) = crate::enemies::init_wasp_ennemy(obj, extro);
                if let Some(k) = scale {
                    obj.scale = [k; 3];
                }
                w.enemies.insert(id, e);
                if extro {
                    w.start_extro_pov.insert(id, crate::extro::START_POV_DELAY);
                }
                Presentation { skin, anim: Some((if extro { "walk" } else { "stand" }, ENEMY_ANIM_SPEED)), ..Default::default() }
            }
            "initsetpovsyn" | "initsetanimplayer" | "initsetfinextro" => {
                let kind = match action.as_str() {
                    "initsetpovsyn" => crate::extro::TriggerKind::PovSyn,
                    "initsetanimplayer" => crate::extro::TriggerKind::AnimPlayer,
                    _ => crate::extro::TriggerKind::FinExtro,
                };
                w.extro_triggers.insert(id, crate::extro::Trigger::from_object(kind, obj));
                alive
            }
            "initmainmenu" | "initmaintitle" => crate::sim::menu::init_item(w, id, obj, action == "initmaintitle"),
            "postutoriallevels" => crate::sim::menu::init_marker(w, id, obj, MARKER_TUTORIAL),
            "posjumplevels" => crate::sim::menu::init_marker(w, id, obj, MARKER_JUMP),
            "posthinklevels" => crate::sim::menu::init_marker(w, id, obj, MARKER_THINK),
            "posbonuslevels" => crate::sim::menu::init_marker(w, id, obj, MARKER_BONUS),
            "initplayerpatrol" => {
                w.walkers.insert(id, crate::intro::Walker::from_object(obj));
                Presentation { anim: Some(("walkB", 0.0)), ..Default::default() }
            }
            "initpathplayer" => {
                w.intro_nodes.insert(id, crate::intro::PathNode::from_object(obj));
                alive
            }
            "initstartennemy" => {
                w.start_triggers.insert(id, crate::enemies::StartTrigger::from_object(obj));
                alive
            }
            "initpath" => {
                w.path_nodes.insert(id, crate::enemies::PathNode::from_object(obj));
                alive
            }
            "initammo1" => crate::sim::wasps::init_ammo(w, id, obj),
            "initboulder" => {
                w.boulders.insert(id, crate::gems::Boulder::from_object(obj));
                Presentation::default()
            }
            "inithive" => {
                w.hives.insert(id, crate::wasps::Hive::from_object(obj));
                Presentation::default()
            }
            "initdoorweak" => {
                w.weak_doors.insert(id, crate::wasps::WeakDoor::from_object(obj));
                Presentation::default()
            }
            "initrestart" => {
                w.checkpoints.insert(id, crate::health::Checkpoint::from_object(obj));
                w.respawnable.insert(id);
                Presentation::default()
            }
            "initbonus3" => {
                let d = crate::pickups::Diamond::from_object(obj);
                if d.snap {
                    w.snap_to_ground.insert(id);
                }
                w.diamonds.insert(id, d);
                Presentation::default()
            }
            "initkey" => {
                crate::sim::pickups::init_key(w, id, obj);
                Presentation::default()
            }
            "initdoor" => {
                w.doors.insert(id, crate::doors::Door::from_object(obj));
                Presentation::default()
            }
            "initexit" => {
                w.exits.insert(id, crate::doors::ExitTile::default());
                Presentation::default()
            }
            "inittitle" => {
                let title = crate::titles::Title::from_object(obj);
                // The sign is drawn where its trigger centre is.
                obj.origin = title.center;
                w.titles.insert(id, title);
                let mut b = Body::new("", obj.origin);
                b.hidden = true;
                w.bodies.insert(id, b);
                Presentation { title: true, ..Default::default() }
            }
            "initdecor" => {
                crate::decor::init_decor(obj);
                Presentation::default()
            }
            "initarbre" => crate::decor::init_arbre(obj),
            "onground" => crate::decor::on_ground(obj),
            "initmdl" => crate::decor::init_mdl(obj),
            _ => Presentation::default(),
        }
    }
}
