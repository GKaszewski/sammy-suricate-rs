//! Collision against the level BSP plus solid map entities (prefab WMBs such as
//! doors, exit tiles and platforms), each with its own clipping hulls, and model
//! bounding boxes. Entity ids are the caller's (`Id`), so any ECS or plain index works.

use std::sync::Arc;

use formats::trace::Trace;
use formats::wmb::Wmb;

mod hull;
mod moves;
mod scene;
mod shapes;

pub use self::{hull::*, moves::*, scene::*, shapes::*};

#[cfg(test)]
mod tests;
