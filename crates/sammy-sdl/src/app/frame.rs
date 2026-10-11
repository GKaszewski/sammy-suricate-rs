//! One frame: input, the game tick, its events and the options' effects, then the views.

use sammy_core::input::Controls;
use sammy_core::sim::{Event, Files};
use sammy_core::time::Secs;
use sammy_present::options::{DisplayInfo, Effect};

use super::App;
use crate::input::Pointer;
use crate::profile::LOG_FRAMES;
use crate::{menu, scene};

/// Longest frame step (seconds): a hitch does not jump the game ahead.
const MAX_FRAME: f32 = 0.25;
/// Effects volume of `play_sound` without one (WDL percent).
const SOUND_VOLUME: f32 = 100.0;

/// The frame's input and window size.
pub(super) struct FrameInput {
    pub(super) dt: Secs,
    pub(super) controls: Controls,
    pub(super) pointer: Pointer,
    /// Drawable size in pixels.
    pub(super) size: (u32, u32),
}

impl FrameInput {
    pub(super) fn window_px(&self) -> [f32; 2] {
        [self.size.0 as f32, self.size.1 as f32]
    }
}

impl App {
    /// One frame; false once the debug screenshots are all taken.
    pub(super) fn frame(&mut self) -> bool {
        let t0 = self.clock.now();
        let input = self.read_input();
        self.prof.input += self.clock.now() - t0;
        let display = self.update(&input);
        self.prof_log();
        self.sync_views(&input);
        let t0 = self.clock.now();
        self.render(&input, display.as_ref());
        if self.shots.due(self.clock.since_start() as f32) {
            self.shots.save(&self.gfx.gl, input.size.0, input.size.1);
            if self.shots.done() {
                return false;
            }
        }
        self.prof.draw += self.clock.now() - t0;
        self.trace("swap");
        let t0 = self.clock.now();
        self.video.window.gl_swap_window();
        self.prof.swap += self.clock.now() - t0;
        self.trace("swapped");
        true
    }

    /// SDL events, the frame step and the controls.
    fn read_input(&mut self) -> FrameInput {
        for e in self.events.poll_iter() {
            self.input.event(&e);
        }
        let now = self.clock.now();
        // The first frame has no elapsed time (as on the Bevy frontend's first update).
        let dt = Secs(self.clock.last.map_or(0.0, |last| ((now - last) as f32).min(MAX_FRAME)));
        self.clock.last = Some(now);
        let size = self.video.window.drawable_size();
        let (w, h) = (size.0 as f32, size.1 as f32);
        self.painter.resize(w, h, sammy_present::hud::ui_scale(w, h, self.sim.save.display.ui_size as u32));
        FrameInput { dt, pointer: self.input.pointer(), controls: self.input.controls(dt), size }
    }

    /// Menu picking, the game tick and everything it asks for. Returns the display
    /// choices the options screen showed.
    fn update(&mut self, input: &FrameInput) -> Option<DisplayInfo> {
        let mut effects = self.options.open_on_key(&self.sim, &input.controls);
        self.menu.modal = self.options.model.open();
        let screen = menu::Screen { scene: &self.scene, cam: self.last_cam.as_ref(), size: input.window_px() };
        self.menu.pick(&mut self.sim, &screen, &input.controls, &input.pointer, &mut self.audio);
        if self.frames < super::TRACED_FRAMES {
            log::info!("frame {}: tick dt {:.3}", self.frames, input.dt.get());
        }
        let t0 = self.clock.now();
        self.sim.tick(self.game_dt(input.dt), &input.controls, &self.files as &dyn Files);
        self.prof.tick += self.clock.now() - t0;
        self.audio.set_volume(self.sim.save.sound);
        self.music.update(&self.sim, self.options.model.mode.is_some(), &mut self.audio, &self.files);
        self.menu.act(&mut self.sim);
        self.game_events(&mut effects);
        let info = self.display.info(&self.video.video, &self.video.window, &self.sim.save.display);
        effects.extend(self.options.update(&mut self.sim, &input.controls, &input.pointer, &self.painter, info.as_ref()));
        self.apply(effects);
        self.display.apply(&self.video.video, &mut self.video.window, &self.sim.save.display);
        self.hud.tick(&self.sim.game, self.game_dt(input.dt).ticks());
        self.frames += 1;
        info
    }

    /// Game time of a frame: none while paused.
    fn game_dt(&self, dt: Secs) -> Secs {
        if self.paused { Secs::ZERO } else { dt }
    }

    /// The game's events: effects, sounds, saves and the options it opens.
    fn game_events(&mut self, effects: &mut Vec<Effect>) {
        let drained: Vec<Event> = self.sim.drain_events().collect();
        for e in drained {
            self.fx.event(&self.sim, &e);
            match e {
                Event::Sound(name) => self.audio.play(name, SOUND_VOLUME),
                Event::Saved => sammy_platform::save::write(&self.storage, &self.sim.save),
                Event::LoadFailed { file } => log::error!("could not load level {file}"),
                Event::Loaded { file } => log::info!("loaded level {file} (slot {:?})", self.sim.flow.slot),
                Event::OpenOptions => effects.extend(self.options.show(sammy_present::options::Mode::Menu)),
                Event::Quit => self.options.model.ask_quit(),
                _ => {}
            }
        }
    }

    /// What the options overlay asks for.
    fn apply(&mut self, effects: Vec<Effect>) {
        for e in effects {
            match e {
                Effect::Sound(name, vol) => self.audio.play(name, vol),
                Effect::Pause(p) => self.paused = p,
                Effect::Abandon => self.sim.abandon(),
                Effect::GiveUp => self.sim.give_up(),
                Effect::Exit => {
                    sammy_platform::save::write(&self.storage, &self.sim.save);
                    self.input.quit = true;
                }
                Effect::SaveChanged => sammy_platform::save::write(&self.storage, &self.sim.save),
            }
        }
    }

    /// The frame-time profile, every [`LOG_FRAMES`] frames.
    fn prof_log(&mut self) {
        if self.frames.is_multiple_of(LOG_FRAMES) {
            self.prof.log(self.frames, self.clock.since_start(), self.scene.stats.get(), self.audio.music_cost());
        }
    }

    /// The camera, particles and extras, and the scene's entity views.
    fn sync_views(&mut self, input: &FrameInput) {
        let (w, h) = input.size;
        let sim = &self.sim;
        self.last_cam = sim.camera.pose.map(|pose| scene::Camera::from_pose(&pose, sim.camera.params.arc, w as f32 / h.max(1) as f32));
        self.fx.update(sim, self.last_cam.as_ref(), self.game_dt(input.dt));
        self.menu.sparkles(sim, &mut self.fx.particles);
        self.scene.extra_lights = self.fx.lights(sim);
        self.scene.sun = Some(self.fx.extras.sun);
        self.trace("sync");
        let t0 = self.clock.now();
        self.scene.sync(&self.gfx, &self.files, sim, self.last_cam.as_ref());
        self.prof.sync += self.clock.now() - t0;
        self.trace("synced");
    }
}
