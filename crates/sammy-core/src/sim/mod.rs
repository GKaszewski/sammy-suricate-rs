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
mod doors;
mod enemies;
mod extro;
mod flow;
mod gems;
mod health;
mod intro;
mod menu;
mod pickups;
mod platforms;
mod player;
mod spawn;
mod wasps;

use std::sync::Arc;

use formats::wmb::Wmb;

pub use assets::{Assets, Files, Model};
pub use camera::Camera;
pub use events::{Effect, Event};
pub use intro::IntroState;
pub use menu::{MENU_ARC, MenuItem, MenuMarker, MenuState};
pub use world::{Animator, Body, Light, Look, SpriteAnim, SpriteFacing, Table, Tag, Visual, World};

use crate::collision::{BoxCollider, Brush, Hit, Hull};
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

/// Debug overrides a frontend may set (from the environment, a console ...).
#[derive(Clone, Debug, Default)]
pub struct DebugOptions {
    /// Sammy's spawn point and pan instead of the level's.
    pub start: Option<([f32; 3], Option<f32>)>,
    /// Starting key colour (255 = master).
    pub key: Option<u32>,
    pub lives: Option<i32>,
    pub hearts: Option<f32>,
    /// Eye of Truth for the whole level.
    pub eye: bool,
    /// Ammo (count, type 1..7).
    pub ammo: Option<(i32, u32)>,
    /// Hold forward.
    pub autowalk: bool,
    /// Fire whenever possible.
    pub autofire: bool,
    /// Level objects with these actions are not spawned (lower case).
    pub skip_actions: Vec<String>,
    /// No boot intro.
    pub no_intro: bool,
    /// Give up the level after this many seconds of play (counts down).
    pub give_up_after: Option<f32>,
    /// A portrait camera on Sammy: (distance, height above his origin, angle from his
    /// facing in degrees), looking at that height.
    pub portrait: Option<[f32; 3]>,
}

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

    /// One frame: `secs` of game time (0 while paused) and this frame's input.
    pub fn tick(&mut self, secs: f32, controls: &Controls, files: &dyn Files) {
        let dt = Dt { t: secs * TICKS_PER_SEC, secs };
        if let Some(file) = self.pending_load.take() {
            self.load(&file, files);
        }
        let mut controls = controls.clone();
        if self.debug.autowalk {
            controls.forward = 1.0;
        }
        if self.playing()
            && let Some(left) = self.debug.give_up_after.as_mut()
        {
            *left -= secs;
            if *left <= 0.0 {
                self.debug.give_up_after = None;
                self.give_up();
            }
        }
        let reset = std::mem::take(&mut self.reset_scan);
        self.touched.clear();
        self.wasp_hits.clear();
        self.warn = false;

        // Before the main loop: who the camera follows, the scanners, the scenes.
        extro::update_pov(self);
        self.trigger_clock.advance(dt.t);
        intro::start(self);
        intro::tick(self, &controls, dt);
        menu::step(self, dt);

        // The main loop, in the order of the original port's frame.
        flow::run(self, &controls, dt);
        extro::fade_out(self, dt);
        spawn::snap_sprites(self);
        platforms::step(self, dt);
        self.animate(dt);
        extro::scripted_anims(self);
        camera::reset_on_respawn(self, reset);
        camera::zones(self, dt);
        doors::run(self, dt);
        player::step(self, &controls, dt);
        platforms::enter_teleports(self);
        health::detect_deaths(self, dt);
        health::collect(self, dt);
        doors::run_exits(self, dt);
        pickups::shrink(self, dt);
        doors::run_door_gems(self, dt);
        gems::surprises(self, dt);
        gems::gravity(self, dt);
        gems::lifetimes(self, dt);
        gems::fly(self, dt);
        pickups::collect(self);
        pickups::snap(self);
        platforms::run_teleports(self, dt);
        wasps::collect_ammo(self);
        wasps::fire(self, &controls, dt);
        wasps::spin_pickups(self, dt);
        wasps::escapees(self, dt);
        enemies::home_wasps(self, dt);
        wasps::fly(self, dt);
        wasps::corpses(self, dt);
        wasps::weak_doors(self);
        wasps::planks(self, dt);
        wasps::hives(self, dt);
        gems::boulders(self, dt);
        wasps::hive_wasps(self, dt);
        wasps::expire_pickups(self, dt);
        enemies::player_collider(self);
        enemies::ai(self, dt);
        extro::run_triggers(self);
        self.ai.bump();
        enemies::colliders(self);
        enemies::start_triggers(self);
        enemies::place(self);
        enemies::reset_calls(self, reset);
        health::warning_light(self, dt);
        health::run_eye(self, dt);
        health::spin(self, dt);
        health::animate_hearts(self, dt);
        health::reset_checkpoints(self, reset);
        pickups::reset_keys(self, reset);
        health::run_death(self, dt);
        extro::titles(self, &controls, dt);
        self.fade_lights(dt);
        // `WarnScoreGoal`: the score panel's blink runs out.
        self.game.warn_score = (self.game.warn_score - dt.t).max(0.0);

        // After the frame: the camera, then what hangs on it.
        camera::follow(self, dt);
        intro::glide(self, dt);
        menu::place_camera(self);
        camera::sky(self, dt);
    }

    /// Model and sprite animation clocks.
    fn animate(&mut self, dt: Dt) {
        for a in self.world.animators.values_mut() {
            a.anim.advance(dt.secs);
        }
        for a in self.world.sprite_anims.values_mut() {
            a.frame = (a.frame + a.fps * dt.secs).rem_euclid(a.frames as f32);
        }
    }

    fn fade_lights(&mut self, dt: Dt) {
        for l in self.world.lights.values_mut() {
            if let Some((rate, until)) = l.fade {
                l.range -= rate * dt.t;
                if l.range <= until {
                    l.range = 0.0;
                    l.fade = None;
                }
            }
        }
    }
}

/// The level and its solid entities, for traces.
pub(crate) struct Space<'a> {
    pub level: Option<&'a Wmb>,
    pub brushes: &'a Table<Brush>,
    pub boxes: &'a Table<BoxCollider>,
}

impl<'a> Space<'a> {
    pub fn new(level: &'a Option<Arc<Wmb>>, world: &'a World) -> Self {
        Self { level: level.as_deref(), brushes: &world.brushes, boxes: &world.boxes }
    }

    fn scene(&self) -> crate::collision::Scene<'a, impl Iterator<Item = (Id, &'a Brush)>, impl Iterator<Item = (Id, &'a BoxCollider)>> {
        crate::collision::Scene { level: self.level, brushes: self.brushes.iter().map(|(i, b)| (*i, b)), boxes: self.boxes.iter().map(|(i, b)| (*i, b)) }
    }
}

impl crate::collision::Space for Space<'_> {
    fn trace(&self, hull: Hull, start: [f32; 3], end: [f32; 3], ignore: Option<Id>) -> Hit<Id> {
        self.scene().trace(hull, start, end, ignore, true)
    }

    fn trace_static(&self, hull: Hull, start: [f32; 3], end: [f32; 3]) -> Hit<Id> {
        self.scene().trace(hull, start, end, None, false)
    }
}
