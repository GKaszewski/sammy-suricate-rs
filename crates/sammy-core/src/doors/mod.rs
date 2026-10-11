//! Doors (`InitDoor` / `movePortail` / `OuvrePortail`, mm1.wdl:2886-3283) and the
//! exit tile (`InitExit` / `TileExit_Hit`, mm1.wdl:3956-3975).

use formats::wmb::Object;

use crate::game::GameState;
use crate::keys::KeyColor;
use crate::time::Ticks;

mod exit;
mod step;

pub use self::exit::*;

/// `RangeScanDoor` (mm1.wdl:165): radius of Sammy's per-tick door scan.
pub const RANGE_SCAN_DOOR: f32 = 500.0;
/// Scan id of Sammy's door scan; a door with another Skill7 never reacts to him.
pub const SAMMY_SCAN_ID: f32 = 25.0;
/// Colour of the exit door (red + yellow + blue).
pub const EXIT_COLOR: KeyColor = KeyColor::EXIT;
/// Rise heights up to this are replaced by [`MIN_RISE`].
const RISE_DEFAULT_BELOW: f32 = 250.0;
const MIN_RISE: f32 = 300.0;
/// Skill8 = 0: ticks held open.
const DEFAULT_HOLD: Ticks = Ticks(4.0);
/// Skill3 = 0: open within this horizontal distance.
const DEFAULT_OPEN_DIST: f32 = RANGE_SCAN_DOOR - 150.0;
/// Hold extension while Sammy or a gem keeps the door busy (`Skill9 = 4`).
const HOLD_REFRESH: Ticks = Ticks(4.0);
/// Close enough to walk under: validates a door a gem is flying into.
const UNDER_DOOR: f32 = 128.0;
/// Recall margin: Sammy beyond `open_dist - 4` recalls a flying gem.
const RECALL_MARGIN: f32 = 4.0;
/// Ticks of the "wrong key" flash.
const WRONG_KEY_FLASH: Ticks = Ticks(3.0);
/// Ticks the score panel blinks when the exit door is refused (`WarnScoreGoal`).
const WARN_SCORE_TICKS: Ticks = Ticks(8.0);
/// Holds longer than this ring bells: a blink every [`BELL_PERIOD`], `Bell1` in the
/// last [`BELL_WINDOW`] ticks (but not the very last [`BELL_QUIET`]).
const BELL_WINDOW: Ticks = Ticks(48.0);
const BELL_PERIOD: Ticks = Ticks(12.0);
const BELL_QUIET: Ticks = Ticks(4.0);
/// Closing clearance (`DoorScanEntities`): first pass rise + 100, later passes 250.
const CLEAR_FIRST_EXTRA: f32 = 100.0;
const CLEAR_LATER: f32 = 250.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorState {
    Down,
    Rising,
    Up { hold: Ticks },
    /// Hold over: wait until nobody is under the door, then lower.
    Clearing,
    Lowering,
    /// Stays open for good (one-shot doors, validated permanent doors).
    Open,
}

#[derive(Clone, Debug)]
pub struct Door {
    z0: f32,
    /// `Skill13 = z0 + rise`.
    top: f32,
    /// Skill4 / Skill5: up and down speed per tick (Skill5 = 0: never closes).
    up_speed: f32,
    pub down_speed: f32,
    /// Skill8: ticks held open.
    hold: Ticks,
    /// Skill3: horizontal open distance.
    open_dist: f32,
    /// Skill6: colour bitmask ([`EXIT_COLOR`] = exit door).
    pub color: KeyColor,
    /// Skill7: scan id the door reacts to.
    scan_id: f32,
    /// Flag2: never for Sammy; Flag5: permanent (a key gem opens it for good);
    /// Flag6: no key needed; Flag7: exact colour only.
    no_sammy: bool,
    permanent: bool,
    keyless: bool,
    exact: bool,
    pub state: DoorState,
    /// Ticks left of the "wrong key" flash.
    flash: Ticks,
    /// Skill2: rise height (at least [`MIN_RISE`]).
    rise: f32,
    /// Skill14: a gem is flying in (or a keyless Flag5 door is engaged).
    pub gem_flying: bool,
    /// Skill15: the gem arrived / Sammy walked under: the door stays open for good.
    pub validated: bool,
    /// Sammy came back into reach during the hold.
    refreshed: bool,
    /// Bell/blink timer (Skill16).
    bell: Ticks,
    /// Closing clearance check pass (first uses the rise, later ones 250).
    clear_pass: u8,
}

/// What the frontend shows for a door step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorEvent {
    Sound(&'static str),
    /// `CreateGemKeyAnim`: the held key (colour) flies from Sammy into the door.
    LaunchGem { color: KeyColor },
    /// A coloured door opens: flash of its colour.
    Flash { color: KeyColor },
    /// A validated permanent door turns into `doordark.wmb`.
    MorphDark,
}

/// The world as a door sees it.
pub struct DoorView<'a> {
    pub sammy: [f32; 3],
    pub sammy_dead: bool,
    /// Living enemies (they keep a door from closing on them).
    pub blockers: &'a [[f32; 3]],
}

/// The prefab a validated permanent door becomes.
pub const DARK_DOOR: &str = "doordark.wmb";

#[cfg(test)]
mod tests;
