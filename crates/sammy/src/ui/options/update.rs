//! The frame's input for the overlay, and applying its effects.

use super::*;

/// Applies the overlay's effects.
pub(super) fn apply(effects: Vec<Effect>, game: &mut Game, storage: &crate::platform::storage::Storage, time: &mut Time<Virtual>, sfx: &mut crate::platform::audio::Sfx, volume: &mut GlobalVolume, exit: &mut MessageWriter<AppExit>) {
    for fx in effects {
        match fx {
            Effect::Sound(name, vol) => sfx.play_vol(name, vol),
            Effect::Pause(true) => time.pause(),
            Effect::Pause(false) => time.unpause(),
            Effect::Abandon => game.abandon(),
            Effect::GiveUp => game.give_up(),
            Effect::Exit => {
                crate::platform::save::write(storage, &game.save);
                exit.write(AppExit::Success);
            }
            Effect::SaveChanged => {
                volume.volume = bevy::audio::Volume::Linear(game.save.sound / 100.0);
                crate::platform::save::write(storage, &game.save);
            }
        }
    }
}

/// The display choices of this window's monitor.
pub(super) fn display(game: &Game, monitor: Option<&Monitor>) -> Option<DisplayInfo> {
    (std::env::var_os("SAMMY_SCREENSHOT").is_none()).then(|| crate::platform::display::display_info(&game.save.display, monitor))
}

/// The plate's top-left in UI pixels (centred) and the pointer on it.
pub(super) fn pointer(window: &Window, ui_scale: f32) -> ([f32; 2], Option<[f32; 2]>) {
    let size = [window.width() / ui_scale, window.height() / ui_scale];
    let origin = [(size[0] - W) / 2.0, (size[1] - H) / 2.0];
    let p = window.cursor_position().map(|c| [c.x / ui_scale - origin[0], c.y / ui_scale - origin[1]]);
    (origin, p)
}

/// The frame's input for the overlay, and its effects.
pub(super) fn update(
    mut state: ResMut<OptionsState>,
    mut game: ResMut<Game>,
    storage: Res<crate::platform::storage::Storage>,
    (mouse, keys, controls): (Res<ButtonInput<MouseButton>>, Res<ButtonInput<KeyCode>>, Res<crate::platform::input::Controls>),
    (windows, monitors, ui_scale): (Query<&Window, With<PrimaryWindow>>, Query<&Monitor, With<PrimaryMonitor>>, Res<UiScale>),
    mut time: ResMut<Time<Virtual>>,
    mut sfx: crate::platform::audio::Sfx,
    mut volume: ResMut<GlobalVolume>,
    mut exit: MessageWriter<AppExit>,
    mut menu: ResMut<crate::ui::menu::MenuState>,
    mut skip: Local<bool>,
) {
    menu.modal = state.open();
    // The key that opened the overlay must not close it in the same frame.
    let was_open = std::mem::replace(&mut *skip, state.open());
    if !was_open {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let (_, p) = pointer(window, ui_scale.0);
    let input = Input {
        pointer: p,
        pressed: mouse.just_pressed(MouseButton::Left),
        held: mouse.pressed(MouseButton::Left),
        released: mouse.just_released(MouseButton::Left),
        nav: controls.nav,
        accept: controls.accept,
        // Esc / Enter / Start / B.
        close: keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter) || controls.options || controls.back,
        yes: keys.any_just_pressed([KeyCode::KeyJ, KeyCode::KeyT, KeyCode::KeyY]),
        no: keys.just_pressed(KeyCode::KeyN),
        pad: controls.pad,
    };
    let info = display(&game, monitors.single().ok());
    let fx = state.update(&input, &mut game.save, info.as_ref());
    apply(fx, &mut game, &storage, &mut time, &mut sfx, &mut volume, &mut exit);
    menu.modal = state.open();
}
