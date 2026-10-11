//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X through hidden rails; platforms and
//! bodies with the same link id copy its motion, carrying whatever stands on them.

use super::{Dt, Sim};
use crate::collision::Hull;
use crate::id::Id;
use crate::percent::Percent;
use crate::platforms::{CAMERA_SNAP, Platform, TeleportStep, UNLOCKED_PLATFORM};
use crate::player::State;
use crate::space::Pos;

/// A platform or body and the engine it follows.
#[derive(Clone, Debug)]
pub struct Linked {
    pub platform: Platform,
    pub engine: Option<Id>,
}

/// Sammy is being teleported from `pad` (input locked).
#[derive(Clone, Debug)]
pub struct Teleporting {
    pub pad: Id,
    pub state: crate::platforms::Teleporting,
}

/// Engines run, platforms follow them, riders are carried.
pub(super) fn step(sim: &mut Sim, dt: Dt) {
    link(sim);
    if sim.playing() {
        run_engines(sim, dt);
    }
    follow_engines(sim);
    ride(sim);
}

/// `SetLinkMoteur`: each new platform attaches to the engine with its link id.
fn link(sim: &mut Sim) {
    let w = &mut sim.world;
    for (id, p) in w.platforms.iter_mut().filter(|(_, p)| p.engine.is_none()) {
        let Some(brush) = w.brushes.get(id) else { continue };
        if let Some((e, eng)) = w.engines.iter().find(|(_, eng)| eng.link == p.platform.link) {
            p.engine = Some(*e);
            p.platform.attach(brush.origin, eng);
        }
    }
}

fn run_engines(sim: &mut Sim, dt: Dt) {
    let Some(level) = sim.level.clone() else { return };
    let w = &mut sim.world;
    // Shift-boost while riding: the carrying engine runs faster (InitEnginePlatForm :2478-2491).
    let boosted = w
        .player()
        .map(|(_, p)| p)
        .filter(|p| p.boosting && p.grounded)
        .and_then(|p| p.ground_entity)
        .and_then(|g| w.platforms.get(&g))
        .and_then(|pl| pl.engine);
    for (id, e) in w.engines.iter_mut() {
        e.step(boosted == Some(*id), dt.t, |a, b| level.trace(0, Hull::Point.index(), a, b));
    }
}

fn follow_engines(sim: &mut Sim) {
    let w = &mut sim.world;
    for (id, p) in w.platforms.iter_mut() {
        let Some(eng) = p.engine.and_then(|e| w.engines.get(&e)) else { continue };
        let Some(brush) = w.brushes.get_mut(id) else { continue };
        p.platform.follow(&mut brush.origin, eng);
        if let Some(b) = w.bodies.get_mut(id) {
            b.pos = Pos(brush.origin);
        }
    }
}

/// Riders move with the platform under them (`PlayerForce` = platform motion);
/// standing on a locked platform with the right key starts its engine.
fn ride(sim: &mut Sim) {
    let w = &mut sim.world;
    let Some(p) = w.players.values_mut().next() else { return };
    if !p.grounded {
        return;
    }
    let Some(ground) = p.ground_entity else { return };
    let Some(plat) = w.platforms.get(&ground) else { return };
    for (pos, d) in p.pos.iter_mut().zip(plat.platform.delta) {
        *pos += d;
    }
    let Some(eng) = plat.engine.and_then(|e| w.engines.get_mut(&e)) else { return };
    if eng.unlocked || !eng.unlocks(sim.game.key) {
        return;
    }
    eng.unlocked = true;
    sim.sound("platform1_son");
    // The platform turns into the plain wooden one (`morph <platfor1.wmb>`).
    sim.morph(ground, UNLOCKED_PLATFORM);
}

/// `Teleport_Hit`: Sammy standing on a pad is faded out, moved and faded back in.
pub(super) fn enter_teleports(sim: &mut Sim) {
    if !sim.playing() {
        return;
    }
    let w = &mut sim.world;
    let Some((me, p)) = w.player() else { return };
    if w.teleporting.contains_key(&me) || !p.grounded || p.vz != 0.0 || p.state == State::Dead {
        return;
    }
    if let Some(g) = p.ground_entity
        && let Some(pad) = w.teleports.get_mut(&g)
        && !pad.busy
    {
        pad.busy = true;
        let state = crate::platforms::Teleporting::new(pad);
        w.teleporting.insert(me, Teleporting { pad: g, state });
    }
}

pub(super) fn run_teleports(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    let Some((&me, p)) = w.players.iter_mut().next() else { return };
    let Some(tp) = w.teleporting.get_mut(&me) else { return };
    p.state = State::Dead; // PlayerMoveMode = 0
    match tp.state.step(dt.t) {
        TeleportStep::Fading => {}
        TeleportStep::Jump => {
            p.teleport(tp.state.dest, tp.state.pan);
            let d = tp.state.dest;
            sim.camera.follow.snap_to([d[0] + CAMERA_SNAP[0], d[1] + CAMERA_SNAP[1], d[2] + CAMERA_SNAP[2]]);
        }
        TeleportStep::Done => {
            p.state = State::Stand;
            if let Some(pad) = w.teleports.get_mut(&tp.pad) {
                pad.busy = false;
            }
            let alpha = tp.state.alpha;
            w.teleporting.remove(&me);
            if let Some(b) = w.bodies.get_mut(&me) {
                b.alpha = Percent(alpha);
            }
            return;
        }
    }
    let alpha = tp.state.alpha;
    if let Some(b) = w.bodies.get_mut(&me) {
        b.alpha = Percent(alpha);
    }
}

impl Sim {
    /// `morph`: swaps a map entity's prefab (model and collision hull).
    pub(super) fn morph(&mut self, id: Id, file: &str) {
        let Some(wmb) = self.assets.prefab(file) else { return };
        if let Some(b) = self.world.brushes.get_mut(&id) {
            b.wmb = wmb;
        }
        if let Some(l) = self.world.looks.get_mut(&id) {
            l.morph(super::Visual::Prefab { file: file.to_owned() });
        }
    }
}
