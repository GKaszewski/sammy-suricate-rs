//! Sammy (`InitMarche`, player.wdl): his entity and the frame's movement step.

use formats::wmb::Object;

use super::{Dt, Event, Sim, Space};
use crate::id::Id;
use crate::input::Controls;
use crate::player::{Frame, Player, SCALE, StepOut, Touch};

pub(super) fn init(sim: &mut Sim, id: Id, obj: &mut Object) {
    obj.scale = [SCALE; 3];
    let mut p = Player::from_object(obj);
    if let Some((pos, pan)) = sim.debug.start {
        p = Player::at(pos, pan.unwrap_or(p.pan), p.speed, p.scan_radius);
    }
    sim.world.players.insert(id, p);
}

pub(super) fn step(sim: &mut Sim, controls: &Controls, dt: Dt) {
    let Sim { world: w, level, game, jump, tuning, flow, events, touched, .. } = sim;
    if level.is_none() {
        return;
    }
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    let touch = |e: Id| {
        if w.key_gems.contains_key(&e) {
            Touch::Gem
        } else if w.enemies.contains_key(&e) {
            Touch::Enemy
        } else if w.platforms.contains_key(&e) || w.hives.contains_key(&e) {
            Touch::Pusher
        } else {
            Touch::Other
        }
    };
    for (&me, p) in w.players.iter_mut() {
        let frame = Frame {
            space: &space,
            controls,
            // Input only counts while the level is being played (`PartieEncours == 1`).
            input: flow.playing(),
            won: matches!(flow.phase, crate::flow::Phase::Won(_)),
            jump,
            tuning,
            me,
            touch: &touch,
            t: dt.t,
        };
        let mut out = StepOut::default();
        p.step(game, w.animators.get_mut(&me).map(|a| &mut a.anim), &frame, &mut out);
        events.extend(out.sounds.into_iter().map(Event::Sound));
        touched.extend(out.touched);
        if let Some(b) = w.bodies.get_mut(&me) {
            b.pos = p.pos;
            b.angles = p.angles();
        }
    }
}
