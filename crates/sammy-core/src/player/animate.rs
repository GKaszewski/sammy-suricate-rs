//! Sammy's animation states (anims.wdl:352-497) and the win dance (`AnimGagnee`).

use super::intent::Intent;
use super::{Player, State};
use crate::angle::Degrees;
use crate::anim::Anim;
use crate::time::TICKS_PER_SEC;

// Animation speeds in % of the scene per tick (mm1.wdl:39-85, anims.wdl).
const ANIM_STAND: f32 = 2.0;
/// Walking: × forward speed (negative plays backwards).
const ANIM_WALK: f32 = 0.5;
const ANIM_TURN: f32 = 14.0;
const ANIM_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP_FAST: f32 = 18.0;
const ANIM_WIN: f32 = 3.0;
/// The win dance spins this many degrees per tick.
const WIN_SPIN: f32 = 12.0;

impl Player {
    /// `AnimGagnee`: "Iwin" while spinning.
    pub(super) fn dance(&mut self, anim: Option<&mut Anim>, t: f32) {
        if self.state != State::Win {
            self.state = State::Win;
            if let Some(a) = anim {
                a.play("Iwin", true, 0.0);
                a.set_speed_pct(ANIM_WIN, TICKS_PER_SEC);
            }
        }
        self.pan += Degrees(WIN_SPIN * t);
    }

    /// Step 6: stand / walk / turn from the intent, and the jump scenes' speeds.
    pub(super) fn animate(&mut self, intent: &Intent, mut anim: Option<&mut Anim>) {
        let (ix, iy) = (intent.turn, intent.forward);
        if matches!(self.state, State::Stand | State::Walk | State::Turn) {
            let next = if iy != 0.0 {
                State::Walk
            } else if ix != 0.0 {
                State::Turn
            } else {
                State::Stand
            };
            if let Some(a) = anim.as_deref_mut() {
                let scene = match next {
                    State::Walk => "walkB",
                    State::Turn => "turn",
                    _ => "stand",
                };
                if next != self.state || a.frames.is_empty() || !a.looping {
                    a.play(scene, true, 0.0);
                }
                let pct = match next {
                    State::Walk => ANIM_WALK * self.speed * iy,
                    State::Turn => ANIM_TURN,
                    _ => ANIM_STAND,
                };
                a.set_speed_pct(pct, TICKS_PER_SEC);
            }
            self.state = next;
        }
        let Some(a) = anim else { return };
        match self.state {
            State::Air => a.set_speed_pct(ANIM_JUMP, TICKS_PER_SEC),
            State::Land => {
                let pct = if self.hard_landing {
                    ANIM_FIN_JUMP * 0.5
                } else if iy != 0.0 || intent.jump_held {
                    ANIM_FIN_JUMP_FAST * 2.0
                } else if ix != 0.0 {
                    ANIM_FIN_JUMP_FAST
                } else {
                    ANIM_FIN_JUMP
                };
                a.set_speed_pct(pct, TICKS_PER_SEC);
            }
            _ => {}
        }
    }
}
