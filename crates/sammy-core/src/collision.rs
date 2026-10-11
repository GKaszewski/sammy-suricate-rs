//! Collision against the level BSP plus solid map entities (prefab WMBs such as
//! doors, exit tiles and platforms), each with its own clipping hulls, and model
//! bounding boxes. Entity ids are the caller's (`Id`), so any ECS or plain index works.

use std::sync::Arc;

use formats::trace::Trace;
use formats::wmb::Wmb;

/// Clipping hull of a traced object (WMB hulls 0-2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hull {
    /// A point (sprites, particles, line of sight).
    Point,
    /// ±16 cube (`narrow`).
    Narrow,
    /// 64×64 footprint, 48 high (`fat`: Sammy and the enemies).
    Fat,
}

impl Hull {
    /// The WMB clipping hull index.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Extents the hull adds around a traced point, (below/negative, above/positive)
    /// (empirical: see `formats::trace`).
    fn extents(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Hull::Point => ([0.0; 3], [0.0; 3]),
            Hull::Narrow => ([16.0; 3], [16.0; 3]),
            Hull::Fat => ([32.0, 32.0, 16.0], [32.0, 32.0, 32.0]),
        }
    }
}

/// A map entity's BSP placed in the level (native origin, pan in degrees).
/// Prefabs are only ever panned in the shipped levels (scale 1, no tilt/roll).
#[derive(Clone, Debug)]
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

/// Axis-aligned box of a model entity (A4 collides models by bounding box).
/// `center` and `half` are native coordinates; updated by whoever moves the entity.
#[derive(Clone, Copy, Debug)]
pub struct BoxCollider {
    pub center: [f32; 3],
    pub half: [f32; 3],
    pub solid: bool,
}

/// Slab test of the segment start→end against an AABB. Returns (fraction, normal)
/// of the entry point, or None if missed or starting inside.
fn segment_box(start: [f32; 3], end: [f32; 3], min: [f32; 3], max: [f32; 3]) -> Option<(f32, [f32; 3])> {
    let mut t_in = 0.0f32;
    let mut t_out = 1.0f32;
    let mut normal = [0.0; 3];
    let mut inside = true;
    for i in 0..3 {
        let d = end[i] - start[i];
        if start[i] < min[i] || start[i] > max[i] {
            inside = false;
        }
        if d.abs() < 1e-6 {
            if start[i] < min[i] || start[i] > max[i] {
                return None;
            }
            continue;
        }
        let (mut t0, mut t1) = ((min[i] - start[i]) / d, (max[i] - start[i]) / d);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > t_in {
            t_in = t0;
            normal = [0.0; 3];
            normal[i] = -d.signum(); // entering face points against the motion
        }
        t_out = t_out.min(t1);
        if t_in > t_out {
            return None;
        }
    }
    if inside {
        return None; // let things escape overlaps
    }
    (t_in < 1.0).then_some(((t_in - 0.001).max(0.0), normal))
}

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
}

/// The floor sonar (`playerClimb`): from 8 above `p` straight down with the narrow hull,
/// whose contact is 16 above the real floor. Returns (floor height, the entity it
/// belongs to, its normal), or None over the void.
pub fn floor_below(space: &impl Space, p: [f32; 3]) -> Option<(f32, Option<crate::id::Id>, [f32; 3])> {
    const SONAR_START: f32 = 8.0;
    const SONAR_DEPTH: f32 = 2000.0;
    const NARROW_HALF: f32 = 16.0;
    let hit = space.trace(Hull::Narrow, [p[0], p[1], p[2] + SONAR_START], [p[0], p[1], p[2] - SONAR_DEPTH], None);
    (hit.trace.hit() && !hit.trace.start_solid).then(|| (hit.trace.end[2] - NARROW_HALF, hit.entity, hit.trace.normal))
}

/// Moves `hull` along `delta`, sliding along walls (approximates A4's `MOVE` glide; up to
/// four bounces). Returns the new position and the last entity bumped into, with the
/// contact normal.
pub fn slide_move(space: &impl Space, hull: Hull, mut pos: [f32; 3], mut delta: [f32; 3], ignore: Option<crate::id::Id>) -> ([f32; 3], Option<(crate::id::Id, [f32; 3])>) {
    let mut touched = None;
    for _ in 0..4 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]];
        let hit = space.trace(hull, pos, end, ignore);
        let tr = hit.trace;
        if tr.start_solid {
            return (pos, touched);
        }
        if let Some(e) = hit.entity
            && tr.hit()
        {
            touched = Some((e, tr.normal));
        }
        pos = tr.end;
        if !tr.hit() {
            break;
        }
        // Remove the part of the remaining motion that goes into the wall.
        let rest = delta.map(|d| d * (1.0 - tr.fraction));
        let into = rest[0] * tr.normal[0] + rest[1] * tr.normal[1] + rest[2] * tr.normal[2];
        delta = [0, 1, 2].map(|i| rest[i] - tr.normal[i] * into);
    }
    (pos, touched)
}

#[cfg(test)]
mod tests {
    use super::segment_box;

    #[test]
    fn segment_hits_box_face() {
        let (f, n) = segment_box([-10.0, 0.0, 0.0], [10.0, 0.0, 0.0], [-1.0; 3], [1.0; 3]).unwrap();
        assert!((f - 0.45).abs() < 0.01, "{f}");
        assert_eq!(n, [-1.0, 0.0, 0.0]);
        let (f, n) = segment_box([0.0, 0.0, 10.0], [0.0, 0.0, -10.0], [-1.0; 3], [1.0; 3]).unwrap();
        assert!((f - 0.45).abs() < 0.01, "{f}");
        assert_eq!(n, [0.0, 0.0, 1.0]);
        assert!(segment_box([-10.0, 5.0, 0.0], [10.0, 5.0, 0.0], [-1.0; 3], [1.0; 3]).is_none());
        assert!(segment_box([0.0; 3], [10.0, 0.0, 0.0], [-1.0; 3], [1.0; 3]).is_none(), "inside start escapes");
    }
}
