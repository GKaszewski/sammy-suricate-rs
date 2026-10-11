//! Tracing through the level and its entities.

use super::*;

/// Result of a world trace: the closest hit across the level and all solid entities.
#[derive(Clone, Copy, Debug)]
pub struct Hit<Id> {
    pub trace: Trace,
    /// The entity that was hit, or None for the level itself / no hit.
    pub entity: Option<Id>,
}

/// What a trace collides with.
pub struct Scene<'a, B, X> {
    pub level: Option<&'a Wmb>,
    pub brushes: B,
    pub boxes: X,
}

impl<'a, Id, B, X> Scene<'a, B, X>
where
    Id: Copy + PartialEq,
    B: Iterator<Item = (Id, &'a Brush)>,
    X: Iterator<Item = (Id, &'a BoxCollider)>,
{
    /// Sweeps `hull` from `start` to `end` against the level, the solid brushes and
    /// (with `models`) the solid boxes, skipping `ignore`.
    pub fn trace(self, hull: Hull, start: [f32; 3], end: [f32; 3], ignore: Option<Id>, models: bool) -> Hit<Id> {
        let mut best = Hit {
            trace: self
                .level
                .map_or(Trace { fraction: 1.0, end, normal: [0.0; 3], start_solid: false, all_solid: false }, |l| l.trace(0, hull.index(), start, end)),
            entity: None,
        };
        for (e, b) in self.brushes {
            if !b.solid || Some(e) == ignore {
                continue;
            }
            let tr = b.wmb.trace(0, hull.index(), b.to_local(start), b.to_local(end));
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
        if !models {
            return best;
        }
        let (below, above) = hull.extents();
        for (e, b) in self.boxes {
            if !b.solid || Some(e) == ignore {
                continue;
            }
            let min = [0, 1, 2].map(|i| b.center[i] - b.half[i] - above[i]);
            let max = [0, 1, 2].map(|i| b.center[i] + b.half[i] + below[i]);
            if let Some((f, normal)) = segment_box(start, end, min, max)
                && f < best.trace.fraction
            {
                best = Hit {
                    trace: Trace { fraction: f, end: [0, 1, 2].map(|i| start[i] + (end[i] - start[i]) * f), normal, start_solid: false, all_solid: false },
                    entity: Some(e),
                };
            }
        }
        best
    }
}

/// Something to sweep hulls through: the level plus its solid entities.
pub trait Space {
    /// Sweeps `hull` from `start` to `end` against everything solid, skipping `ignore`.
    fn trace(&self, hull: Hull, start: [f32; 3], end: [f32; 3], ignore: Option<crate::id::Id>) -> Hit<crate::id::Id>;

    /// Like [`Space::trace`] but against level geometry and map entities only (WDL
    /// `IGNORE_MODELS`).
    fn trace_static(&self, hull: Hull, start: [f32; 3], end: [f32; 3]) -> Hit<crate::id::Id>;
}
