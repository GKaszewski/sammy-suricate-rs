//! Moving through a collision space: floors and sliding.

use super::*;

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
