//! Sammy Suricate on SDL2 + OpenGL ES 2: the game ([`sammy_core::sim::Sim`]) with a
//! light renderer, for handhelds (PS Vita) and any desktop.
//!
//! `sammy-sdl [--data <game folder>] [slot | file.wmb | menu]` (or `SAMMY_LEVEL`). Debug: the `SAMMY_*`
//! overrides of [`sammy_platform::debug`], and `SAMMY_SCREENSHOT=out.png` with
//! `SAMMY_SHOT_SECS=2.5,4` to save frames and quit.

mod app;
mod audio;
mod display;
mod fx;
mod gfx;
mod hud;
mod input;
mod logger;
mod menu;
mod music;
mod options;
mod profile;
mod scene;
mod shot;
mod text;
mod ui;

fn main() {
    logger::install();
    if let Err(e) = run() {
        log::error!("{e:#}");
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    app::App::new(app::Args::from_env())?.run();
    Ok(())
}
