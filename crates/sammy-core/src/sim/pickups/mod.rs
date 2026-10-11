//! Diamonds (`InitBonus3`/`Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey`/`Key_Hit`/`EjectOldKey`/`LaunchYouKey`, gems.wdl:40-960), picked up by
//! Sammy's scan (radius `BlobZoneScan`).

use formats::wmb::Object;

use crate::sim::world::{Light, World};
use super::{Effect, Event, Sim, Space};
use crate::collision::{Hull, Space as _};
use crate::gems::{GemGravity, HEIGHT_GEM, Surprise};
use crate::id::Id;
use crate::keys::{KeyOutcome, MASTER_KEY, light_color, take_key};
use crate::pickups::{KeySpec, NARROW_FLOOR_OFFSET, SHRINK_GONE, SHRINK_PER_TICK, SNAP_START, in_scan};
use crate::player::State;

mod collect;
mod keys;

pub(super) use self::{collect::*, keys::*};

/// Key pickup flash: start range, shrink per tick, cut-off range.
const KEY_FLASH: (f32, f32, f32) = (300.0, 15.0, 100.0);
/// Gem-mix stars burst this far above Sammy.
const MIX_HEIGHT: f32 = 40.0;
/// Floor probe depth when snapping pickups.
const FLOOR_PROBE: f32 = 2000.0;

impl Sim {
    /// `AddPoints`, playing `ScoreReach_son` when the goal is reached.
    pub(super) fn add_points(&mut self, points: i32) {
        if self.game.add_points(points) {
            self.sound("ScoreReach_son");
        }
    }
}
