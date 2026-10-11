//! Ammo pickups lying in the level.

use super::*;
use crate::time::Ticks;

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

/// Hive pickups shrink over their last ticks, then burst.
pub const PICKUP_FADE: Ticks = Ticks(16.0);
pub const PICKUP_FADE_SHRINK: f32 = 0.85;
