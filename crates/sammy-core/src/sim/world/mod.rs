//! The simulation's entities: ids handed out in spawn order, and one table per kind of
//! state (a tiny, static ECS). Tables iterate by id, so update order is spawn order on
//! every frontend.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use formats::wmb::Object;

use crate::anim::Anim;
use crate::collision::{BoxCollider, Brush};
use crate::id::Id;
use crate::sim::assets::Model;

mod components;

pub use self::components::*;

/// State of one kind, by entity.
pub type Table<T> = BTreeMap<Id, T>;
/// Entities carrying a marker.
pub type Tag = BTreeSet<Id>;

macro_rules! world {
    ($( $(#[$doc:meta])* $field:ident : $ty:ty ),* $(,)?) => {
        /// Every entity's state, one table per kind.
        #[derive(Clone, Debug, Default)]
        pub struct World {
            next: u64,
            $( $(#[$doc])* pub $field: $ty, )*
        }

        impl World {
            /// Removes the entity from every table.
            pub fn despawn(&mut self, id: Id) {
                $( self.$field.remove(&id); )*
            }
        }
    };
}

world! {
    /// Position, orientation, scale, visibility and opacity of everything shown.
    bodies: Table<Body>,
    looks: Table<Look>,
    animators: Table<Animator>,
    sprite_anims: Table<SpriteAnim>,
    brushes: Table<Brush>,
    boxes: Table<BoxCollider>,
    /// The level-file record an entity was spawned from.
    objects: Table<Object>,
    lights: Table<Light>,

    // Sammy.
    players: Table<crate::player::Player>,
    teleporting: Table<crate::sim::platforms::Teleporting>,
    pending_shots: Table<crate::wasps::PendingShots>,
    dying: Table<crate::health::Dying>,
    /// Scripted animation (extro); `true` while the wave (`salut`) plays.
    scripted: Table<bool>,
    /// Height of Sammy's collision box centre above his origin.
    player_box: Table<f32>,
    /// The warning light is fading.
    warning_fade: Tag,

    // Platforms and teleports.
    engines: Table<crate::platforms::Engine>,
    platforms: Table<crate::sim::platforms::Linked>,
    teleports: Table<crate::platforms::Teleport>,

    // Doors.
    doors: Table<crate::doors::Door>,
    exits: Table<crate::doors::ExitTile>,

    // Pickups and gems.
    diamonds: Table<crate::pickups::Diamond>,
    /// Key gems by colour.
    key_gems: Table<u32>,
    level_keys: Table<crate::sim::pickups::LevelKey>,
    collectable: Tag,
    flying_gems: Table<crate::gems::FlyingGem>,
    gem_lifetimes: Table<crate::gems::GemLifetime>,
    gem_gravity: Table<crate::gems::GemGravity>,
    surprises: Table<crate::gems::Surprise>,
    decorative: Tag,
    boulders: Table<crate::gems::Boulder>,
    door_gems: Table<crate::sim::gems::DoorGemFlight>,
    hover_from_wed: Tag,
    respawnable: Tag,
    snap_to_ground: Tag,
    shrinking: Tag,
    collected: Tag,
    extra_height: Table<f32>,

    // Health.
    small_hearts: Table<crate::sim::health::Scaled<crate::health::SmallHeart>>,
    big_hearts: Table<crate::health::BigHeart>,
    boosts: Table<crate::sim::health::Scaled<crate::health::BoostPickup>>,
    checkpoints: Table<crate::health::Checkpoint>,
    death_tiles: Tag,
    lava_tiles: Tag,
    eyes: Table<crate::sim::health::Scaled<crate::eye::EyePickup>>,

    // Wasps.
    ammo_pickups: Table<crate::wasps::AmmoPickup>,
    /// Idle pickups spinning (their angles).
    spinning: Table<[f32; 3]>,
    wasps: Table<crate::wasps::Wasp>,
    weak_doors: Table<crate::wasps::WeakDoor>,
    planks: Table<crate::wasps::Plank>,
    hives: Table<crate::wasps::Hive>,
    hive_wasps: Table<crate::wasps::HiveWasp>,
    pickup_lifetimes: Table<crate::time::Ticks>,
    /// Old-type wasps flying off after a type switch.
    escapees: Table<crate::wasps::Escapee>,
    /// Wasps that hit a lion, recoiling.
    wasp_corpses: Table<crate::wasps::Corpse>,

    // Enemies.
    enemies: Table<crate::enemies::Enemy>,
    path_nodes: Table<crate::enemies::PathNode>,
    start_triggers: Table<crate::enemies::StartTrigger>,

    // Extro.
    /// `StartExtro`: ticks until the camera target becomes this wasp.
    start_extro_pov: Table<f32>,
    extro_triggers: Table<crate::extro::Trigger>,

    titles: Table<crate::titles::Title>,
    camera_zones: Table<crate::camera::CameraZone>,
    sky_follow: Table<crate::decor::SkyFollow>,
    /// Sprites to snap onto the floor below: (editor origin, height above the floor).
    sprite_ground: Table<([f32; 3], f32)>,

    // Menu and intro.
    menu_items: Table<crate::sim::menu::MenuItem>,
    menu_markers: Table<crate::sim::menu::MenuMarker>,
    walkers: Table<crate::intro::Walker>,
    intro_nodes: Table<crate::intro::PathNode>,
}

impl World {
    /// A new id (ids are never reused).
    pub fn spawn(&mut self) -> Id {
        self.next += 1;
        Id(self.next)
    }

    /// A new entity with a body.
    pub fn spawn_body(&mut self, body: Body) -> Id {
        let id = self.spawn();
        self.bodies.insert(id, body);
        id
    }

    /// Removes every entity (ids keep counting up).
    pub fn clear(&mut self) {
        *self = Self { next: self.next, ..Self::default() };
    }

    /// Sammy (the level's player entity).
    pub fn player(&self) -> Option<(Id, &crate::player::Player)> {
        self.players.iter().next().map(|(id, p)| (*id, p))
    }

    pub fn player_mut(&mut self) -> Option<(Id, &mut crate::player::Player)> {
        self.players.iter_mut().next().map(|(id, p)| (*id, p))
    }

    /// Where an entity is (its body).
    pub fn pos(&self, id: Id) -> Option<[f32; 3]> {
        self.bodies.get(&id).map(|b| b.pos)
    }
}
