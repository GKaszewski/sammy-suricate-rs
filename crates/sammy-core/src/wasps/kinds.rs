//! The seven wasp kinds: pickup and firing tables, sounds and colours.

use super::*;
use crate::time::Ticks;

/// Wasp types 1..=7 (orange, yellow, blue, green, magenta, red, white).
pub const KINDS: u32 = 7;

/// Index into the per-type tables of a wasp type (1..=7).
pub(super) fn idx(kind: u32) -> usize {
    (kind.clamp(1, KINDS) - 1) as usize
}

/// A pickup's poison and size by type (docs/wasps.md §1.3).
pub(super) struct PickupSpec {
    pub(super) poison: f32,
    pub(super) scale: Option<f32>,
}

pub(super) const PICKUPS: [PickupSpec; 7] = [
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
pub(super) const FIRE_FACTOR: [f32; 7] = [1.0, 2.2, 1.5, 1.2, 1.2, 1.0, 3.0];
/// Throw animation speed (percent per tick, × the type factor for "TirA").
pub(super) const THROW_ANIM: f32 = 20.0;
/// A fired wasp's wings flap at half its speed (percent per tick).
pub const WING_ANIM: f32 = 0.5;
/// The wasp leaves this far behind Sammy and this high.
pub(super) const SPAWN_BEHIND: f32 = 40.0;
pub(super) const SPAWN_HEIGHT: f32 = 50.0;
/// Types that throw a 3-wasp fan: offsets (degrees) and 2-tick spacing.
pub(super) const FAN_SPACING: Ticks = Ticks(2.0);
/// Invisible until this far from Sammy (it starts behind him).
pub(super) const SHOW_DISTANCE: f32 = 80.0;
/// Burst sounds, picked at random.
pub const BURST_SOUNDS: [&str; 3] = ["WaspKaputt_son", "WaspKaputt2_son", "WaspKaputt3_son"];
/// Type 5/6 climb, hold and ease phases (ticks) and dive rate (degrees per tick).
pub(super) const CLIMB: [(u32, [f32; 4]); 2] = [(5, [3.0, 5.0, 5.0, 0.2]), (6, [4.0, 8.0, 10.0, 0.1])];
pub(super) const CLIMB_RATE: f32 = 20.0;

/// The light flashed when a pickup is taken, by type.
pub const AMMO_COLORS: [[f32; 3]; 7] =
    [[250.0, 100.0, 0.0], [250.0, 250.0, 0.0], [0.0, 0.0, 250.0], [0.0, 250.0, 0.0], [250.0, 0.0, 250.0], [250.0, 0.0, 0.0], [150.0, 150.0, 150.0]];

pub fn burst_sound(rng: &mut Rng) -> &'static str {
    BURST_SOUNDS[(rng.random(3.0) as usize).min(2)]
}
