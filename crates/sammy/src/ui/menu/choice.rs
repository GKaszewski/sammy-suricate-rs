//! Acting on menu choices.

use super::*;

pub fn in_menu(game: Res<Game>) -> bool {
    game.in_menu()
}

/// `MainMenuSammy` (interfac.wdl:403-624): the game acts on the choice.
pub(crate) fn act(mut choices: MessageReader<MenuChoice>, mut game: ResMut<Game>) {
    for MenuChoice(code) in choices.read().copied() {
        game.menu_choice(code);
    }
}

/// Debug: `SAMMY_MENU_VIEW=3` starts on a level grid, `SAMMY_OPTIONS=1` opens the options,
/// `SAMMY_MENU_CHOICE=n` clicks item n.
pub(crate) fn debug_start(mut game: ResMut<Game>, mut req: ResMut<crate::ui::options::OptionsRequest>, mut choices: MessageWriter<MenuChoice>, mut done: Local<bool>) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    if let Some(v) = std::env::var("SAMMY_MENU_VIEW").ok().and_then(|v| v.parse().ok()) {
        game.menu.view = v;
    }
    // SAMMY_MENU_CHOICE=105: as if that item had been clicked.
    if let Some(c) = std::env::var("SAMMY_MENU_CHOICE").ok().and_then(|v| v.parse().ok()) {
        choices.write(MenuChoice(c));
    }
    if std::env::var_os("SAMMY_OPTIONS").is_some() {
        req.open = Some(crate::ui::options::OptionsMode::Menu);
    }
}
