//! Sammy Suricate on SDL2 + OpenGL ES 2: the game ([`sammy_core::sim::Sim`]) with a
//! light renderer, for handhelds (PS Vita) and any desktop.
//!
//! `sammy-sdl [--data <game folder>] [slot | file.wmb | menu]`. Debug: the `SAMMY_*`
//! overrides of [`sammy_platform::debug`], and `SAMMY_SCREENSHOT=out.png` with
//! `SAMMY_SHOT_SECS=2.5,4` to save frames and quit.

mod audio;
mod gfx;
mod hud;
mod input;
mod scene;
mod shot;

use std::time::Instant;

use glow::HasContext;
use sammy_core::sim::{Event, Files};

/// The PS Vita's screen, also the desktop window's size.
const SCREEN: (u32, u32) = (960, 544);
/// Longest frame step (seconds): a hitch does not jump the game ahead.
const MAX_FRAME: f32 = 0.25;
/// Effects volume of `play_sound` without one (WDL percent).
const SOUND_VOLUME: f32 = 100.0;

struct Log;

impl log::Log for Log {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            eprintln!("{} {}: {}", r.level(), r.target(), r.args());
        }
    }
    fn flush(&self) {}
}

fn main() {
    let _ = log::set_logger(&Log).map(|_| log::set_max_level(log::LevelFilter::Info));
    if let Err(e) = run() {
        log::error!("{e:#}");
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let data = argv.iter().position(|a| a == "--data").map(|i| {
        let dir = argv.get(i + 1).cloned().unwrap_or_default();
        argv.drain(i..(i + 2).min(argv.len()));
        std::path::PathBuf::from(dir)
    });
    let storage = sammy_platform::Storage::user();
    let files = sammy_platform::game_files(data, &storage)?;
    let mut sim = sammy_platform::debug::new_sim(sammy_platform::save::load(&storage, &files));
    let level = match argv.first().map(String::as_str) {
        None | Some("menu") => sammy_core::levels::MENU_FILE.to_owned(),
        Some(l) => l.parse::<u32>().ok().and_then(sammy_core::levels::file_for).map_or_else(|| l.to_owned(), str::to_owned),
    };
    sim.request_load(level);

    let sdl = sdl2::init().map_err(anyhow::Error::msg)?;
    let video = sdl.video().map_err(anyhow::Error::msg)?;
    let attr = video.gl_attr();
    attr.set_context_profile(sdl2::video::GLProfile::GLES);
    attr.set_context_version(2, 0);
    attr.set_depth_size(24);
    let window = video.window("Sammy Suricate", SCREEN.0, SCREEN.1).opengl().resizable().build()?;
    let _ctx = window.gl_create_context().map_err(anyhow::Error::msg)?;
    let _ = video.gl_set_swap_interval(1);
    let gl = unsafe { glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };
    log::info!("GL {}", unsafe { gl.get_parameter_string(glow::VERSION) });
    let gfx = gfx::Gfx::new(gl)?;
    let mut input = input::Input::new(&sdl);
    let mut audio = audio::Audio::new(&sdl, &files);
    let mut scene = scene::Scene::default();
    let mut hud = hud::HudView::new(&gfx, &files);
    let mut shots = shot::Shots::from_env();
    let mut events = sdl.event_pump().map_err(anyhow::Error::msg)?;

    let start = Instant::now();
    let mut last = start;
    let mut first = true;
    while !input.quit {
        for e in events.poll_iter() {
            input.event(&e, &sdl);
        }
        let now = Instant::now();
        // The first frame has no elapsed time (as on the Bevy frontend's first update).
        let dt = if first { 0.0 } else { (now - last).as_secs_f32().min(MAX_FRAME) };
        first = false;
        last = now;
        let controls = input.controls(dt);
        sim.tick(dt, &controls, &files as &dyn Files);
        audio.set_volume(sim.save.sound);
        let drained: Vec<Event> = sim.drain_events().collect();
        for e in drained {
            match e {
                Event::Sound(name) => audio.play(name, SOUND_VOLUME),
                Event::Saved => sammy_platform::save::write(&storage, &sim.save),
                Event::LoadFailed { file } => log::error!("could not load level {file}"),
                Event::Loaded { file } => log::info!("loaded level {file} (slot {:?})", sim.flow.slot),
                Event::Quit => input.quit = true,
                _ => {}
            }
        }
        hud.tick(&sim.game, dt * sammy_core::time::TICKS_PER_SEC);
        scene.sync(&gfx, &files, &sim);

        let (w, h) = window.drawable_size();
        unsafe {
            gfx.gl.viewport(0, 0, w as i32, h as i32);
            gfx.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gfx.gl.depth_mask(true);
            gfx.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
        if let Some(pose) = sim.camera.pose {
            let cam = scene::Camera::from_pose(&pose, sim.camera.params.arc, w as f32 / h.max(1) as f32);
            scene.draw(&gfx, &sim, &cam);
        }
        if sim.flow.phase != sammy_core::flow::Phase::Menu {
            hud.draw(&gfx, &sim.game, w as f32, h as f32, sim.save.display.ui_size as u32);
        }
        if shots.due(start.elapsed().as_secs_f32()) {
            shots.save(&gfx.gl, w, h);
            if shots.done() {
                break;
            }
        }
        window.gl_swap_window();
    }
    Ok(())
}
