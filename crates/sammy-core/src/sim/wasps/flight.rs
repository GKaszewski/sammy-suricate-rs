//! Wasps in flight, escaping and dying.

use super::*;
use crate::angle::Angles;
use crate::space::Pos;

/// Escaping wasps spiral upward and vanish.
pub(in crate::sim) fn escapees(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    let mut gone = Vec::new();
    for (id, e) in w.escapees.iter_mut() {
        if e.step(dt.t) {
            gone.push(*id);
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = Pos(e.pos);
            b.angles = Angles::from_array([e.pan, e.tilt, 0.0]);
        }
    }
    for id in gone {
        w.despawn(id);
    }
}

/// Wasps that hit a lion: hit puffs, a second death sound, the recoil.
pub(in crate::sim) fn corpses(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, rng, events, .. } = sim;
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let mut gone = Vec::new();
    for (id, c) in w.wasp_corpses.iter_mut() {
        let step = c.step(w.animators.get_mut(id).map(|a| &mut a.anim), dt.t, |a, b| space.trace_static(Hull::Point, a, b).trace);
        if step.puffs > 0 {
            events.push(Event::Effect(Effect::HitStars { at: Pos(c.pos), count: step.puffs }));
        }
        if step.sound {
            events.push(Event::Sound(crate::wasps::burst_sound(rng)));
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = Pos(c.pos);
            b.hidden = !step.visible;
        }
        if step.done {
            gone.push(*id);
        }
    }
    for id in gone {
        w.despawn(id);
    }
}

pub(in crate::sim) fn fly(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, rng, events, wasp_hits, weak_door_hits, .. } = sim;
    let sammy = w.player().map(|(_, p)| p.pos);
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let (mut gone, mut corpses) = (Vec::new(), Vec::new());
    for (id, wasp) in w.wasps.iter_mut() {
        let step = wasp.step(&space, sammy, dt.t);
        if let Some(b) = w.bodies.get_mut(id) {
            if wasp.visible {
                b.hidden = false;
            }
            b.pos = Pos(wasp.pos);
            b.angles = Angles::from_array([wasp.pan, wasp.tilt, 0.0]);
        }
        if let WaspStep::Burst { hit } = step {
            events.push(Event::Sound(crate::wasps::burst_sound(rng)));
            if let Some(target) = hit {
                if let Some(door) = w.weak_doors.get_mut(&target) {
                    door.hit(wasp.poison);
                    weak_door_hits.push(target);
                } else {
                    wasp_hits.push(WaspHit { target, poison: wasp.poison, bounce_force: wasp.bounce_force, wasp_pan: wasp.pan, wasp_disp: wasp.last_disp });
                    // A lion keeps the wasp as a visible corpse (`wasptat` 1004).
                    if w.enemies.get(&target).is_some_and(|e| e.keeps_wasp_corpses()) {
                        corpses.push((*id, Corpse::new(wasp)));
                        continue;
                    }
                }
            }
            events.push(Event::Effect(Effect::WaspExplode { at: Pos(wasp.pos), kind: wasp.kind, spread: WASP_BURST_SPREAD }));
            gone.push(*id);
        }
    }
    for id in gone {
        w.despawn(id);
    }
    for (id, c) in corpses {
        w.wasps.remove(&id);
        if let Some(a) = w.animators.get_mut(&id) {
            a.anim.play("death", false, 0.0);
            a.anim.fps = 0.0;
        }
        w.wasp_corpses.insert(id, c);
    }
}
