//! Diamonds (`InitBonus3`/`Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey`/`Key_Hit`/`EjectOldKey`/`LaunchYouKey`, gems.wdl:40-960), picked up by
//! Sammy's scan (radius `BlobZoneScan`).

use formats::wmb::Object;

use super::world::{Light, World};
use super::{Effect, Event, Sim, Space};
use crate::collision::{Hull, Space as _};
use crate::gems::{GemGravity, HEIGHT_GEM, Surprise};
use crate::id::Id;
use crate::keys::{KeyOutcome, MASTER_KEY, light_color, take_key};
use crate::pickups::{KeySpec, NARROW_FLOOR_OFFSET, SHRINK_GONE, SHRINK_PER_TICK, SNAP_START, in_scan};
use crate::player::State;

/// Key pickup flash: start range, shrink per tick, cut-off range.
const KEY_FLASH: (f32, f32, f32) = (300.0, 15.0, 100.0);
/// Gem-mix stars burst this far above Sammy.
const MIX_HEIGHT: f32 = 40.0;
/// Floor probe depth when snapping pickups.
const FLOOR_PROBE: f32 = 2000.0;

/// A key placed in the level (or settled with `respawns`): restored at its home on each
/// respawn, at its level size.
#[derive(Clone, Debug)]
pub struct LevelKey {
    pub home: Option<[f32; 3]>,
    pub scale: [f32; 3],
}

pub(super) fn init_key(w: &mut World, id: Id, obj: &Object) {
    let spec = KeySpec::from_object(obj);
    w.key_gems.insert(id, spec.color);
    w.level_keys.insert(id, LevelKey { home: None, scale: obj.scale.map(f32::abs) });
    w.respawnable.insert(id);
    match spec.surprise {
        // Surprise gem: not collectable until its colour has been rolled.
        Some(add) => {
            w.surprises.insert(id, Surprise::new(add, obj.origin[0] + obj.origin[1]));
        }
        None => {
            w.collectable.insert(id);
        }
    }
    if spec.gravity {
        w.gem_gravity.insert(id, GemGravity { rest: HEIGHT_GEM });
    }
    if spec.hover_from_wed {
        w.hover_from_wed.insert(id);
    }
    if spec.snap {
        w.snap_to_ground.insert(id);
    }
}

/// Snap to `floor + HEIGHT_GEM` on the first frame (`sonar my,2000`).
pub(super) fn snap(sim: &mut Sim) {
    let Sim { world: w, level, .. } = sim;
    if level.is_none() {
        return;
    }
    let space = Space { level: level.as_deref(), brushes: &w.brushes, boxes: &w.boxes };
    for id in std::mem::take(&mut w.snap_to_ground) {
        let Some(b) = w.bodies.get_mut(&id) else { continue };
        let [x, y, z] = b.pos;
        let tr = space.trace(Hull::Narrow, [x, y, z + SNAP_START], [x, y, z - FLOOR_PROBE], None).trace;
        let floor = tr.end[2] - NARROW_FLOOR_OFFSET;
        if w.hover_from_wed.contains(&id)
            && let Some(g) = w.gem_gravity.get_mut(&id)
        {
            // Flag5: keep the WED height above the floor (+HeightGem) as the rest height.
            g.rest = z - floor + HEIGHT_GEM;
        }
        if tr.hit() && !tr.start_solid {
            b.pos[2] = floor + HEIGHT_GEM + w.extra_height.get(&id).copied().unwrap_or(0.0);
        }
    }
}

pub(super) fn collect(sim: &mut Sim) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    if p.state == State::Dead {
        return;
    }
    let p = p.clone();
    let near = |q: [f32; 3]| in_scan(p.pos, p.scan_radius, q);
    let w = &sim.world;
    let ready = |id: &Id| !w.shrinking.contains(id) && !w.snap_to_ground.contains(id);

    let taken: Vec<(Id, [f32; 3], crate::pickups::Diamond)> =
        w.diamonds.iter().filter(|(id, _)| ready(id)).filter_map(|(id, d)| Some((*id, w.pos(*id)?, *d))).filter(|(_, at, _)| near(*at)).collect();
    for (id, at, d) in taken {
        sim.world.shrinking.insert(id);
        sim.effect(Effect::DiamondBurst { at });
        sim.sound("Diam_son");
        if d.has_popup() {
            sim.sound("clog3");
            sim.emit(Event::Popup { at, points: d.points });
        }
        sim.add_points(d.points);
    }

    let w = &sim.world;
    let keys: Vec<Id> = w.key_gems.keys().copied().filter(|id| !w.shrinking.contains(id) && !w.snap_to_ground.contains(id) && !w.collected.contains(id)).collect();
    // All key gems vanish once the goal is reached (gems.wdl:77).
    if sim.game.goal_reached {
        for id in keys {
            if sim.world.collectable.contains(&id) {
                sim.world.shrinking.insert(id);
            }
        }
        return;
    }
    // A gem flying into a door blocks pickups (`PlayerOldKey == 999`).
    if sim.game.key_in_flight {
        return;
    }
    // Key_Hit: take the gem; mix primaries, otherwise throw the old key back out.
    for id in keys {
        let w = &mut sim.world;
        let Some(at) = w.pos(id) else { continue };
        if !w.collectable.contains(&id) || !near(at) {
            continue;
        }
        let color = w.key_gems[&id];
        let (range, rate, until) = KEY_FLASH;
        w.shrinking.insert(id);
        w.lights.insert(id, Light::flash(light_color(color), range, rate, until));
        if let Some(lk) = w.level_keys.get_mut(&id) {
            lk.home.get_or_insert(at);
        }
        let life = w.gem_lifetimes.get(&id).map_or(0.0, |l| l.ticks);
        sim.sound("ColoredGem_son");
        sim.effect(Effect::GemCatch { at, color });
        let held = sim.game.key;
        let (key, outcome) = take_key(held, color);
        sim.game.key = key;
        match outcome {
            KeyOutcome::Mixed => {
                sim.sound("GemMixing_son");
                // AnimGemMix1 end: each source primary bursts 70 stars (gems.wdl:596-605).
                sim.effect(Effect::GemMix { at: [p.pos[0], p.pos[1], p.pos[2] + MIX_HEIGHT], colors: [held, color] });
            }
            KeyOutcome::Ejected(old) => super::gems::throw_back_key(sim, old, &p, life),
            KeyOutcome::Taken => {}
        }
        break; // one gem per scan
    }
}

/// Picked up: shrink ×0.8 per tick, then gone (`Bonus3_Hit`, `ScaleDownGem`). Entities
/// that can come back (level keys, checkpoints) are hidden instead.
pub(super) fn shrink(sim: &mut Sim, dt: super::Dt) {
    let k = SHRINK_PER_TICK.powf(dt.t);
    let w = &mut sim.world;
    let mut gone = Vec::new();
    for id in &w.shrinking {
        let Some(b) = w.bodies.get_mut(id) else { continue };
        b.shrink(k);
        if b.scale[1] < SHRINK_GONE {
            gone.push(*id);
        }
    }
    for id in gone {
        if w.respawnable.contains(&id) {
            w.shrinking.remove(&id);
            w.collected.insert(id);
            if let Some(b) = w.bodies.get_mut(&id) {
                b.hidden = true;
            }
        } else {
            w.despawn(id);
        }
    }
}

/// Reset scan (id 1000) for keys: level keys come back home (or vanish for good once
/// Sammy holds the master key); runtime keys are removed.
pub(super) fn reset_keys(sim: &mut Sim, reset: bool) {
    if !reset {
        return;
    }
    let master = sim.game.key == MASTER_KEY;
    let w = &mut sim.world;
    let ids: Vec<Id> = w.key_gems.keys().copied().collect();
    for id in ids {
        match w.level_keys.get(&id) {
            Some(lk) if !master => {
                let (home, scale) = (lk.home, lk.scale);
                if let Some(b) = w.bodies.get_mut(&id) {
                    if let Some(h) = home {
                        b.pos = h;
                    }
                    b.scale = scale;
                    b.hidden = false;
                }
                w.collected.remove(&id);
                w.shrinking.remove(&id);
                // InitKey runs again: a surprise gem re-rolls (its colour keeps climbing).
                match w.surprises.get_mut(&id) {
                    Some(s) => {
                        s.rearm(&mut sim.rng);
                        w.collectable.remove(&id);
                    }
                    None => {
                        w.collectable.insert(id);
                    }
                }
            }
            _ => w.despawn(id),
        }
    }
}

impl Sim {
    /// `AddPoints`, playing `ScoreReach_son` when the goal is reached.
    pub(super) fn add_points(&mut self, points: i32) {
        if self.game.add_points(points) {
            self.sound("ScoreReach_son");
        }
    }
}
