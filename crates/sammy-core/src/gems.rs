//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5):
//! thrown-back keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` /
//! `LaunchNewKey`: boulders, Sammy's death gem), lifetimes, gravity for Flag7 gems,
//! surprise colours, boulders, and the gem that flies into a permanent door
//! (`CreateGemKeyAnim` / `AnimGemKey`).

use formats::trace::Trace;
use formats::wmb::Object;

use crate::physics::JumpParams;
use crate::rng::Rng;

/// `HeightGem`: resting height of a gem above the floor.
pub const HEIGHT_GEM: f32 = 25.0;
/// `GemVerticalSpeed` / `GemVerticalFriction` (InitLvl.wdl:63-64).
pub const GEM_VSPEED: f32 = 10.0;
pub const GEM_VFRIC: f32 = 2.5;
/// `SpeedGemKey`: gem flying into a door, quants per tick.
pub const SPEED_GEM_KEY: f32 = 12.0;
/// A thrown key leaves 50 above Sammy and never rises more than 32 above that.
pub const THROW_HEIGHT: f32 = 50.0;
const THROW_CEILING: f32 = 32.0;
/// Bounce from which a flying gem can be picked up.
const COLLECTABLE_BOUNCE: u32 = 5;
/// Floor search depth below a gem.
const FLOOR_PROBE: f32 = 2000.0;
/// Ticks between "last good position" snapshots (escape when stuck in a wall).
const GOOD_POS_PERIOD: f32 = 16.0;
/// Bounce sounds by bounce number (1..4, the 4th repeats).
const BOUNCE_SOUNDS: [&str; 4] = ["ding6", "ding5", "ding3", "ding7"];
/// Lifetime blink: animation speed it starts at (frames per tick) and gains per blink.
pub const BLINK_START_SPEED: f32 = 3.0;
pub const BLINK_SPEEDUP: f32 = 0.1;

/// Skill1 of launched gems (and wasps): what becomes of them once they land.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fate {
    /// 0: decorative, shrinks and bursts, never collectable.
    Decorative,
    /// 1..999: collectable for that many ticks.
    Lifetime(f32),
    /// 1000: stays.
    Stays,
    /// 1001: stays and comes back after a respawn (like a level-placed key).
    Respawns,
}

impl Fate {
    pub fn from_skill(v: f32) -> Self {
        match v {
            v if v <= 0.0 => Self::Decorative,
            1000.0 => Self::Stays,
            1001.0 => Self::Respawns,
            v if v < 1000.0 => Self::Lifetime(v),
            _ => Self::Stays,
        }
    }
}

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
    pan: f32,
    speed: f32,
    vz: f32,
    vz_org: f32,
    fric: f32,
    bounces: u32,
    /// Thrown keys never rise above their launch height + 32.
    ceiling: f32,
    stuck: f32,
    last_good: [f32; 3],
    good_timer: f32,
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

    fn new(kind: FlightKind, pos: [f32; 3], pan: f32, speed: f32, vz: f32, fric: f32, ceiling: f32) -> Self {
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
    pub fn step(&mut self, collectable: bool, jump: &JumpParams, rng: &mut Rng, t: f32, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, sounds: &mut Vec<&'static str>) -> FlightStep {
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

/// Remaining lifetime (Skill9) of a runtime gem; counts down once it rests.
#[derive(Clone, Debug)]
pub struct GemLifetime {
    pub ticks: f32,
    /// Blink: shown small during the second half of each animation cycle.
    pub small: bool,
    /// Blink animation speed (frames per tick), rising each blink.
    pub speed: f32,
}

impl GemLifetime {
    pub fn new(ticks: f32) -> Self {
        Self { ticks, small: false, speed: BLINK_START_SPEED }
    }

    /// Counts down; true when the gem bursts (`GemExplose`).
    pub fn tick(&mut self, t: f32) -> bool {
        self.ticks -= t;
        self.ticks <= 0.0
    }
}

/// Flag7 gem: falls at `Gravite` per tick down to its rest height, rides platforms.
#[derive(Clone, Copy, Debug)]
pub struct GemGravity {
    pub rest: f32,
}

impl GemGravity {
    /// New height for a gem at `z` above `floor`.
    pub fn fall(&self, z: f32, floor: f32, jump: &JumpParams, t: f32) -> f32 {
        if z > floor + self.rest { (z + jump.gravity * t).max(floor + self.rest) } else { floor + self.rest }
    }
}

/// Surprise gem (Skill5): not collectable for 1-8 ticks, then its colour climbs.
#[derive(Clone, Debug)]
pub struct Surprise {
    pub add: f32,
    delay: f32,
}

/// Highest colour a surprise gem climbs to (green).
const SURPRISE_MAX_COLOR: u32 = 6;

impl Surprise {
    /// Armed with a 1-8 tick delay (derived from the gem until the reset scan re-rolls it).
    pub fn new(add: f32, seed: f32) -> Self {
        Self { add, delay: (seed.abs() as u32 % 8) as f32 + 1.0 }
    }

    /// Re-roll on respawn (the colour keeps climbing, original quirk).
    pub fn rearm(&mut self, rng: &mut Rng) {
        self.delay = rng.random(8.0).trunc() + 1.0;
    }

    /// Counts down; returns the new colour when it becomes collectable.
    pub fn step(&mut self, color: u32, rng: &mut Rng, t: f32) -> Option<u32> {
        if self.delay <= 0.0 {
            return None;
        }
        self.delay -= t;
        (self.delay <= 0.0).then(|| (color + rng.random(self.add) as u32 + 1).min(SURPRISE_MAX_COLOR))
    }
}

/// A gem to launch (`EjectNewKey`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Launch {
    pub color: u32,
    pub pos: [f32; 3],
    pub pan: f32,
    pub hspeed: f32,
    pub vz: f32,
    pub fate: Fate,
}

/// Boulder emitting gems (`InitBoulder`, gems.wdl:1568-1657).
#[derive(Clone, Debug)]
pub struct Boulder {
    fate: Fate,
    vz: f32,
    /// Id whose impact makes it emit ([`WASP_ID`] = Sammy's wasps).
    trigger: f32,
    /// Skill6: gem colour, [`RANDOM_COLOR`] = random 1..6.
    color: f32,
    hspeed: f32,
    /// Skill8: ticks between automatic gems (0 = impact only).
    period: f32,
    timer: f32,
    spread: f32,
    pan: f32,
    pos: [f32; 3],
    impact: bool,
}

/// Scan id of Sammy's wasps.
pub const WASP_ID: f32 = 9999.0;
/// Boulder Skill6: a random colour per gem.
const RANDOM_COLOR: f32 = 999.0;
/// Ticks before a boulder's first automatic gem (plus its period).
const BOULDER_START: f32 = 48.0;
/// Automatic gems leave this far above the boulder; impact gems 50.
const AUTO_HEIGHT: f32 = 150.0;
const IMPACT_HEIGHT: f32 = 50.0;

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
            timer: BOULDER_START + obj.skill(8),
            spread,
            pan: obj.angles[0],
            pos: obj.origin,
            impact: !obj.flag(2),
        }
    }

    fn color(&self, rng: &mut Rng) -> u32 {
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
        Some(Launch { color, pos, pan, hspeed, vz: self.vz, fate: self.fate })
    }

    /// Auto mode: a gem every Skill8 ticks.
    pub fn step(&mut self, rng: &mut Rng, t: f32) -> Option<Launch> {
        if self.period <= 0.0 {
            return None;
        }
        self.timer -= t;
        if self.timer > 0.0 {
            return None;
        }
        self.timer += self.period;
        let pan = self.pan + rng.random(self.spread) - self.spread * 0.5;
        let hspeed = self.hspeed + rng.random(3.0);
        let vz = self.vz + rng.random(4.0);
        let color = self.color(rng);
        let pos = [self.pos[0], self.pos[1], self.pos[2] + AUTO_HEIGHT];
        Some(Launch { color, pos, pan, hspeed, vz, fate: self.fate })
    }
}

/// Gem flying from Sammy into a permanent door (`AnimGemKey`, gems.wdl:1043-1233):
/// phase 1 at [`SPEED_GEM_KEY`] for ≤80 ticks, phase 2 closes 70 % of the gap per tick
/// for ≤16 ticks; a recalled gem flies back to Sammy.
#[derive(Clone, Debug)]
pub struct DoorGem {
    pub color: u32,
    pub pos: [f32; 3],
    phase: DoorGemPhase,
    timer: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DoorGemPhase {
    Approach,
    Final,
    Return,
}

/// A thrown gem stuck in the air hops this far up.
const STUCK_HOP: f32 = 8.0;

/// The gem aims this high above the door's origin.
const DOOR_AIM_HEIGHT: f32 = 100.0;
/// The gem leaves this high above Sammy.
const DOOR_GEM_START: f32 = 40.0;
const APPROACH_TICKS: f32 = 80.0;
const FINAL_TICKS: f32 = 16.0;

/// What the door says about the gem.
#[derive(Clone, Copy, Debug)]
pub struct DoorGemTarget {
    pub door_pos: [f32; 3],
    /// The door still waits for it (Skill14).
    pub wanted: bool,
    pub validated: bool,
}

fn move_toward(pos: &mut [f32; 3], to: [f32; 3], dist_step: f32) -> f32 {
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

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

impl DoorGem {
    pub fn new(color: u32, sammy: [f32; 3]) -> Self {
        Self { color, pos: [sammy[0], sammy[1], sammy[2] + DOOR_GEM_START], phase: DoorGemPhase::Approach, timer: APPROACH_TICKS }
    }

    /// One frame: `Some(true)` arrived (validate the door), `Some(false)` back with Sammy
    /// (return the key), `None` still flying.
    pub fn fly(&mut self, target: DoorGemTarget, sammy: [f32; 3], t: f32) -> Option<bool> {
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

#[cfg(test)]
mod tests;
