//! Doors (`InitDoor`/`movePortail`/`OuvrePortail`, mm1.wdl:2886-3283), the gem flying
//! into a permanent door, and the exit tile (`InitExit`/`TileExit_Hit`, mm1.wdl:3956-3975).

use super::world::Light;
use super::{Dt, Effect, Event, Sim};
use crate::doors::{DARK_DOOR, DoorEvent, DoorView};
use crate::gems::DoorGemTarget;
use crate::keys::KeyColor;
use crate::keys::{MASTER_KEY, light_color};
use crate::player::State;
use crate::space::Pos;

/// Door flash when a key fits: start range, shrink per tick, cut-off range.
const DOOR_FLASH: (f32, f32, f32) = (600.0, 50.0, 100.0);

pub(super) fn run(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    let (sammy, sammy_dead) = (p.pos, p.state == State::Dead);
    let blockers: Vec<[f32; 3]> = sim.world.enemies.values().filter(|e| e.alive()).map(|e| e.pos).collect();
    let view = DoorView { sammy: sammy.get(), sammy_dead, blockers: &blockers };
    let mut events = Vec::new();
    let ids: Vec<_> = sim.world.doors.keys().copied().collect();
    for id in ids {
        let w = &mut sim.world;
        let (Some(door), Some(brush)) = (w.doors.get_mut(&id), w.brushes.get_mut(&id)) else { continue };
        door.step(&mut brush.origin, &mut sim.game, &view, dt.t, &mut events);
        if let Some(b) = w.bodies.get_mut(&id) {
            b.pos = Pos(brush.origin);
        }
        for ev in events.drain(..) {
            match ev {
                DoorEvent::Sound(name) => sim.sound(name),
                DoorEvent::LaunchGem { color } => spawn_door_gem(sim, id, color, sammy.get()),
                DoorEvent::Flash { color } => {
                    let (range, rate, until) = DOOR_FLASH;
                    sim.world.lights.insert(id, Light::flash(light_color(color), range, rate, until));
                }
                DoorEvent::MorphDark => sim.morph(id, DARK_DOOR),
            }
        }
    }
}

/// `CreateGemKeyAnim`: the key flies from Sammy into the door.
fn spawn_door_gem(sim: &mut Sim, door: crate::id::Id, color: KeyColor, sammy: [f32; 3]) {
    let gem = crate::gems::DoorGem::new(color, sammy);
    let pos = gem.pos;
    if let Some(id) = super::spawn::gem_sprite(sim, "door gem", color, pos, 1.0, -super::gems::GEM_FPS) {
        sim.world.door_gems.insert(id, super::gems::DoorGemFlight { door, gem });
    }
}

/// Moves gems flying into permanent doors; arrival validates the door, a recall
/// returns the key to Sammy.
pub(super) fn run_door_gems(sim: &mut Sim, dt: Dt) {
    let Some((_, p)) = sim.world.player() else { return };
    let sammy = p.pos;
    let ids: Vec<_> = sim.world.door_gems.keys().copied().collect();
    for id in ids {
        let w = &mut sim.world;
        let Some(g) = w.door_gems.get_mut(&id) else { continue };
        let (Some(door), Some(brush)) = (w.doors.get_mut(&g.door), w.brushes.get(&g.door)) else {
            w.despawn(id);
            continue;
        };
        let target = DoorGemTarget { door_pos: brush.origin, wanted: door.gem_flying, validated: door.validated };
        match g.gem.fly(target, sammy.get(), dt.t) {
            Some(true) => {
                door.validated = true;
                let (at, color) = (g.gem.pos, g.gem.color);
                sim.game.key_in_flight = false;
                w.despawn(id);
                sim.sound("broken4");
                sim.effect(Effect::GemExplode { at: Pos(at), color });
            }
            Some(false) => {
                // Back with Sammy: the key is returned. The original also overwrote a master
                // key won during the flight (gems.wdl:1200), leaving only a colour the exit
                // refuses: a soft-lock, so the port keeps the master key.
                if sim.game.key != MASTER_KEY {
                    sim.game.key = g.gem.color;
                }
                sim.game.key_in_flight = false;
                w.despawn(id);
            }
            None => {
                let pos = g.gem.pos;
                if let Some(b) = w.bodies.get_mut(&id) {
                    b.pos = Pos(pos);
                }
            }
        }
    }
}

pub(super) fn run_exits(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    let (grounded, ground) = (p.grounded, p.ground_entity);
    let mut sounds = Vec::new();
    for (id, exit) in sim.world.exits.iter_mut() {
        let on = grounded && ground == Some(*id);
        sounds.extend(exit.step(on, &mut sim.game, dt.t));
    }
    sim.events.extend(sounds.into_iter().map(Event::Sound));
}
