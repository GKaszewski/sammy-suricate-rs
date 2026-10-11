//! The display rows' labels and steps.

use super::*;

pub(super) fn row_label(r: Row, save: &SaveGame, display: Option<&DisplayInfo>) -> Vec<u8> {
    let d = &save.display;
    let label = match r {
        Row::Mode => format!(
            "<  Tryb: {}  >",
            match d.mode {
                DisplayMode::Windowed => "okno",
                DisplayMode::Borderless => "bez ramki",
                DisplayMode::Fullscreen => "pełny ekran",
            }
        ),
        Row::Size => match (d.mode, display.and_then(|i| i.current)) {
            (DisplayMode::Borderless, Some([w, h])) => format!("Pulpit {w} x {h}"),
            (_, Some([w, h])) => format!("<  {w} x {h}  >"),
            _ => String::new(),
        },
        Row::UiSize => format!("<  Interfejs: {}  >", ["duży", "średni", "mały"][d.ui_size.min(UI_SIZE_MAX) as usize]),
    };
    cp1250(&label)
}

pub(super) fn step_row(r: Row, step: i32, save: &mut SaveGame, display: Option<&DisplayInfo>) {
    let d = &mut save.display;
    let cycle = |i: usize, n: usize| (i as i32 + step).rem_euclid(n as i32) as usize;
    match r {
        Row::Mode => {
            let modes = [DisplayMode::Windowed, DisplayMode::Borderless, DisplayMode::Fullscreen];
            let i = modes.iter().position(|m| *m == d.mode).unwrap_or(0);
            d.mode = modes[cycle(i, modes.len())];
        }
        Row::Size => {
            let Some(info) = display.filter(|i| !i.sizes.is_empty()) else { return };
            let cur = info.current.unwrap_or(info.sizes[0]);
            let i = info.sizes.iter().position(|s| *s == cur).unwrap_or(0);
            let next = info.sizes[cycle(i, info.sizes.len())];
            match d.mode {
                DisplayMode::Windowed => d.window = next,
                DisplayMode::Fullscreen => d.fullscreen = Some(next),
                DisplayMode::Borderless => {}
            }
        }
        Row::UiSize => d.ui_size = (UI_SIZE_MAX as usize - cycle(UI_SIZE_MAX as usize - d.ui_size.min(UI_SIZE_MAX) as usize, UI_SIZE_MAX as usize + 1)) as u8,
    }
}
