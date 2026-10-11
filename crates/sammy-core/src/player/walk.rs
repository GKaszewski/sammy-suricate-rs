//! Turning, walking (steered by slopes) and bumping into things (`Player_Bong`).

use super::intent::Intent;
use super::{Frame, HULL, Player, StepOut, Touch};
use crate::angle::Degrees;
use crate::collision::{Space, slide_move};

/// Slopes: medium ones damp turning, big ones (tuning) strafe and steer downhill.
const MEDIUM_SLOPE: f32 = 2.0;
const SPEED_ROTATION_PENTE: f32 = 0.05;
/// Air control (forward speed factor).
const AIR_CONTROL: f32 = 0.75;
/// Turning: half rate in the air, ×0.6 while boosting.
const AIR_TURN: f32 = 0.5;
const BOOST_TURN: f32 = 0.6;

/// The slope Sammy slides on, as it affects walking and jumping.
#[derive(Clone, Copy, Debug)]
pub(super) struct SlopeFeel {
    /// Sliding faster than the tuning's big slope.
    pub(super) big: bool,
    /// Downhill direction (degrees).
    pub(super) pan: f32,
}

/// `a - b` wrapped to -180..180.
pub(super) fn wrap(a: f32, b: f32) -> f32 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

impl Player {
    /// Steps 1-2 of `HandleSpeedPlayer`: turning, then the forward move along the
    /// ground, damped or steered by the slope slide.
    pub(super) fn walk<S: Space>(&mut self, intent: &Intent, f: &Frame<S>, out: &mut StepOut, t: f32) -> SlopeFeel {
        let tuning = f.tuning;
        let (ix, iy) = (intent.turn, intent.forward);
        let mut rate = if iy != 0.0 { if self.boosting { BOOST_TURN * tuning.turn_move } else { tuning.turn_move } } else { tuning.turn_stand };
        if !self.grounded {
            rate *= AIR_TURN;
        }
        let mut angle_force = ix * rate * t;

        let mut v = if self.grounded { iy * self.speed * t } else { iy * self.speed * AIR_CONTROL * t };
        let mut strafe = 0.0;
        let big = self.pente[0].abs() > tuning.big_slope || self.pente[1].abs() > tuning.big_slope;
        let medium = self.pente[0].abs() > MEDIUM_SLOPE || self.pente[1].abs() > MEDIUM_SLOPE;
        let slope_pan = self.pente[1].atan2(self.pente[0]).to_degrees();
        if big {
            v *= 0.75;
            strafe = -ix * self.speed * 0.75 * t;
            angle_force = 0.0;
            self.pan += Degrees(wrap(slope_pan, self.pan.0) * SPEED_ROTATION_PENTE * t);
        } else if medium {
            v *= 0.9;
            strafe = -ix * self.speed * 0.5 * t;
            angle_force *= 0.25;
        }
        self.pan -= Degrees(angle_force);
        let (s, c) = self.pan.radians().sin_cos();
        let (pos, touched) = slide_move(f.space, HULL, self.pos, [c * v - s * strafe, s * v + c * strafe, 0.0], Some(f.me));
        self.pos = pos;
        if let Some((e, n)) = touched {
            out.touched = Some(e);
            self.bump((f.touch)(e), n, f.tuning.bounce);
        }
        SlopeFeel { big, pan: slope_pan }
    }

    /// `Player_Bong` (player.wdl:520-697): bumping an entity cancels the slope slide;
    /// others knock Sammy back along the contact normal, pushers cancel it.
    fn bump(&mut self, touch: Touch, normal: [f32; 3], bounce: f32) {
        match touch {
            Touch::Gem | Touch::Enemy => {}
            Touch::Pusher => {
                self.pente = [0.0; 3];
                self.knock = [0.0; 3];
            }
            Touch::Other => {
                self.pente = [0.0; 3];
                self.knock = [normal[0] * bounce, normal[1] * bounce, 0.0];
            }
        }
    }
}
