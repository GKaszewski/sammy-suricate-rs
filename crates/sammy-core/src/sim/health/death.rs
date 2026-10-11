//! Dying, respawning and the checkpoint reset.

use super::*;

/// Hazards under Sammy's feet and running out of health start a death.
pub(in crate::sim) fn detect_deaths(sim: &mut Sim, dt: Dt) {
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

pub(in crate::sim) fn run_death(sim: &mut Sim, dt: Dt) {
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
            DeathEvent::DropKey(l) => crate::sim::gems::launch(sim, l),
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

pub(super) fn set_state(sim: &mut Sim, me: Id, state: State) {
    if let Some(p) = sim.world.players.get_mut(&me) {
        p.state = state;
    }
}

/// Reset scan for checkpoints (`RestartWaitForRestart`): taken ones come back.
pub(in crate::sim) fn reset_checkpoints(sim: &mut Sim, reset: bool) {
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
