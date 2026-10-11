//! Effects from the game's events and state.

use super::*;

// ---------------------------------------------------------------- the game's effects

use sammy_core::player::State;
use sammy_core::sim::{Effect, Event, Sim};

/// Idle emitters further than this from the camera target are skipped (the original
/// relied on its frame budget instead).
pub(super) const IDLE_RANGE: f32 = 3000.0;
/// Sammy's teleport sparkle is this high above his origin.
pub(super) const TELEPORT_SPARKLE_HEIGHT: f32 = 20.0;

/// The game's one-shot effects; a new level starts without particles.
pub fn play(ps: &mut Particles, e: &Event) {
    match *e {
        Event::Loaded { .. } => ps.list.clear(),
        Event::Effect(fx) => match fx {
            Effect::GemExplode { at, color } => gem_explode(ps, at, color),
            Effect::DiamondBurst { at } => diamond_burst(ps, at),
            Effect::GemCatch { at, color } => gem_catch(ps, at, color),
            Effect::GemMix { at, colors } => {
                for c in colors {
                    gem_mix(ps, at, c);
                }
            }
            Effect::SmallHeart { at } => small_heart(ps, at),
            Effect::GetLife { at, delay } => get_life(ps, at, delay),
            Effect::WaspExplode { at, kind, spread } => wasp_explode(ps, at, kind, spread),
            Effect::HiveBurst { at } => hive_burst(ps, at),
            Effect::HitStars { at, count } => hit_stars(ps, at, count),
            Effect::TakeEye { at } => take_eye(ps, at),
        },
        _ => {}
    }
}

/// Per-tick emitters on the game's entities (docs/particles.md §3).
pub fn emit_trails(ps: &mut Particles, game: &Sim) {
    let w = &game.world;
    let pov = game.pov.pos;
    let near = |p: [f32; 3]| (0..3).map(|i| (p[i] - pov[i]).powi(2)).sum::<f32>() < IDLE_RANGE * IDLE_RANGE;
    let pos = |id: &sammy_core::id::Id| w.bodies.get(id).map(|b| b.pos);
    for _ in 0..ps.ticks {
        for wasp in w.wasps.values() {
            wasp_trail(ps, wasp.pos, wasp.kind, 1.0);
        }
        for e in w.escapees.values() {
            wasp_trail(ps, e.pos, e.kind, sammy_core::wasps::ESCAPE_SCALE);
        }
        for hw in w.hive_wasps.values() {
            if let Some((kind, p)) = hw.trail() {
                hive_wasp_trail(ps, p, kind);
            }
        }
        for e in w.enemies.values() {
            if e.gone() {
                continue;
            }
            if e.class() == WIN_WASP_CLASS {
                // "You Win" wasp: a burst from a random vertex (bullets.wdl:1810-1821).
                let at = [e.pos[0] + ps.random(120.0) - 60.0, e.pos[1] + ps.random(120.0) - 60.0, e.pos[2] + ps.random(80.0) - 40.0];
                win_sparkle(ps, at);
            } else if (WASP_SPEC_BASE + 1..=WASP_SPEC_BASE + 7).contains(&e.wasp_spec) {
                wasp_trail(ps, e.pos, e.wasp_spec - WASP_SPEC_BASE, 1.0);
            }
        }
        for id in w.flying_gems.keys() {
            if let (Some(at), Some(color)) = (pos(id), w.key_gems.get(id)) {
                gem_trail(ps, at, *color);
            }
        }
        for (id, g) in &w.door_gems {
            if let Some(at) = pos(id) {
                gem_trail(ps, at, g.gem.color);
            }
        }
        // Death tiles every 2 ticks, lava 1/9 of that (mm1.wdl:4033,4878); Flag3 = no smoke.
        for (id, lava) in w.death_tiles.iter().map(|id| (id, false)).chain(w.lava_tiles.iter().map(|id| (id, true))) {
            let (Some(at), Some(obj)) = (pos(id), w.objects.get(id)) else { continue };
            if obj.flags & FLAG3 != 0 || !near(at) {
                continue;
            }
            if ps.random(if lava { 18.0 } else { 2.0 }) < 1.0 {
                tile_smoke(ps, at);
            }
        }
        // Idle teleport pads (Flag3 != 1), 1/4 per tick (mm1.wdl:4109).
        for id in w.teleports.keys() {
            let (Some(at), Some(obj)) = (pos(id), w.objects.get(id)) else { continue };
            if obj.flags & FLAG3 == 0 && near(at) && ps.random(4.0) < 1.0 {
                teleport_sparkle(ps, at, false, 1);
            }
        }
        for id in w.checkpoints.keys().filter(|id| !w.collected.contains(id)) {
            if let Some(at) = pos(id)
                && near(at)
                && ps.random(4.0) < 1.0
            {
                restart_sparkle(ps, at);
            }
        }
        // Boost arrows: 2 stars at mesh vertices 3 and 11, 1/6 per tick (mm1.wdl:4360-4362).
        for id in w.boosts.keys() {
            let Some(b) = w.bodies.get(id) else { continue };
            if !b.hidden && near(b.pos) && ps.random(6.0) < 1.0 {
                for side in [-1.0, 1.0] {
                    let at = [b.pos[0] + side * 12.0, b.pos[1] + ps.random(16.0) - 8.0, b.pos[2] + 8.0];
                    boost_sparkle(ps, at);
                }
            }
        }
        // The Eye of Truth sparkles while it waits.
        for (id, eye) in &w.eyes {
            if eye.item.idle()
                && !w.snap_to_ground.contains(id)
                && let Some(at) = pos(id)
            {
                eye_idle(ps, at);
            }
        }
    }
}

/// Sammy's dust, boost stars and teleport sparkle (docs/particles.md §3.5); `prev` is
/// his state and teleporting last frame.
pub fn emit_sammy(ps: &mut Particles, game: &Sim, prev: &mut Option<(State, bool)>) {
    let Some((me, p)) = game.world.player() else { return };
    let teleporting = game.world.teleporting.contains_key(&me);
    let ground = [p.pos[0], p.pos[1], p.floor];
    let fwd = (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt();
    let (was, was_tp) = prev.unwrap_or((p.state, teleporting));
    let above = [p.pos[0], p.pos[1], p.pos[2] + TELEPORT_SPARKLE_HEIGHT];
    // Landing: one big puff on a hard landing, a smaller one landing on the move.
    if p.state == State::Land && was != State::Land {
        if p.hard_landing {
            dust(ps, ground, DUST_HARD_LANDING, fwd);
        } else if fwd > 1.0 && ps.random(2.0) < 1.0 {
            dust(ps, ground, DUST_WALK, fwd);
        }
    }
    if teleporting && !was_tp {
        teleport_sparkle(ps, above, true, 5);
    }
    *prev = Some((p.state, teleporting));
    for _ in 0..ps.ticks {
        // `ticks1 % TicksPoussiere` sampled per frame: walk 4 → ≈ f/4, turn 8 → ≈ f/8 per tick.
        let (chance, target) = match p.state {
            State::Walk => (F / 4.0, DUST_WALK),
            State::Turn => (F / 8.0, DUST_TURN),
            State::Land => (1.0, DUST_WALK),
            _ => (0.0, 0.0),
        };
        if chance > 0.0 && ps.random(1.0) < chance {
            dust(ps, ground, target, fwd);
        }
        // `LessBoost`: every 2 frames while boosting.
        if p.boosting {
            let n = (F / 2.0) as usize + (ps.random(1.0) < (F / 2.0).fract()) as usize;
            for _ in 0..n {
                speedy(ps, p.pos);
            }
        }
        if teleporting {
            teleport_sparkle(ps, above, true, 2);
        }
    }
}
