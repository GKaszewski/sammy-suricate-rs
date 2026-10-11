//! Entity collision shapes: BSP brushes and boxes.

use super::*;

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
    pub(super) fn to_local(&self, p: [f32; 3]) -> [f32; 3] {
        let (s, c) = (-self.pan).to_radians().sin_cos();
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        [d[0] * c - d[1] * s, d[0] * s + d[1] * c, d[2]]
    }

    pub(super) fn dir_to_world(&self, n: [f32; 3]) -> [f32; 3] {
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
pub(super) fn segment_box(start: [f32; 3], end: [f32; 3], min: [f32; 3], max: [f32; 3]) -> Option<(f32, [f32; 3])> {
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
