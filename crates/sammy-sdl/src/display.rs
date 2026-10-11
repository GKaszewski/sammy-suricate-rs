//! Desktop window modes.

/// Window modes on desktops (windowed / borderless / fullscreen at a resolution) from
/// the save's display settings; handhelds have one fixed screen.
#[derive(Default)]
pub struct Display {
    pub(super) applied: Option<sammy_core::save::DisplaySettings>,
}

impl Display {
    /// Desktops change the display only when not making screenshots.
    pub(super) fn enabled() -> bool {
        !cfg!(target_os = "vita") && std::env::var_os("SAMMY_SCREENSHOT").is_none()
    }

    /// The options screen's display choices (None on fixed screens).
    pub fn info(&self, video: &sdl2::VideoSubsystem, window: &sdl2::video::Window, d: &sammy_core::save::DisplaySettings) -> Option<sammy_present::options::DisplayInfo> {
        if !Self::enabled() {
            return None;
        }
        let index = window.display_index().unwrap_or(0);
        let desktop = video.desktop_display_mode(index).ok().map(|m| [m.w as u32, m.h as u32]);
        let modes: Vec<[u32; 2]> = (0..video.num_display_modes(index).unwrap_or(0)).filter_map(|i| video.display_mode(index, i).ok()).map(|m| [m.w as u32, m.h as u32]).collect();
        Some(sammy_present::options::DisplayInfo::new(d, desktop, &modes))
    }

    /// Applies `d` when it changed (and at start).
    pub fn apply(&mut self, video: &sdl2::VideoSubsystem, window: &mut sdl2::video::Window, d: &sammy_core::save::DisplaySettings) {
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
