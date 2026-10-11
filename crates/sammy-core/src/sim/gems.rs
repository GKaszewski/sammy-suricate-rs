//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5): thrown-back
//! keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` / `LaunchNewKey`:
//! boulders, Sammy's death gem), lifetimes with blinking, gravity for Flag7 gems,
//! surprise colours, boulders, and the gem flying into a permanent door.

use super::pickups::LevelKey;
use super::{Dt, Effect, Event, Sim, Space};
use crate::collision::{Hull, Space as _};
use crate::gems::{FlyingGem, GemGravity, GemLifetime, HEIGHT_GEM, Launch, Rest};
use crate::id::Id;
use crate::player::Player;
use crate::time::{TICKS_PER_SEC, Ticks};

/// Gem sprite animation speed (frames per second), backwards for door gems.
pub(super) const GEM_FPS: f32 = 48.0;
/// Size of a flying / runtime gem, and while blinking small.
const GEM_SCALE: f32 = 1.5;
const BLINK_SCALE: f32 = 0.75;
/// Floor probe depth below a falling gem.
const FLOOR_PROBE: f32 = 2000.0;

/// A gem flying from Sammy into a permanent door.
#[derive(Clone, Debug)]
pub struct DoorGemFlight {
    pub door: Id,
    pub gem: crate::gems::DoorGem,
}

/// A runtime key gem in flight.
fn spawn_flying(sim: &mut Sim, name: &str, color: u32, gem: FlyingGem) {
    let pos = gem.pos;
    if let Some(id) = super::spawn::gem_sprite(sim, name, color, pos, GEM_SCALE, GEM_FPS) {
        sim.world.key_gems.insert(id, color);
        sim.world.flying_gems.insert(id, gem);
    }
}

/// `EjectOldKey` + `LaunchYouKey`: the old key flies forward at 2× Sammy's walk speed.
pub(super) fn throw_back_key(sim: &mut Sim, color: u32, p: &Player, life: f32) {
    spawn_flying(sim, "thrown key", color, FlyingGem::thrown(p.pos, p.pan, p.speed, life));
}

/// `EjectNewKey` → `LaunchNewKey`: a gem launched from `pos` (boulders, Sammy's death).
pub(super) fn launch(sim: &mut Sim, l: Launch) {
    let gem = FlyingGem::launched(l.pos, l.pan, l.hspeed, l.vz, l.fate, &sim.jump);
    spawn_flying(sim, "launched gem", l.color, gem);
}

/// After a flight: the gem becomes collectable and gets its resting behaviour.
fn settle(sim: &mut Sim, id: Id, g: &FlyingGem) {
    let w = &mut sim.world;
    w.flying_gems.remove(&id);
    match g.rest() {
        Rest::Collectable { lifetime } => {
            w.collectable.insert(id);
            if let Some(l) = lifetime {
                w.gem_lifetimes.insert(id, GemLifetime::new(l));
            }
        }
        Rest::Decorative => {
            w.decorative.insert(id);
            w.shrinking.insert(id);
        }
        Rest::Gravity { lifetime, respawns } => {
            w.collectable.insert(id);
            w.gem_gravity.insert(id, GemGravity { rest: HEIGHT_GEM });
            if let Some(l) = lifetime {
                w.gem_lifetimes.insert(id, GemLifetime::new(l));
            } else if respawns {
                w.level_keys.insert(id, LevelKey { home: Some(g.pos), scale: [1.0; 3] });
                w.respawnable.insert(id);
            }
        }
    }
}

pub(super) fn fly(sim: &mut Sim, dt: Dt) {
    // The original loops run per frame with `mintime = min(time, 1)`.
    let t = dt.t.min(Ticks(1.0));
    let mut settled = Vec::new();
    let mut sounds = Vec::new();
    {
        let Sim { world: w, level, jump, rng, .. } = sim;
        let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
        for (id, g) in w.flying_gems.iter_mut() {
            let collectable = w.collectable.contains(id);
            let step = g.step(collectable, jump, rng, t, |a, b| space.trace_static(Hull::Point, a, b).trace, &mut sounds);
            if step.collectable {
                w.collectable.insert(*id);
            }
            if let Some(b) = w.bodies.get_mut(id) {
                b.pos = g.pos;
            }
            if step.done {
                settled.push((*id, g.clone()));
            }
        }
    }
    sim.events.extend(sounds.into_iter().map(Event::Sound));
    for (id, g) in settled {
        settle(sim, id, &g);
    }
}

/// Resting runtime gems: lifetime blink and burst (GemWaitForPlayer with Skill9).
/// The blink follows the sprite animation: small in the second half of each cycle.
pub(super) fn lifetimes(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let w = &mut sim.world;
    let mut burst = Vec::new();
    for (id, l) in w.gem_lifetimes.iter_mut() {
        if w.flying_gems.contains_key(id) || w.shrinking.contains(id) {
            continue;
        }
        let (Some(anim), Some(body)) = (w.sprite_anims.get_mut(id), w.bodies.get_mut(id)) else { continue };
        anim.fps = l.speed * TICKS_PER_SEC;
        let half = anim.frames as f32 / 2.0;
        if anim.frame > half && !l.small {
            l.small = true;
            body.scale = [BLINK_SCALE; 3];
            sim.events.push(Event::Sound("cligno1_son"));
        } else if anim.frame < half && l.small {
            l.small = false;
            l.speed += crate::gems::BLINK_SPEEDUP * t;
            body.scale = [GEM_SCALE; 3];
            sim.events.push(Event::Sound("cligno1_son"));
        }
        if l.tick(t) {
            burst.push((*id, body.pos, w.key_gems.get(id).copied().unwrap_or(0)));
        }
    }
    for (id, at, color) in burst {
        sim.world.despawn(id);
        sim.sound("broken4"); // GemExplose
        sim.effect(Effect::GemExplode { at, color });
    }
}

/// Flag7 gems: fall at `Gravite` per tick to floor + rest, carried by moving platforms.
pub(super) fn gravity(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, level, jump, .. } = sim;
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    for (id, g) in &w.gem_gravity {
        if w.flying_gems.contains_key(id) || w.shrinking.contains(id) {
            continue;
        }
        let Some(b) = w.bodies.get_mut(id) else { continue };
        let p = b.pos;
        let hit = space.trace(Hull::Point, p, [p[0], p[1], p[2] - FLOOR_PROBE], None);
        if !hit.trace.hit() {
            continue;
        }
        if let Some(plat) = hit.entity.and_then(|e| w.platforms.get(&e)) {
            for (c, d) in b.pos.iter_mut().zip(plat.platform.delta) {
                *c += d;
            }
        }
        b.pos[2] = g.fall(b.pos[2], hit.trace.end[2], jump, dt.t);
    }
}

/// Surprise gems: the colour climbs after a 1-8 tick delay, then they can be collected.
pub(super) fn surprises(sim: &mut Sim, dt: Dt) {
    let Sim { world: w, rng, .. } = sim;
    for (id, s) in w.surprises.iter_mut() {
        let Some(color) = w.key_gems.get_mut(id) else { continue };
        if let Some(c) = s.step(*color, rng, dt.t) {
            *color = c;
            w.collectable.insert(*id);
        }
    }
}

pub(super) fn boulders(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let mut launches = Vec::new();
    // Impact mode: a wasp hitting the boulder launches a gem back toward the shooter.
    for h in &sim.wasp_hits {
        let Some(b) = sim.world.boulders.get(&h.target) else { continue };
        launches.extend(b.hit_by_wasp(h.wasp_pan, h.wasp_disp, &mut sim.rng));
    }
    // Auto mode: every Skill8 ticks.
    for b in sim.world.boulders.values_mut() {
        launches.extend(b.step(&mut sim.rng, dt.t));
    }
    for l in launches {
        launch(sim, l);
    }
}
