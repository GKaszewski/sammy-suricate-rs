//! Events and the frame's advance: call rings, popups, the lose title, the sun.

use sammy_core::space::Pos;

use super::*;

impl Extras {
    /// Takes a game event (call rings and popups start, a new level clears).
    pub fn event(&mut self, sim: &Sim, e: &Event) {
        match *e {
            Event::Loaded { .. } => {
                *self = Self { wasp_pan: self.wasp_pan, view: self.view, ..Self::default() };
                if sim.flow.slot == Some(sammy_core::levels::EXTRO_SLOT) {
                    self.sun[0] = EXTRO_SUN_PAN;
                }
            }
            Event::CallRings { at } => {
                for (dz, delay, skin) in sammy_core::enemies::CALL_RINGS {
                    self.calls.push(CallRing { delay, scale: CALL_START_SCALE, frame: 1.0, origin: [at[0], at[1], at[2] + dz], skin });
                }
            }
            Event::Popup { at, points } => {
                let Some(&(_, file, life)) = POPUPS.iter().find(|p| p.0 == points) else { return };
                let pos = [at[0], at[1], at[2] + POPUP_LIFT];
                // Pan toward the camera, tilted up (`InvDirCamera`).
                let cam = self.view.pos;
                let pan = (cam[1] - pos[1]).atan2(cam[0] - pos[0]);
                let tilt = POPUP_TILT_DEG.to_radians();
                let dir = [pan.cos() * tilt.cos(), pan.sin() * tilt.cos(), tilt.sin()];
                let frames = sammy_core::sim::assets::sprite_frames(file);
                self.popups.push(Popup { file, pos, dir, life, frame: 0.0, frames });
            }
            _ => {}
        }
    }

    /// Advances `secs` of game time seen from `view`; the lose title smokes into
    /// `particles` (once per particle tick).
    pub fn update(&mut self, sim: &Sim, view: View, secs: sammy_core::time::Secs, particles: &mut Particles) {
        self.view = view;
        let t = secs.ticks().get();
        self.wasp_pan += WASP_SPIN * t;
        if matches!(sim.flow.phase, Phase::Won(_) | Phase::Lost(_)) {
            self.sun[0] += SUN_SPIN_RATE * t;
        }
        self.calls.retain_mut(|r| {
            if r.delay > 0.0 {
                r.delay -= t;
                return true;
            }
            if r.scale < CALL_EASE_FROM {
                r.scale += CALL_GROW * t;
            } else {
                r.scale += (CALL_EASE_TARGET - r.scale) * CALL_EASE_RATE * t;
            }
            r.frame += CALL_FRAME_RATE * t;
            r.frame < CALL_END_FRAME && r.scale < CALL_END_SCALE
        });
        self.popups.retain_mut(|p| {
            for (c, d) in p.pos.iter_mut().zip(p.dir) {
                *c += d * POPUP_SPEED * t;
            }
            p.life -= t;
            p.frame = (p.frame + POPUP_FRAMES_PER_TICK * t).rem_euclid(p.frames.max(1) as f32);
            p.life > 0.0
        });
        // The lose title: flies in, slows, stops; camera-relative.
        if matches!(sim.flow.phase, Phase::Lost(_)) {
            let l = self.lose.get_or_insert(LoseTitle { depth: LOSE_START_DEPTH, speed: LOSE_START_SPEED, pos: view.pos });
            if l.depth < LOSE_STOP_DEPTH {
                l.depth += l.speed * t;
                l.speed = (l.speed - LOSE_DECEL * t).max(LOSE_MIN_SPEED);
            }
            let (f, u) = (view.forward(), view.up());
            l.pos = [0, 1, 2].map(|i| view.pos[i] + f[i] * l.depth + u[i] * LOSE_RAISE);
            for _ in 0..particles.ticks {
                crate::effects::tile_smoke(particles, Pos(l.pos));
            }
        } else {
            self.lose = None;
        }
    }
}
