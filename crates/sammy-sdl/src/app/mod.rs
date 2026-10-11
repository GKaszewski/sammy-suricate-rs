//! The running game: the window, the game, its views and the frame loop.

mod args;
mod frame;
mod render;

use glow::HasContext;
use sammy_core::sim::Sim;
use sammy_platform::{GameFiles, Storage};

use crate::{audio, display, fx, gfx, hud, input, menu, music, options, profile, scene, shot, text, ui};

pub use self::args::Args;

/// The PS Vita's screen, also the desktop window's size.
const SCREEN: (u32, u32) = (960, 544);

/// The window and its GL context.
struct Video {
    video: sdl2::VideoSubsystem,
    window: sdl2::video::Window,
    _context: sdl2::video::GLContext,
}

impl Video {
    /// A GLES 2 window with depth and stencil (model shadows darken each floor pixel once).
    fn open(sdl: &sdl2::Sdl) -> anyhow::Result<(Self, gfx::Gfx)> {
        let video = sdl.video().map_err(anyhow::Error::msg)?;
        let attr = video.gl_attr();
        attr.set_context_profile(sdl2::video::GLProfile::GLES);
        attr.set_context_version(2, 0);
        attr.set_depth_size(24);
        attr.set_stencil_size(8);
        let window = video.window("Sammy Suricate", SCREEN.0, SCREEN.1).opengl().resizable().build()?;
        let context = window.gl_create_context().map_err(anyhow::Error::msg)?;
        let _ = video.gl_set_swap_interval(1);
        let gl = unsafe { glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };
        log::info!("GL {}", unsafe { gl.get_parameter_string(glow::VERSION) });
        let gfx = gfx::Gfx::new(gl)?;
        Ok((Self { video, window, _context: context }, gfx))
    }
}

/// SDL's clock in seconds (std's monotonic clock is not reliable on every handheld).
struct Clock {
    timer: sdl2::TimerSubsystem,
    start: f64,
    /// Last frame's time, none before the first frame.
    last: Option<f64>,
}

impl Clock {
    fn now(&self) -> f64 {
        self.timer.ticks64() as f64 / 1000.0
    }

    /// Seconds since the clock started.
    fn since_start(&self) -> f64 {
        self.now() - self.start
    }
}

/// Everything a frame works on.
pub struct App {
    sim: Sim,
    files: GameFiles,
    storage: Storage,
    video: Video,
    gfx: gfx::Gfx,
    events: sdl2::EventPump,
    mouse: sdl2::mouse::MouseUtil,
    clock: Clock,
    input: input::Input,
    audio: audio::Audio,
    music: music::Music,
    scene: scene::Scene,
    hud: hud::HudView,
    fx: fx::Fx,
    painter: ui::Painter,
    fonts: text::Fonts,
    menu: menu::MenuView,
    options: options::OptionsView,
    display: display::Display,
    /// A level is frozen while the options overlay is up.
    paused: bool,
    /// The camera of the last frame (menu picking runs before this frame's tick).
    last_cam: Option<scene::Camera>,
    shots: shot::Shots,
    frames: u64,
    prof: profile::Profile,
    /// Debug: `SAMMY_DRAW` lists what to draw (scene,level,sky,entities,hud).
    draw: String,
    /// The SDL context, kept alive for its subsystems.
    _sdl: sdl2::Sdl,
}

impl App {
    /// Opens the game files, the save, the window and the views, and asks for the
    /// first level.
    pub fn new(args: Args) -> anyhow::Result<Self> {
        let storage = Storage::user();
        sammy_platform::debug::load_env_file(&storage);
        let files = sammy_platform::game_files(args.data.clone(), &storage)?;
        let mut sim = sammy_platform::debug::new_sim(sammy_platform::save::load(&storage, &files));
        sim.request_load(args.level_file());

        let sdl = sdl2::init().map_err(anyhow::Error::msg)?;
        let (video, gfx) = Video::open(&sdl)?;
        let mut options = options::OptionsView::new(&gfx, &files);
        if std::env::var_os("SAMMY_OPTIONS").is_some() {
            options.show(sammy_present::options::Mode::Menu);
        }
        let timer = sdl.timer().map_err(anyhow::Error::msg)?;
        let start = timer.ticks64() as f64 / 1000.0;
        Ok(Self {
            input: input::Input::new(&sdl),
            audio: audio::Audio::new(&sdl, &files),
            music: music::Music::new(&storage, &files),
            scene: scene::Scene::default(),
            hud: hud::HudView::new(&gfx, &files),
            fx: fx::Fx::new(&gfx),
            painter: ui::Painter::new(&gfx),
            fonts: text::Fonts::new(&gfx, &files),
            menu: menu::MenuView::new(&gfx, &files),
            options,
            display: display::Display::default(),
            paused: false,
            last_cam: None,
            shots: shot::Shots::from_env(),
            frames: 0,
            prof: profile::Profile::default(),
            draw: std::env::var("SAMMY_DRAW").unwrap_or_else(|_| "scene,level,sky,entities,hud".into()),
            events: sdl.event_pump().map_err(anyhow::Error::msg)?,
            mouse: sdl.mouse(),
            clock: Clock { timer, start, last: None },
            sim,
            files,
            storage,
            video,
            gfx,
            _sdl: sdl,
        })
    }

    /// Runs frames until the player quits (or the debug screenshots are taken).
    pub fn run(&mut self) {
        while !self.input.quit && self.frame() {}
    }

    /// Logs `what` with the time during the first frames (start-up tracing).
    fn trace(&self, what: &str) {
        if self.frames < TRACED_FRAMES {
            log::info!("frame {}: {what} {:.2}", self.frames, self.clock.since_start());
        }
    }
}

/// The first frames are traced in the log.
const TRACED_FRAMES: u64 = 6;
