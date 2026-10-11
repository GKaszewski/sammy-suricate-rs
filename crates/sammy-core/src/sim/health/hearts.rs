//! Hearts and boosts ducking away, growing back and spinning.

use super::*;
use crate::time::Ticks;

/// Small hearts that ducked away (health was full) shrink, wait and grow back.
pub(in crate::sim) fn animate_hearts(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let w = &mut sim.world;
    for (id, h) in w.small_hearts.iter_mut() {
        let Some(left) = h.item.away else { continue };
        if w.shrinking.contains(id) {
            continue;
        }
        let Some(b) = w.bodies.get_mut(id) else { continue };
        let left = left - t;
        if left > Ticks::ZERO {
            b.shrink(HEART_AWAY_SHRINK.powf(t.get()));
            if b.scale[1] < HEART_HIDDEN_BELOW {
                b.hidden = true;
            }
            h.item.away = Some(left);
        } else {
            b.hidden = false;
            let k = HEART_REGROW.powf(t.get());
            b.scale = std::array::from_fn(|i| (b.scale[i].max(HEART_HIDDEN_BELOW) * k).min(h.scale[i]));
            if b.scale[1] >= h.scale[1] {
                h.item.away = None;
            }
        }
    }
}

/// Big hearts spin; checkpoints spin when Flag3 is set.
pub(in crate::sim) fn spin(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let w = &mut sim.world;
    for id in w.big_hearts.keys() {
        if w.checkpoints.contains_key(id) {
            continue;
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.angles[0] += BIG_HEART_SPIN * t;
        }
    }
    for (id, c) in w.checkpoints.iter_mut() {
        if w.big_hearts.contains_key(id) {
            continue;
        }
        if let Some(spin) = c.spin {
            for (a, s) in c.angles.iter_mut().zip(spin) {
                *a += s * t;
            }
            if let Some(b) = w.bodies.get_mut(id) {
                b.angles = c.angles;
            }
        }
    }
}
