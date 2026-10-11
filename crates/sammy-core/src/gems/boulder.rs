//! Boulders that launch gems when hit.

use super::*;
use crate::keys::KeyColor;
use crate::time::Ticks;

/// A gem to launch (`EjectNewKey`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Launch {
    pub color: KeyColor,
    pub pos: [f32; 3],
    pub pan: f32,
    pub hspeed: f32,
    pub vz: f32,
    pub fate: Fate,
}

/// Boulder emitting gems (`InitBoulder`, gems.wdl:1568-1657).
#[derive(Clone, Debug)]
pub struct Boulder {
    pub(super) fate: Fate,
    pub(super) vz: f32,
    /// Id whose impact makes it emit ([`WASP_ID`] = Sammy's wasps).
    pub(super) trigger: f32,
    /// Skill6: gem colour, [`RANDOM_COLOR`] = random 1..6.
    pub(super) color: f32,
    pub(super) hspeed: f32,
    /// Skill8: ticks between automatic gems (0 = impact only).
    pub(super) period: f32,
    pub(super) timer: Ticks,
    pub(super) spread: f32,
    pub(super) pan: f32,
    pub(super) pos: [f32; 3],
    pub(super) impact: bool,
}

/// Scan id of Sammy's wasps.
pub const WASP_ID: f32 = 9999.0;
/// Boulder Skill6: a random colour per gem.
pub(super) const RANDOM_COLOR: f32 = 999.0;
/// Ticks before a boulder's first automatic gem (plus its period).
pub(super) const BOULDER_START: f32 = 48.0;
/// Automatic gems leave this far above the boulder; impact gems 50.
pub(super) const AUTO_HEIGHT: f32 = 150.0;
pub(super) const IMPACT_HEIGHT: f32 = 50.0;

impl Boulder {
    pub fn from_object(obj: &Object) -> Self {
        // Flag3..6 widen the launch spread by 20, 45, 90 and 180 degrees.
        let spread = [(3, 20.0), (4, 45.0), (5, 90.0), (6, 180.0)].iter().filter(|(n, _)| obj.flag(*n)).map(|(_, d)| d).sum();
        Self {
            fate: Fate::from_skill(obj.skill(1)),
            vz: obj.skill(4),
            trigger: obj.skill(5),
            color: obj.skill(6),
            hspeed: obj.skill(7),
            period: obj.skill(8),
            timer: Ticks(BOULDER_START + obj.skill(8)),
            spread,
            pan: obj.angles[0],
            pos: obj.origin,
            impact: !obj.flag(2),
        }
    }

    pub(super) fn color(&self, rng: &mut Rng) -> u32 {
        if self.color == RANDOM_COLOR { rng.random(6.0) as u32 + 1 } else { self.color as u32 }
    }

    /// Impact mode: a wasp hitting the boulder launches a gem back toward the shooter.
    pub fn hit_by_wasp(&self, wasp_pan: f32, wasp_disp: [f32; 3], rng: &mut Rng) -> Option<Launch> {
        if !self.impact || self.trigger != WASP_ID {
            return None;
        }
        let pan = wasp_pan - 180.0 + rng.random(self.spread) - self.spread * 0.5;
        let hspeed = (wasp_disp[0] * self.hspeed).abs() + (wasp_disp[1] * self.hspeed).abs();
        let color = self.color(rng);
        let pos = [self.pos[0] + wasp_disp[0], self.pos[1] + wasp_disp[1], self.pos[2] + IMPACT_HEIGHT];
        Some(Launch { color: KeyColor(color), pos, pan, hspeed, vz: self.vz, fate: self.fate })
    }

    /// Auto mode: a gem every Skill8 ticks.
    pub fn step(&mut self, rng: &mut Rng, t: Ticks) -> Option<Launch> {
        if self.period <= 0.0 {
            return None;
        }
        self.timer -= t;
        if self.timer > Ticks::ZERO {
            return None;
        }
        self.timer += Ticks(self.period);
        let pan = self.pan + rng.random(self.spread) - self.spread * 0.5;
        let hspeed = self.hspeed + rng.random(3.0);
        let vz = self.vz + rng.random(4.0);
        let color = self.color(rng);
        let pos = [self.pos[0], self.pos[1], self.pos[2] + AUTO_HEIGHT];
        Some(Launch { color: KeyColor(color), pos, pan, hspeed, vz, fate: self.fate })
    }
}
