//! Hives and their wasps.

use super::*;
use crate::time::Ticks;

pub(in crate::sim) fn hives(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let ids: Vec<Id> = sim.world.hives.keys().copied().collect();
    for id in ids {
        let Some(h) = sim.world.hives.get_mut(&id) else { continue };
        let Some((l, exhausted)) = h.step(&sim.game, &mut sim.rng, dt.t) else { continue };
        let pos = h.pos;
        if sim.assets.model(WASP_MODEL).is_none() {
            continue;
        }
        // Hive wasps have the overlay look (their Flag3 is never set).
        let wid = spawn_wasp_body(sim, "hive wasp", pos, [l.pan, l.tilt, 0.0], l.scale, l.kind, true, HIVE_WASP_ANIM);
        sim.world.hive_wasps.insert(wid, HiveWasp::new(&l, pos));
        if exhausted {
            // Exhausted: the hive bursts (bullets.wdl:1255-1272).
            let at = sim.world.pos(id).unwrap_or(pos);
            sim.effect(Effect::HiveBurst { at });
            sim.world.despawn(id);
        }
    }
}

/// `LaunchNewWasp` flight: ballistic hops with damped bounces until it settles, then
/// it becomes a 1-ammo pickup.
pub(in crate::sim) fn hive_wasps(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, rng, events, .. } = sim;
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let mut sounds = Vec::new();
    let mut settled = Vec::new();
    for (id, hw) in w.hive_wasps.iter_mut() {
        let done = hw.step(|a, b| space.trace_static(Hull::Point, a, b).trace, rng, dt.t, &mut sounds);
        events.extend(sounds.drain(..).map(Event::Sound));
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = hw.pos;
            b.angles = [hw.pan, hw.tilt, 0.0];
        }
        if let Some((pickup, lifetime)) = done {
            settled.push((*id, pickup, lifetime, [hw.pan, PICKUP_TILT, 0.0]));
        }
    }
    for (id, pickup, lifetime, angles) in settled {
        w.hive_wasps.remove(&id);
        w.ammo_pickups.insert(id, pickup);
        w.spinning.insert(id, angles);
        if let Some(l) = lifetime {
            w.pickup_lifetimes.insert(id, Ticks(l));
        }
    }
}
