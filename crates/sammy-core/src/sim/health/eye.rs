//! The Eye of Truth and the chase warning light.

use super::*;
use crate::angle::Angles;

pub(in crate::sim) fn init_eye(w: &mut World, id: Id, obj: &Object) {
    let eye = EyePickup::from_object(obj);
    if eye.snap {
        w.snap_to_ground.insert(id);
        w.extra_height.insert(id, EXTRA_HEIGHT);
    }
    w.eyes.insert(id, Scaled { item: eye, scale: obj.scale.map(f32::abs) });
}

/// `Warning_attack`: an orange light on Sammy when a lion starts a chase, fading out.
pub(in crate::sim) fn warning_light(sim: &mut Sim, dt: Dt) {
    let warn = sim.warn;
    let w = &mut sim.world;
    let Some((me, _)) = w.player() else { return };
    let fading = w.warning_fade.contains(&me);
    match w.lights.get_mut(&me) {
        Some(l) if fading => {
            l.range -= WARNING_FADE_RATE * dt.t;
            if l.range <= WARNING_FADE_END {
                l.range = 0.0;
                w.warning_fade.remove(&me);
            }
        }
        // Only when Sammy carries no light already (`lightrange == 0`).
        Some(l) if warn && l.range == 0.0 => {
            *l = Light { color: WARNING_COLOR, range: WARNING_RANGE, fade: None };
            w.warning_fade.insert(me);
        }
        None if warn => {
            w.lights.insert(me, Light { color: WARNING_COLOR, range: WARNING_RANGE, fade: None });
            w.warning_fade.insert(me);
        }
        _ => {}
    }
}

/// The Eye of Truth pickups and the effect wearing off.
pub(in crate::sim) fn run_eye(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let playing = sim.playing();
    let Some((_, p)) = sim.world.player() else { return };
    let (sammy, radius, dead) = (p.pos, p.scan_radius, p.state == State::Dead);
    crate::eye::wear_off(&mut sim.game, dead, t);
    let Sim { world: w, game, events, .. } = sim;
    for (id, eye) in w.eyes.iter_mut() {
        if w.snap_to_ground.contains(id) {
            continue;
        }
        let Some(b) = w.bodies.get_mut(id) else { continue };
        if eye.item.spin(t) {
            b.angles = Angles::from_array(eye.item.angles);
        }
        let reach = playing && !dead && in_scan(sammy, radius, b.pos);
        match eye.item.step(game, reach, t) {
            EyeStage::Idle => {}
            EyeStage::Taken => {
                events.push(Event::Sound("EyeTruth_Son"));
                events.push(Event::Effect(Effect::TakeEye { at: b.pos }));
            }
            EyeStage::Shrinking => b.shrink(crate::eye::SHRINK_RATE.powf(t.get())),
            EyeStage::Hidden => b.hidden = true,
            EyeStage::Back => {
                b.hidden = false;
                b.scale = eye.scale;
            }
        }
    }
}
