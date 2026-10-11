//! The gem flying into a permanent door.

use super::*;
use crate::time::Ticks;
use crate::keys::KeyColor;

/// Gem flying from Sammy into a permanent door (`AnimGemKey`, gems.wdl:1043-1233):
/// phase 1 at [`SPEED_GEM_KEY`] for ≤80 ticks, phase 2 closes 70 % of the gap per tick
/// for ≤16 ticks; a recalled gem flies back to Sammy.
#[derive(Clone, Debug)]
pub struct DoorGem {
    pub color: KeyColor,
    pub pos: [f32; 3],
    pub(super) phase: DoorGemPhase,
    pub(super) timer: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum DoorGemPhase {
    Approach,
    Final,
    Return,
}

/// The gem aims this high above the door's origin.
pub(super) const DOOR_AIM_HEIGHT: f32 = 100.0;
/// The gem leaves this high above Sammy.
pub(super) const DOOR_GEM_START: f32 = 40.0;
pub(super) const APPROACH_TICKS: f32 = 80.0;
pub(super) const FINAL_TICKS: f32 = 16.0;

/// What the door says about the gem.
#[derive(Clone, Copy, Debug)]
pub struct DoorGemTarget {
    pub door_pos: [f32; 3],
    /// The door still waits for it (Skill14).
    pub wanted: bool,
    pub validated: bool,
}

pub(super) fn move_toward(pos: &mut [f32; 3], to: [f32; 3], dist_step: f32) -> f32 {
    let d = [to[0] - pos[0], to[1] - pos[1], to[2] - pos[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > 1e-3 {
        let k = (dist_step / len).min(1.0);
        for i in 0..3 {
            pos[i] += d[i] * k;
        }
    }
    len
}

pub(super) fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

impl DoorGem {
    pub fn new(color: KeyColor, sammy: [f32; 3]) -> Self {
        Self { color: color, pos: [sammy[0], sammy[1], sammy[2] + DOOR_GEM_START], phase: DoorGemPhase::Approach, timer: APPROACH_TICKS }
    }

    /// One frame: `Some(true)` arrived (validate the door), `Some(false)` back with Sammy
    /// (return the key), `None` still flying.
    pub fn fly(&mut self, target: DoorGemTarget, sammy: [f32; 3], t: Ticks) -> Option<bool> {
        let t = t.get();
        let aim = [target.door_pos[0], target.door_pos[1], target.door_pos[2] + DOOR_AIM_HEIGHT];
        match self.phase {
            DoorGemPhase::Approach => {
                let dist = move_toward(&mut self.pos, aim, SPEED_GEM_KEY * t);
                self.timer -= t;
                if dist <= 20.0 || !target.wanted || self.timer <= 0.0 || target.validated {
                    self.phase = DoorGemPhase::Final;
                    self.timer = FINAL_TICKS;
                }
                None
            }
            DoorGemPhase::Final => {
                let len = distance(self.pos, aim);
                move_toward(&mut self.pos, aim, len * 0.7 * t);
                self.timer -= t;
                let dist = distance(self.pos, aim);
                if dist <= 16.0 || self.timer <= 0.0 {
                    if dist <= 20.0 && target.wanted {
                        return Some(true);
                    }
                    self.phase = DoorGemPhase::Return;
                    self.timer = APPROACH_TICKS;
                }
                None
            }
            DoorGemPhase::Return => {
                let home = [sammy[0], sammy[1], sammy[2] + DOOR_AIM_HEIGHT];
                let len = distance(self.pos, home);
                self.timer -= t;
                if len <= 32.0 || self.timer <= 0.0 {
                    return Some(false);
                }
                let v = if len > 128.0 { 40.0 * t } else { len * 0.9 * t };
                move_toward(&mut self.pos, home, v);
                None
            }
        }
    }
}
