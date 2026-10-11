//! Mirrors the game's entities as Bevy entities: one per entity with a
//! [`Look`](sammy_core::sim::Look), spawned when it appears, rebuilt when its visual
//! is swapped, despawned when it is gone, and updated every frame (transform,
//! visibility, opacity, animation frame, light).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use sammy_core::id::Id;
use sammy_core::sim::{Look, SpriteFacing, Visual};

use super::assets::Loader;
use super::{Billboard, NEAR_LAYER, SKY_LAYER, ScreenAligned};
use crate::game::Game;
use crate::render::coords;
use crate::render::lights::DynamicLight;
use crate::render::model::LoadedModel;
use crate::render::shadow::CastsShadow;

mod spawn;
mod update;

pub(super) use self::{spawn::*, update::*};

/// Grey tint of a darkened (absent) level icon in the menu.
const DARK_TINT: f32 = 0.35;

/// Game entity → Bevy entity, with the visual revision it was built from.
#[derive(Resource, Default)]
pub struct Mirror(HashMap<Id, (Entity, u32)>);

impl Mirror {
    /// The Bevy entity showing a game entity.
    pub fn entity(&self, id: Id) -> Option<Entity> {
        self.0.get(&id).map(|(e, _)| *e)
    }
}

/// The game entity a Bevy entity shows.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mirrored(pub Id);

/// An animated model: its vertex frames, and the blend shown.
#[derive(Component)]
pub struct ModelView {
    model: Arc<LoadedModel>,
    shown: Option<(usize, usize, f32)>,
}

/// The material an entity was built with, and its own copy while it is translucent,
/// tinted or plays a sprite strip.
#[derive(Component)]
pub struct MaterialView {
    base: Handle<StandardMaterial>,
    own: Option<Handle<StandardMaterial>>,
    /// The base material's alpha mode (an opaque copy goes back to it).
    base_mode: AlphaMode,
    /// Always uses its own blended copy (fading signs).
    blend: bool,
    /// Sprite strip frames (UV offsets).
    frames: u32,
}
