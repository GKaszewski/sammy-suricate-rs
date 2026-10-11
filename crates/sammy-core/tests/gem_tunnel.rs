//! Launched and thrown gems never pass through walls: in every level, gems fly from
//! Sammy's start and every placed gem in many directions and speeds, and each step's
//! move is checked against the level (a move the same trace would stop is a tunnel).
//! Needs the game files (skipped without them): `cargo test --release -p sammy-core --test gem_tunnel`.

use sammy_core::collision::Hull;
use sammy_core::gems::{Fate, FlyingGem};
use sammy_core::input::Controls;
use sammy_core::levels::LEVELS;
use sammy_core::save::SaveGame;
use sammy_core::sim::Sim;
use sammy_core::time::{Secs, Ticks};

mod common;

#[test]
fn gems_stay_inside_the_level() {
    let Some(files) = common::files() else {
        eprintln!("skipped: no game data");
        return;
    };
    let mut tunnels = 0;
    for &(slot, file) in LEVELS {
        let mut sim = Sim::new(SaveGame { readonly: true, ..Default::default() }, 1);
        sim.request_load(file);
        for f in 0..3 {
            sim.tick(if f == 0 { Secs(0.0) } else { Secs(1.0 / 60.0) }, &Controls::default(), files);
        }
        sim.drain_events().for_each(drop);
        let mut starts: Vec<[f32; 3]> = sim.world.players.values().map(|p| p.pos.get()).collect();
        starts.extend(sim.world.key_gems.keys().filter_map(|id| sim.world.bodies.get(id)).map(|b| b.pos.to_array()));
        let mut rng = sammy_core::rng::Rng::seeded(7);
        let jump = sim.jump;
        let mut bad = 0;
        for (si, &start) in starts.iter().enumerate() {
            for k in 0..24 {
                let pan = k as f32 * 15.0;
                let (hs, vz) = [(0.0, 10.0), (20.0, 20.0), (40.0, 5.0), (10.0, 30.0)][k % 4];
                let mut gems = [FlyingGem::launched(start, pan, hs, vz, Fate::Stays, &jump), FlyingGem::thrown(start, pan, 20.0, 0.0)];
                for (gi, g) in gems.iter_mut().enumerate() {
                    let mut collectable = false;
                    for step in 0..2000 {
                        let before = g.pos;
                        let s = g.step(collectable, &jump, &mut rng, Ticks(1.0), |a, b| sim.trace_static(Hull::Point, a, b).trace, &mut Vec::new());
                        collectable |= s.collectable;
                        let tr = sim.trace_static(Hull::Point, before, g.pos);
                        if tr.trace.hit() && !tr.trace.start_solid {
                            bad += 1;
                            if bad <= 3 {
                                eprintln!("level {slot} start {si} pan {pan} gem {gi} step {step}: {before:?} -> {:?} crosses at {:?} n {:?} ent {:?}", g.pos, tr.trace.end, tr.trace.normal, tr.entity);
                            }
                            break;
                        }
                        if s.done {
                            break;
                        }
                    }
                }
            }
        }
        if bad > 0 {
            eprintln!("level {slot}: {bad} tunnels from {} starts", starts.len());
        }
        tunnels += bad;
    }
    assert_eq!(tunnels, 0);
}
