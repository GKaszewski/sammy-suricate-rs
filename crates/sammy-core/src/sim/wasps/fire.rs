//! Firing wasps and spawning their bodies.

use super::*;
use crate::angle::Angles;

/// A `wasp1.mdl` entity of `kind` (skin) playing "walk" at `wing` percent per tick.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_wasp_body(sim: &mut Sim, name: &str, pos: [f32; 3], angles: [f32; 3], scale: f32, kind: u32, overlay: bool, wing: f32) -> Id {
    let mut body = Body::new(name, pos).scaled(scale);
    body.angles = Angles::from_array(angles);
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

/// `PlayerTire`: fire when Sammy is in a normal ground state.
pub(in crate::sim) fn fire(sim: &mut Sim, controls: &Controls, dt: Dt) {
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

pub(super) fn spawn_wasp(sim: &mut Sim, shot: Shot, shooter: Id) {
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
