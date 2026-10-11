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

mod corpse;
mod door;
mod escapee;
mod fire;
mod flight;
mod hive;
mod kinds;
mod pickup;

pub use self::{corpse::*, door::*, escapee::*, fire::*, flight::*, hive::*, kinds::*, pickup::*};

fn forward(pan: f32, tilt: f32) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

#[cfg(test)]
mod tests;
