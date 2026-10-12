//! Drawing the overlay as UI nodes.

use super::*;

/// Draws the overlay: rebuilt when what it shows changes; the focus frame follows the
/// pad focus.
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    state: Res<OptionsState>,
    game: Res<Game>,
    strings: Option<Res<Strings>>,
    files: Res<GameFiles>,
    mut images: ResMut<Assets<Image>>,
    (windows, monitors, ui_scale, controls): (Query<&Window, With<PrimaryWindow>>, crate::platform::display::Monitors, Res<UiScale>, Res<crate::platform::input::Controls>),
    roots: Query<Entity, With<OptionsRoot>>,
    mut cache: Local<HashMap<String, Handle<Image>>>,
    mut shown: Local<Vec<Item>>,
) {
    let (Some(strings), Ok(window)) = (strings, windows.single()) else { return };
    if !state.open() {
        for e in &roots {
            commands.entity(e).despawn();
        }
        shown.clear();
        return;
    }
    let (origin, p) = pointer(window, ui_scale.0);
    let info = display(&game, crate::platform::display::game_monitor(&monitors));
    let items = state.elements(&strings, &game.save, info.as_ref(), p, controls.pad);
    let focus = controls.pad.then(|| state.focus_area(info.as_ref())).flatten();
    if items == *shown && !roots.is_empty() && focus.is_none() {
        return;
    }
    *shown = items.clone();
    for e in &roots {
        commands.entity(e).despawn();
    }
    let dim = if state.mode.is_some() { OVERLAY_ALPHA } else { 0.0 };
    let mut img = |name: &str| cache.entry(name.to_owned()).or_insert_with(|| load_ui_image(&files, &mut images, name).map(|(h, _)| h).unwrap_or_default()).clone();
    let abs = |a: [f32; 4]| Node { position_type: PositionType::Absolute, left: px(a[0]), top: px(a[1]), width: px(a[2]), height: px(a[3]), ..default() };
    commands
        .spawn((
            OptionsRoot,
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, dim)),
            GlobalZIndex(Z_OVERLAY),
        ))
        .with_children(|root| {
            root.spawn((Plate, abs([origin[0], origin[1], W, H]))).with_children(|plate| {
                for item in items {
                    match item.element {
                        Element::Image { image, src, area } => {
                            let rect = src.map(|s| Rect::new(s[0], s[1], s[0] + s[2], s[1] + s[3]));
                            plate.spawn((abs(area), ImageNode { image: img(&image), rect, ..default() }));
                        }
                        Element::Text { font, narrow, bytes, at, centre_in } => {
                            let text = if narrow { BitmapText::new(font, bytes).narrow() } else { BitmapText::new(font, bytes) };
                            match centre_in {
                                Some(a) => {
                                    plate.spawn(Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..abs(a) }).with_children(|r| {
                                        r.spawn((Node { position_type: PositionType::Relative, ..default() }, text));
                                    });
                                }
                                None => {
                                    plate.spawn((Node { position_type: PositionType::Absolute, left: px(at[0]), top: px(at[1]), ..default() }, text));
                                }
                            }
                        }
                    }
                }
                // A frame around the focused control while the pad is in use.
                if let Some(a) = focus {
                    let [r, g, b] = FRAME_RGB;
                    plate.spawn((
                        FocusFrame,
                        abs([a[0] - FRAME_PAD / 2.0, a[1] - FRAME_PAD / 2.0, a[2] + FRAME_PAD, a[3] + FRAME_PAD]),
                        BorderColor::all(Color::srgb(r, g, b)),
                        BackgroundColor(Color::srgba(r, g, b, FRAME_FILL_ALPHA)),
                        GlobalZIndex(Z_FOCUS_FRAME),
                    ))
                    .insert(Node { border: UiRect::all(px(FRAME_BORDER)), ..abs([a[0] - FRAME_PAD / 2.0, a[1] - FRAME_PAD / 2.0, a[2] + FRAME_PAD, a[3] + FRAME_PAD]) });
                }
            });
        });
}
