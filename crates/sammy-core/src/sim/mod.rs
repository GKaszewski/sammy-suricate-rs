//! The whole game as one value: the loaded level's entities ([`World`]), the WDL globals,
//! the level flow, the camera and the menu/intro scenes. A frontend loads files through
//! [`Files`], calls [`Sim::tick`] once per frame with the elapsed time and the input,
//! draws [`Sim::world`] and the camera pose, and presents the drained [`Event`]s.
//!
//! The frame runs the original main loop's steps in a fixed order (see [`Sim::tick`]);
//! entities update in spawn order.

pub mod assets;
pub mod events;
pub mod world;

mod camera;
mod debug;
mod doors;
mod enemies;
mod extro;
mod flow;
mod frame;
mod gems;
mod health;
mod intro;
mod menu;
mod pickups;
mod platforms;
mod player;
mod space;
mod spawn;
mod wasps;

use std::sync::Arc;

use formats::wmb::Wmb;

pub use assets::{Assets, Files, Model};
pub use camera::Camera;
pub use debug::DebugOptions;
pub use events::{Effect, Event};
pub use intro::IntroState;
pub use menu::{MENU_ARC, MenuItem, MenuMarker, MenuState};
pub(crate) use space::Space;
pub use world::{Animator, Body, Light, Look, SpriteAnim, SpriteFacing, Table, Tag, Visual, World};

use crate::collision::{Hit, Hull};
use crate::enemies::{AiClock, FriendCalls, LionGlobals};
use crate::extro::{Pov, Scanners};
use crate::flow::LevelFlow;
use crate::game::GameState;
use crate::id::Id;
use crate::input::Controls;
use crate::physics::JumpParams;
use crate::player::PlayerTuning;
use crate::rng::Rng;
use crate::save::SaveGame;
use crate::time::{TICKS_PER_SEC, TickClock};

/// The game.
pub struct Sim {
    pub world: World,
    pub assets: Assets,
    /// The level's BSP (collision).
    pub level: Option<Arc<Wmb>>,
    pub flow: LevelFlow,
    /// Counts level loads (reloads of the same file included).
    pub loads: u32,
    pub game: GameState,
    pub save: SaveGame,
    pub rng: Rng,
    pub jump: JumpParams,
    pub tuning: PlayerTuning,
    pub lions: LionGlobals,
    /// Kept across levels.
    pub calls: FriendCalls,
    pub ai: AiClock,
    pub camera: Camera,
    /// `pov_Syn`: whom the camera follows and enemies measure to.
    pub pov: Pov,
    /// This frame's id-25 scanners.
    pub scanners: Scanners,
    /// The extro triggers' 16 Hz clock.
    trigger_clock: TickClock,
    /// `SetFinExtro` fade to black (fog percent), once started.
    pub fin_extro: Option<f32>,
    pub intro: IntroState,
    pub menu: MenuState,
    pub debug: DebugOptions,
    pending_load: Option<String>,
    events: Vec<Event>,
    /// `RestartSammy`'s reset scan, read by the next frame.
    reset_scan: bool,
    /// This frame's messages between steps.
    touched: Vec<Id>,
    wasp_hits: Vec<WaspHit>,
    /// Weak doors hit this frame.
    weak_door_hits: Vec<Id>,
    warn: bool,
}

/// A wasp hit something solid (sent to targets such as lions and boulders).
#[derive(Clone, Copy, Debug)]
pub(crate) struct WaspHit {
    pub target: Id,
    pub poison: f32,
    pub bounce_force: f32,
    pub wasp_pan: f32,
    pub wasp_disp: [f32; 3],
}

/// One frame's time: ticks (`time`) and seconds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dt {
    pub t: f32,
    pub secs: f32,
}

impl Sim {
    pub fn new(save: SaveGame, seed: u64) -> Self {
        Self {
            world: World::default(),
            assets: Assets::default(),
            level: None,
            flow: LevelFlow::default(),
            loads: 0,
            game: GameState::default(),
            save,
            rng: Rng::seeded(seed),
            jump: JumpParams::default(),
            tuning: PlayerTuning::default(),
            lions: LionGlobals::default(),
            calls: FriendCalls::default(),
            ai: AiClock::default(),
            camera: Camera::default(),
            pov: Pov::default(),
            scanners: Scanners::default(),
            trigger_clock: TickClock::default(),
            fin_extro: None,
            intro: IntroState::default(),
            menu: MenuState::default(),
            debug: DebugOptions::default(),
            pending_load: None,
            events: Vec::new(),
            reset_scan: false,
            touched: Vec::new(),
            wasp_hits: Vec::new(),
            weak_door_hits: Vec::new(),
            warn: false,
        }
    }

    /// Loads `file` at the start of the next tick.
    pub fn request_load(&mut self, file: impl Into<String>) {
        self.pending_load = Some(file.into());
    }

    /// The file waiting to be loaded, if any.
    pub fn pending_load(&self) -> Option<&str> {
        self.pending_load.as_deref()
    }

    /// The level is being played (`PartieEncours == 1`).
    pub fn playing(&self) -> bool {
        self.flow.playing()
    }

    /// Sweeps `hull` through the level and its map entities (not models), e.g. to find
    /// the floor under something.
    pub fn trace_static(&self, hull: Hull, start: [f32; 3], end: [f32; 3]) -> Hit<Id> {
        crate::collision::Space::trace_static(&Space::new(&self.level, &self.world), hull, start, end)
    }

    /// Takes this frame's events.
    pub fn drain_events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    pub(crate) fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    pub(crate) fn sound(&mut self, name: &'static str) {
        self.events.push(Event::Sound(name));
    }

    pub(crate) fn effect(&mut self, e: Effect) {
        self.events.push(Event::Effect(e));
    }

}
