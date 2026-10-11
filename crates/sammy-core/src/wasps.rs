//! Sammy's wasps (docs/wasps.md): ammo pickups (`InitAmmo1`, bullets.wdl:21-177),
//! firing (`PlayerTire`, player.wdl:271-384), flight and hits (`InitWasp` / `Wasp` /
//! `Wasp_Hit`, bullets.wdl:203-747), weak doors (`InitDoorWeak`, mm1.wdl:4466-4715),
//! their planks, and hives (`InitHive` / `EmitWasp`, bullets.wdl:1197-1321).

use formats::trace::Trace;
use formats::wmb::Object;

use crate::anim::Anim;
use crate::collision::{Hull, Space};
use crate::game::GameState;
use crate::gems::Fate;
use crate::id::Id;
use crate::player::{Player, State};
use crate::rng::Rng;
use crate::time::TICKS_PER_SEC;

/// Wasp types 1..=7 (orange, yellow, blue, green, magenta, red, white).
pub const KINDS: u32 = 7;

/// Index into the per-type tables of a wasp type (1..=7).
fn idx(kind: u32) -> usize {
    (kind.clamp(1, KINDS) - 1) as usize
}

/// A pickup's poison and size by type (docs/wasps.md §1.3).
struct PickupSpec {
    poison: f32,
    scale: Option<f32>,
}

const PICKUPS: [PickupSpec; 7] = [
    PickupSpec { poison: 3.0, scale: None },
    PickupSpec { poison: 2.0, scale: Some(0.8) },
    PickupSpec { poison: 1.0, scale: Some(0.65) },
    PickupSpec { poison: 1.0, scale: Some(0.65) },
    PickupSpec { poison: 4.0, scale: Some(0.8) },
    PickupSpec { poison: 6.0, scale: Some(0.8) },
    PickupSpec { poison: 1.0, scale: Some(0.65) },
];

/// A fired wasp by type (docs/wasps.md §3).
#[derive(Clone, Copy, Debug)]
pub struct FiredSpec {
    pub scale: f32,
    /// Quants per tick.
    pub speed: f32,
    /// Homing range (0 = flies straight).
    pub homing_range: f32,
    pub tilt_gain: f32,
    /// Knockback multiplier on a lion.
    pub bounce_force: f32,
    /// Ticks before it bursts.
    pub life: f32,
}

pub const FIRED: [FiredSpec; 7] = [
    FiredSpec { scale: 1.0, speed: 32.0, homing_range: 500.0, tilt_gain: 0.1, bounce_force: 1.0, life: 128.0 },
    FiredSpec { scale: 0.7, speed: 100.0, homing_range: 0.0, tilt_gain: 0.0, bounce_force: 2.0, life: 128.0 },
    FiredSpec { scale: 0.5, speed: 100.0, homing_range: 0.0, tilt_gain: 0.0, bounce_force: 2.0, life: 128.0 },
    FiredSpec { scale: 0.5, speed: 100.0, homing_range: 500.0, tilt_gain: 0.1, bounce_force: 2.0, life: 128.0 },
    FiredSpec { scale: 0.7, speed: 36.0, homing_range: 700.0, tilt_gain: 0.3, bounce_force: 3.0, life: 128.0 },
    FiredSpec { scale: 1.0, speed: 38.4, homing_range: 1000.0, tilt_gain: 0.2, bounce_force: 2.0, life: 200.0 },
    FiredSpec { scale: 0.5, speed: 100.0, homing_range: 0.0, tilt_gain: 0.0, bounce_force: 2.0, life: 128.0 },
];

/// "TirA" speed factor by type (player.wdl:287-314).
const FIRE_FACTOR: [f32; 7] = [1.0, 2.2, 1.5, 1.2, 1.2, 1.0, 3.0];
/// Throw animation speed (percent per tick, × the type factor for "TirA").
const THROW_ANIM: f32 = 20.0;
/// A fired wasp's wings flap at half its speed (percent per tick).
pub const WING_ANIM: f32 = 0.5;
/// The wasp leaves this far behind Sammy and this high.
const SPAWN_BEHIND: f32 = 40.0;
const SPAWN_HEIGHT: f32 = 50.0;
/// Types that throw a 3-wasp fan: offsets (degrees) and 2-tick spacing.
const FAN_SPACING: f32 = 2.0;
/// Invisible until this far from Sammy (it starts behind him).
const SHOW_DISTANCE: f32 = 80.0;
/// Burst sounds, picked at random.
pub const BURST_SOUNDS: [&str; 3] = ["WaspKaputt_son", "WaspKaputt2_son", "WaspKaputt3_son"];
/// Type 5/6 climb, hold and ease phases (ticks) and dive rate (degrees per tick).
const CLIMB: [(u32, [f32; 4]); 2] = [(5, [3.0, 5.0, 5.0, 0.2]), (6, [4.0, 8.0, 10.0, 0.1])];
const CLIMB_RATE: f32 = 20.0;

/// The light flashed when a pickup is taken, by type.
pub const AMMO_COLORS: [[f32; 3]; 7] =
    [[250.0, 100.0, 0.0], [250.0, 250.0, 0.0], [0.0, 0.0, 250.0], [0.0, 250.0, 0.0], [250.0, 0.0, 250.0], [250.0, 0.0, 0.0], [150.0, 150.0, 150.0]];

pub fn burst_sound(rng: &mut Rng) -> &'static str {
    BURST_SOUNDS[(rng.random(3.0) as usize).min(2)]
}

/// An ammo pickup.
#[derive(Clone, Copy, Debug)]
pub struct AmmoPickup {
    pub kind: u32,
    pub amount: i32,
    pub poison: f32,
}

/// How a WED ammo pickup looks.
#[derive(Clone, Copy, Debug)]
pub struct AmmoLook {
    /// Model skin (the type).
    pub skin: u32,
    /// Overrides the WED scale.
    pub scale: Option<f32>,
}

/// Idle pickups lean 20° and spin 1° per tick.
pub const PICKUP_TILT: f32 = 20.0;
pub const PICKUP_SPIN: f32 = 1.0;
/// Model animation speed of a pickup.
pub const PICKUP_ANIM: f32 = 3.0;

impl AmmoPickup {
    /// `InitAmmo1`: Skill1 type, Skill2 amount, Skill3 poison, Skill7 skin.
    pub fn from_object(obj: &Object) -> (Self, AmmoLook) {
        let mut kind = obj.skill(1) as u32;
        let mut poison = obj.skill(3);
        let mut look = AmmoLook { skin: obj.skill(7) as u32, scale: None };
        if kind <= 1 {
            kind = 1;
            poison = PICKUPS[0].poison;
        } else if look.skin == 0 {
            let spec = &PICKUPS[idx(kind)];
            look = AmmoLook { skin: kind, scale: spec.scale };
            poison = spec.poison;
        }
        if poison == 0.0 {
            poison = 1.0;
        }
        (Self { kind, amount: obj.skill(2).max(1.0) as i32, poison }, look)
    }

    /// Taking it (`Ammo1_Hit`, bullets.wdl:121-177): a different type replaces the old
    /// ammo. Returns the old type and how many of its wasps escape
    /// (`EscapeOldWasps`, at most [`ESCAPE_MAX`]).
    pub fn take(&self, game: &mut GameState) -> Option<(u32, usize)> {
        let mut escape = None;
        if game.ammo_type != self.kind {
            if game.ammo_type != 0 {
                escape = Some((game.ammo_type, game.ammo.clamp(0, ESCAPE_MAX) as usize));
                game.ammo = 0;
            }
            game.ammo_type = self.kind;
            game.ammo_poison = self.poison;
        }
        game.ammo += self.amount;
        escape
    }
}

/// Shots still to fire in a fan: (ticks left, pan offset), from one spawn point.
#[derive(Clone, Debug)]
pub struct PendingShots {
    shots: Vec<(f32, f32)>,
    spawn: [f32; 3],
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
        let (s, c) = (p.pan + 180.0).to_radians().sin_cos();
        let spawn = [p.pos[0] + c * SPAWN_BEHIND, p.pos[1] + s * SPAWN_BEHIND, p.pos[2] + SPAWN_HEIGHT];
        game.ammo -= 1;
        let shots = if matches!(kind, 3 | 4) {
            let mut shots = vec![(0.0, -5.0)];
            for (i, off) in [0.0, 5.0].into_iter().enumerate() {
                game.ammo -= 1;
                if game.ammo < 0 {
                    break;
                }
                shots.push((FAN_SPACING * (i + 1) as f32, off));
            }
            shots
        } else {
            vec![(0.0, 0.0)]
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
    pub fn release(&mut self, kind: u32, poison: f32, pan: f32, t: f32, out: &mut Vec<Shot>) -> bool {
        let spawn = self.spawn;
        self.shots.retain_mut(|(left, off)| {
            *left -= t;
            if *left > 0.0 {
                return true;
            }
            out.push(Shot { kind, poison: poison.max(1.0), pos: spawn, pan: pan + *off });
            false
        });
        self.shots.is_empty()
    }
}

/// A flying wasp.
#[derive(Clone, Debug)]
pub struct Wasp {
    pub kind: u32,
    pub poison: f32,
    pub pan: f32,
    pub tilt: f32,
    pub pos: [f32; 3],
    speed: f32,
    pub homing_range: f32,
    pub tilt_gain: f32,
    pub bounce_force: f32,
    life: f32,
    /// Type 5/6 climb, hold and ease timers and dive rate.
    climb: Option<[f32; 4]>,
    /// Last frame's displacement (lions read it as knockback).
    pub last_disp: [f32; 3],
    /// 16-tick cooldown of the fear tag (`FindEnnemy`).
    pub find_cd: f32,
    /// The thrower: ignored by the wasp's MOVE (`YOU = shooter`, bullets.wdl:377).
    shooter: Option<Id>,
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
    pub fn step(&mut self, space: &impl Space, sammy: Option<[f32; 3]>, t: f32) -> WaspStep {
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

/// Escaping wasps on a type switch: at most this many, shown at half size, for this many
/// ticks, turning `ESCAPE_SPIN` degrees per tick (bullets.wdl:749-921).
pub const ESCAPE_MAX: i32 = 15;
pub const ESCAPE_SCALE: f32 = 0.5;
const ESCAPE_TICKS: f32 = 80.0;
const ESCAPE_SPIN: f32 = 20.0;
/// Pan `random(360)`, tilt `60 + random(40)`, speed `20 + random(10)`.
const ESCAPE_TILT: (f32, f32) = (60.0, 40.0);
const ESCAPE_SPEED: (f32, f32) = (20.0, 10.0);

/// A wasp of the old type escaping when Sammy takes another type (`WaspEscape`,
/// bullets.wdl:847-921): it spirals upward and vanishes. Purely cosmetic (passable).
#[derive(Clone, Debug)]
pub struct Escapee {
    pub kind: u32,
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
    speed: f32,
    timer: f32,
}

impl Escapee {
    pub fn new(kind: u32, pos: [f32; 3], rng: &mut Rng) -> Self {
        let pan = rng.random(360.0);
        let tilt = ESCAPE_TILT.0 + rng.random(ESCAPE_TILT.1);
        let speed = ESCAPE_SPEED.0 + rng.random(ESCAPE_SPEED.1);
        Self { kind, pos, pan, tilt, speed, timer: 0.0 }
    }

    /// One frame; true when it is gone.
    pub fn step(&mut self, t: f32) -> bool {
        let dir = forward(self.pan, self.tilt);
        for (p, d) in self.pos.iter_mut().zip(dir) {
            *p += d * self.speed * t;
        }
        self.pan += ESCAPE_SPIN * t;
        self.timer += t;
        self.timer >= ESCAPE_TICKS
    }
}

/// A wasp that hit a lion (`wasptat` 1004, bullets.wdl:577-644): it stays visible,
/// puffs `particle_hit1` (counts at ticks after the hit), plays a second death sound,
/// then recoils backward along its flight while the death animation plays, and
/// vanishes 4 ticks later.
#[derive(Clone, Debug)]
pub struct Corpse {
    pub kind: u32,
    pub pos: [f32; 3],
    bounce: [f32; 3],
    /// Death animation step (percent).
    step: f32,
    elapsed: f32,
    hidden: f32,
}

const CORPSE_PUFFS: [(f32, usize); 5] = [(0.0, 10), (3.0, 15), (5.0, 10), (6.0, 5), (7.0, 5)];
const CORPSE_SOUND_AT: f32 = 3.0;
const RECOIL_START: f32 = 7.0;
/// Recoil: share of the flight velocity (horizontal, vertical), clamp, damping and
/// gravity per tick.
const RECOIL_XY: f32 = 0.7;
const RECOIL_Z: f32 = 0.5;
const RECOIL_MAX: f32 = 50.0;
const RECOIL_DAMP: f32 = 0.05;
const RECOIL_GRAVITY: f32 = 0.5;
/// The death animation runs at 6 % per tick until 99 %, then the corpse hides for 4 ticks.
const DEATH_ANIM: f32 = 6.0;
const DEATH_END: f32 = 99.0;
const CORPSE_HIDDEN: f32 = 4.0;

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
        Self { kind: w.kind, pos: w.pos, bounce: [-v[0] * RECOIL_XY, -v[1] * RECOIL_XY, -v[2] * RECOIL_Z], step: 0.0, elapsed: 0.0, hidden: 0.0 }
    }

    /// One frame; `trace` sweeps a point through the level (the corpse is passable to
    /// entities).
    pub fn step(&mut self, anim: Option<&mut Anim>, t: f32, trace: impl Fn([f32; 3], [f32; 3]) -> Trace) -> CorpseStep {
        let (before, now) = (self.elapsed, self.elapsed + t);
        self.elapsed = now;
        let due = |at: f32| at >= before && at < now;
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

fn forward(pan: f32, tilt: f32) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

/// A door wasps can break (`InitDoorWeak`).
#[derive(Clone, Copy, Debug)]
pub struct WeakDoor {
    pub hp: f32,
}

/// What a damaged weak door turns into.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorDamage {
    Untouched,
    /// A more broken prefab.
    Morph(&'static str),
    /// Gone: planks fly out.
    Destroyed,
}

/// Sammy's id as a weak door's Skill1 (meaning "use Skill2").
const SAMMY_ID: f32 = 3333.0;
/// Planks a destroyed door breaks into.
pub const PLANKS: [&str; 6] = ["plank1.mdl", "plank2.mdl", "plank3.mdl", "plank1.mdl", "plank2.mdl", "plank3.mdl"];

impl WeakDoor {
    pub fn from_object(obj: &Object) -> Self {
        let hp = if obj.skill(1) == 0.0 || obj.skill(1) == SAMMY_ID { obj.skill(2) } else { obj.skill(1) };
        Self { hp }
    }

    /// A wasp of `poison` hit it.
    pub fn hit(&mut self, poison: f32) {
        self.hp = (self.hp - poison).trunc();
    }

    /// `ExplodeDoorWeak`: HP 3/2/1 show doorwk2/3/4, 0 or less breaks it.
    pub fn damage(&self) -> DoorDamage {
        match self.hp as i32 {
            1 => DoorDamage::Morph("doorwk4.wmb"),
            2 => DoorDamage::Morph("doorwk3.wmb"),
            3 => DoorDamage::Morph("doorwk2.wmb"),
            4.. => DoorDamage::Untouched,
            _ => DoorDamage::Destroyed,
        }
    }
}

/// A plank of a broken door (simplified `LaunchPlank`): ballistic tumble with bounces,
/// settle flat, then vanish.
#[derive(Clone, Debug)]
pub struct Plank {
    v: [f32; 3],
    spin: [f32; 2],
    pub angles: [f32; 3],
    pub pos: [f32; 3],
    still: f32,
}

const PLANK_GRAVITY: f32 = 1.5;
const PLANK_STILL: f32 = 24.0;

impl Plank {
    /// One plank around a door at `origin`.
    pub fn burst(origin: [f32; 3], rng: &mut Rng) -> Self {
        let pos = [origin[0] + rng.random(100.0) - 50.0, origin[1] + rng.random(100.0) - 50.0, origin[2] + rng.random(300.0)];
        let angles = [rng.random(360.0), 0.0, 0.0];
        Self {
            v: [rng.random(40.0) - 20.0, rng.random(40.0) - 20.0, 30.0],
            spin: [30.0 * rng.random(2.0) - 1.0, 30.0 * rng.random(2.0) - 1.0],
            angles,
            pos,
            still: PLANK_STILL,
        }
    }

    /// One frame; returns (bounce sound, gone).
    pub fn step(&mut self, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, t: f32) -> (bool, bool) {
        self.v[2] -= PLANK_GRAVITY * t;
        let target = [0, 1, 2].map(|i| self.pos[i] + self.v[i] * t);
        let hit = trace(self.pos, target);
        self.pos = hit.end;
        let mut sound = false;
        if hit.hit() && hit.normal[2] > 0.5 {
            sound = self.v[2].abs() > 6.0;
            self.v[2] = -self.v[2] * 0.5;
            self.v[0] *= 0.6;
            self.v[1] *= 0.6;
        }
        let speed = self.v[0].abs() + self.v[1].abs();
        if speed < 2.0 && hit.hit() {
            self.still -= t;
            self.angles[1] += (90.0 - self.angles[1]) * (0.5 * t).min(1.0);
        } else {
            self.angles[1] += self.spin[0] * t;
            self.angles[2] += self.spin[1] * t;
        }
        (sound, self.still <= 0.0)
    }
}

/// A hive emitting wasps.
#[derive(Clone, Debug)]
pub struct Hive {
    /// Skill1: launched wasp fate (1001 = frozen at spawn).
    fate: f32,
    /// Skill2: random pan spread.
    spread: f32,
    /// int(Skill3): wasps left ([`UNLIMITED`] when 0); frac×100: cap on Sammy's ammo.
    total: f32,
    cap: i32,
    /// Skill4 / Skill7: vertical and horizontal launch speed.
    vz: f32,
    hspeed: f32,
    /// Skill6: wasp type ([`RANDOM_KIND`] = random 1-6).
    kind: u32,
    /// Skill8: emission period (0 = never).
    period: f32,
    timer: f32,
    pan: f32,
    pub pos: [f32; 3],
}

const UNLIMITED: f32 = 9990.0;
const DEFAULT_CAP: i32 = 10;
const RANDOM_KIND: u32 = 999;
/// `waitt 32` before the first period.
const HIVE_START: f32 = 32.0;
/// Hive wasp pickup table (`initSkinScaleWaspHive2`): (poison, scale) by type.
const HIVE_PICKUP: [(f32, f32); 7] = [(3.0, 1.0), (2.0, 1.0), (1.0, 0.8), (1.0, 0.8), (4.0, 1.0), (5.0, 1.0), (1.0, 0.8)];
/// Wing animation of a hive wasp (percent per tick).
pub const HIVE_WASP_ANIM: f32 = 20.0;

/// A wasp a hive launches.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HiveLaunch {
    pub kind: u32,
    pub pan: f32,
    pub tilt: f32,
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
            timer: HIVE_START + obj.skill(8),
            pan: obj.angles[0],
            pos: obj.origin,
        }
    }

    /// One frame; returns a launch and whether the hive is now exhausted.
    pub fn step(&mut self, game: &GameState, rng: &mut Rng, t: f32) -> Option<(HiveLaunch, bool)> {
        if self.period <= 0.0 {
            return None;
        }
        self.timer -= t;
        if self.timer > 0.0 {
            return None;
        }
        self.timer += self.period;
        if game.ammo >= self.cap && game.ammo_type == self.kind {
            return None; // capped: the hive looks empty
        }
        let kind = if self.kind == RANDOM_KIND { rng.random(6.0) as u32 + 1 } else { self.kind.clamp(1, KINDS) };
        let pan = if self.spread != 0.0 { self.pan + rng.random(self.spread) - rng.random(self.spread * 0.5) } else { self.pan };
        let launch = [self.hspeed + rng.random(5.0), self.vz + rng.random(4.0)];
        self.total -= 1.0;
        let tilt = rng.random(20.0) + 20.0;
        let l = HiveLaunch { kind, pan, tilt, scale: HIVE_PICKUP[idx(kind)].1, speed: launch, fate: self.fate };
        Some((l, self.total <= 0.0))
    }
}

/// A wasp launched by a hive (`LaunchNewWasp`), bouncing until it settles into a pickup.
#[derive(Clone, Debug)]
pub struct HiveWasp {
    pub kind: u32,
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
    speed: f32,
    vz: f32,
    vz_org: f32,
    /// Bounce counter; starts at the fate (they share a skill slot, original quirk).
    bounces: f32,
    fate: f32,
}

/// Frozen hive wasps never move.
const FROZEN: f32 = 1001.0;

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
    pub fn step(&mut self, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, rng: &mut Rng, t: f32, sounds: &mut Vec<&'static str>) -> Option<(AmmoPickup, Option<f32>)> {
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
        self.tilt -= if self.bounces == 0.0 { 2.0 * t } else { self.tilt * 0.1 * t };
        let (s, c) = self.pan.to_radians().sin_cos();
        let d = [c * self.speed * t.min(1.0), s * self.speed * t.min(1.0), vstep];
        let hit = trace(self.pos, [self.pos[0] + d[0], self.pos[1] + d[1], self.pos[2] + d[2]]);
        let before = self.pos;
        self.pos = hit.end;
        if hit.hit() && hit.normal[2].abs() < 0.7 {
            // Wasp_Bounce: turn off the wall, lose speed.
            let n = hit.normal;
            self.pan = n[1].atan2(n[0]).to_degrees();
            self.speed = (self.speed - 8.0).max(0.1);
        }
        self.speed -= 0.05 * t.min(1.0);
        let moved = (self.pos[0] - before[0]).abs() + (self.pos[1] - before[1]).abs();
        let mut settled = false;
        if moved < 1.0 {
            self.vz_org -= 0.5 * t.min(1.0);
            settled |= self.vz_org.abs() < 1.5;
        } else {
            self.vz_org -= 0.02 * t.min(1.0);
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

/// Hive pickups shrink over their last ticks, then burst.
pub const PICKUP_FADE: f32 = 16.0;
pub const PICKUP_FADE_SHRINK: f32 = 0.85;

#[cfg(test)]
mod tests;
