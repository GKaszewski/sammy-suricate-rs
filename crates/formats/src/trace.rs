//! Swept-box collision against a WMB clipping hull (Quake `SV_RecursiveHullCheck`).
//!
//! Hull 1 is a ±16 cube, hull 2 the "fat" 64×64 box (header z −32…+16, but floors
//! stop the origin 16 above them in practice). Coordinates are native Acknex quants.

use crate::wmb::{contents, Wmb};

/// Nudge used to keep the end point off the plane (same as Quake).
const DIST_EPSILON: f32 = 0.03125;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trace {
    /// 0..1 portion of the move completed.
    pub fraction: f32,
    pub end: [f32; 3],
    /// Normal of the plane hit (zero if nothing was hit).
    pub normal: [f32; 3],
    pub start_solid: bool,
    pub all_solid: bool,
}

impl Trace {
    pub fn hit(&self) -> bool {
        self.fraction < 1.0
    }
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl Wmb {
    /// Sweeps a point through `hull` (1..=3) of `model` from `start` to `end`.
    pub fn trace(&self, model: usize, hull: usize, start: [f32; 3], end: [f32; 3]) -> Trace {
        let mut tr = Trace { fraction: 1.0, end, normal: [0.0; 3], start_solid: false, all_solid: true };
        let Some(m) = self.models.get(model) else { return tr };
        let head = m.head_nodes[hull.clamp(1, 3)];
        self.hull_check(head, 0.0, 1.0, start, end, &mut tr);
        if tr.all_solid {
            tr.start_solid = true;
        }
        if tr.fraction == 1.0 {
            tr.end = end;
        }
        tr
    }

    fn leaf_contents(&self, mut num: i32, p: [f32; 3]) -> i32 {
        while num >= 0 {
            let Some(c) = self.clipnodes.get(num as usize) else { return contents::SOLID };
            let pl = &self.planes[c.plane as usize];
            num = c.children[usize::from(dot(pl.normal, p) - pl.dist < 0.0)];
        }
        num
    }

    /// Returns false if the trace was stopped inside this subtree.
    fn hull_check(&self, num: i32, p1f: f32, p2f: f32, p1: [f32; 3], p2: [f32; 3], tr: &mut Trace) -> bool {
        if num < 0 {
            if num != contents::SOLID {
                tr.all_solid = false;
            } else {
                tr.start_solid = true;
            }
            return true;
        }
        let Some(node) = self.clipnodes.get(num as usize) else { return true };
        let plane = &self.planes[node.plane as usize];
        let t1 = dot(plane.normal, p1) - plane.dist;
        let t2 = dot(plane.normal, p2) - plane.dist;
        if t1 >= 0.0 && t2 >= 0.0 {
            return self.hull_check(node.children[0], p1f, p2f, p1, p2, tr);
        }
        if t1 < 0.0 && t2 < 0.0 {
            return self.hull_check(node.children[1], p1f, p2f, p1, p2, tr);
        }

        // Split: put the cross point DIST_EPSILON pixels on the near side.
        let mut frac = if t1 < 0.0 { (t1 + DIST_EPSILON) / (t1 - t2) } else { (t1 - DIST_EPSILON) / (t1 - t2) };
        frac = frac.clamp(0.0, 1.0);
        let mut midf = p1f + (p2f - p1f) * frac;
        let mut mid = lerp(p1, p2, frac);
        let side = usize::from(t1 < 0.0);

        if !self.hull_check(node.children[side], p1f, midf, p1, mid, tr) {
            return false;
        }
        if self.leaf_contents(node.children[side ^ 1], mid) != contents::SOLID {
            return self.hull_check(node.children[side ^ 1], midf, p2f, mid, p2, tr);
        }
        if tr.all_solid {
            return false; // never got out of the solid area
        }

        // The other side of the node is solid: this is the impact point.
        tr.normal = if side == 0 { plane.normal } else { plane.normal.map(|v| -v) };
        // Back up until the midpoint is outside the solid (Quake loop).
        let start_num = num;
        let mut tries = 0;
        while self.leaf_contents(start_num, mid) == contents::SOLID {
            frac -= 0.1;
            if frac < 0.0 || tries > 10 {
                tr.fraction = midf;
                tr.end = mid;
                return false;
            }
            tries += 1;
            midf = p1f + (p2f - p1f) * frac;
            mid = lerp(p1, p2, frac);
        }
        tr.fraction = midf;
        tr.end = mid;
        false
    }
}
