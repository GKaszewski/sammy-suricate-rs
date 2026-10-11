//! The Eye of Truth (`InitEyeOfTruth`): taking it shows the lions' range rings for a
//! while (`EyeOfTruth` ticks); the pickup ducks away and comes back.

use formats::wmb::Object;

use crate::game::GameState;
use crate::gems::HEIGHT_GEM;

/// Taken: shrinks for [`SHRINK_TICKS`], stays hidden [`HIDDEN_TICKS`], then grows back.
pub const HIDDEN_TICKS: f32 = 128.0;
const SHRINK_TICKS: f32 = 12.0;
pub const SHRINK_RATE: f32 = 0.8;
/// It floats twice the gem height above the floor.
pub const EXTRA_HEIGHT: f32 = HEIGHT_GEM;

/// What the pickup does this frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EyeStage {
    /// Waiting to be taken (idle sparkles).
    Idle,
    /// Just taken by Sammy.
    Taken,
    /// Shrinking after being taken.
    Shrinking,
    Hidden,
    /// Back to full size.
    Back,
}

#[derive(Clone, Debug)]
pub struct EyePickup {
    /// Taken: ticks until it is back.
    away: Option<f32>,
    /// Flag3: spins by Skill2-4 per tick.
    spin: Option<[f32; 3]>,
    pub angles: [f32; 3],
    /// Snaps onto the floor (Flag6 keeps the WED height).
    pub snap: bool,
}

impl EyePickup {
    pub fn from_object(obj: &Object) -> Self {
        Self { away: None, spin: obj.flag(3).then_some([obj.skill(2), obj.skill(3), obj.skill(4)]), angles: obj.angles, snap: !obj.flag(6) }
    }

    /// Waiting to be taken (shown, sparkling).
    pub fn idle(&self) -> bool {
        self.away.is_none()
    }

    /// Spins; true if it has a spin.
    pub fn spin(&mut self, t: f32) -> bool {
        let Some(spin) = self.spin else { return false };
        for (a, s) in self.angles.iter_mut().zip(spin) {
            *a += s * t;
        }
        true
    }

    /// One frame: taken when Sammy (alive, playing) is within `radius`.
    pub fn step(&mut self, game: &mut GameState, in_reach: bool, t: f32) -> EyeStage {
        match self.away {
            Some(left) => {
                let left = left - t;
                if left <= 0.0 {
                    self.away = None;
                    EyeStage::Back
                } else {
                    self.away = Some(left);
                    if left > HIDDEN_TICKS { EyeStage::Shrinking } else { EyeStage::Hidden }
                }
            }
            None if in_reach => {
                game.eye_ticks = game.eye_duration;
                self.away = Some(HIDDEN_TICKS + SHRINK_TICKS);
                EyeStage::Taken
            }
            None => EyeStage::Idle,
        }
    }
}

/// The Eye's effect wears off; dying (`Player_Kaputt`) ends it at once.
pub fn wear_off(game: &mut GameState, sammy_dead: bool, t: f32) {
    if sammy_dead {
        game.eye_ticks = 0.0;
    }
    game.eye_ticks = (game.eye_ticks - t).max(0.0);
}
