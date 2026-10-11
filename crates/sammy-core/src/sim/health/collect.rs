//! Taking hearts, boosts and checkpoints.

use super::*;
use crate::time::Ticks;

/// A pickup that ducks away and grows back to its level size.
#[derive(Clone, Debug)]
pub struct Scaled<T> {
    pub item: T,
    pub scale: [f32; 3],
}

pub(in crate::sim) fn init_small_heart(w: &mut World, id: Id, obj: &Object) {
    let heart = SmallHeart::from_object(obj);
    if heart.snap {
        w.snap_to_ground.insert(id);
    }
    w.small_hearts.insert(id, Scaled { item: heart, scale: obj.scale.map(f32::abs) });
}

pub(in crate::sim) fn collect(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    if p.state == State::Dead {
        return;
    }
    let (sammy, radius) = (p.pos, p.scan_radius);
    let near = |q: [f32; 3]| in_scan(sammy.get(), radius, q);
    let Sim { world: w, game, events, .. } = sim;
    let shrinking = w.shrinking.clone();

    for (id, h) in w.small_hearts.iter_mut() {
        if shrinking.contains(id) || w.snap_to_ground.contains(id) {
            continue;
        }
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if h.item.away.is_some() || !near(at.to_array()) {
            continue;
        }
        if !crate::health::take_small_heart(&mut h.item, game) {
            continue; // full: duck away and come back
        }
        events.push(Event::Sound("Gagne1"));
        events.push(Event::Sound("Chimes2"));
        events.push(Event::Effect(Effect::SmallHeart { at }));
        w.shrinking.insert(*id);
    }
    for (id, b) in &w.big_hearts {
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if shrinking.contains(id) || !near(at.to_array()) || !crate::health::take_big_heart(b, game) {
            continue;
        }
        events.push(Event::Sound("Getlife_son"));
        for i in 0..GET_LIFE_COUNT {
            events.push(Event::Effect(Effect::GetLife { at, delay: i as f32 * GET_LIFE_EVERY }));
        }
        let (color, range, rate, until) = LIFE_FLASH;
        w.shrinking.insert(*id);
        w.lights.insert(*id, Light::flash(color, range, rate, until));
    }
    for (id, b) in w.boosts.iter_mut() {
        if shrinking.contains(id) {
            continue;
        }
        let Some(body) = w.bodies.get_mut(id) else { continue };
        if let Some(left) = b.item.away {
            // Ducked away because boost was full: shrink, hide, grow back.
            let left = left - dt.t;
            if left > BOOST_HIDDEN {
                body.shrink(BOOST_AWAY_SHRINK.powf(dt.t.get()));
            } else {
                body.hidden = true;
            }
            if left <= Ticks::ZERO {
                body.hidden = false;
                body.scale = b.scale;
                b.item.away = None;
            } else {
                b.item.away = Some(left);
            }
            continue;
        }
        if !near(body.pos.to_array()) || !crate::health::take_boost(&mut b.item, game) {
            continue;
        }
        events.push(Event::Sound("chimes2"));
        let (color, range, rate, until) = BOOST_FLASH;
        w.shrinking.insert(*id);
        w.lights.insert(*id, Light::flash(color, range, rate, until));
    }
    for (id, c) in &w.checkpoints {
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if shrinking.contains(id) || w.collected.contains(id) || !near(at.to_array()) {
            continue;
        }
        events.push(Event::Sound("Restart_Son"));
        crate::health::take_checkpoint(c, game);
        w.shrinking.insert(*id);
    }
}
