//! The 2D overlay: version text, tooltips, the cursor and the hidden HUD.

use sammy_present::menu::{CURSOR_HOTSPOT, CURSOR_SIZE, VERSION_POS, VERSION_TEXT};

use super::*;

/// UI stacking: version text, tooltips, cursor.
const Z_VERSION: i32 = 20;
const Z_TOOLTIP: i32 = 21;
const Z_CURSOR: i32 = 100;

pub(crate) fn setup_overlay(mut commands: Commands, files: Res<GameFiles>, mut images: ResMut<Assets<Image>>) {
    let abs = |x: f32, y: f32| Node { position_type: PositionType::Absolute, left: px(x), top: px(y), ..default() };
    commands.spawn((VersionText, abs(VERSION_POS[0], VERSION_POS[1]), BitmapText::new(FontKind::Standard, VERSION_TEXT.to_vec()), GlobalZIndex(Z_VERSION), Visibility::Hidden));
    commands.spawn((TooltipTitle, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    commands.spawn((TooltipMessage, abs(0.0, 0.0), BitmapText::new(FontKind::Small, Vec::new()), GlobalZIndex(Z_TOOLTIP), Visibility::Hidden));
    if let Some((image, _)) = load_ui_image(&files, &mut images, sammy_present::menu::CURSOR_FILE) {
        commands.spawn((Cursor, Node { position_type: PositionType::Absolute, width: px(CURSOR_SIZE), height: px(CURSOR_SIZE), ..default() }, ImageNode::new(image), GlobalZIndex(Z_CURSOR), Visibility::Hidden));
    }
}

/// Version text and the level tooltips that follow the cursor (interfac.wdl:840-902, :1016-1112).
#[allow(clippy::type_complexity)]
pub(crate) fn overlay(
    state: Res<MenuState>,
    game: Res<Game>,
    strings: Option<Res<Strings>>,
    ui_scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut version: Query<&mut Visibility, (With<VersionText>, Without<TooltipTitle>, Without<TooltipMessage>)>,
    mut title: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipTitle>, Without<TooltipMessage>)>,
    mut message: Query<(&mut BitmapText, &mut Node, &mut Visibility), (With<TooltipMessage>, Without<TooltipTitle>)>,
) {
    let Some(strings) = strings else { return };
    if let Ok(mut v) = version.single_mut() {
        *v = if state.clicked_once || state.modal { Visibility::Hidden } else { Visibility::Inherited };
    }
    let cursor = windows.single().ok().and_then(|w| w.cursor_position()).map(|c| c / ui_scale.0).unwrap_or_default();
    let tip = game.menu.hovered.and_then(|h| game.world.menu_items.get(&h)).and_then(|i| sammy_present::menu::tooltip(i, &game.save, &strings, cursor.to_array()));
    let (Ok((mut tt, mut tn, mut tv)), Ok((mut mt, mut mn, mut mv))) = (title.single_mut(), message.single_mut()) else { return };
    let Some(tip) = tip else {
        *tv = Visibility::Hidden;
        *mv = Visibility::Hidden;
        return;
    };
    if tt.bytes != tip.title {
        tt.bytes = tip.title;
    }
    *tv = Visibility::Inherited;
    (tn.left, tn.top) = (px(tip.title_at[0]), px(tip.title_at[1]));
    match tip.message {
        Some((bytes, at)) => {
            if mt.bytes != bytes {
                mt.bytes = bytes;
            }
            (mn.left, mn.top) = (px(at[0]), px(at[1]));
            *mv = Visibility::Inherited;
        }
        None => *mv = Visibility::Hidden,
    }
}

/// The diamond cursor (`arrow.pcx`, hot spot 8,8) replaces the system one in the menu
/// and in modal overlays.
pub(crate) fn show_cursor(
    game: Res<Game>,
    state: Res<MenuState>,
    controls: Res<crate::platform::input::Controls>,
    ui_scale: Res<UiScale>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor: Query<(&mut Node, &mut Visibility), With<Cursor>>,
) {
    let Ok((win, mut opts)) = windows.single_mut() else { return };
    let on = game.in_menu() || state.modal;
    if opts.visible == on {
        opts.visible = !on;
    }
    let Ok((mut n, mut v)) = cursor.single_mut() else { return };
    match win.cursor_position().filter(|_| on && !controls.pad) {
        Some(c) => {
            let c = c / ui_scale.0;
            (n.left, n.top) = (px(c.x - CURSOR_HOTSPOT), px(c.y - CURSOR_HOTSPOT));
            *v = Visibility::Inherited;
        }
        None => *v = Visibility::Hidden,
    }
}

/// The HUD only shows in a game; the menu's texts only in the menu.
#[allow(clippy::type_complexity)]
pub(crate) fn hide_hud(
    game: Res<Game>,
    mut hud: ResMut<crate::ui::hud::HudVisible>,
    mut texts: Query<&mut Visibility, Or<(With<VersionText>, With<TooltipTitle>, With<TooltipMessage>)>>,
) {
    let show = game.flow.phase != Phase::Menu;
    if hud.0 != show {
        hud.0 = show;
    }
    // `Texte_Version` only appears once `MainMenuSammy` starts.
    if show || game.intro.active {
        for mut v in &mut texts {
            *v = Visibility::Hidden;
        }
    }
}
