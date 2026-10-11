//! Opening the overlay: menu items, Esc and debug requests.

use super::*;

/// The menu's Options and Quit items.
pub(super) fn menu_requests(mut events: MessageReader<GameEvent>, mut req: ResMut<OptionsRequest>) {
    for GameEvent(e) in events.read() {
        match e {
            sammy_core::sim::Event::OpenOptions => req.open = Some(OptionsMode::Menu),
            sammy_core::sim::Event::Quit => req.quit = true,
            _ => {}
        }
    }
}

/// Esc opens the overlay in a level or the menu (the intro skips instead).
pub(super) fn open_on_esc(controls: Res<crate::platform::input::Controls>, game: Res<Game>, state: Res<OptionsState>, mut req: ResMut<OptionsRequest>) {
    if !controls.options || state.open() || game.intro.active {
        return;
    }
    req.open = match game.flow.phase {
        Phase::Menu => Some(OptionsMode::Menu),
        Phase::Playing if game.flow.slot == Some(sammy_core::levels::EXTRO_SLOT) => Some(OptionsMode::Extro),
        Phase::Playing => Some(OptionsMode::Level),
        _ => None,
    };
}

/// Requested openings (debug `SAMMY_OPTIONS=audio` opens the audio tab).
pub(super) fn open(
    mut req: ResMut<OptionsRequest>,
    mut state: ResMut<OptionsState>,
    mut game: ResMut<Game>,
    storage: Res<crate::platform::storage::Storage>,
    mut time: ResMut<Time<Virtual>>,
    mut sfx: crate::platform::audio::Sfx,
    mut volume: ResMut<GlobalVolume>,
    mut exit: MessageWriter<AppExit>,
    mut opened: Local<bool>,
) {
    *opened = false;
    if let Some(mode) = req.open.take() {
        let tab = std::env::var("SAMMY_OPTIONS").is_ok_and(|v| v == "audio").then_some(Tab::Audio);
        let fx = state.show(mode, tab);
        apply(fx, &mut game, &storage, &mut time, &mut sfx, &mut volume, &mut exit);
    }
    if std::mem::take(&mut req.quit) {
        state.ask_quit();
    }
}
