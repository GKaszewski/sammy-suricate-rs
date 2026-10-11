//! Main menu rules (`MainMenuSammy`, interfac.wdl:23-624; docs/menu.md §3): what each
//! item does, how the 3D signs bob and spin, and how level icons look from the save.

use formats::wmb::Object;

use crate::levels::{LevelSlot, file_for};
use crate::save::{LevelState, SaveGame};

mod bob;
mod choice;
mod look;

pub use self::{bob::*, choice::*, look::*};

#[cfg(test)]
mod tests;
