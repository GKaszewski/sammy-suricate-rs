//! Sammy Suricate on SDL2 + OpenGL ES 2: the game ([`sammy_core::sim::Sim`]) with a
//! light renderer, for handhelds (PS Vita) and any desktop.
//!
//! `sammy-sdl [--data <game folder>] [slot | file.wmb | menu]` (or `SAMMY_LEVEL`). Debug: the `SAMMY_*`
//! overrides of [`sammy_platform::debug`], and `SAMMY_SCREENSHOT=out.png` with
//! `SAMMY_SHOT_SECS=2.5,4` to save frames and quit.

mod audio;
mod fx;
mod gfx;
mod hud;
mod input;
mod menu;
mod music;
mod options;
mod scene;
mod shot;
mod text;
mod ui;

use glow::HasContext;
use sammy_core::sim::{Event, Files};

/// The PS Vita's screen, also the desktop window's size.
const SCREEN: (u32, u32) = (960, 544);
/// Longest frame step (seconds): a hitch does not jump the game ahead.
const MAX_FRAME: f32 = 0.25;
/// Progress is logged every this many frames.
const LOG_FRAMES: u64 = 60;
/// Effects volume of `play_sound` without one (WDL percent).
const SOUND_VOLUME: f32 = 100.0;

/// Logs to stderr, and on the Vita also to `log.txt` in the game's storage (opened per
/// line, so nothing is lost if the app is killed).
struct Log {
    file: Option<std::sync::Mutex<std::path::PathBuf>>,
}

impl log::Log for Log {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let line = format!("{} {}: {}", r.level(), r.target(), r.args());
            eprintln!("{line}");
            if let Some(path) = &self.file
                && let Ok(path) = path.lock()
                && let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&*path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{line}");
            }
        }
    }
    fn flush(&self) {}
}

fn main() {
    let file = if cfg!(target_os = "vita") {
        let storage = sammy_platform::Storage::user();
        storage.dir().map(|d| {
            let path = d.join("log.txt");
            let _ = std::fs::write(&path, b"");
            path
        })
    } else {
        None
    };
    let logger: &'static Log = Box::leak(Box::new(Log { file: file.map(std::sync::Mutex::new) }));
    let _ = log::set_logger(logger).map(|_| log::set_max_level(log::LevelFilter::Info));
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
    sammy_platform::debug::load_env_file(&storage);
    let files = sammy_platform::game_files(data, &storage)?;
    let mut sim = sammy_platform::debug::new_sim(sammy_platform::save::load(&storage, &files));
    // The level: the command line, else `SAMMY_LEVEL` (e.g. from the Vita's env.txt).
    let level_arg = argv.first().cloned().or_else(|| std::env::var("SAMMY_LEVEL").ok());
    let level = match level_arg.as_deref() {
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
    // Model shadows darken each floor pixel once.
    attr.set_stencil_size(8);
    let mut window = video.window("Sammy Suricate", SCREEN.0, SCREEN.1).opengl().resizable().build()?;
    let _ctx = window.gl_create_context().map_err(anyhow::Error::msg)?;
    let _ = video.gl_set_swap_interval(1);
    let gl = unsafe { glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };
    log::info!("GL {}", unsafe { gl.get_parameter_string(glow::VERSION) });
    let gfx = gfx::Gfx::new(gl)?;
    let mut input = input::Input::new(&sdl);
    let mut audio = audio::Audio::new(&sdl, &files);
    let mut music = music::Music::new(&storage, &files);
    let mut scene = scene::Scene::default();
    let mut hud = hud::HudView::new(&gfx, &files);
    let mut fx = fx::Fx::new(&gfx);
    let mut painter = ui::Painter::new(&gfx);
    let fonts = text::Fonts::new(&gfx, &files);
    let mut menu = menu::MenuView::new(&gfx, &files);
    let mut options = options::OptionsView::new(&gfx, &files);
    // A level is frozen while the options overlay is up.
    let mut paused = false;
    let mut display = Display::default();
    if std::env::var_os("SAMMY_OPTIONS").is_some() {
        options.show(sammy_present::options::Mode::Menu);
    }
    let mouse = sdl.mouse();
    // The camera of the last frame (menu picking runs before this frame's tick).
    let mut last_cam: Option<scene::Camera> = None;
    let mut shots = shot::Shots::from_env();
    let mut events = sdl.event_pump().map_err(anyhow::Error::msg)?;
    // SDL's clock (std's monotonic clock is not reliable on every handheld).
    let timer = sdl.timer().map_err(anyhow::Error::msg)?;
    let secs = || timer.ticks64() as f64 / 1000.0;
    let start = secs();
    let mut last = start;
    let mut first = true;
    let mut frames = 0u64;
    let mut prof = Profile::default();
    // Debug: SAMMY_DRAW lists what to draw (scene,level,sky,entities,hud).
    let draw = std::env::var("SAMMY_DRAW").unwrap_or_else(|_| "scene,level,sky,entities,hud".into());
    while !input.quit {
        let t0 = secs();
        for e in events.poll_iter() {
            input.event(&e);
        }

        let now = secs();
        // The first frame has no elapsed time (as on the Bevy frontend's first update).
        let dt = if first { 0.0 } else { ((now - last) as f32).min(MAX_FRAME) };
        first = false;
        last = now;
        let pointer = input.pointer();
        let controls = input.controls(dt);
        let (w, h) = window.drawable_size();
        let window_px = [w as f32, h as f32];
        painter.resize(w as f32, h as f32, sammy_present::hud::ui_scale(w as f32, h as f32, sim.save.display.ui_size as u32));
        prof.input += secs() - t0;
        let mut effects = options.open_on_key(&sim, &controls);
        menu.modal = options.model.open();
        menu.pick(&mut sim, &menu::Screen { scene: &scene, cam: last_cam.as_ref(), size: window_px }, &controls, &pointer, &mut audio);
        if frames < 6 {
            log::info!("frame {frames}: tick dt {dt:.3}");
        }
        let t0 = secs();
        sim.tick(if paused { 0.0 } else { dt }, &controls, &files as &dyn Files);
        prof.tick += secs() - t0;
        audio.set_volume(sim.save.sound);
        music.update(&sim, options.model.mode.is_some(), &mut audio, &files);
        menu.act(&mut sim);
        let drained: Vec<Event> = sim.drain_events().collect();
        for e in drained {
            fx.event(&sim, &e);
            match e {
                Event::Sound(name) => audio.play(name, SOUND_VOLUME),
                Event::Saved => sammy_platform::save::write(&storage, &sim.save),
                Event::LoadFailed { file } => log::error!("could not load level {file}"),
                Event::Loaded { file } => log::info!("loaded level {file} (slot {:?})", sim.flow.slot),
                Event::OpenOptions => effects.extend(options.show(sammy_present::options::Mode::Menu)),
                Event::Quit => options.model.ask_quit(),
                _ => {}
            }
        }
        let info = display.info(&video, &window, &sim.save.display);
        effects.extend(options.update(&mut sim, &controls, &pointer, &painter, info.as_ref()));
        for fx in effects {
            use sammy_present::options::Effect;
            match fx {
                Effect::Sound(name, vol) => audio.play(name, vol),
                Effect::Pause(p) => paused = p,
                Effect::Abandon => sim.abandon(),
                Effect::GiveUp => sim.give_up(),
                Effect::Exit => {
                    sammy_platform::save::write(&storage, &sim.save);
                    input.quit = true;
                }
                Effect::SaveChanged => sammy_platform::save::write(&storage, &sim.save),
            }
        }
        display.apply(&video, &mut window, &sim.save.display);
        hud.tick(&sim.game, if paused { 0.0 } else { dt * sammy_core::time::TICKS_PER_SEC });
        frames += 1;
        if frames.is_multiple_of(LOG_FRAMES) {
            prof.log(frames, secs() - start, scene.stats.get(), audio.music_cost());
        }
        let trace = |what: &str| {
            if frames < 6 {
                log::info!("frame {frames}: {what} {:.2}", secs() - start);
            }
        };
        last_cam = sim.camera.pose.map(|pose| scene::Camera::from_pose(&pose, sim.camera.params.arc, w as f32 / h.max(1) as f32));
        fx.update(&sim, last_cam.as_ref(), if paused { 0.0 } else { dt });
        menu.sparkles(&sim, &mut fx.particles);
        scene.extra_lights = fx.lights(&sim);
        scene.sun = Some(fx.extras.sun);
        trace("sync");
        let t0 = secs();
        scene.sync(&gfx, &files, &sim, last_cam.as_ref());
        prof.sync += secs() - t0;
        trace("synced");
        let t0 = secs();

        unsafe {
            gfx.gl.viewport(0, 0, w as i32, h as i32);
            gfx.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gfx.gl.depth_mask(true);
            gfx.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
        }
        if let Some(cam) = last_cam.as_ref().filter(|_| draw.contains("scene")) {
            scene.draw(&gfx, &sim, cam, &draw);
            fx.draw_world(&gfx, &files, &mut scene, &sim, cam);
            scene.draw_near(&gfx, &sim, cam);
            let shown = sim.flow.phase != sammy_core::flow::Phase::Menu && draw.contains("hud");
            fx.draw_over(&gfx, &files, &mut scene, &sim, cam, &fx::HudScreen { window: window_px, ui_scale: painter.scale, shown });
            trace("scene drawn");
        }
        // The intro: black fog over the view, the logo in front of it.
        let (logo, fog) = last_cam.as_ref().map_or((None, 0.0), |c| sammy_present::intro::overlay(&sim, c.pos.into(), c.axes.x_axis.into()));
        if fog > 0.0 {
            painter.begin(&gfx);
            painter.rect(&gfx, [0.0, 0.0, painter.size[0], painter.size[1]], [0.0, 0.0, 0.0, fog]);
        }
        if let Some(cam) = &last_cam {
            scene.draw_logo(&gfx, &files, cam, logo.as_ref());
        }
        painter.begin(&gfx);
        if sim.flow.phase != sammy_core::flow::Phase::Menu && draw.contains("hud") {
            hud.draw(&gfx, &painter, &sim.game);
        }
        let canvas = ui::Canvas { gfx: &gfx, ui: &painter, fonts: &fonts };
        menu.draw(&canvas, &sim, &menu::Screen { scene: &scene, cam: last_cam.as_ref(), size: window_px }, &pointer);
        options.draw(&canvas, &sim, &pointer, controls.pad, info.as_ref());
        painter.end(&gfx);
        mouse.show_cursor(!menu.owns_cursor(&sim));
        if shots.due((secs() - start) as f32) {
            shots.save(&gfx.gl, w, h);
            if shots.done() {
                break;
            }
        }
        prof.draw += secs() - t0;
        trace("swap");
        let t0 = secs();
        window.gl_swap_window();
        prof.swap += secs() - t0;
        trace("swapped");
    }
    Ok(())
}

/// Where the frame time goes, logged every [`LOG_FRAMES`] frames: SDL events and input,
/// the game tick, the music's share of real time (audio thread), scene
/// sync (animated meshes), draw submission, and the buffer swap (waiting on the GPU
/// and vsync), with the scene's counts.
#[derive(Default)]
struct Profile {
    tick: f64,
    sync: f64,
    draw: f64,
    swap: f64,
    input: f64,
    since: f64,
}

impl Profile {
    fn log(&mut self, frames: u64, now: f64, s: scene::Stats, music: f64) {
        let n = LOG_FRAMES as f64;
        let ms = |v: f64| v / n * 1000.0;
        let fps = n / (now - self.since).max(1e-6);
        log::info!(
            "frame {frames} at {now:.1} s: {fps:.0} fps | ms tick {:.1} sync {:.1} draw {:.1} swap {:.1} input {:.1} | music {:.1}% | drawn {} culled {} animated {} lights {}",
            ms(self.tick),
            ms(self.sync),
            ms(self.draw),
            ms(self.swap),
            ms(self.input),
            music * 100.0,
            s.drawn,
            s.culled,
            s.animated,
            s.lights
        );
        *self = Self { since: now, ..Self::default() };
    }
}

/// Window modes on desktops (windowed / borderless / fullscreen at a resolution) from
/// the save's display settings; handhelds have one fixed screen.
#[derive(Default)]
struct Display {
    applied: Option<sammy_core::save::DisplaySettings>,
}

impl Display {
    /// Desktops change the display only when not making screenshots.
    fn enabled() -> bool {
        !cfg!(target_os = "vita") && std::env::var_os("SAMMY_SCREENSHOT").is_none()
    }

    /// The options screen's display choices (None on fixed screens).
    fn info(&self, video: &sdl2::VideoSubsystem, window: &sdl2::video::Window, d: &sammy_core::save::DisplaySettings) -> Option<sammy_present::options::DisplayInfo> {
        if !Self::enabled() {
            return None;
        }
        let index = window.display_index().unwrap_or(0);
        let desktop = video.desktop_display_mode(index).ok().map(|m| [m.w as u32, m.h as u32]);
        let modes: Vec<[u32; 2]> = (0..video.num_display_modes(index).unwrap_or(0)).filter_map(|i| video.display_mode(index, i).ok()).map(|m| [m.w as u32, m.h as u32]).collect();
        Some(sammy_present::options::DisplayInfo::new(d, desktop, &modes))
    }

    /// Applies `d` when it changed (and at start).
    fn apply(&mut self, video: &sdl2::VideoSubsystem, window: &mut sdl2::video::Window, d: &sammy_core::save::DisplaySettings) {
        if !Self::enabled() || self.applied == Some(*d) {
            return;
        }
        self.applied = Some(*d);
        use sammy_core::save::DisplayMode;
        use sdl2::video::FullscreenType;
        let result = match d.mode {
            DisplayMode::Windowed => window.set_fullscreen(FullscreenType::Off).and_then(|_| window.set_size(d.window[0], d.window[1]).map_err(|e| e.to_string())),
            DisplayMode::Borderless => window.set_fullscreen(FullscreenType::Desktop),
            DisplayMode::Fullscreen => {
                let index = window.display_index().unwrap_or(0);
                let size = d.fullscreen.or_else(|| video.desktop_display_mode(index).ok().map(|m| [m.w as u32, m.h as u32]));
                let mode = size.and_then(|[w, h]| video.closest_display_mode(index, &sdl2::video::DisplayMode::new(sdl2::pixels::PixelFormatEnum::Unknown, w as i32, h as i32, 0)).ok());
                window.set_display_mode(mode).and_then(|_| window.set_fullscreen(FullscreenType::True))
            }
        };
        if let Err(e) = result {
            log::warn!("display mode: {e}");
        }
    }
}
