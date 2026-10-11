//! Level keys: placing, snapping and resetting.

use super::*;

/// A key placed in the level (or settled with `respawns`): restored at its home on each
/// respawn, at its level size.
#[derive(Clone, Debug)]
pub struct LevelKey {
    pub home: Option<[f32; 3]>,
    pub scale: [f32; 3],
}

pub(in crate::sim) fn init_key(w: &mut World, id: Id, obj: &Object) {
    let spec = KeySpec::from_object(obj);
    w.key_gems.insert(id, spec.color);
    w.level_keys.insert(id, LevelKey { home: None, scale: obj.scale.map(f32::abs) });
    w.respawnable.insert(id);
    match spec.surprise {
        // Surprise gem: not collectable until its colour has been rolled.
        Some(add) => {
            w.surprises.insert(id, Surprise::new(add, obj.origin[0] + obj.origin[1]));
        }
        None => {
            w.collectable.insert(id);
        }
    }
    if spec.gravity {
        w.gem_gravity.insert(id, GemGravity { rest: HEIGHT_GEM });
    }
    if spec.hover_from_wed {
        w.hover_from_wed.insert(id);
    }
    if spec.snap {
        w.snap_to_ground.insert(id);
    }
}

/// Snap to `floor + HEIGHT_GEM` on the first frame (`sonar my,2000`).
pub(in crate::sim) fn snap(sim: &mut Sim) {
    let Sim { world: w, level, .. } = sim;
    if level.is_none() {
        return;
    }
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    for id in std::mem::take(&mut w.snap_to_ground) {
        let Some(b) = w.bodies.get_mut(&id) else { continue };
        let [x, y, z] = b.pos;
        let tr = space.trace(Hull::Narrow, [x, y, z + SNAP_START], [x, y, z - FLOOR_PROBE], None).trace;
        let floor = tr.end[2] - NARROW_FLOOR_OFFSET;
        if w.hover_from_wed.contains(&id)
            && let Some(g) = w.gem_gravity.get_mut(&id)
        {
            // Flag5: keep the WED height above the floor (+HeightGem) as the rest height.
            g.rest = z - floor + HEIGHT_GEM;
        }
        if tr.hit() && !tr.start_solid {
            b.pos[2] = floor + HEIGHT_GEM + w.extra_height.get(&id).copied().unwrap_or(0.0);
        }
    }
}

/// Reset scan (id 1000) for keys: level keys come back home (or vanish for good once
/// Sammy holds the master key); runtime keys are removed.
pub(in crate::sim) fn reset_keys(sim: &mut Sim, reset: bool) {
    if !reset {
        return;
    }
    let master = sim.game.key == MASTER_KEY;
    let w = &mut sim.world;
    let ids: Vec<Id> = w.key_gems.keys().copied().collect();
    for id in ids {
        match w.level_keys.get(&id) {
            Some(lk) if !master => {
                let (home, scale) = (lk.home, lk.scale);
                if let Some(b) = w.bodies.get_mut(&id) {
                    if let Some(h) = home {
                        b.pos = h;
                    }
                    b.scale = scale;
                    b.hidden = false;
                }
                w.collected.remove(&id);
                w.shrinking.remove(&id);
                // InitKey runs again: a surprise gem re-rolls (its colour keeps climbing).
                match w.surprises.get_mut(&id) {
                    Some(s) => {
                        s.rearm(&mut sim.rng);
                        w.collectable.remove(&id);
                    }
                    None => {
                        w.collectable.insert(id);
                    }
                }
            }
            _ => w.despawn(id),
        }
    }
}
