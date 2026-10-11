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

/// Axis-aligned box of a model entity (A4 collides models by bounding box).
/// `center` and `half` are native coordinates; updated by whoever moves the entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct BoxCollider {
    pub center: [f32; 3],
    pub half: [f32; 3],
    pub solid: bool,
}

/// Extents a hull adds around a traced point (empirical: see formats::trace).
fn hull_extents(hull: usize) -> ([f32; 3], [f32; 3]) {
    match hull {
        0 => ([0.0; 3], [0.0; 3]),
        1 => ([16.0; 3], [16.0; 3]),
        _ => ([32.0, 32.0, 16.0], [32.0, 32.0, 32.0]), // (below/negative, above/positive)
    }
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
        let mut n = -d.signum();
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > t_in {
            t_in = t0;
            normal = [0.0; 3];
            if d > 0.0 {
                n = -1.0;
            } else {
                n = 1.0;
            }
            normal[i] = n;
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
    boxes: Query<'w, 's, (Entity, &'static BoxCollider)>,
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
        let (below, above) = hull_extents(hull);
        for (e, b) in &self.boxes {
            if !b.solid || Some(e) == ignore {
                continue;
            }
            let min = [0, 1, 2].map(|i| b.center[i] - b.half[i] - above[i]);
            let max = [0, 1, 2].map(|i| b.center[i] + b.half[i] + below[i]);
            if let Some((f, normal)) = segment_box(start, end, min, max)
                && f < best.trace.fraction
            {
                best = Hit {
                    trace: Trace {
                        fraction: f,
                        end: [0, 1, 2].map(|i| start[i] + (end[i] - start[i]) * f),
                        normal,
                        start_solid: false,
                        all_solid: false,
                    },
                    entity: Some(e),
                };
            }
        }
        best
    }
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
