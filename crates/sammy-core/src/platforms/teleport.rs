//! Teleport pads and Sammy being teleported.

use super::*;
use crate::angle::Degrees;
use crate::percent::Percent;
use crate::time::Ticks;

/// A teleport pad.
#[derive(Clone, Debug)]
pub struct Teleport {
    /// Skill1-3: destination; Skill4: pan on arrival.
    pub dest: [f32; 3],
    pub pan: Degrees,
    /// Flag2: lions are teleported too.
    pub lions: bool,
    /// `Skill12`: busy while someone is being teleported (shared by Sammy and lions).
    pub busy: bool,
}

impl Teleport {
    pub fn from_object(obj: &Object) -> Self {
        Self { dest: [obj.skill(1), obj.skill(2), obj.skill(3)], pan: Degrees(obj.skill(4)), lions: obj.flag(2), busy: false }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum TeleportStage {
    /// `waitt 4` while Sammy settles onto the pad.
    Settle(Ticks),
    FadeOut,
    FadeIn,
}

/// Ticks Sammy settles on a pad before fading.
pub(super) const SETTLE_TICKS: Ticks = Ticks(4.0);
/// Fade speed in alpha percent per tick.
pub(super) const FADE_SPEED: f32 = 10.0;
/// After the jump the camera is put this far from the destination (x, y, z).
pub const CAMERA_SNAP: [f32; 3] = [32.0, 32.0, 150.0];

/// Sammy being teleported (input locked).
#[derive(Clone, Debug)]
pub struct Teleporting {
    pub dest: [f32; 3],
    pub pan: Degrees,
    pub(super) stage: TeleportStage,
    /// Sammy's opacity (percent).
    pub alpha: Percent,
}

/// What a teleport step asks of the frontend.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TeleportStep {
    Fading,
    /// Faded out: move Sammy to `dest`, facing `pan`, and snap the camera.
    Jump,
    /// Faded back in: Sammy can move again and the pad is free.
    Done,
}

impl Teleporting {
    pub fn new(pad: &Teleport) -> Self {
        Self { dest: pad.dest, pan: pad.pan, stage: TeleportStage::Settle(SETTLE_TICKS), alpha: Percent::FULL }
    }

    pub fn step(&mut self, t: Ticks) -> TeleportStep {
        let mut out = TeleportStep::Fading;
        self.stage = match self.stage {
            TeleportStage::Settle(left) if left - t > Ticks(0.0) => TeleportStage::Settle(left - t),
            TeleportStage::Settle(_) => TeleportStage::FadeOut,
            TeleportStage::FadeOut => {
                self.alpha = (self.alpha - Percent(FADE_SPEED * t)).max(Percent::ZERO);
                if self.alpha <= Percent::ZERO {
                    out = TeleportStep::Jump;
                    TeleportStage::FadeIn
                } else {
                    TeleportStage::FadeOut
                }
            }
            TeleportStage::FadeIn => {
                self.alpha = (self.alpha + Percent(FADE_SPEED * t)).min(Percent::FULL);
                if self.alpha >= Percent::FULL {
                    out = TeleportStep::Done;
                }
                TeleportStage::FadeIn
            }
        };
        out
    }
}
