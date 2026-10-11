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

/// State of one kind, by entity.
pub type Table<T> = BTreeMap<Id, T>;
/// Entities carrying a marker.
pub type Tag = BTreeSet<Id>;

/// Where an entity is and how it shows (native coordinates, degrees).
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub pos: [f32; 3],
    /// Pan, tilt, roll.
    pub angles: [f32; 3],
    /// Native axes (X forward, Y left, Z up).
    pub scale: [f32; 3],
    pub hidden: bool,
    /// Opacity in percent (100 = opaque).
    pub alpha: f32,
}

impl Body {
    pub fn new(name: impl Into<String>, pos: [f32; 3]) -> Self {
        Self { name: name.into(), pos, angles: [0.0; 3], scale: [1.0; 3], hidden: false, alpha: OPAQUE }
    }

    /// Uniform scale.
    pub fn scaled(mut self, s: f32) -> Self {
        self.scale = [s; 3];
        self
    }

    /// Scales all axes by `k`.
    pub fn shrink(&mut self, k: f32) {
        self.scale = self.scale.map(|s| s * k);
    }
}

/// Opacity of an opaque body (percent).
pub const OPAQUE: f32 = 100.0;

/// How a sprite is turned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SpriteFacing {
    /// Turns around the vertical axis toward the camera (A4 sprites).
    #[default]
    Billboard,
    /// Parallel to the screen (`facing`).
    Screen,
    /// Fixed in the world by its angles.
    Oriented,
}

/// What an entity is drawn with.
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// MDL model; `frame` fixes a vertex frame, otherwise the entity's [`Animator`] drives it.
    Model { file: String, skin: u32, overlay: bool, frame: Option<usize> },
    /// WMB prefab (map entity).
    Prefab { file: String },
    /// PCX sprite (`name+N.pcx` = N-frame strip, played by the entity's [`SpriteAnim`]).
    Sprite { file: String, facing: SpriteFacing },
}

/// An entity's visual and its render options. `rev` changes when the visual is swapped
/// (`morph`).
#[derive(Clone, Debug)]
pub struct Look {
    pub visual: Visual,
    pub rev: u32,
    /// Unlit, behind the world (sky domes).
    pub sky: bool,
    /// Drawn last, over the world (help signs).
    pub near: bool,
    /// Casts the engine's model shadow.
    pub shadow: bool,
    /// Own blended material (fading signs, translucent sprites).
    pub blend: bool,
}

impl Look {
    pub fn new(visual: Visual) -> Self {
        Self { visual, rev: 0, sky: false, near: false, shadow: false, blend: false }
    }

    /// Swaps the visual (WDL `morph`).
    pub fn morph(&mut self, visual: Visual) {
        self.visual = visual;
        self.rev += 1;
    }
}

/// A model's animation clock.
#[derive(Clone, Debug)]
pub struct Animator {
    pub model: Arc<Model>,
    pub anim: Anim,
}

impl Animator {
    pub fn new(model: Arc<Model>, frames: Vec<usize>, fps: f32) -> Self {
        let anim = Anim::new(model.scenes.clone(), frames, fps);
        Self { model, anim }
    }
}

/// A sprite strip's clock (frames per second; negative plays backwards).
#[derive(Clone, Copy, Debug)]
pub struct SpriteAnim {
    pub frames: u32,
    pub frame: f32,
    pub fps: f32,
}

/// A dynamic light on an entity: colour 0..255, range in quants (0 = off); a fading
/// light shrinks `rate` per tick and goes out at `until`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub color: [f32; 3],
    pub range: f32,
    pub fade: Option<(f32, f32)>,
}

impl Light {
    /// A flash: start range, shrink per tick, cut-off range.
    pub fn flash(color: [f32; 3], range: f32, rate: f32, until: f32) -> Self {
        Self { color, range, fade: Some((rate, until)) }
    }
}

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
    teleporting: Table<super::platforms::Teleporting>,
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
    platforms: Table<super::platforms::Linked>,
    teleports: Table<crate::platforms::Teleport>,

    // Doors.
    doors: Table<crate::doors::Door>,
    exits: Table<crate::doors::ExitTile>,

    // Pickups and gems.
    diamonds: Table<crate::pickups::Diamond>,
    /// Key gems by colour.
    key_gems: Table<u32>,
    level_keys: Table<super::pickups::LevelKey>,
    collectable: Tag,
    flying_gems: Table<crate::gems::FlyingGem>,
    gem_lifetimes: Table<crate::gems::GemLifetime>,
    gem_gravity: Table<crate::gems::GemGravity>,
    surprises: Table<crate::gems::Surprise>,
    decorative: Tag,
    boulders: Table<crate::gems::Boulder>,
    door_gems: Table<super::gems::DoorGemFlight>,
    hover_from_wed: Tag,
    respawnable: Tag,
    snap_to_ground: Tag,
    shrinking: Tag,
    collected: Tag,
    extra_height: Table<f32>,

    // Health.
    small_hearts: Table<super::health::Scaled<crate::health::SmallHeart>>,
    big_hearts: Table<crate::health::BigHeart>,
    boosts: Table<super::health::Scaled<crate::health::BoostPickup>>,
    checkpoints: Table<crate::health::Checkpoint>,
    death_tiles: Tag,
    lava_tiles: Tag,
    eyes: Table<super::health::Scaled<crate::eye::EyePickup>>,

    // Wasps.
    ammo_pickups: Table<crate::wasps::AmmoPickup>,
    /// Idle pickups spinning (their angles).
    spinning: Table<[f32; 3]>,
    wasps: Table<crate::wasps::Wasp>,
    weak_doors: Table<crate::wasps::WeakDoor>,
    planks: Table<crate::wasps::Plank>,
    hives: Table<crate::wasps::Hive>,
    hive_wasps: Table<crate::wasps::HiveWasp>,
    pickup_lifetimes: Table<f32>,

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
    menu_items: Table<super::menu::MenuItem>,
    menu_markers: Table<super::menu::MenuMarker>,
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
