//! Sammy firing wasps, and the queued shots of a fan.

use super::*;
use crate::time::Ticks;

/// Shots still to fire in a fan: (ticks left, pan offset), from one spawn point.
#[derive(Clone, Debug)]
pub struct PendingShots {
    pub(super) shots: Vec<(Ticks, f32)>,
    pub(super) spawn: [f32; 3],
}

/// A wasp to launch.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shot {
    pub kind: u32,
    pub poison: f32,
    pub pos: [f32; 3],
    pub pan: f32,
}

/// Result of a firing step.
#[derive(Clone, Debug, Default)]
pub struct Fire {
    /// Fire pressed without ammo (`Error1`).
    pub empty: bool,
    /// The throw released a new fan (it starts launching next frame).
    pub thrown: Option<PendingShots>,
}

/// `PlayerTire`: firing from a normal ground state, the throw animation, then the
/// wasps (types 3 and 4 throw a 3-wasp fan, 2 ticks apart, costing ammo each).
pub fn fire(p: &mut Player, anim: &mut Anim, game: &mut GameState, trigger: bool, playing: bool) -> Fire {
    let mut out = Fire::default();
    let kind = game.ammo_type.clamp(1, KINDS);
    if trigger && playing && matches!(p.state, State::Stand | State::Walk | State::Turn) {
        if game.ammo <= 0 {
            out.empty = true;
        } else {
            p.state = State::FireA;
            anim.play("TirA", false, 0.0);
            anim.set_speed_pct(THROW_ANIM * FIRE_FACTOR[idx(kind)], TICKS_PER_SEC);
        }
    }
    if p.state == State::FireA && anim.finished() {
        let (s, c) = (p.pan.0 + 180.0).to_radians().sin_cos();
        let spawn = [p.pos[0] + c * SPAWN_BEHIND, p.pos[1] + s * SPAWN_BEHIND, p.pos[2] + SPAWN_HEIGHT];
        game.ammo -= 1;
        let shots = if matches!(kind, 3 | 4) {
            let mut shots = vec![(Ticks::ZERO, -5.0)];
            for (i, off) in [0.0, 5.0].into_iter().enumerate() {
                game.ammo -= 1;
                if game.ammo < 0 {
                    break;
                }
                shots.push((FAN_SPACING * (i + 1) as f32, off));
            }
            shots
        } else {
            vec![(Ticks::ZERO, 0.0)]
        };
        if game.ammo <= 0 {
            game.ammo = 0;
            game.ammo_type = 0;
            game.ammo_poison = 0.0;
        }
        out.thrown = Some(PendingShots { shots, spawn });
        p.state = State::FireB;
        anim.play("TirB", false, 10.0);
        anim.set_speed_pct(THROW_ANIM, TICKS_PER_SEC);
    } else if p.state == State::FireB && anim.finished() {
        p.state = State::Stand;
    }
    out
}

impl PendingShots {
    /// Releases due shots (type as of the frame's start, poison at least 1); true once
    /// the fan is done.
    pub fn release(&mut self, kind: u32, poison: f32, pan: f32, t: Ticks, out: &mut Vec<Shot>) -> bool {
        let spawn = self.spawn;
        self.shots.retain_mut(|(left, off)| {
            *left -= t;
            if *left > Ticks(0.0) {
                return true;
            }
            out.push(Shot { kind, poison: poison.max(1.0), pos: spawn, pan: pan + *off });
            false
        });
        self.shots.is_empty()
    }
}
