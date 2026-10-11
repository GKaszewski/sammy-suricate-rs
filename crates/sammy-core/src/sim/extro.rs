//! The camera target `pov_Syn` and the id-25 scanners, the extro's scripted triggers
//! and fade-out ([`crate::extro`]), and the help signs ([`crate::titles`]) they trigger.

use super::{Dt, Event, Sim};
use crate::extro::{Body, FADE_SPEED, RESUME_LEVEL, TriggerKind, Wanted, cut_camera, scripted_anim};
use crate::id::Id;
use crate::input::Controls;
use crate::levels::MENU_FILE;
use crate::player::State;

/// The fade starts at `FADE_START` and the extro ends at `FADE_FULL` (fog percent).
const FADE_START: f32 = 1.0;
pub const FADE_FULL: f32 = 100.0;

/// Who the camera follows (Sammy, the intro's walker, or a retargeted enemy) and the
/// frame's scanners.
pub(super) fn update_pov(sim: &mut Sim) {
    let w = &sim.world;
    sim.scanners.0.clear();
    let sammy: Option<Body> = w.player().map(|(id, p)| (id, p.pos, p.pan, p.vel));
    if let Some((_, pos, ..)) = sammy {
        sim.scanners.0.push(pos);
    }
    // The intro's scripted Sammy scans id 25 (camera zones, hint title) while it runs.
    let walker: Option<Body> = w.walkers.iter().next().map(|(id, wk)| (*id, wk.pos, wk.pan, wk.vel));
    if let (Some((_, pos, ..)), true) = (walker, sim.intro.active) {
        sim.scanners.0.push(pos);
    }
    sim.scanners.0.extend(w.enemies.values().filter(|e| e.scans_25()).map(|e| e.pos));
    let target: Option<Body> = sim.pov.target.and_then(|t| w.enemies.get(&t).map(|e| (t, e))).filter(|(_, e)| !e.gone()).map(|(t, e)| (t, e.pos, e.pan, e.vel));
    sim.pov.resolve(sammy, walker, target);
}

pub(super) fn run_triggers(sim: &mut Sim) {
    for _ in 0..sim.trigger_clock.ticks {
        let mut started = Vec::new();
        for (id, left) in sim.world.start_extro_pov.iter_mut() {
            *left -= 1.0;
            if *left <= 0.0 {
                started.push(*id);
            }
        }
        for id in started {
            sim.pov.target = Some(id);
            sim.world.start_extro_pov.remove(&id);
        }
        let ids: Vec<Id> = sim.world.extro_triggers.keys().copied().collect();
        for me in ids {
            let w = &mut sim.world;
            let Some(tr) = w.extro_triggers.get_mut(&me) else { continue };
            if !tr.scans() {
                continue;
            }
            // First matching entity in the sphere.
            let hit: Option<(Option<Id>, [f32; 3])> = match tr.wanted() {
                Wanted::Sammy => w.players.values().next().filter(|p| tr.within(p.pos)).map(|p| (None, p.pos)),
                Wanted::Enemy { class, spec } => w
                    .enemies
                    .iter()
                    .find(|(_, e)| e.class() as f32 == class && !e.gone() && spec.is_none_or(|s| e.spec_id() as f32 == s) && tr.within(e.pos))
                    .map(|(id, e)| (Some(*id), e.pos)),
            };
            let Some((target, at)) = hit else { continue };
            match tr.kind {
                TriggerKind::PovSyn => {
                    sim.pov.target = target;
                    let p = &sim.camera.params;
                    let cam = cut_camera(at, p.back, p.dist, p.height);
                    sim.camera.follow.snap_to(cam);
                }
                TriggerKind::AnimPlayer => {
                    let scene = tr.scene();
                    if let Some((&pe, pl)) = w.players.iter_mut().next() {
                        pl.state = State::Scripted;
                        if let Some(a) = w.animators.get_mut(&pe) {
                            scene.start(&mut a.anim);
                        }
                        w.scripted.insert(pe, scene.salut);
                    }
                }
                TriggerKind::FinExtro => sim.fin_extro = Some(FADE_START),
            }
            // All shipped triggers are one-shot (Skill4 = 0).
            sim.world.despawn(me);
        }
    }
}

/// Sammy in a scripted animation: `salut` waves once, then loops "loopjambes".
pub(super) fn scripted_anims(sim: &mut Sim) {
    let w = &mut sim.world;
    for (id, salut) in w.scripted.iter_mut() {
        if let Some(a) = w.animators.get_mut(id) {
            *salut = scripted_anim(*salut, &mut a.anim);
        }
    }
}

/// `SetFinExtro`: fog +6/tick to 100, then back to the menu.
pub(super) fn fade_out(sim: &mut Sim, dt: Dt) {
    let Some(f) = sim.fin_extro else { return };
    let f = (f + FADE_SPEED * dt.t).min(FADE_FULL);
    sim.fin_extro = Some(f);
    if f >= FADE_FULL {
        sim.fin_extro = None;
        // `Branch MainMenu` (Extro.wdl:152); Play then resumes level 20 (Extro.wdl:114).
        sim.save.last_level = RESUME_LEVEL;
        sim.emit(Event::Saved);
        sim.pending_load = Some(MENU_FILE.to_owned());
    }
}

/// Help signs: shown near any id-25 scanner (Intro2.wdl:547-556), dismissed by Enter.
pub(super) fn titles(sim: &mut Sim, controls: &Controls, dt: Dt) {
    let w = &mut sim.world;
    for (id, title) in w.titles.iter_mut() {
        let nearest = sim.scanners.nearest(title.center).map(|(_, d)| d);
        let shown = title.step(nearest, controls.confirm, dt.t);
        if let Some(b) = w.bodies.get_mut(id) {
            b.hidden = !shown;
            if shown {
                b.alpha = title.alpha;
            }
        }
    }
}
