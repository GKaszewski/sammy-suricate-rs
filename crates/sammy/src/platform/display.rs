//! Port additions for modern screens: window mode (windowed / borderless / exclusive
//! fullscreen), resolutions, Alt+Enter / F11, and whole-number UI scaling of the
//! 640×480 interface art (`UiScale`) with a UI size setting.

use bevy::prelude::*;
use bevy::window::{Monitor, MonitorSelection, PrimaryMonitor, PrimaryWindow, VideoModeSelection, WindowMode};

use crate::platform::save::{DisplayMode, DisplaySettings, SaveGame};

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (hotkeys, apply, ui_scale).chain());
    }
}

/// The interface's design resolution (the original's mode 6).
const W: f32 = 640.0;
const H: f32 = 480.0;
/// Smallest fullscreen mode offered (the original art size).
const MIN_FULLSCREEN: [u32; 2] = [640, 480];
/// Windows smaller than the art fit it fractionally, but never below this scale.
const MIN_FRACTIONAL_SCALE: f32 = 0.25;

/// Windowed sizes offered (those that fit the monitor).
const WINDOW_SIZES: [[u32; 2]; 10] = [[640, 480], [800, 600], [1024, 768], [1280, 720], [1280, 960], [1366, 768], [1600, 900], [1920, 1080], [2560, 1440], [3840, 2160]];

/// Sizes selectable for the current mode, for the options screen.
pub fn sizes(mode: DisplayMode, monitor: Option<&Monitor>) -> Vec<[u32; 2]> {
    let max = monitor.map_or([u32::MAX; 2], |m| [m.physical_width, m.physical_height]);
    match mode {
        DisplayMode::Windowed => WINDOW_SIZES.iter().copied().filter(|s| s[0] <= max[0] && s[1] <= max[1]).collect(),
        DisplayMode::Fullscreen => {
            let mut v: Vec<[u32; 2]> = monitor.map(|m| m.video_modes.iter().map(|v| [v.physical_size.x, v.physical_size.y]).collect()).unwrap_or_default();
            v.sort();
            v.dedup();
            v.retain(|s| s[0] >= MIN_FULLSCREEN[0] && s[1] >= MIN_FULLSCREEN[1]);
            v
        }
        DisplayMode::Borderless => Vec::new(),
    }
}

/// The resolution a mode uses now (for display).
pub fn current_size(d: &DisplaySettings, monitor: Option<&Monitor>) -> Option<[u32; 2]> {
    let desktop = monitor.map(|m| [m.physical_width, m.physical_height]);
    match d.mode {
        DisplayMode::Windowed => Some(d.window),
        DisplayMode::Borderless => desktop,
        DisplayMode::Fullscreen => d.fullscreen.or(desktop),
    }
}

/// Alt+Enter and F11 toggle between the window and the last fullscreen mode.
fn hotkeys(keys: Res<ButtonInput<KeyCode>>, mut save: ResMut<SaveGame>, mut last_full: Local<Option<DisplayMode>>) {
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !(keys.just_pressed(KeyCode::F11) || (alt && keys.just_pressed(KeyCode::Enter))) {
        return;
    }
    save.display.mode = match save.display.mode {
        DisplayMode::Windowed => last_full.unwrap_or(DisplayMode::Borderless),
        full => {
            *last_full = Some(full);
            DisplayMode::Windowed
        }
    };
    crate::platform::save::write(&save);
}

/// Applies `save.display` to the window whenever it changes (and at start).
fn apply(save: Res<SaveGame>, monitors: Query<&Monitor, With<PrimaryMonitor>>, mut windows: Query<&mut Window, With<PrimaryWindow>>, mut applied: Local<Option<DisplaySettings>>) {
    // Scripted screenshot runs keep the default window.
    if std::env::var_os("SAMMY_SCREENSHOT").is_some() || *applied == Some(save.display) {
        return;
    }
    let Ok(mut win) = windows.single_mut() else { return };
    let d = save.display;
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

/// Whole multiples of the 640×480 art in physical pixels (sharp pixels): "large" is the
/// biggest multiple that fits, "medium"/"small" one/two less; windows smaller than
/// 640×480 get a fractional fit.
fn ui_scale(save: Res<SaveGame>, windows: Query<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let Ok(w) = windows.single() else { return };
    let fit = (w.physical_height() as f32 / H).min(w.physical_width() as f32 / W);
    let s = if fit < 1.0 { fit.max(MIN_FRACTIONAL_SCALE) } else { (fit.floor() - save.display.ui_size as f32).max(1.0) };
    let s = s / w.scale_factor();
    if (scale.0 - s).abs() > 1e-4 {
        scale.0 = s;
    }
}
