//! Lions, scorpions, the elephant and wasp enemies ([`crate::enemies`]) on the 16 Hz AI
//! clock, their collision boxes, and Sammy's box (so they can touch him).

use super::{Dt, Effect, Event, Sim, Space};
use crate::angle::Angles;
use crate::collision::BoxCollider;
use crate::enemies::{Impact, Out, Shared, World};
use crate::id::Id;
use crate::percent::Percent;
use crate::player::{SCALE, State};
use crate::space::Pos;

/// Wasp homing and fear tagging.
pub(super) fn home_wasps(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    for wasp in w.wasps.values_mut() {
        let mut refs: Vec<&mut crate::enemies::Enemy> = w.enemies.values_mut().collect();
        crate::enemies::home_wasp(wasp, &mut refs, dt.t);
    }
}

/// Sammy's own collision box (scaled model bbox), so lions and wasps can touch him.
pub(super) fn player_collider(sim: &mut Sim) {
    let w = &mut sim.world;
    for (id, p) in &w.players {
        if !w.boxes.contains_key(id)
            && let Some(a) = w.animators.get(id)
            && let Some((half, cz)) = crate::enemies::player_box(&a.model.mdl, SCALE)
        {
            w.boxes.insert(*id, BoxCollider { center: [p.pos[0], p.pos[1], p.pos[2] + cz], half, solid: true });
            w.player_box.insert(*id, cz);
        }
    }
    for (id, p) in &w.players {
        if let (Some(c), Some(off)) = (w.boxes.get_mut(id), w.player_box.get(id)) {
            c.center = [p.pos[0], p.pos[1], p.pos[2] + off];
            c.solid = p.state != State::Dead;
        }
    }
}

/// The AI ticks of this frame.
pub(super) fn ai(sim: &mut Sim, dt: Dt) {
    let playing = sim.playing();
    sim.ai.advance(dt.t, playing);
    let Sim { world: w, level, rng, calls, game, lions, ai, pov, touched, wasp_hits, events, warn, .. } = sim;
    // Wasp hits (EVENT_IMPACT on the lion, §5.1) are applied as they arrive.
    for h in wasp_hits.iter() {
        if let Some(e) = w.enemies.get_mut(&h.target) {
            crate::enemies::apply_impact(e, &Impact { poison: h.poison, bounce_force: h.bounce_force, wasp_pan: h.wasp_pan, wasp_disp: h.wasp_disp });
        }
    }
    // Distances and line of sight are measured to `pov_Syn` (lion.wdl:948-950).
    let nodes: Vec<crate::enemies::PathNode> = w.path_nodes.values().cloned().collect();
    let platforms: Vec<(Id, [f32; 3])> = w.platforms.iter().map(|(id, p)| (*id, p.platform.delta)).collect();
    let mut pads: Vec<(Id, crate::platforms::Teleport)> = w.teleports.iter().map(|(id, p)| (*id, p.clone())).collect();
    let (sammy_entity, sammy_alive) = match w.player() {
        Some((id, p)) => (Some(id), p.state != State::Dead),
        None => (None, false),
    };
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let view = World {
        space: &space,
        sammy: pov.entity.map(|e| (e, pov.pos, pov.vel)),
        sammy_entity,
        sammy_alive,
        touched,
        nodes: &nodes,
        platforms: &platforms,
        ticks1: ai.ticks1,
    };
    let mut out = Out::default();
    for _ in 0..ai.ticks {
        for (id, e) in w.enemies.iter_mut() {
            let Some(a) = w.animators.get_mut(id) else { continue };
            let mut shared = Shared { g: lions, rng, calls, game, pads: &mut pads };
            let player = w.players.values_mut().next();
            crate::enemies::tick(e, *id, &mut a.anim, &a.model.mdl, player, &view, &mut shared, &mut out);
            if e.blinking()
                && let Some(b) = w.bodies.get_mut(id)
            {
                b.hidden = e.hidden;
            }
        }
        crate::enemies::end_tick(calls);
    }
    for (id, p) in pads {
        if let Some(pad) = w.teleports.get_mut(&id) {
            pad.busy = p.busy;
        }
    }
    events.extend(out.sounds.into_iter().map(Event::Sound));
    events.extend(out.hit_stars.into_iter().map(|(at, count)| Event::Effect(Effect::HitStars { at: Pos(at), count })));
    *warn |= out.warn;
    events.extend(out.call_rings.into_iter().map(|at| Event::CallRings { at: Pos(at) }));
}

/// Adds an enemy's collision box once it is initialised.
pub(super) fn colliders(sim: &mut Sim) {
    let w = &mut sim.world;
    for (id, e) in &w.enemies {
        if !w.boxes.contains_key(id)
            && let Some((center, half, _)) = e.collider()
        {
            w.boxes.insert(*id, BoxCollider { center, half, solid: true });
        }
    }
}

pub(super) fn start_triggers(sim: &mut Sim) {
    let w = &mut sim.world;
    for _ in 0..sim.ai.ticks {
        for tr in w.start_triggers.values_mut() {
            let mut refs: Vec<&mut crate::enemies::Enemy> = w.enemies.values_mut().collect();
            tr.tick(&mut refs);
        }
    }
}

/// Boxes follow their owners (dead enemies stop colliding); opacity; the render pose
/// between AI ticks.
pub(super) fn place(sim: &mut Sim) {
    let a = sim.ai.acc.get().clamp(0.0, 1.0);
    let w = &mut sim.world;
    for (id, e) in &w.enemies {
        if let (Some(c), Some((center, _, solid))) = (w.boxes.get_mut(id), e.collider()) {
            c.center = center;
            c.solid = solid;
        }
        let Some(b) = w.bodies.get_mut(id) else { continue };
        b.alpha = Percent(e.alpha);
        if let Some((pos, angles)) = e.pose(a) {
            b.pos = Pos(pos);
            b.angles = Angles::from_array(angles);
        }
    }
}

/// `RestartSammy` sets `AttackCall = 0` (player.wdl:894).
pub(super) fn reset_calls(sim: &mut Sim, reset: bool) {
    if reset {
        sim.calls.attack_call = 0.0;
    }
}
