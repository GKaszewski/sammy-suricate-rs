//! Port additions for modern screens: window mode (windowed / borderless / exclusive
//! fullscreen), resolutions, Alt+Enter / F11, and whole-number UI scaling of the
//! 640×480 interface art (`UiScale`) with a UI size setting.

use bevy::prelude::*;
use bevy::window::{Monitor, MonitorSelection, PrimaryMonitor, PrimaryWindow, VideoModeSelection, WindowMode};

use crate::game::Game;
use crate::platform::save::{DisplayMode, DisplaySettings};

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (hotkeys, apply, ui_scale).chain());
    }
}

/// The options screen's display choices on this monitor.
pub fn display_info(d: &DisplaySettings, monitor: Option<&Monitor>) -> sammy_present::options::DisplayInfo {
    let desktop = monitor.map(|m| [m.physical_width, m.physical_height]);
    let modes: Vec<[u32; 2]> = monitor.map(|m| m.video_modes.iter().map(|v| [v.physical_size.x, v.physical_size.y]).collect()).unwrap_or_default();
    sammy_present::options::DisplayInfo::new(d, desktop, &modes)
}

/// The resolution a mode uses now.
pub fn current_size(d: &DisplaySettings, monitor: Option<&Monitor>) -> Option<[u32; 2]> {
    display_info(d, monitor).current
}

/// Alt+Enter and F11 toggle between the window and the last fullscreen mode.
fn hotkeys(keys: Res<ButtonInput<KeyCode>>, storage: Res<crate::platform::storage::Storage>, mut game: ResMut<Game>, mut last_full: Local<Option<DisplayMode>>) {
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !(keys.just_pressed(KeyCode::F11) || (alt && keys.just_pressed(KeyCode::Enter))) {
        return;
    }
    game.save.display.mode = match game.save.display.mode {
        DisplayMode::Windowed => last_full.unwrap_or(DisplayMode::Borderless),
        full => {
            *last_full = Some(full);
            DisplayMode::Windowed
        }
    };
    crate::platform::save::write(&storage, &game.save);
}

/// Applies `game.save.display` to the window whenever it changes (and at start).
fn apply(game: Res<Game>, monitors: Query<&Monitor, With<PrimaryMonitor>>, mut windows: Query<&mut Window, With<PrimaryWindow>>, mut applied: Local<Option<DisplaySettings>>) {
    // Scripted screenshot runs keep the default window.
    if std::env::var_os("SAMMY_SCREENSHOT").is_some() || *applied == Some(game.save.display) {
        return;
    }
    let Ok(mut win) = windows.single_mut() else { return };
    let d = game.save.display;
    let monitor = monitors.single().ok();
    win.mode = match d.mode {
        DisplayMode::Windowed => WindowMode::Windowed,
        DisplayMode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        DisplayMode::Fullscreen => {
            // The highest refresh rate at the chosen size, else the desktop mode.
            let mode = d.fullscreen.and_then(|[w, h]| {
                monitor?.video_modes.iter().filter(|v| v.physical_size == UVec2::new(w, h)).max_by_key(|v| (v.refresh_rate_millihertz, v.bit_depth)).cloned()
            });
            WindowMode::Fullscreen(MonitorSelection::Current, mode.map_or(VideoModeSelection::Current, VideoModeSelection::Specific))
        }
    };
    if d.mode == DisplayMode::Windowed {
        win.resolution.set_physical_resolution(d.window[0], d.window[1]);
    }
    *applied = Some(d);
}

/// Whole multiples of the 640×480 art in physical pixels ([`sammy_present::hud::ui_scale`]).
fn ui_scale(game: Res<Game>, windows: Query<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let Ok(w) = windows.single() else { return };
    let s = sammy_present::hud::ui_scale(w.physical_width() as f32, w.physical_height() as f32, game.save.display.ui_size as u32);
    let s = s / w.scale_factor();
    if (scale.0 - s).abs() > 1e-4 {
        scale.0 = s;
    }
}
