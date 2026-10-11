//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X at Skill2 quants/tick through hidden
//! rails in the level. On impact it either reflects (Flag3) or turns by Skill4/5/7 and
//! pauses Skill3 ticks. Platforms and bodies with the same link id (Skill1) copy the
//! engine's motion at a fixed offset, carrying whatever stands on them. An engine with
//! Skill6 is locked until Sammy stands on its platform holding a matching key.

use formats::trace::Trace;
use formats::wmb::Object;

use crate::angle::ang;

mod engine;
mod platform;
mod teleport;

pub use self::{engine::*, platform::*, teleport::*};

/// Default link id when Skill1 is 0 (`8465`).
pub const DEFAULT_LINK: f32 = 8465.0;
/// The engine carrying a Shift-boosting rider runs this much faster.
pub const BOOST_FACTOR: f32 = 3.0;
/// The plain wooden platform an unlocked platform turns into.
pub const UNLOCKED_PLATFORM: &str = "platfor1.wmb";
