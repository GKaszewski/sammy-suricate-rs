//! Hives and the wasps they launch.

use super::*;
use crate::angle::Degrees;
use crate::time::Ticks;

/// A hive emitting wasps.
#[derive(Clone, Debug)]
pub struct Hive {
    /// Skill1: launched wasp fate (1001 = frozen at spawn).
    pub(super) fate: f32,
    /// Skill2: random pan spread.
    pub(super) spread: f32,
    /// int(Skill3): wasps left ([`UNLIMITED`] when 0); frac×100: cap on Sammy's ammo.
    pub(super) total: f32,
    pub(super) cap: i32,
    /// Skill4 / Skill7: vertical and horizontal launch speed.
    pub(super) vz: f32,
    pub(super) hspeed: f32,
    /// Skill6: wasp type ([`RANDOM_KIND`] = random 1-6).
    pub(super) kind: u32,
    /// Skill8: emission period (0 = never).
    pub(super) period: f32,
    pub(super) timer: Ticks,
    pub(super) pan: f32,
    pub pos: [f32; 3],
}

pub(super) const UNLIMITED: f32 = 9990.0;
pub(super) const DEFAULT_CAP: i32 = 10;
pub(super) const RANDOM_KIND: u32 = 999;
/// `waitt 32` before the first period.
pub(super) const HIVE_START: f32 = 32.0;
/// Hive wasp pickup table (`initSkinScaleWaspHive2`): (poison, scale) by type.
pub(super) const HIVE_PICKUP: [(f32, f32); 7] = [(3.0, 1.0), (2.0, 1.0), (1.0, 0.8), (1.0, 0.8), (4.0, 1.0), (5.0, 1.0), (1.0, 0.8)];
/// Wing animation of a hive wasp (percent per tick).
pub const HIVE_WASP_ANIM: f32 = 20.0;

/// A wasp a hive launches.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HiveLaunch {
    pub kind: u32,
    pub pan: Degrees,
    pub tilt: Degrees,
    pub scale: f32,
    /// Horizontal and vertical launch speed.
    pub speed: [f32; 2],
    pub fate: f32,
}

impl Hive {
    pub fn from_object(obj: &Object) -> Self {
        let s3 = obj.skill(3);
        let cap = ((s3 - s3.trunc()) * 100.0 + 0.5).trunc() as i32;
        let total = s3.trunc();
        Self {
            fate: obj.skill(1),
            spread: obj.skill(2),
            total: if total == 0.0 { UNLIMITED } else { total },
            cap: if cap == 0 { DEFAULT_CAP } else { cap },
            vz: obj.skill(4),
            hspeed: obj.skill(7),
            kind: obj.skill(6) as u32,
            period: obj.skill(8),
            timer: Ticks(HIVE_START + obj.skill(8)),
            pan: obj.angles[0],
            pos: obj.origin,
        }
    }

    /// One frame; returns a launch and whether the hive is now exhausted.
    pub fn step(&mut self, game: &GameState, rng: &mut Rng, t: Ticks) -> Option<(HiveLaunch, bool)> {
        if self.period <= 0.0 {
            return None;
        }
        self.timer -= t;
        if self.timer > Ticks::ZERO {
            return None;
        }
        self.timer += Ticks(self.period);
        if game.ammo >= self.cap && game.ammo_type == self.kind {
            return None; // capped: the hive looks empty
        }
        let kind = if self.kind == RANDOM_KIND { rng.random(6.0) as u32 + 1 } else { self.kind.clamp(1, KINDS) };
        let pan = if self.spread != 0.0 { self.pan + rng.random(self.spread) - rng.random(self.spread * 0.5) } else { self.pan };
        let launch = [self.hspeed + rng.random(5.0), self.vz + rng.random(4.0)];
        self.total -= 1.0;
        let tilt = rng.random(20.0) + 20.0;
        let l = HiveLaunch { kind, pan: Degrees(pan), tilt: Degrees(tilt), scale: HIVE_PICKUP[idx(kind)].1, speed: launch, fate: self.fate };
        Some((l, self.total <= 0.0))
    }
}

/// A wasp launched by a hive (`LaunchNewWasp`), bouncing until it settles into a pickup.
#[derive(Clone, Debug)]
pub struct HiveWasp {
    pub kind: u32,
    pub pos: [f32; 3],
    pub pan: Degrees,
    pub tilt: Degrees,
    pub(super) speed: f32,
    pub(super) vz: f32,
    pub(super) vz_org: f32,
    /// Bounce counter; starts at the fate (they share a skill slot, original quirk).
    pub(super) bounces: f32,
    pub(super) fate: f32,
}

/// Frozen hive wasps never move.
pub(super) const FROZEN: f32 = 1001.0;

impl HiveWasp {
    pub fn new(l: &HiveLaunch, pos: [f32; 3]) -> Self {
        Self {
            kind: l.kind,
            pos,
            pan: l.pan,
            tilt: l.tilt,
            speed: (l.speed[0].abs() + 5.0).max(15.0),
            vz: l.speed[1],
            vz_org: l.speed[1],
            bounces: l.fate,
            fate: l.fate,
        }
    }

    /// `Particle_WaspHive` trail while it still rises (`BlobVspeedOrg > 5`, bullets.wdl:1139).
    pub fn trail(&self) -> Option<(u32, [f32; 3])> {
        (self.vz_org > 5.0).then_some((self.kind, self.pos))
    }

    /// One frame; `trace` sweeps a point through the level and map entities. Returns the
    /// pickup it settles into (and that pickup's lifetime, if any).
    pub fn step(&mut self, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, rng: &mut Rng, t: Ticks, sounds: &mut Vec<&'static str>) -> Option<(AmmoPickup, Option<f32>)> {
        if self.fate == FROZEN {
            return None;
        }
        let ground = {
            let tr = trace(self.pos, [self.pos[0], self.pos[1], self.pos[2] - 2000.0]);
            if tr.hit() { tr.end[2] } else { self.pos[2] - 2000.0 }
        };
        if self.vz == 0.0 && self.pos[2] - ground > 26.0 {
            self.vz = -2.0;
        }
        let mut vstep = 0.0;
        if self.vz != 0.0 {
            self.vz -= 1.5 * t;
            if self.pos[2] < ground + 25.0 {
                self.bounces += 1.0;
                if self.speed > 0.5 {
                    if self.vz_org > 15.0 {
                        sounds.push(burst_sound(rng));
                    }
                    self.vz_org *= 0.8;
                    self.vz = self.vz_org - 1.5 * t;
                    vstep = self.vz * t;
                } else {
                    vstep = ground + 25.0 - self.pos[2];
                    self.vz = 0.0;
                }
            } else {
                vstep = self.vz * t;
            }
        }
        self.tilt -= if self.bounces == 0.0 { Degrees(2.0 * t) } else { Degrees(self.tilt.0 * 0.1 * t) };
        let (s, c) = self.pan.radians().sin_cos();
        let d = [c * self.speed * t.min(Ticks(1.0)), s * self.speed * t.min(Ticks(1.0)), vstep];
        let hit = trace(self.pos, [self.pos[0] + d[0], self.pos[1] + d[1], self.pos[2] + d[2]]);
        let before = self.pos;
        self.pos = hit.end;
        if hit.hit() && hit.normal[2].abs() < 0.7 {
            // Wasp_Bounce: turn off the wall, lose speed.
            let n = hit.normal;
            self.pan = Degrees(n[1].atan2(n[0]).to_degrees());
            self.speed = (self.speed - 8.0).max(0.1);
        }
        self.speed -= 0.05 * t.min(Ticks(1.0));
        let moved = (self.pos[0] - before[0]).abs() + (self.pos[1] - before[1]).abs();
        let mut settled = false;
        if moved < 1.0 {
            self.vz_org -= 0.5 * t.min(Ticks(1.0));
            settled |= self.vz_org.abs() < 1.5;
        } else {
            self.vz_org -= 0.02 * t.min(Ticks(1.0));
        }
        if self.speed < 1.0 {
            self.speed = 1.0;
            settled |= self.vz_org.abs() < 1.5;
        }
        settled.then(|| {
            let pickup = AmmoPickup { kind: self.kind, amount: 1, poison: HIVE_PICKUP[idx(self.kind)].0 };
            let lifetime = matches!(Fate::from_skill(self.fate), Fate::Lifetime(_)).then_some(self.fate + self.bounces);
            (pickup, lifetime)
        })
    }
}
