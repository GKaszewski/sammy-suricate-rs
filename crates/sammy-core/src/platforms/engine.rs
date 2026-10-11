//! Platform engines: movement along their path.

use super::*;
use crate::keys::KeyColor;
use crate::time::Ticks;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EngineState {
    Moving,
    Paused(Ticks),
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub link: f32,
    pub pos: [f32; 3],
    pub(super) angles: [f32; 3],
    /// Skill2: quants per tick along local +X.
    pub(super) speed: f32,
    /// Skill3: pause after an impact.
    pub(super) pause: f32,
    /// Skill4/5/7: pan/tilt/roll change on impact.
    pub(super) turn: [f32; 3],
    /// Flag3: reflect off walls instead of turning.
    pub(super) reflect: bool,
    /// Skill6: key colour that unlocks it (0 = always running); Flag7 = exact colour.
    pub(super) lock: KeyColor,
    pub(super) lock_exact: bool,
    pub unlocked: bool,
    pub(super) state: EngineState,
    /// Motion this frame.
    pub delta: [f32; 3],
}

impl Engine {
    pub fn from_object(obj: &Object) -> Self {
        Self {
            link: if obj.skill(1) == 0.0 { DEFAULT_LINK } else { obj.skill(1) },
            pos: obj.origin,
            angles: obj.angles,
            speed: obj.skill(2),
            pause: obj.skill(3),
            turn: [obj.skill(4), obj.skill(5), obj.skill(7)],
            reflect: obj.flag(3),
            lock: KeyColor(obj.skill(6) as u32),
            lock_exact: obj.flag(7),
            unlocked: obj.skill(6) == 0.0,
            state: EngineState::Moving,
            delta: [0.0; 3],
        }
    }

    /// Whether `key` starts a locked engine.
    pub fn unlocks(&self, key: KeyColor) -> bool {
        key.fits(self.lock, self.lock_exact)
    }

    /// One frame; `trace` sweeps a point through the level (the engine is a tiny sprite).
    pub fn step(&mut self, boosted: bool, t: Ticks, trace: impl Fn([f32; 3], [f32; 3]) -> Trace) {
        let speed = if boosted { self.speed * BOOST_FACTOR } else { self.speed };
        self.delta = [0.0; 3];
        if !self.unlocked {
            return;
        }
        self.state = match self.state {
            EngineState::Paused(left) if left - t > Ticks(0.0) => EngineState::Paused(left - t),
            EngineState::Paused(_) => EngineState::Moving,
            EngineState::Moving => {
                let dir = crate::angle::forward(self.angles[0], self.angles[1]);
                let target = [0, 1, 2].map(|i| self.pos[i] + dir[i] * speed * t);
                let tr = trace(self.pos, target);
                if tr.start_solid {
                    EngineState::Moving
                } else {
                    self.delta = [0, 1, 2].map(|i| tr.end[i] - self.pos[i]);
                    self.pos = tr.end;
                    if !tr.hit() {
                        EngineState::Moving
                    } else if self.reflect {
                        // `To_angle Angles,BOUNCE`: face along the reflected direction.
                        let n = tr.normal;
                        let d = dir[0] * n[0] + dir[1] * n[1] + dir[2] * n[2];
                        let r = [0, 1, 2].map(|i| dir[i] - 2.0 * d * n[i]);
                        self.angles[0] = r[1].atan2(r[0]).to_degrees();
                        EngineState::Moving
                    } else {
                        // `moteurBonus_Hit`; roll uses the *new tilt* (original bug, mm1.wdl:2591).
                        self.angles[0] = ang(self.angles[0] + self.turn[0]);
                        self.angles[1] = ang(self.angles[1] + self.turn[1]);
                        self.angles[2] = ang(self.angles[1] + self.turn[2]);
                        if self.pause > 0.0 { EngineState::Paused(Ticks(self.pause)) } else { EngineState::Moving }
                    }
                }
            }
        };
    }
}
