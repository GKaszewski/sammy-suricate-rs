//! A fired wasp in flight.

use super::*;
use crate::time::Ticks;

/// A flying wasp.
#[derive(Clone, Debug)]
pub struct Wasp {
    pub kind: u32,
    pub poison: f32,
    pub pan: f32,
    pub tilt: f32,
    pub pos: [f32; 3],
    pub(super) speed: f32,
    pub homing_range: f32,
    pub tilt_gain: f32,
    pub bounce_force: f32,
    pub(super) life: f32,
    /// Type 5/6 climb, hold and ease timers and dive rate.
    pub(super) climb: Option<[f32; 4]>,
    /// Last frame's displacement (lions read it as knockback).
    pub last_disp: [f32; 3],
    /// 16-tick cooldown of the fear tag (`FindEnnemy`).
    pub find_cd: f32,
    /// The thrower: ignored by the wasp's MOVE (`YOU = shooter`, bullets.wdl:377).
    pub(super) shooter: Option<Id>,
    pub visible: bool,
}

/// Result of a wasp step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WaspStep {
    Flying,
    /// Burst against something (the entity hit, if any) or at the end of its life.
    Burst { hit: Option<Id> },
}

impl Wasp {
    pub fn new(shot: Shot, shooter: Option<Id>) -> Self {
        let spec = FIRED[idx(shot.kind)];
        let climb = CLIMB.iter().find(|(k, _)| *k == shot.kind).map(|(_, c)| *c);
        Self {
            kind: shot.kind,
            poison: shot.poison,
            pan: shot.pan,
            tilt: 0.0,
            pos: shot.pos,
            speed: spec.speed,
            homing_range: spec.homing_range,
            tilt_gain: spec.tilt_gain,
            bounce_force: spec.bounce_force,
            life: spec.life,
            climb,
            last_disp: [0.0; 3],
            find_cd: 0.0,
            shooter,
            visible: false,
        }
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Homing only runs once the climb/hold/ease phase is over.
    pub fn can_home(&self) -> bool {
        self.homing_range > 0.0 && self.climb.is_none_or(|c| c[2] <= 0.0)
    }

    /// One frame.
    pub fn step(&mut self, space: &impl Space, sammy: Option<[f32; 3]>, t: Ticks) -> WaspStep {
        let t = t.get();
        // Types 5/6: climb, hold, ease to level, then a slow dive (HandleVspeed, bullets.wdl:922-948).
        if let Some(mut c) = self.climb {
            if c[0] > 0.0 {
                c[0] -= t;
                self.tilt += CLIMB_RATE * t;
            } else if c[1] > 0.0 {
                c[1] -= t;
            } else if c[2] > 0.0 && self.tilt > 1.0 {
                c[2] -= t;
                self.tilt -= self.tilt * (0.2 * t).min(0.2);
                if c[2] <= 0.0 {
                    self.tilt = 0.0;
                }
            } else {
                self.tilt -= c[3] * t;
            }
            self.climb = Some(c);
        }
        let dir = forward(self.pan, self.tilt);
        let target = [0, 1, 2].map(|i| self.pos[i] + dir[i] * self.speed * t);
        let hit = space.trace(Hull::Point, self.pos, target, self.shooter);
        self.last_disp = [0, 1, 2].map(|i| hit.trace.end[i] - self.pos[i]);
        self.pos = hit.trace.end;
        if !self.visible && sammy.is_some_and(|s| (0..3).map(|i| (s[i] - self.pos[i]).powi(2)).sum::<f32>().sqrt() > SHOW_DISTANCE) {
            self.visible = true;
        }
        self.life -= t;
        self.find_cd -= t;
        if hit.trace.hit() || hit.trace.start_solid {
            WaspStep::Burst { hit: hit.entity }
        } else if self.life <= 0.0 {
            WaspStep::Burst { hit: None }
        } else {
            WaspStep::Flying
        }
    }
}
