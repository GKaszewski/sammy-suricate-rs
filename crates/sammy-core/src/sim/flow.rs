//! Level flow (mm1.wdl:542-1156, Loads.wdl): loading a level (`InitLevelGlobal`,
//! `AfterLoadInitLevel`) and the phases ([`crate::flow::LevelFlow`]).

use super::{Dt, Event, Files, Sim};
use crate::camera::{CameraParams, CameraRuntime, FollowState};
use crate::enemies::LionGlobals;
use crate::extro::Pov;
use crate::flow::{FlowInput, LevelFlow, Phase};
use crate::game::GameState;
use crate::input::Controls;
use crate::levels::{EXTRO_SLOT, MENU_FILE, slot_for};
use crate::physics::JumpParams;
use crate::player::PlayerTuning;

/// Level 10 hard-codes 4 lives (Loads.wdl:80).
const FIXED_LIVES_SLOT: u32 = 10;
const FIXED_LIVES: i32 = 4;
/// Debug eye: Eye of Truth ticks for the whole level.
const DEBUG_EYE_TICKS: f32 = 1.0e9;
/// Poison per ammo type 1..7 for the debug ammo override.
const AMMO_POISON: [f32; 7] = [3.0, 2.0, 1.0, 1.0, 4.0, 6.0, 1.0];

impl Sim {
    /// Replaces the world with level `file`.
    pub(super) fn load(&mut self, file: &str, files: &dyn Files) {
        self.world.clear();
        self.level = None;
        // `InitLevelGlobal`: per-level globals back to their defaults; the level's config
        // entities then override them.
        self.game = GameState::default();
        if let Some(k) = self.debug.key {
            self.game.key = k;
        }
        self.jump = JumpParams::default();
        self.tuning = PlayerTuning::default();
        self.lions = LionGlobals::default();
        self.camera.params = CameraParams::default();
        self.camera.runtime = CameraRuntime::default();
        self.camera.follow = FollowState::default();
        self.pov = Pov::default();
        self.fin_extro = None;
        let slot = slot_for(file);
        let phase = if file.eq_ignore_ascii_case(MENU_FILE) { Phase::Menu } else { Phase::Intro };
        self.flow = LevelFlow { slot, file: file.to_owned(), phase, intro_ticks: 0.0 };
        self.loads += 1;
        if !self.spawn_level(file, files) {
            self.emit(Event::LoadFailed { file: file.to_owned() });
            return;
        }
        if slot == Some(EXTRO_SLOT) && !self.save.extro_seen {
            // `ExtroVue = 1` (Extro.wdl:15): unlocks the Bonus menu entry.
            self.save.extro_seen = true;
            self.emit(Event::Saved);
        }
        // AfterLoadInitLevel: start at the checkpoint saved for this level (InitLvl.wdl:232-237).
        if let Some(cp) = slot.and_then(|s| self.save.checkpoints.get(s as usize).copied().flatten()) {
            let mut pan = 0.0;
            for p in self.world.players.values_mut() {
                pan = p.pan;
                p.teleport(cp, pan);
            }
            self.game.start = Some((cp, pan));
            self.game.checkpoint = true; // [I] kept until the next game end stores it again
        }
        if slot == Some(FIXED_LIVES_SLOT) {
            self.game.lives = FIXED_LIVES;
        }
        // Debug overrides applied after the level's own config entities.
        if let Some(l) = self.debug.lives {
            self.game.lives = l;
        }
        if let Some(h) = self.debug.hearts {
            self.game.hearts = h;
        }
        if self.debug.eye {
            self.game.eye_ticks = DEBUG_EYE_TICKS;
        }
        if let Some((n, k)) = self.debug.ammo {
            self.game.ammo = n;
            self.game.ammo_type = k;
            self.game.ammo_poison = AMMO_POISON[(k.clamp(1, AMMO_POISON.len() as u32) - 1) as usize];
        }
        self.emit(Event::Loaded { file: file.to_owned() });
    }
}

/// The level flow's frame: sweep, play, win/lose screens.
pub(super) fn run(sim: &mut Sim, controls: &Controls, dt: Dt) {
    let input = FlowInput {
        t: dt.t,
        skip: controls.confirm || controls.options || controls.accept,
        any: controls.any,
        converged: sim.camera.follow.converged,
    };
    let step = sim.flow.step(input, &sim.game, &mut sim.save);
    if step.saved {
        sim.emit(Event::Saved);
    }
    if let Some(file) = step.load {
        sim.pending_load = Some(file);
    }
}

impl Sim {
    /// Leaves the level for the menu from the options: the game is stored like a lost
    /// one (no lose screen).
    pub fn abandon(&mut self) {
        if let Some(slot) = self.flow.slot {
            let checkpoint = self.game.checkpoint.then_some(self.game.start).flatten().map(|(p, _)| p);
            self.save.record_game(slot, self.game.score, checkpoint, false, None);
            self.emit(Event::Saved);
        }
        self.request_load(MENU_FILE);
    }

    /// `OuiRestartLevel`: no lives left → the lose screen, then the same level again.
    pub fn give_up(&mut self) {
        self.game.lives = 0;
        self.game.lost = true;
    }
}
