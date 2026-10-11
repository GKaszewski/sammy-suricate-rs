//! What the player asks for this frame, and the boost and U-turn that change it.

use super::{Frame, Player, State};
use crate::angle::ang;
use crate::anim::Anim;
use crate::collision::Space;
use crate::game::{BOOST_MAX, GameState};
use crate::time::TICKS_PER_SEC;
use crate::angle::Degrees;

/// Boost: the engine doubles `Key_Force`; the script keeps turning ×0.75 (net ×1.5).
const BOOST_TURN_FORCE: f32 = 1.5;
const BOOST_FORWARD_FORCE: f32 = 2.0;
/// U-turn easing toward +180: first step ×0.25, then ×0.4, done below 2°.
const UTURN_FIRST: f32 = 0.25;
const UTURN_NEXT: f32 = 0.4;
const UTURN_DONE: f32 = 2.0;
/// U-turn animation speed (% of the scene per tick).
const ANIM_UTURN: f32 = 28.0;

/// `Key_Force` and the jump key, after the state's restrictions.
#[derive(Clone, Copy, Debug)]
pub(super) struct Intent {
    /// Right (+) / left (-).
    pub(super) turn: f32,
    /// Forward (+) / back (-).
    pub(super) forward: f32,
    pub(super) jump_held: bool,
    pub(super) jump_hit: bool,
}

impl Intent {
    /// The controls, or nothing while input is off or Sammy is dead.
    pub(super) fn read<S: Space>(f: &Frame<S>, state: State) -> Self {
        let c = f.controls;
        if !f.input || state == State::Dead {
            return Self { turn: 0.0, forward: 0.0, jump_held: false, jump_hit: false };
        }
        Self { turn: c.turn, forward: c.forward, jump_held: c.jump, jump_hit: c.jump_hit }
    }

    /// Turning and moving happen only in the normal and jump states (etat 1 / 4).
    pub(super) fn restrict_to(&mut self, state: State) {
        if !matches!(state, State::Stand | State::Walk | State::Turn | State::Air | State::Land) {
            self.turn = 0.0;
            self.forward = 0.0;
        }
    }
}

impl Player {
    /// `HandleBoostPlayer`: Shift speeds Sammy up while the boost gauge lasts.
    pub(super) fn boost<S: Space>(&mut self, game: &mut GameState, f: &Frame<S>, intent: &mut Intent, t: f32) {
        self.boosting = false;
        if f.controls.boost && f.input && game.boost < BOOST_MAX && self.state != State::Dead {
            game.boost = (game.boost + t).min(BOOST_MAX); // LessBoost: ConsoBoost 1/tick
            intent.turn *= BOOST_TURN_FORCE;
            intent.forward *= BOOST_FORWARD_FORCE;
            self.boosting = true;
        }
    }

    /// `RetourneSammy`: in a normal state, pan eases toward +180.
    pub(super) fn u_turn<S: Space>(&mut self, f: &Frame<S>, anim: Option<&mut Anim>, sounds: &mut Vec<&'static str>, t: f32) {
        if f.controls.uturn_hit && f.input && self.grounded && matches!(self.state, State::Stand | State::Walk | State::Turn) {
            self.state = State::UTurn;
            self.uturn = Some((self.pan.0 + 180.0, false));
            sounds.push("Blouit_son");
            if let Some(a) = anim {
                a.play("turn", true, 0.0);
                a.set_speed_pct(ANIM_UTURN, TICKS_PER_SEC);
            }
        }
        if let (State::UTurn, Some((dest, started))) = (self.state, self.uturn) {
            let step = ang((dest - self.pan.0) * if started { UTURN_NEXT } else { UTURN_FIRST });
            if step > UTURN_DONE {
                self.pan += Degrees(step * t);
                self.uturn = Some((dest, true));
            } else {
                self.uturn = None;
                self.state = State::Stand;
            }
        }
    }
}
