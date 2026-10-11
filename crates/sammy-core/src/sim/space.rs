//! The level and its solid entities as one collision space.

use std::sync::Arc;

use formats::wmb::Wmb;

use super::World;
use super::world::Table;
use crate::collision::{BoxCollider, Brush, Hit, Hull};
use crate::id::Id;

/// The level and its solid entities, for traces.
pub(crate) struct Space<'a> {
    pub level: Option<&'a Wmb>,
    pub brushes: &'a Table<Brush>,
    pub boxes: &'a Table<BoxCollider>,
}

impl<'a> Space<'a> {
    pub fn new(level: &'a Option<Arc<Wmb>>, world: &'a World) -> Self {
        Self { level: level.as_deref(), brushes: &world.brushes, boxes: &world.boxes }
    }

    fn scene(&self) -> crate::collision::Scene<'a, impl Iterator<Item = (Id, &'a Brush)>, impl Iterator<Item = (Id, &'a BoxCollider)>> {
        crate::collision::Scene { level: self.level, brushes: self.brushes.iter().map(|(i, b)| (*i, b)), boxes: self.boxes.iter().map(|(i, b)| (*i, b)) }
    }
}

impl crate::collision::Space for Space<'_> {
    fn trace(&self, hull: Hull, start: [f32; 3], end: [f32; 3], ignore: Option<Id>) -> Hit<Id> {
        self.scene().trace(hull, start, end, ignore, true)
    }

    fn trace_static(&self, hull: Hull, start: [f32; 3], end: [f32; 3]) -> Hit<Id> {
        self.scene().trace(hull, start, end, None, false)
    }
}
