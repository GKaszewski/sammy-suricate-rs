//! Level flow (mm1.wdl:542-1156, Loads.wdl): the level-start camera sweep, playing, and
//! the win / lose screens with their progression.

use crate::game::GameState;
use crate::levels::{EXTRO_SLOT, LAST_OF_WORLD, MENU_FILE, file_for, next_slot};
use crate::save::{LevelState, SaveGame};
use crate::time::Ticks;

/// `PartieEncours` / `PartieGagnee` reduced to the phases the port needs.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Phase {
    /// Level-start camera sweep (input disabled).
    #[default]
    Intro,
    Playing,
    /// Win / lose screens, with elapsed ticks.
    Won(f32),
    Lost(f32),
    /// The 3D main menu scene (no game running).
    Menu,
}

/// The sweep ends when the camera converges (but not before this many ticks), after
/// [`SWEEP_MAX`] ticks, or on a skip key (mm1.wdl:744-813).
pub const SWEEP_MIN: f32 = 8.0;
pub const SWEEP_MAX: f32 = 128.0;
/// The win screen waits for a key (after this many ticks: `AnimGagnee` blocks, mm1.wdl:947).
pub const WIN_KEY_DELAY: f32 = 16.0;
/// The lose screen ends by itself after this many ticks.
pub const LOSE_TICKS: f32 = 320.0;

/// The level being played and where it is in its flow.
#[derive(Clone, Debug, Default)]
pub struct LevelFlow {
    pub slot: Option<u32>,
    pub file: String,
    pub phase: Phase,
    /// Ticks of the level-start sweep so far.
    pub intro_ticks: f32,
}

/// What the flow reads this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlowInput {
    pub t: Ticks,
    /// Enter / Esc / pad: ends the sweep.
    pub skip: bool,
    /// Any key or button this frame.
    pub any: bool,
    /// The follow camera reached Sammy.
    pub converged: bool,
}

/// What a flow step changed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlowStep {
    /// The game ended and was recorded in the save (write it out).
    pub saved: bool,
    /// The level file to load next.
    pub load: Option<String>,
}

impl LevelFlow {
    pub fn playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    /// One frame.
    pub fn step(&mut self, input: FlowInput, game: &GameState, save: &mut SaveGame) -> FlowStep {
        let mut out = FlowStep::default();
        let t = input.t.get();
        self.phase = match self.phase {
            Phase::Intro => {
                self.intro_ticks += t;
                if (input.converged && self.intro_ticks > SWEEP_MIN) || self.intro_ticks >= SWEEP_MAX || input.skip { Phase::Playing } else { Phase::Intro }
            }
            Phase::Playing if game.won || (game.lost && self.slot != Some(EXTRO_SLOT)) => {
                // End of a game: StoreCurrentSkills, best score, ValideLevel, Save_Info
                // "BestSco" (mm1.wdl:972-997).
                if let Some(slot) = self.slot {
                    let checkpoint = game.checkpoint.then_some(game.start).flatten().map(|(p, _)| p);
                    let next = game.won.then(|| next_slot(slot)).filter(|&n| n != slot);
                    save.record_game(slot, game.score, checkpoint, game.won, next);
                    if game.won && slot == EXTRO_SLOT {
                        save.extro_seen = true;
                    }
                    out.saved = true;
                }
                if game.won { Phase::Won(0.0) } else { Phase::Lost(0.0) }
            }
            Phase::Won(ticks) | Phase::Lost(ticks) if (ticks > WIN_KEY_DELAY && input.any) || (matches!(self.phase, Phase::Lost(_)) && ticks >= LOSE_TICKS) => {
                let won = matches!(self.phase, Phase::Won(_));
                out.load = Some(match self.slot {
                    Some(slot) if won => after_win(slot, save).to_owned(),
                    _ => self.file.clone(), // lose: same level again
                });
                self.phase
            }
            Phase::Won(ticks) => Phase::Won(ticks + t),
            Phase::Lost(ticks) => Phase::Lost(ticks + t),
            p => p,
        };
        out
    }
}

/// After a win (mm1.wdl:988-1123): the last level of a world returns to the menu, unless
/// the last jump and think levels are both reached, which starts the extro; otherwise
/// the next level starts.
pub fn after_win(slot: u32, save: &SaveGame) -> &'static str {
    let [_, last_jump, last_think] = LAST_OF_WORLD;
    let both = save.state(last_jump) == LevelState::Unlocked && save.state(last_think) == LevelState::Unlocked;
    if (slot == last_jump || slot == last_think) && both {
        return file_for(EXTRO_SLOT).unwrap_or(MENU_FILE);
    }
    if LAST_OF_WORLD.contains(&slot) || slot == EXTRO_SLOT {
        return MENU_FILE;
    }
    file_for(next_slot(slot)).unwrap_or(MENU_FILE)
}

#[cfg(test)]
mod tests;
