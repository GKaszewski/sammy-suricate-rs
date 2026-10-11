//! Weak doors hit by wasps, and their planks.

use super::*;
use crate::angle::Angles;

/// `ExplodeDoorWeak`: HP 3/2/1 morph to doorwk2/3/4; otherwise the door breaks into planks.
pub(in crate::sim) fn weak_doors(sim: &mut Sim) {
    for id in std::mem::take(&mut sim.weak_door_hits) {
        let Some(door) = sim.world.weak_doors.get(&id) else { continue };
        match door.damage() {
            DoorDamage::Untouched => {}
            DoorDamage::Morph(file) => {
                sim.sound("plank1_Son");
                let Some(wmb) = sim.assets.prefab(file) else { continue };
                if sim.world.brushes.get(&id).is_some_and(|b| !std::sync::Arc::ptr_eq(&b.wmb, &wmb)) {
                    sim.morph(id, file);
                }
            }
            DoorDamage::Destroyed => {
                // The door vanishes and six planks fly out (`LaunchPlank`).
                let origin = sim.world.objects.get(&id).map_or([0.0; 3], |o| o.origin);
                sim.world.despawn(id);
                for file in PLANKS {
                    if sim.assets.model(file).is_none() {
                        continue;
                    }
                    let plank = Plank::burst(origin, &mut sim.rng);
                    let mut body = Body::new("plank", plank.pos);
                    body.angles = Angles::from_array(plank.angles);
                    let pid = sim.world.spawn_body(body);
                    sim.world.looks.insert(pid, Look::new(Visual::Model { file: file.into(), skin: 1, overlay: false, frame: Some(0) }));
                    sim.world.planks.insert(pid, plank);
                }
            }
        }
    }
}

pub(in crate::sim) fn planks(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, events, .. } = sim;
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let mut gone = Vec::new();
    for (id, p) in w.planks.iter_mut() {
        let (sound, done) = p.step(|a, b| space.trace(Hull::Point, a, b, None).trace, dt.t);
        if sound {
            events.push(Event::Sound("plank1_Son"));
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = p.pos;
            b.angles = Angles::from_array(p.angles);
        }
        if done {
            gone.push(*id);
        }
    }
    for id in gone {
        w.despawn(id);
    }
}
