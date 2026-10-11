//! The game cameras: the type-3 follow camera (camera.wdl `HandleCamera3`, simplified),
//! the type-2 camera (`HandleCamera`), camera zones (`SetCamera`) and the per-level
//! settings, in native coordinates.

use formats::wmb::{Object, Wmb};

use crate::angle::ang as angle_diff;
use crate::collision::Hull;
use crate::extro::{RANGE_SCAN_25, Scanners};
use crate::id::Id;

mod follow;
mod params;
mod type2;
mod zones;

pub use self::{follow::*, params::*, zones::*};
use self::{type2::*};
