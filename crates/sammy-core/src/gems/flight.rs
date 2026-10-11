//! Gems flying, bouncing and coming to rest (thrown keys, launched gems).

use super::*;
use crate::time::Ticks;

pub(super) const THROW_CEILING: f32 = 32.0;
/// Bounce from which a flying gem can be picked up.
pub(super) const COLLECTABLE_BOUNCE: u32 = 5;
/// Floor search depth below a gem.
pub(super) const FLOOR_PROBE: f32 = 2000.0;
/// Ticks between "last good position" snapshots (escape when stuck in a wall).
pub(super) const GOOD_POS_PERIOD: Ticks = Ticks(16.0);
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
    pub(super) good_timer: Ticks,
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
        Self { kind, pos, pan, speed, vz, vz_org: vz, fric, bounces: 0, ceiling, stuck: 0.0, last_good: pos, good_timer: Ticks(0.0) }
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

}
