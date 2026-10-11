//! Taking pickups and shrinking taken ones.

use super::*;

pub(in crate::sim) fn collect(sim: &mut Sim) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    if p.state == State::Dead {
        return;
    }
    let p = p.clone();
    let near = |q: [f32; 3]| in_scan(p.pos, p.scan_radius, q);
    let w = &sim.world;
    let ready = |id: &Id| !w.shrinking.contains(id) && !w.snap_to_ground.contains(id);

    let taken: Vec<(Id, [f32; 3], crate::pickups::Diamond)> =
        w.diamonds.iter().filter(|(id, _)| ready(id)).filter_map(|(id, d)| Some((*id, w.pos(*id)?, *d))).filter(|(_, at, _)| near(*at)).collect();
    for (id, at, d) in taken {
        sim.world.shrinking.insert(id);
        sim.effect(Effect::DiamondBurst { at });
        sim.sound("Diam_son");
        if d.has_popup() {
            sim.sound("clog3");
            sim.emit(Event::Popup { at, points: d.points });
        }
        sim.add_points(d.points);
    }

    let w = &sim.world;
    let keys: Vec<Id> = w.key_gems.keys().copied().filter(|id| !w.shrinking.contains(id) && !w.snap_to_ground.contains(id) && !w.collected.contains(id)).collect();
    // All key gems vanish once the goal is reached (gems.wdl:77).
    if sim.game.goal_reached {
        for id in keys {
            if sim.world.collectable.contains(&id) {
                sim.world.shrinking.insert(id);
            }
        }
        return;
    }
    // A gem flying into a door blocks pickups (`PlayerOldKey == 999`).
    if sim.game.key_in_flight {
        return;
    }
    // Key_Hit: take the gem; mix primaries, otherwise throw the old key back out.
    for id in keys {
        let w = &mut sim.world;
        let Some(at) = w.pos(id) else { continue };
        if !w.collectable.contains(&id) || !near(at) {
            continue;
        }
        let color = w.key_gems[&id];
        let (range, rate, until) = KEY_FLASH;
        w.shrinking.insert(id);
        w.lights.insert(id, Light::flash(light_color(color), range, rate, until));
        if let Some(lk) = w.level_keys.get_mut(&id) {
            lk.home.get_or_insert(at);
        }
        let life = w.gem_lifetimes.get(&id).map_or(0.0, |l| l.ticks);
        sim.sound("ColoredGem_son");
        sim.effect(Effect::GemCatch { at, color });
        let held = sim.game.key;
        let (key, outcome) = take_key(held, color);
        sim.game.key = key;
        match outcome {
            KeyOutcome::Mixed => {
                sim.sound("GemMixing_son");
                // AnimGemMix1 end: each source primary bursts 70 stars (gems.wdl:596-605).
                sim.effect(Effect::GemMix { at: [p.pos[0], p.pos[1], p.pos[2] + MIX_HEIGHT], colors: [held, color] });
            }
            KeyOutcome::Ejected(old) => crate::sim::gems::throw_back_key(sim, old, &p, life),
            KeyOutcome::Taken => {}
        }
        break; // one gem per scan
    }
}

/// Picked up: shrink ×0.8 per tick, then gone (`Bonus3_Hit`, `ScaleDownGem`). Entities
/// that can come back (level keys, checkpoints) are hidden instead.
pub(in crate::sim) fn shrink(sim: &mut Sim, dt: crate::sim::Dt) {
    let k = SHRINK_PER_TICK.powf(dt.t.get());
    let w = &mut sim.world;
    let mut gone = Vec::new();
    for id in &w.shrinking {
        let Some(b) = w.bodies.get_mut(id) else { continue };
        b.shrink(k);
        if b.scale[1] < SHRINK_GONE {
            gone.push(*id);
        }
    }
    for id in gone {
        if w.respawnable.contains(&id) {
            w.shrinking.remove(&id);
            w.collected.insert(id);
            if let Some(b) = w.bodies.get_mut(&id) {
                b.hidden = true;
            }
        } else {
            w.despawn(id);
        }
    }
}
