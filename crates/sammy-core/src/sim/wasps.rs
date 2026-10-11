//! Sammy's wasps (docs/wasps.md): ammo pickups (`InitAmmo1`, bullets.wdl:21-177),
//! firing (`PlayerTire`, player.wdl:271-384), flight and hits (`InitWasp`/`Wasp`/
//! `Wasp_Hit`, bullets.wdl:203-747), weak doors (`InitDoorWeak`, mm1.wdl:4466-4715) and
//! hives.

use formats::wmb::Object;

use super::spawn::MODEL_FPS;
use super::world::{Animator, Body, Light, Look, Visual, World};
use super::{Dt, Effect, Event, Sim, Space, WaspHit};
use crate::collision::{Hull, Space as _};
use crate::id::Id;
use crate::level::Presentation;
use crate::input::Controls;
use crate::player::State;
use crate::time::TICKS_PER_SEC;
use crate::wasps::{AMMO_COLORS, AmmoPickup, Corpse, DoorDamage, ESCAPE_SCALE, Escapee, FIRED, HIVE_WASP_ANIM, HiveWasp, KINDS, PICKUP_ANIM, PICKUP_FADE, PICKUP_FADE_SHRINK, PICKUP_SPIN, PICKUP_TILT, PLANKS, Plank, Shot, WING_ANIM, Wasp, WaspStep};

/// Pickup flash on taking ammo: start range, shrink per tick, cut-off range.
const TAKE_FLASH: (f32, f32, f32) = (300.0, 15.0, 100.0);
/// Spread of a wasp's death burst (`wasp_explode`).
const WASP_BURST_SPREAD: f32 = 2.0;
/// The flying wasp model.
const WASP_MODEL: &str = "wasp1.mdl";
/// Wasp type 1, the "queen" (overlay look).
const QUEEN: u32 = 1;

/// `InitAmmo1`: the type's skin and size; the overlay look unless Flag3 (flare).
pub(super) fn init_ammo(w: &mut World, id: Id, obj: &mut Object) -> Presentation {
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

pub(super) fn spin_pickups(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    for (id, a) in w.spinning.iter_mut() {
        a[0] += PICKUP_SPIN * dt.t;
        if let Some(b) = w.bodies.get_mut(id) {
            b.angles = *a;
        }
    }
}

pub(super) fn collect_ammo(sim: &mut Sim) {
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

/// A `wasp1.mdl` entity of `kind` (skin) playing "walk" at `wing` percent per tick.
#[allow(clippy::too_many_arguments)]
fn spawn_wasp_body(sim: &mut Sim, name: &str, pos: [f32; 3], angles: [f32; 3], scale: f32, kind: u32, overlay: bool, wing: f32) -> Id {
    let mut body = Body::new(name, pos).scaled(scale);
    body.angles = angles;
    let id = sim.world.spawn_body(body);
    sim.world.looks.insert(id, Look::new(Visual::Model { file: WASP_MODEL.into(), skin: kind, overlay, frame: None }));
    if let Some(model) = sim.assets.model(WASP_MODEL) {
        let mut a = Animator::new(model, Vec::new(), MODEL_FPS);
        a.anim.play("walk", true, 0.0);
        a.anim.set_speed_pct(wing, TICKS_PER_SEC);
        sim.world.animators.insert(id, a);
    }
    id
}

/// Escaping wasps spiral upward and vanish.
pub(super) fn escapees(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    let mut gone = Vec::new();
    for (id, e) in w.escapees.iter_mut() {
        if e.step(dt.t) {
            gone.push(*id);
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = e.pos;
            b.angles = [e.pan, e.tilt, 0.0];
        }
    }
    for id in gone {
        w.despawn(id);
    }
}

/// Wasps that hit a lion: hit puffs, a second death sound, the recoil.
pub(super) fn corpses(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, rng, events, .. } = sim;
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let mut gone = Vec::new();
    for (id, c) in w.wasp_corpses.iter_mut() {
        let step = c.step(w.animators.get_mut(id).map(|a| &mut a.anim), dt.t, |a, b| space.trace_static(Hull::Point, a, b).trace);
        if step.puffs > 0 {
            events.push(Event::Effect(Effect::HitStars { at: c.pos, count: step.puffs }));
        }
        if step.sound {
            events.push(Event::Sound(crate::wasps::burst_sound(rng)));
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = c.pos;
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

/// `PlayerTire`: fire when Sammy is in a normal ground state.
pub(super) fn fire(sim: &mut Sim, controls: &Controls, dt: Dt) {
    let playing = sim.playing();
    let trigger = controls.fire_hit || sim.debug.autofire;
    let w = &mut sim.world;
    let Some((&me, p)) = w.players.iter_mut().next() else { return };
    let Some(anim) = w.animators.get_mut(&me) else { return };
    let kind = sim.game.ammo_type.clamp(1, KINDS);
    let fired = crate::wasps::fire(p, &mut anim.anim, &mut sim.game, trigger, playing);
    let pan = p.pan;
    let mut shots = Vec::new();
    // Release due shots of the current fan.
    if let Some(pending) = w.pending_shots.get_mut(&me)
        && pending.release(kind, sim.game.ammo_poison, pan, dt.t, &mut shots)
    {
        w.pending_shots.remove(&me);
    }
    if let Some(new) = fired.thrown {
        w.pending_shots.insert(me, new);
    }
    if fired.empty {
        sim.sound("Error1");
    }
    for shot in shots {
        spawn_wasp(sim, shot, me);
    }
}

fn spawn_wasp(sim: &mut Sim, shot: Shot, shooter: Id) {
    if sim.assets.model(WASP_MODEL).is_none() {
        return;
    }
    let w = Wasp::new(shot, Some(shooter));
    sim.sound("Wasp_son");
    // Type 1, the "queen", has the colour-keyed overlay look.
    let id = spawn_wasp_body(sim, "wasp", shot.pos, [shot.pan, 0.0, 0.0], FIRED[(shot.kind - 1) as usize].scale, shot.kind, shot.kind == QUEEN, WING_ANIM * w.speed());
    if let Some(b) = sim.world.bodies.get_mut(&id) {
        b.hidden = true;
    }
    sim.world.wasps.insert(id, w);
}

pub(super) fn fly(sim: &mut Sim, dt: Dt) {
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
            b.pos = wasp.pos;
            b.angles = [wasp.pan, wasp.tilt, 0.0];
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
            events.push(Event::Effect(Effect::WaspExplode { at: wasp.pos, kind: wasp.kind, spread: WASP_BURST_SPREAD }));
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

/// `ExplodeDoorWeak`: HP 3/2/1 morph to doorwk2/3/4; otherwise the door breaks into planks.
pub(super) fn weak_doors(sim: &mut Sim) {
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
                    body.angles = plank.angles;
                    let pid = sim.world.spawn_body(body);
                    sim.world.looks.insert(pid, Look::new(Visual::Model { file: file.into(), skin: 1, overlay: false, frame: Some(0) }));
                    sim.world.planks.insert(pid, plank);
                }
            }
        }
    }
}

pub(super) fn planks(sim: &mut Sim, dt: Dt) {
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
            b.angles = p.angles;
        }
        if done {
            gone.push(*id);
        }
    }
    for id in gone {
        w.despawn(id);
    }
}

pub(super) fn hives(sim: &mut Sim, dt: Dt) {
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
pub(super) fn hive_wasps(sim: &mut Sim, dt: Dt) {
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
            w.pickup_lifetimes.insert(id, l);
        }
    }
}

/// Hive pickups fade over their last ticks, then burst.
pub(super) fn expire_pickups(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, events, .. } = sim;
    let mut gone = Vec::new();
    for (id, life) in w.pickup_lifetimes.iter_mut() {
        *life -= dt.t;
        let Some(b) = w.bodies.get_mut(id) else { continue };
        if *life < PICKUP_FADE {
            b.shrink(PICKUP_FADE_SHRINK.powf(dt.t));
        }
        if *life <= 0.0 {
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
