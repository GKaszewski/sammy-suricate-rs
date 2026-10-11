//! A dead wasp falling and recoiling.

use super::*;
use crate::time::Ticks;

/// A wasp that hit a lion (`wasptat` 1004, bullets.wdl:577-644): it stays visible,
/// puffs `particle_hit1` (counts at ticks after the hit), plays a second death sound,
/// then recoils backward along its flight while the death animation plays, and
/// vanishes 4 ticks later.
#[derive(Clone, Debug)]
pub struct Corpse {
    pub kind: u32,
    pub pos: [f32; 3],
    pub(super) bounce: [f32; 3],
    /// Death animation step (percent).
    pub(super) step: f32,
    pub(super) elapsed: Ticks,
    pub(super) hidden: Ticks,
}

pub(super) const CORPSE_PUFFS: [(f32, usize); 5] = [(0.0, 10), (3.0, 15), (5.0, 10), (6.0, 5), (7.0, 5)];
pub(super) const CORPSE_SOUND_AT: f32 = 3.0;
pub(super) const RECOIL_START: Ticks = Ticks(7.0);
/// Recoil: share of the flight velocity (horizontal, vertical), clamp, damping and
/// gravity per tick.
pub(super) const RECOIL_XY: f32 = 0.7;
pub(super) const RECOIL_Z: f32 = 0.5;
pub(super) const RECOIL_MAX: f32 = 50.0;
pub(super) const RECOIL_DAMP: f32 = 0.05;
pub(super) const RECOIL_GRAVITY: f32 = 0.5;
/// The death animation runs at 6 % per tick until 99 %, then the corpse hides for 4 ticks.
pub(super) const DEATH_ANIM: f32 = 6.0;
pub(super) const DEATH_END: f32 = 99.0;
pub(super) const CORPSE_HIDDEN: Ticks = Ticks(4.0);

/// What a corpse does this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CorpseStep {
    /// `particle_hit1` puffs to emit at the corpse.
    pub puffs: usize,
    /// The second death sound.
    pub sound: bool,
    pub visible: bool,
    pub done: bool,
}

impl Corpse {
    /// From the wasp as it hit (its velocity per tick gives the recoil).
    pub fn new(w: &Wasp) -> Self {
        let v = forward(w.pan, w.tilt).map(|c| c * w.speed);
        Self { kind: w.kind, pos: w.pos, bounce: [-v[0] * RECOIL_XY, -v[1] * RECOIL_XY, -v[2] * RECOIL_Z], step: 0.0, elapsed: Ticks(0.0), hidden: Ticks(0.0) }
    }

    /// One frame; `trace` sweeps a point through the level (the corpse is passable to
    /// entities).
    pub fn step(&mut self, anim: Option<&mut Anim>, t: Ticks, trace: impl Fn([f32; 3], [f32; 3]) -> Trace) -> CorpseStep {
        let (before, now) = (self.elapsed, self.elapsed + t);
        self.elapsed = now;
        let due = |at: f32| Ticks(at) >= before && Ticks(at) < now;
        let mut out = CorpseStep { visible: true, ..Default::default() };
        out.puffs = CORPSE_PUFFS.iter().filter(|(at, _)| due(*at)).map(|(_, n)| n).sum();
        out.sound = due(CORPSE_SOUND_AT);
        if now < RECOIL_START {
            return out;
        }
        if self.step < DEATH_END {
            if let Some(a) = anim {
                a.set_step(self.step);
            }
            self.step += DEATH_ANIM * t;
            if self.bounce[0] != 0.0 || self.bounce[1] != 0.0 {
                let mut v = self.bounce;
                if v[0].abs() > RECOIL_MAX || v[1].abs() > RECOIL_MAX {
                    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                    v = v.map(|c| c / len * RECOIL_MAX);
                }
                let to = [0, 1, 2].map(|i| self.pos[i] + v[i] * t);
                self.pos = trace(self.pos, to).end;
                let k = 1.0 - (RECOIL_DAMP * t).min(1.0);
                self.bounce = v.map(|c| c * k);
                self.bounce[2] -= RECOIL_GRAVITY * t;
            }
            return out;
        }
        self.hidden += t;
        out.visible = false;
        out.done = self.hidden >= CORPSE_HIDDEN;
        out
    }
}
