//! Level loading: the WMB's BSP and its objects, each initialised by its action
//! (the synchronous part of the WDL action, before its first `wait`).

use formats::wmb::{Entity as WmbEntity, Object, entity_flags};

use super::world::{Animator, Body, Look, SpriteAnim, SpriteFacing, Visual};
use super::{Files, Sim};
use crate::collision::{BoxCollider, Brush, Hull};
use crate::id::Id;
use crate::level::{Presentation, is_marker, is_passable};

/// Frame rate a model animator starts with (the A4 default); speeds are then set in
/// percent with `set_speed_pct`.
pub const MODEL_FPS: f32 = 8.0;
/// Animation a model plays without an `anim` from its init: `("stand", speed %)`.
const DEFAULT_ANIM: (&str, f32) = ("stand", 2.0);
/// Stand-animation speeds: the boost pickup, and lions / wasp enemies (also the extro
/// wasp's walk).
const BOOST_ANIM_SPEED: f32 = 10.0;
const ENEMY_ANIM_SPEED: f32 = 3.0;
/// Knuth's multiplicative hash and shift: the stand-in for WDL `random(frames)`.
const START_FRAME_HASH: u32 = 2654435761;
const START_FRAME_SHIFT: u32 = 7;
/// `InitBonus3` sprite animation speed (frames per second).
const BONUS3_FPS: f32 = 48.0;
/// Menu level-grid markers (`ChoixMenu` view codes): tutorial, jump and think worlds, bonus.
const MARKER_TUTORIAL: u32 = 2;
const MARKER_JUMP: u32 = 3;
const MARKER_THINK: u32 = 4;
const MARKER_BONUS: u32 = 50;
/// Floor probe depth for sprites snapped to the ground.
const GROUND_PROBE: f32 = 2000.0;

/// Files gameplay may need at runtime (morphs, projectiles, debris).
const PRELOAD_MODELS: &[&str] = &["wasp1.mdl", "plank1.mdl", "plank2.mdl", "plank3.mdl"];
const PRELOAD_PREFABS: &[&str] = &["doorwk1.wmb", "doorwk2.wmb", "doorwk3.wmb", "doorwk4.wmb", "doordark.wmb", "platfor1.wmb"];

impl Sim {
    /// Parses the level, replaces the world with its objects. Returns false if the file
    /// could not be read.
    pub(super) fn spawn_level(&mut self, name: &str, files: &dyn Files) -> bool {
        let Some(wmb) = files.read(name).and_then(|d| formats::wmb::parse(&d).ok()) else { return false };
        let wmb = std::sync::Arc::new(wmb);
        self.level = Some(wmb.clone());
        for m in PRELOAD_MODELS {
            self.assets.load_model(files, m);
        }
        for p in PRELOAD_PREFABS {
            self.assets.load_prefab(files, p);
        }
        let pov_z = wmb
            .entities
            .iter()
            .find_map(|e| match e {
                WmbEntity::Object(o) if o.action.eq_ignore_ascii_case("initmarche") => Some(o.origin[2]),
                _ => None,
            })
            .unwrap_or(0.0);
        for ent in &wmb.entities {
            if let WmbEntity::Object(obj) = ent {
                self.spawn_object(obj.clone(), pov_z, files);
            }
        }
        true
    }

    fn spawn_object(&mut self, mut obj: Object, pov_z: f32, files: &dyn Files) {
        if obj.flags & entity_flags::INVISIBLE != 0 || obj.file.is_empty() {
            return;
        }
        if self.debug.skip_actions.iter().any(|a| a.eq_ignore_ascii_case(&obj.action)) {
            return;
        }
        let file = obj.file.to_lowercase();
        // A4 drops a map-entity prefab onto its origin: the bounding-box bottom sits at
        // the entity z (menu titles only line up with their diamonds this way) [U].
        if file.ends_with(".wmb")
            && let Some(wmb) = self.assets.load_prefab(files, &file)
        {
            let min_z = wmb.vertices.iter().map(|v| v[2]).fold(f32::MAX, f32::min);
            if min_z < 0.0 {
                obj.origin[2] -= min_z * obj.scale[2].abs();
            }
        }
        let id = self.world.spawn();
        let init = self.init(id, &mut obj, pov_z);
        if init.hidden {
            self.world.despawn(id);
            return;
        }
        self.world.objects.insert(id, obj.clone());
        if init.hidden_but_alive {
            return; // logic only: nothing to show
        }
        let body = Body { name: obj.name.clone(), pos: obj.origin, angles: obj.angles, scale: obj.scale.map(f32::abs), hidden: false, alpha: super::world::OPAQUE };
        // Some inits set their own body (titles start hidden).
        let body = match self.world.bodies.remove(&id) {
            Some(b) => Body { hidden: b.hidden, ..body },
            None => body,
        };
        if file.ends_with(".mdl") {
            let Some(model) = self.assets.load_model(files, &file) else { return self.world.despawn(id) };
            let skin = init.skin.unwrap_or(1);
            // Scenery without an action (or `InitMdl`) keeps a fixed frame; A4 frames are 1-based.
            let frame = init.frame.or(obj.action.is_empty().then_some(1.0));
            let mut look = Look::new(Visual::Model { file: file.clone(), skin, overlay: init.overlay, frame: None });
            look.sky = init.sky;
            if let Some(f) = frame {
                let index = (f.floor().max(1.0) as usize - 1).min(model.mdl.frames.len().saturating_sub(1));
                look.visual = Visual::Model { file, skin, overlay: init.overlay, frame: Some(index) };
                look.shadow = obj.flags & entity_flags::SHADOW != 0;
                if init.solid_box && let Some(fr) = model.mdl.frames.get(index) {
                    let sc = obj.scale.map(f32::abs);
                    let center = std::array::from_fn(|i| obj.origin[i] + (fr.bbox_min[i] + fr.bbox_max[i]) * 0.5 * sc[i]);
                    let half = std::array::from_fn(|i| (fr.bbox_max[i] - fr.bbox_min[i]) * 0.5 * sc[i]);
                    self.world.boxes.insert(id, BoxCollider { center, half, solid: true });
                }
            } else {
                let (scene, pct) = init.anim.unwrap_or(DEFAULT_ANIM);
                let mut a = Animator::new(model, Vec::new(), MODEL_FPS);
                a.anim.play(scene, true, 0.0);
                a.anim.set_speed_pct(pct, crate::time::TICKS_PER_SEC);
                self.world.animators.insert(id, a);
            }
            // Sammy and the intro's Sammy cast the engine model shadow.
            if self.world.walkers.contains_key(&id) || self.world.players.contains_key(&id) {
                look.shadow = true;
            }
            self.world.bodies.insert(id, body);
            self.world.looks.insert(id, look);
        } else if file.ends_with(".wmb") {
            let Some(wmb) = self.assets.load_prefab(files, &file) else { return self.world.despawn(id) };
            // Map entities are solid unless passable (A4 collides with their BSP hulls).
            let solid = obj.flags & entity_flags::PASSABLE == 0 && !is_passable(&obj.action);
            self.world.brushes.insert(id, Brush { wmb, origin: obj.origin, pan: obj.angles[0], solid });
            self.world.bodies.insert(id, body);
            self.world.looks.insert(id, Look::new(Visual::Prefab { file }));
        } else if file.ends_with(".pcx") {
            let Some([_, height]) = self.assets.load_sprite(files, &file) else { return self.world.despawn(id) };
            let frames = super::assets::sprite_frames(&file);
            let mut look = Look::new(Visual::Sprite {
                file: file.clone(),
                facing: if init.title || init.facing {
                    SpriteFacing::Screen
                } else if init.oriented {
                    SpriteFacing::Oriented
                } else {
                    SpriteFacing::Billboard
                },
            });
            let mut body = body;
            if init.title {
                look.blend = true;
                look.near = true;
            } else if let Some(a) = init.alpha.filter(|a| *a < super::world::OPAQUE) {
                look.blend = true;
                body.alpha = a;
            } else if frames > 1 {
                // Deterministic stand-in for WDL `random(frames)` start frame.
                let start = ((id.0 as u32).wrapping_mul(START_FRAME_HASH) >> START_FRAME_SHIFT) % frames;
                let fps = if obj.action.eq_ignore_ascii_case("initbonus3") { BONUS3_FPS } else { 0.0 };
                self.world.sprite_anims.insert(id, SpriteAnim { frames, frame: start as f32, fps });
            }
            if let Some(dz) = init.ground {
                let half_h = height as f32 * 0.5 * obj.scale[2].abs();
                self.world.sprite_ground.insert(id, (obj.origin, half_h + dz));
            }
            self.world.bodies.insert(id, body);
            self.world.looks.insert(id, look);
        } else {
            self.world.despawn(id);
        }
    }

    /// The action's initialisation: config entities set globals, the others get their
    /// feature state; returns how the object is shown.
    fn init(&mut self, id: Id, obj: &mut Object, pov_z: f32) -> Presentation {
        let action = obj.action.to_ascii_lowercase();
        match action.as_str() {
            "initgame" => self.game.apply_init_game(obj),
            "initgame2" => {
                self.game.apply_init_game2(obj);
                self.camera.params.apply_init_game2(obj);
            }
            "initcamera1" => self.camera.params.apply_init_camera1(obj),
            "initgame3" => self.jump.apply_init_game3(obj),
            "initlionskills1" => self.lions.apply_lion_skills1(obj),
            "initlionskills2" => self.lions.apply_lion_skills2(obj),
            "initgame5" => self.tuning.apply_init_game5(obj),
            "initgame4" => self.tuning.apply_init_game4(obj),
            "initgame6" => {
                self.lions.apply_game6(obj);
                self.calls.apply_game6(obj);
            }
            "initgame7" => {
                self.lions.apply_game7(obj);
                self.game.apply_init_game7(obj);
                if obj.skills[0] != 0.0 {
                    self.jump.bounce_on_enemy = obj.skills[0];
                }
            }
            "initcamera3" => self.camera.params.apply_init_camera3(obj),
            "initcamera2" => self.camera.params.apply_init_camera2(obj),
            _ => {}
        }
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
                super::player::init(self, id, obj);
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
                super::health::init_small_heart(w, id, obj);
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
                w.platforms.insert(id, super::platforms::Linked { platform: crate::platforms::Platform::from_object(obj, action == "initbodyplatform"), engine: None });
                Presentation::default()
            }
            "initteleport" => {
                w.teleports.insert(id, crate::platforms::Teleport::from_object(obj));
                Presentation::default()
            }
            "initboost" => {
                w.boosts.insert(id, super::health::Scaled { item: crate::health::BoostPickup::from_object(obj), scale: obj.scale.map(f32::abs) });
                Presentation { anim: Some(("stand", BOOST_ANIM_SPEED)), ..Default::default() }
            }
            "setcamera" => {
                w.camera_zones.insert(id, crate::camera::CameraZone::from_object(obj));
                alive
            }
            "initeyeoftruth" => {
                super::health::init_eye(w, id, obj);
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
            "initmainmenu" | "initmaintitle" => super::menu::init_item(w, id, obj, action == "initmaintitle"),
            "postutoriallevels" => super::menu::init_marker(w, id, obj, MARKER_TUTORIAL),
            "posjumplevels" => super::menu::init_marker(w, id, obj, MARKER_JUMP),
            "posthinklevels" => super::menu::init_marker(w, id, obj, MARKER_THINK),
            "posbonuslevels" => super::menu::init_marker(w, id, obj, MARKER_BONUS),
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
            "initammo1" => {
                let (skin, anim) = super::wasps::init_ammo(w, id, obj);
                Presentation { skin: Some(skin), anim: Some(("stand", anim)), ..Default::default() }
            }
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
                super::pickups::init_key(w, id, obj);
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

/// Sprites snapped onto the floor below their editor origin (once the level is there).
pub(super) fn snap_sprites(sim: &mut Sim) {
    let Some(level) = sim.level.clone() else { return };
    let space = super::Space { level: Some(&level), brushes: &sim.world.brushes, boxes: &sim.world.boxes };
    let mut done = Vec::new();
    for (id, (origin, dz)) in &sim.world.sprite_ground {
        let [x, y, z] = *origin;
        let tr = crate::collision::Space::trace_static(&space, Hull::Point, [x, y, z], [x, y, z - GROUND_PROBE]).trace;
        done.push((*id, (tr.hit() && !tr.start_solid).then(|| [x, y, tr.end[2] + dz])));
    }
    for (id, pos) in done {
        sim.world.sprite_ground.remove(&id);
        if let (Some(p), Some(b)) = (pos, sim.world.bodies.get_mut(&id)) {
            b.pos = p;
        }
    }
}

/// A runtime gem sprite of `color` at `pos`.
pub(super) fn gem_sprite(sim: &mut Sim, name: &str, color: u32, pos: [f32; 3], scale: f32, fps: f32) -> Option<Id> {
    let file = crate::keys::gem_sprite(color)?;
    let frames = super::assets::sprite_frames(file);
    let id = sim.world.spawn_body(Body::new(name, pos).scaled(scale));
    sim.world.looks.insert(id, Look::new(Visual::Sprite { file: file.to_owned(), facing: SpriteFacing::Billboard }));
    sim.world.sprite_anims.insert(id, SpriteAnim { frames, frame: 0.0, fps });
    Some(id)
}
