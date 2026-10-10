//! Collision against the level BSP plus solid map entities (prefab WMBs such as
//! doors, exit tiles and platforms), each with its own clipping hulls.

use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use formats::trace::Trace;
use formats::wmb::Wmb;

use crate::actions::player::LevelCollision;

/// A map entity's BSP placed in the level (native origin, pan in degrees).
/// Prefabs are only ever panned in the shipped levels (scale 1, no tilt/roll).
#[derive(Component)]
pub struct Brush {
    pub wmb: Arc<Wmb>,
    pub origin: [f32; 3],
    pub pan: f32,
    /// Blocks movement (not `passable`).
    pub solid: bool,
}

impl Brush {
    fn to_local(&self, p: [f32; 3]) -> [f32; 3] {
        let (s, c) = (-self.pan).to_radians().sin_cos();
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        [d[0] * c - d[1] * s, d[0] * s + d[1] * c, d[2]]
    }

    fn dir_to_world(&self, n: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.pan.to_radians().sin_cos();
        [n[0] * c - n[1] * s, n[0] * s + n[1] * c, n[2]]
    }
}

/// Result of a world trace: the closest hit across the level and all solid brushes.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub trace: Trace,
    /// The brush entity that was hit, or None for the level itself / no hit.
    pub entity: Option<Entity>,
}

#[derive(SystemParam)]
pub struct CollisionWorld<'w, 's> {
    level: Option<Res<'w, LevelCollision>>,
    brushes: Query<'w, 's, (Entity, &'static Brush)>,
}

impl CollisionWorld<'_, '_> {
    pub fn ready(&self) -> bool {
        self.level.is_some()
    }

    /// Sweeps `hull` from `start` to `end`, optionally ignoring one brush entity.
    pub fn trace(&self, hull: usize, start: [f32; 3], end: [f32; 3], ignore: Option<Entity>) -> Hit {
        let mut best = Hit {
            trace: self.level.as_ref().map_or(
                Trace { fraction: 1.0, end, normal: [0.0; 3], start_solid: false, all_solid: false },
                |l| l.0.trace(0, hull, start, end),
            ),
            entity: None,
        };
        for (e, b) in &self.brushes {
            if !b.solid || Some(e) == ignore {
                continue;
            }
            let tr = b.wmb.trace(0, hull, b.to_local(start), b.to_local(end));
            if tr.start_solid && tr.all_solid {
                continue; // overlapping a moving brush: let the player escape
            }
            if tr.fraction < best.trace.fraction {
                let f = tr.fraction;
                best = Hit {
                    trace: Trace {
                        fraction: f,
                        end: [0, 1, 2].map(|i| start[i] + (end[i] - start[i]) * f),
                        normal: b.dir_to_world(tr.normal),
                        start_solid: tr.start_solid,
                        all_solid: false,
                    },
                    entity: Some(e),
                };
            }
        }
        best
    }
}
