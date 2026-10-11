//! Bevy access to [`sammy_core::collision`]: brushes and boxes are components, the
//! level BSP a resource.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::actions::player::LevelCollision;

pub use sammy_core::collision::Hull;

/// A solid map entity ([`sammy_core::collision::Brush`]).
#[derive(Component, Deref, DerefMut)]
pub struct Brush(pub sammy_core::collision::Brush);

/// A model's bounding box ([`sammy_core::collision::BoxCollider`]).
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut)]
pub struct BoxCollider(pub sammy_core::collision::BoxCollider);

pub type Hit = sammy_core::collision::Hit<Entity>;

#[derive(SystemParam)]
pub struct CollisionWorld<'w, 's> {
    level: Option<Res<'w, LevelCollision>>,
    brushes: Query<'w, 's, (Entity, &'static Brush)>,
    boxes: Query<'w, 's, (Entity, &'static BoxCollider)>,
}

impl CollisionWorld<'_, '_> {
    pub fn ready(&self) -> bool {
        self.level.is_some()
    }

    /// Trace against level geometry and map entities only (WDL `IGNORE_MODELS`).
    pub fn trace_static(&self, hull: Hull, start: [f32; 3], end: [f32; 3]) -> Hit {
        self.scene().trace(hull, start, end, None, false)
    }

    /// Sweeps `hull` from `start` to `end`, optionally ignoring one entity.
    pub fn trace(&self, hull: Hull, start: [f32; 3], end: [f32; 3], ignore: Option<Entity>) -> Hit {
        self.scene().trace(hull, start, end, ignore, true)
    }

    fn scene(
        &self,
    ) -> sammy_core::collision::Scene<'_, impl Iterator<Item = (Entity, &sammy_core::collision::Brush)>, impl Iterator<Item = (Entity, &sammy_core::collision::BoxCollider)>>
    {
        sammy_core::collision::Scene {
            level: self.level.as_ref().map(|l| &*l.0),
            brushes: self.brushes.iter().map(|(e, b)| (e, &b.0)),
            boxes: self.boxes.iter().map(|(e, b)| (e, &b.0)),
        }
    }
}
