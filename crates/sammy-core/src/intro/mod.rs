//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): a scripted Sammy walks
//! the path-node loop of the menu scene while the A5, publisher and Suricate logos fade
//! in front of the camera; at node key 999 (or a skip key) the camera glides to the
//! menu pose.

use formats::wmb::Object;

use crate::angle::ang;
use crate::time::Ticks;

mod logo;
mod walker;

pub use self::{logo::*, walker::*};

/// `MoveMenu` (interfac.wdl:625-653): position and angles ease 0.2 per tick toward the
/// menu pose; true once within 2 quants in X and Z (then snap exactly).
pub fn glide(pos: &mut [f32; 3], pan: &mut f32, tilt: &mut f32, to: ([f32; 3], f32, f32), t: Ticks) -> bool {
    let t = t.get();
    let k = (GLIDE_RATE * t).min(1.0);
    for (p, to) in pos.iter_mut().zip(to.0) {
        *p += (to - *p) * k;
    }
    *pan += ang(to.1 - *pan) * k;
    *tilt += (to.2 - *tilt) * k;
    if (pos[0] - to.0[0]).abs() < GLIDE_DONE && (pos[2] - to.0[2]).abs() < GLIDE_DONE {
        (*pos, *pan, *tilt) = to;
        return true;
    }
    false
}

const GLIDE_RATE: f32 = 0.2;
const GLIDE_DONE: f32 = 2.0;
/// While gliding the fog clears by this much per tick.
pub const GLIDE_FOG_CLEAR: f32 = 5.0;
/// The intro ends at the first node with this key.
pub const END_KEY: f32 = 999.0;
/// `CameraPosOrigine` / `CameraAngleOrigine` of the menu (`InitGame2`).
pub const MENU_ORIGIN: ([f32; 3], f32, f32) = ([1.0, -369.0, 115.0], 90.0, 1.0);
/// `Camera_PlusTilt` during the intro.
pub const PLUS_TILT: f32 = 50.0;
/// After the first 12 ticks the follow frictions become 0.1 / 0.5.
pub const FRICTION_AT: f32 = 12.0;
pub const FRICTION: (f32, f32) = (0.1, 0.5);
