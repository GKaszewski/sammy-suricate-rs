//! Health, lives and hazards:
//! - `InitTileDeath`/`TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava`/`Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitRestart`
//!   checkpoints (mm1.wdl:3771), `InitBoost`
//! - the Eye of Truth (`InitEyeOfTruth`, mm1.wdl:4954-5103) and the warning light on
//!   Sammy when a lion starts a chase (`Warning_attack`, player.wdl:1091).

use formats::wmb::Object;

use super::world::{Light, World};
use super::{Dt, Effect, Event, Sim};
use crate::eye::{EXTRA_HEIGHT, EyePickup, EyeStage};
use crate::health::{BIG_HEART_SPIN, BOOST_AWAY_SHRINK, BOOST_HIDDEN, DeathEvent, Dying, HEART_AWAY_SHRINK, HEART_HIDDEN_BELOW, HEART_REGROW, SmallHeart, Victim};
use crate::id::Id;
use crate::pickups::in_scan;
use crate::platforms::CAMERA_SNAP;
use crate::player::State;

/// Big heart flash: colour, start range, shrink per tick, cut-off range.
const LIFE_FLASH: ([f32; 3], f32, f32, f32) = ([50.0, 250.0, 10.0], 300.0, 40.0, 64.0);
/// Boost pickup flash.
const BOOST_FLASH: ([f32; 3], f32, f32, f32) = ([150.0, 50.0, 100.0], 200.0, 10.0, 64.0);
/// One `particle_Getlife` every `GET_LIFE_EVERY` ticks, `GET_LIFE_COUNT` times
/// (mm1.wdl:2696-2706).
const GET_LIFE_COUNT: usize = 6;
const GET_LIFE_EVERY: f32 = 2.0;
/// The warning light: colour, start range (`LightRange1`); it shrinks
/// `WARNING_FADE_RATE` per tick and goes out at `WARNING_FADE_END`.
const WARNING_COLOR: [f32; 3] = [250.0, 100.0, 0.0];
pub const WARNING_RANGE: f32 = 300.0;
const WARNING_FADE_RATE: f32 = 5.0;
const WARNING_FADE_END: f32 = 100.0;

/// A pickup that ducks away and grows back to its level size.
#[derive(Clone, Debug)]
pub struct Scaled<T> {
    pub item: T,
    pub scale: [f32; 3],
}

pub(super) fn init_small_heart(w: &mut World, id: Id, obj: &Object) {
    let heart = SmallHeart::from_object(obj);
    if heart.snap {
        w.snap_to_ground.insert(id);
    }
    w.small_hearts.insert(id, Scaled { item: heart, scale: obj.scale.map(f32::abs) });
}

pub(super) fn init_eye(w: &mut World, id: Id, obj: &Object) {
    let eye = EyePickup::from_object(obj);
    if eye.snap {
        w.snap_to_ground.insert(id);
        w.extra_height.insert(id, EXTRA_HEIGHT);
    }
    w.eyes.insert(id, Scaled { item: eye, scale: obj.scale.map(f32::abs) });
}

/// Hazards under Sammy's feet and running out of health start a death.
pub(super) fn detect_deaths(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let w = &mut sim.world;
    let Some((me, p)) = w.player() else { return };
    // `Player_Kaputt` returns at once while `PlayerEtat == 3` (extro scripted Sammy).
    if w.dying.contains_key(&me) || matches!(p.state, State::Dead | State::Scripted) {
        return;
    }
    let ground = p.ground_entity.filter(|_| p.grounded && p.vz == 0.0);
    let death = ground.is_some_and(|g| w.death_tiles.contains(&g));
    let lava = ground.is_some_and(|g| w.lava_tiles.contains(&g));
    if let Some(kind) = crate::health::check_death(&mut sim.game, death, lava, dt.t) {
        w.dying.insert(me, Dying::new(kind));
    }
}

pub(super) fn run_death(sim: &mut Sim, dt: Dt) {
    let w = &mut sim.world;
    let Some((me, p)) = w.player() else { return };
    let victim = Victim { pos: p.pos, pan: p.pan, vel: p.vel };
    let Some(d) = w.dying.get_mut(&me) else { return };
    let mut events = Vec::new();
    // Opacity is applied on frames that run a fade stage (not on the frame one starts).
    let fading = d.fading();
    d.step(&mut sim.game, w.animators.get_mut(&me).map(|a| &mut a.anim), victim, dt.t, &mut events);
    let alpha = d.alpha;
    if fading && let Some(b) = w.bodies.get_mut(&me) {
        b.alpha = alpha;
    }
    for ev in events {
        match ev {
            DeathEvent::Started => set_state(sim, me, State::Dead),
            DeathEvent::DropKey(l) => super::gems::launch(sim, l),
            DeathEvent::Sound(s) => sim.sound(s),
            DeathEvent::CameraLookDown { t } => {
                let f = &mut sim.camera.follow;
                f.tilt_scale = Some(crate::health::look_down(f.tilt_scale.unwrap_or(1.0), t));
            }
            DeathEvent::GameOver => {
                sim.camera.follow.tilt_scale = Some(0.0);
                sim.camera.follow.orbit = true;
            }
            DeathEvent::Respawn(at) => {
                sim.reset_scan = true;
                if let Some((pos, pan)) = at {
                    if let Some(p) = sim.world.players.get_mut(&me) {
                        p.teleport(pos, pan);
                    }
                    sim.camera.follow.snap_to([pos[0] + CAMERA_SNAP[0], pos[1] + CAMERA_SNAP[1], pos[2] + CAMERA_SNAP[2]]);
                }
            }
            DeathEvent::Visible(v) => {
                if let Some(b) = sim.world.bodies.get_mut(&me) {
                    b.hidden = !v;
                }
            }
            DeathEvent::Finished => {
                set_state(sim, me, State::Stand);
                sim.world.dying.remove(&me);
            }
        }
    }
}

fn set_state(sim: &mut Sim, me: Id, state: State) {
    if let Some(p) = sim.world.players.get_mut(&me) {
        p.state = state;
    }
}

pub(super) fn collect(sim: &mut Sim, dt: Dt) {
    if !sim.playing() {
        return;
    }
    let Some((_, p)) = sim.world.player() else { return };
    if p.state == State::Dead {
        return;
    }
    let (sammy, radius) = (p.pos, p.scan_radius);
    let near = |q: [f32; 3]| in_scan(sammy, radius, q);
    let Sim { world: w, game, events, .. } = sim;
    let shrinking = w.shrinking.clone();

    for (id, h) in w.small_hearts.iter_mut() {
        if shrinking.contains(id) || w.snap_to_ground.contains(id) {
            continue;
        }
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if h.item.away.is_some() || !near(at) {
            continue;
        }
        if !crate::health::take_small_heart(&mut h.item, game) {
            continue; // full: duck away and come back
        }
        events.push(Event::Sound("Gagne1"));
        events.push(Event::Sound("Chimes2"));
        events.push(Event::Effect(Effect::SmallHeart { at }));
        w.shrinking.insert(*id);
    }
    for (id, b) in &w.big_hearts {
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if shrinking.contains(id) || !near(at) || !crate::health::take_big_heart(b, game) {
            continue;
        }
        events.push(Event::Sound("Getlife_son"));
        for i in 0..GET_LIFE_COUNT {
            events.push(Event::Effect(Effect::GetLife { at, delay: i as f32 * GET_LIFE_EVERY }));
        }
        let (color, range, rate, until) = LIFE_FLASH;
        w.shrinking.insert(*id);
        w.lights.insert(*id, Light::flash(color, range, rate, until));
    }
    for (id, b) in w.boosts.iter_mut() {
        if shrinking.contains(id) {
            continue;
        }
        let Some(body) = w.bodies.get_mut(id) else { continue };
        if let Some(left) = b.item.away {
            // Ducked away because boost was full: shrink, hide, grow back.
            let left = left - dt.t;
            if left > BOOST_HIDDEN {
                body.shrink(BOOST_AWAY_SHRINK.powf(dt.t));
            } else {
                body.hidden = true;
            }
            if left <= 0.0 {
                body.hidden = false;
                body.scale = b.scale;
                b.item.away = None;
            } else {
                b.item.away = Some(left);
            }
            continue;
        }
        if !near(body.pos) || !crate::health::take_boost(&mut b.item, game) {
            continue;
        }
        events.push(Event::Sound("chimes2"));
        let (color, range, rate, until) = BOOST_FLASH;
        w.shrinking.insert(*id);
        w.lights.insert(*id, Light::flash(color, range, rate, until));
    }
    for (id, c) in &w.checkpoints {
        let Some(at) = w.bodies.get(id).map(|b| b.pos) else { continue };
        if shrinking.contains(id) || w.collected.contains(id) || !near(at) {
            continue;
        }
        events.push(Event::Sound("Restart_Son"));
        crate::health::take_checkpoint(c, game);
        w.shrinking.insert(*id);
    }
}

/// Small hearts that ducked away (health was full) shrink, wait and grow back.
pub(super) fn animate_hearts(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let w = &mut sim.world;
    for (id, h) in w.small_hearts.iter_mut() {
        let Some(left) = h.item.away else { continue };
        if w.shrinking.contains(id) {
            continue;
        }
        let Some(b) = w.bodies.get_mut(id) else { continue };
        let left = left - t;
        if left > 0.0 {
            b.shrink(HEART_AWAY_SHRINK.powf(t));
            if b.scale[1] < HEART_HIDDEN_BELOW {
                b.hidden = true;
            }
            h.item.away = Some(left);
        } else {
            b.hidden = false;
            let k = HEART_REGROW.powf(t);
            b.scale = std::array::from_fn(|i| (b.scale[i].max(HEART_HIDDEN_BELOW) * k).min(h.scale[i]));
            if b.scale[1] >= h.scale[1] {
                h.item.away = None;
            }
        }
    }
}

/// Big hearts spin; checkpoints spin when Flag3 is set.
pub(super) fn spin(sim: &mut Sim, dt: Dt) {
    let t = dt.t;
    let w = &mut sim.world;
    for id in w.big_hearts.keys() {
        if w.checkpoints.contains_key(id) {
            continue;
        }
        if let Some(b) = w.bodies.get_mut(id) {
            b.angles[0] += BIG_HEART_SPIN * t;
        }
    }
    for (id, c) in w.checkpoints.iter_mut() {
        if w.big_hearts.contains_key(id) {
            continue;
        }
        if let Some(spin) = c.spin {
            for (a, s) in c.angles.iter_mut().zip(spin) {
                *a += s * t;
            }
            if let Some(b) = w.bodies.get_mut(id) {
                b.angles = c.angles;
            }
        }
    }
}

/// Reset scan for checkpoints (`RestartWaitForRestart`): taken ones come back.
pub(super) fn reset_checkpoints(sim: &mut Sim, reset: bool) {
    if !reset {
        return;
    }
    let w = &mut sim.world;
    for (id, c) in &w.checkpoints {
        if w.collected.remove(id)
            && let Some(b) = w.bodies.get_mut(id)
        {
            b.pos = c.home;
            b.scale = [1.0; 3];
            b.hidden = false;
        }
    }
}

/// `Warning_attack`: an orange light on Sammy when a lion starts a chase, fading out.
pub(super) fn warning_light(sim: &mut Sim, dt: Dt) {
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
pub(super) fn run_eye(sim: &mut Sim, dt: Dt) {
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
            b.angles = eye.item.angles;
        }
        let reach = playing && !dead && in_scan(sammy, radius, b.pos);
        match eye.item.step(game, reach, t) {
            EyeStage::Idle => {}
            EyeStage::Taken => {
                events.push(Event::Sound("EyeTruth_Son"));
                events.push(Event::Effect(Effect::TakeEye { at: b.pos }));
            }
            EyeStage::Shrinking => b.shrink(crate::eye::SHRINK_RATE.powf(t)),
            EyeStage::Hidden => b.hidden = true,
            EyeStage::Back => {
                b.hidden = false;
                b.scale = eye.scale;
            }
        }
    }
}
