//! Ammo pickups: spawning, spinning, taking and expiring.

use super::*;
use crate::time::Ticks;

/// `InitAmmo1`: the type's skin and size; the overlay look unless Flag3 (flare).
pub(in crate::sim) fn init_ammo(w: &mut World, id: Id, obj: &mut Object) -> Presentation {
    let (pickup, look) = AmmoPickup::from_object(obj);
    if let Some(sc) = look.scale {
        obj.scale = [sc; 3];
    }
    obj.angles = [0.0, PICKUP_TILT, 0.0];
    w.ammo_pickups.insert(id, pickup);
    w.spinning.insert(id, obj.angles);
    let flare = obj.flag(3);
    Presentation { skin: Some(look.skin), anim: Some(("stand", PICKUP_ANIM)), overlay: !flare, flare, ..Default::default() }
}

pub(in crate::sim) fn spin_pickups(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    for (id, a) in w.spinning.iter_mut() {
        a[0] += PICKUP_SPIN * dt.t;
        if let Some(b) = w.bodies.get_mut(id) {
            b.angles = *a;
        }
    }
}

pub(in crate::sim) fn collect_ammo(sim: &mut Sim) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    if p.state == State::Dead {
        return;
    }
    let (sammy, radius) = (p.pos, p.scan_radius);
    let mut escapes = Vec::new();
    let Sim { world: w, game, events, .. } = sim;
    for (id, a) in &w.ammo_pickups {
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if w.shrinking.contains(id) || !crate::pickups::in_scan(sammy, radius, at) {
            continue;
        }
        escapes.extend(a.take(game));
        events.push(Event::Sound("clog3"));
        events.push(Event::Sound("TakeWasp_son"));
        let color = AMMO_COLORS[(a.kind.clamp(1, KINDS) - 1) as usize];
        let (range, rate, until) = TAKE_FLASH;
        w.shrinking.insert(*id);
        w.lights.insert(*id, Light::flash(color, range, rate, until));
    }
    // `EscapeOldWasps`: the old type's wasps fly off from Sammy (`Pov_Syn`).
    let from = sim.pov.pos;
    for (kind, n) in escapes {
        for _ in 0..n {
            let e = Escapee::new(kind, from, &mut sim.rng);
            let id = spawn_wasp_body(sim, "escaping wasp", e.pos, [e.pan, e.tilt, 0.0], ESCAPE_SCALE, kind, false, 0.0);
            sim.world.escapees.insert(id, e);
        }
    }
}

/// Hive pickups fade over their last ticks, then burst.
pub(in crate::sim) fn expire_pickups(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, events, .. } = sim;
    let mut gone = Vec::new();
    for (id, life) in w.pickup_lifetimes.iter_mut() {
        *life -= dt.t;
        let Some(b) = w.bodies.get_mut(id) else { continue };
        if *life < PICKUP_FADE {
            b.shrink(PICKUP_FADE_SHRINK.powf(dt.t.get()));
        }
        if *life <= Ticks::ZERO {
            // `InitWaspWed` countdown over: the wasp bursts (bullets.wdl:80-108).
            if let Some(a) = w.ammo_pickups.get(id) {
                events.push(Event::Effect(Effect::WaspExplode { at: b.pos, kind: a.kind, spread: WASP_BURST_SPREAD }));
            }
            gone.push(*id);
        }
    }
    for id in gone {
        w.despawn(id);
    }
}
