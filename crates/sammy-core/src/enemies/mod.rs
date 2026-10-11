//! Lions, scorpions, the elephant and wasp enemies (`lion.wdl`, `anims.wdl:2-350`,
//! `InitWaspEnnemy`), ported from docs/enemy-ai.md. The AI runs at a fixed 16 Hz with
//! `time = 1` (the only safe reading of the original, §2.9); a frontend interpolates
//! models between ticks ([`Enemy::pose`]).

use formats::mdl::Mdl;
use formats::wmb::Object;

use crate::anim::Anim;
use crate::collision::{Hull, Space};
use crate::game::GameState;
use crate::id::Id;
use crate::platforms::Teleport;
use crate::player::Player;
use crate::rng::Rng;
use crate::time::TICKS_PER_SEC as TICKS;

mod anim;
mod behavior;
mod clock;
mod contact;
mod enemy;
mod globals;
mod homing;
mod init;
mod math;
mod movement;
mod patrol;
mod sense;
mod start;
mod tick;

pub use self::{clock::*, contact::*, enemy::*, globals::*, homing::*, init::*, start::*, tick::*};
use self::{anim::*, behavior::*, math::*, movement::*, patrol::*, sense::*};

/// Class ids (`IdEnnemy` after init).
const LION: u32 = 8888;
/// `Win.MDL` in the extro (`IdEnnemy == 99`): stand cycle at 6 %/tick, never a target.
const WIN_MODEL: u32 = 99;
const ELEPHANT: u32 = 1111;
/// Enemies collide with the fat hull.
const HULL: Hull = Hull::Fat;
