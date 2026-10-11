//! Gems flying, bouncing and coming to rest (thrown keys, launched gems).

use super::*;
use crate::time::Ticks;

pub(super) const THROW_CEILING: f32 = 32.0;
/// Bounce from which a flying gem can be picked up.
pub(super) const COLLECTABLE_BOUNCE: u32 = 5;
/// Floor search depth below a gem.
pub(super) const FLOOR_PROBE: f32 = 2000.0;
/// Ticks between "last good position" snapshots (escape when stuck in a wall).
pub(super) const GOOD_POS_PERIOD: f32 = 16.0;
/// Bounce sounds by bounce number (1..4, the 4th repeats).
pub(super) const BOUNCE_SOUNDS: [&str; 4] = ["ding6", "ding5", "ding3", "ding7"];

/// A thrown gem stuck in the air hops this far up.
pub(super) const STUCK_HOP: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FlightKind {
    /// `LaunchYouKey`: a key Sammy threw back (keeps its remaining lifetime).
    Thrown { life: f32 },
    /// `LaunchNewKey`: launched by a boulder or on Sammy's death.
    Launched(Fate),
}

/// A gem in flight with damped bounces (per-frame loops of gems.wdl:784-934 / 1394-1530).
#[derive(Clone, Debug)]
pub struct FlyingGem {
    pub kind: FlightKind,
    pub pos: [f32; 3],
    pub(super) pan: f32,
    pub(super) speed: f32,
    pub(super) vz: f32,
    pub(super) vz_org: f32,
    pub(super) fric: f32,
    pub(super) bounces: u32,
    /// Thrown keys never rise above their launch height + 32.
    pub(super) ceiling: f32,
    pub(super) stuck: f32,
    pub(super) last_good: [f32; 3],
    pub(super) good_timer: f32,
}

/// How a gem rests once its flight ends.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rest {
    /// Collectable, optionally with a lifetime (ticks).
    Collectable { lifetime: Option<f32> },
    /// Collectable, floating at [`HEIGHT_GEM`] over the floor with gravity.
    Gravity { lifetime: Option<f32>, respawns: bool },
    /// Shrinks and bursts.
    Decorative,
}

/// Result of one flight step.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct FlightStep {
    /// Collectable from now on (the 5th bounce).
    pub collectable: bool,
    /// The flight is over: settle with [`FlyingGem::rest`].
    pub done: bool,
}

impl FlyingGem {
    /// `EjectOldKey` + `LaunchYouKey`: the old key flies forward at 2× Sammy's speed.
    pub fn thrown(sammy: [f32; 3], pan: f32, sammy_speed: f32, life: f32) -> Self {
        let pos = [sammy[0], sammy[1], sammy[2] + THROW_HEIGHT];
        Self::new(FlightKind::Thrown { life }, pos, pan, sammy_speed * 2.0, GEM_VSPEED, GEM_VFRIC, pos[2] + THROW_CEILING)
    }

    /// `EjectNewKey` → `LaunchNewKey`: a gem launched from `pos`.
    pub fn launched(pos: [f32; 3], pan: f32, hspeed: f32, vz: f32, fate: Fate, jump: &JumpParams) -> Self {
        Self::new(FlightKind::Launched(fate), pos, pan, (hspeed + 5.0).max(25.0), vz, jump.acc_gravity, f32::MAX)
    }

    pub(super) fn new(kind: FlightKind, pos: [f32; 3], pan: f32, speed: f32, vz: f32, fric: f32, ceiling: f32) -> Self {
        Self { kind, pos, pan, speed, vz, vz_org: vz, fric, bounces: 0, ceiling, stuck: 0.0, last_good: pos, good_timer: 0.0 }
    }

    /// Its resting behaviour after the flight.
    pub fn rest(&self) -> Rest {
        match self.kind {
            FlightKind::Thrown { life } => Rest::Collectable { lifetime: (life > 0.0).then_some(life) },
            FlightKind::Launched(Fate::Decorative) => Rest::Decorative,
            FlightKind::Launched(Fate::Lifetime(l)) => Rest::Gravity { lifetime: Some(l), respawns: false },
            FlightKind::Launched(Fate::Stays) => Rest::Gravity { lifetime: None, respawns: false },
            FlightKind::Launched(Fate::Respawns) => Rest::Gravity { lifetime: None, respawns: true },
        }
    }

    /// One frame (`t` ticks, capped at 1 like the original `mintime`). `trace` sweeps a
    /// point against the level and map entities; sounds go to `sounds`.
    pub fn step(&mut self, collectable: bool, jump: &JumpParams, rng: &mut Rng, t: Ticks, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, sounds: &mut Vec<&'static str>) -> FlightStep {
        let t = t.get();
        let mut out = FlightStep::default();
        let floor = {
            let tr = trace(self.pos, [self.pos[0], self.pos[1], self.pos[2] - FLOOR_PROBE]);
            if tr.hit() { tr.end[2] } else { self.pos[2] - FLOOR_PROBE }
        };
        let thrown = matches!(self.kind, FlightKind::Thrown { .. });
        if self.vz == 0.0 && self.pos[2] - floor > HEIGHT_GEM + 1.0 {
            self.vz = jump.gravity;
            self.fric = jump.acc_gravity;
        }
        if thrown && self.pos[2] > self.ceiling {
            self.vz = jump.gravity;
            self.fric = jump.acc_gravity;
        }
        let mut dz = 0.0;
        if self.vz != 0.0 {
            self.vz -= self.fric * t;
            if self.pos[2] < floor + HEIGHT_GEM {
                // Touchdown = one bounce; collectable from the 5th on.
                self.bounces += 1;
                let decorative = self.kind == FlightKind::Launched(Fate::Decorative);
                if self.bounces == COLLECTABLE_BOUNCE && !decorative && !collectable {
                    out.collectable = true;
                }
                let (min_speed, keep, loud) = if thrown { (2.0, 0.95, 7.0) } else { (0.5, 0.9, 5.0) };
                if self.speed > min_speed {
                    if self.vz_org > loud {
                        sounds.push(BOUNCE_SOUNDS[(self.bounces.clamp(1, 4) - 1) as usize]);
                    }
                    if thrown {
                        self.speed -= 5.0;
                    }
                    self.vz_org *= keep;
                    self.vz = self.vz_org - self.fric * t;
                    dz = self.vz * t;
                } else {
                    dz = floor + HEIGHT_GEM - self.pos[2];
                    self.vz = 0.0;
                }
            } else {
                dz = self.vz * t;
            }
        }
        self.good_timer -= t;
        if self.good_timer <= 0.0 {
            self.good_timer = GOOD_POS_PERIOD;
            self.last_good = self.pos;
        }
        let (s, c) = self.pan.to_radians().sin_cos();
        let target = [self.pos[0] + c * self.speed * t, self.pos[1] + s * self.speed * t, self.pos[2] + dz];
        let hit = trace(self.pos, target);
        let before = self.pos;
        if hit.start_solid {
            self.pos = self.last_good;
            out.done = true;
        } else {
            self.pos = hit.end;
        }
        // Gem_Hit: reflect off walls while not yet collectable.
        if hit.hit() && hit.normal[2].abs() < 0.7 && !collectable {
            let n = hit.normal;
            let d = c * n[0] + s * n[1];
            self.pan = (s - 2.0 * d * n[1]).atan2(c - 2.0 * d * n[0]).to_degrees();
            self.speed = (self.speed - 8.0).max(0.1);
        }
        let moved_z = self.pos[2] - before[2];
        if thrown {
            if dz != 0.0 && moved_z.abs() < 1e-3 {
                if self.pos[2] > floor + HEIGHT_GEM {
                    // Stuck in the air: hop off in a random direction.
                    self.stuck += 1.0;
                    if self.stuck > 24.0 {
                        out.done = true;
                    }
                    sounds.push("ding3");
                    self.speed = 30.0;
                    self.vz = GEM_VSPEED;
                    self.vz_org = GEM_VSPEED;
                    self.pan = rng.random(360.0);
                    // Port: the hop and the snap to rest height are swept too, so a gem
                    // under a low ceiling can't be set inside it (and fall out of the level).
                    self.pos = trace(self.pos, [self.pos[0], self.pos[1], self.pos[2] + STUCK_HOP]).end;
                } else {
                    self.pos = trace(self.pos, [self.pos[0], self.pos[1], floor + HEIGHT_GEM]).end;
                    out.done = true;
                }
            }
            self.speed -= 0.5 * t;
            if self.speed < 0.1 {
                self.speed = 0.1;
                if self.vz == 0.0 {
                    out.done = true;
                }
            }
        } else {
            self.speed -= 0.05 * t;
            let blocked = (self.pos[0] - before[0]).abs() + (self.pos[1] - before[1]).abs() < 1.0;
            self.vz_org -= if blocked { 0.5 } else { 0.02 } * t;
            if blocked && self.vz_org.abs() < jump.acc_gravity.abs() {
                out.done = true;
            }
            if self.speed < 1.0 {
                self.speed = 1.0;
                if self.vz_org.abs() < jump.acc_gravity.abs() {
                    out.done = true;
                }
            }
        }
        out
    }
}
