#[allow(unused_imports)]
use super::*;
use crate::ui::hud::{H, W};

/// `MenuOptionsGame`: 0 front-end, 1 level, 2 extro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptionsMode {
    Menu,
    Level,
    Extro,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Tab {
    Audio,
    #[default]
    Help,
    Credits,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Question {
    Quit,
    Restart,
}

pub(crate) const TRACK: f32 = 227.0;

pub(crate) const THUMB: f32 = 32.0;

/// An `w×h` element at (x, y) on the 640×480 options plate (options.wdl).
#[derive(Clone, Copy)]
struct Area {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Area {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn node(self) -> Node {
        at(self.x, self.y, self.w, self.h)
    }
}

/// Tab buttons.
const TAB_AUDIO: Area = Area::new(41.0, 14.0, 218.0, 80.0);
const TAB_HELP: Area = Area::new(41.0, 105.0, 226.0, 47.0);
const TAB_CREDITS: Area = Area::new(41.0, 152.0, 226.0, 46.0);
/// Resume / back / quit / restart buttons.
const BTN_RESUME: Area = Area::new(0.0, 335.0, 306.0, 42.0);
const BTN_BACK: Area = Area::new(0.0, 377.0, 306.0, 40.0);
const BTN_QUIT: Area = Area::new(0.0, 417.0, 306.0, 41.0);
const BTN_RESTART: Area = Area::new(48.0, 268.0, 204.0, 50.0);
/// Yes/No dialog: plate, text origin and the two buttons.
const DIALOG_PLATE: Area = Area::new(223.0, 135.0, 199.0, 68.0);
const DIALOG_TEXT_POS: (f32, f32) = (228.0, 138.0);
const DIALOG_YES: Area = Area::new(235.0, 171.0, 68.0, 28.0);
const DIALOG_NO: Area = Area::new(341.0, 171.0, 68.0, 28.0);
/// Audio tab: music/sound art (and the sub-rect of `volmus` it shows).
const VOLMUS: Area = Area::new(347.0, 17.0, 266.0, 249.0);
/// Slider tracks: x, thumb height and the two rows' y.
const SLIDER_X: f32 = 365.0;
const SLIDER_H: f32 = 29.0;
const SLIDER_MUSIC_Y: f32 = 60.0;
const SLIDER_SOUND_Y: f32 = 161.0;
/// Slider range (percent) and the pad step.
const SLIDER_MAX: f32 = 100.0;
const SLIDER_STEP: f32 = 5.0;
/// Display rows: x, size and the three rows' y.
const ROW_X: f32 = 352.0;
const ROW_W: f32 = 256.0;
const ROW_H: f32 = 24.0;
const ROW_MODE_Y: f32 = 272.0;
const ROW_SIZE_Y: f32 = 302.0;
const ROW_UI_SIZE_Y: f32 = 346.0;
/// Highest `ui_size` setting (0 large, 1 medium, 2 small).
const UI_SIZE_MAX: u8 = 2;
/// Help tab: text origin, picture, previous / next page buttons.
const HELP_TEXT_POS: (f32, f32) = (334.0, 20.0);
const HELP_PICTURE: Area = Area::new(330.0, 297.0, 297.0, 113.0);
const HELP_PREV: Area = Area::new(330.0, 408.0, 65.0, 62.0);
const HELP_NEXT: Area = Area::new(560.0, 408.0, 65.0, 62.0);
/// Help pages 1-7; page 2 has no picture.
const HELP_LAST_PAGE: u32 = 7;
const HELP_NO_PICTURE_PAGE: u32 = 2;
/// Credits tab: team picture and the "Thanksto" text origin.
const CREDITS_TEAM: Area = Area::new(340.0, 25.0, 282.0, 294.0);
const CREDITS_THANKS_POS: (f32, f32) = (334.0, 320.0);
/// Volume of the button / row sounds.
const UI_SFX_VOL: f32 = 50.0;
/// Overlay dimming alpha and stacking: overlay, dialog, focus frame.
const OVERLAY_ALPHA: f32 = 0.7;
const Z_OVERLAY: i32 = 50;
const Z_DIALOG: i32 = 60;
const Z_FOCUS_FRAME: i32 = 70;
/// Focus frame: border, glow colour (RGB), fill alpha and padding around the item.
const FRAME_BORDER: f32 = 2.0;
const FRAME_RGB: (f32, f32, f32) = (1.0, 0.85, 0.2);
const FRAME_FILL_ALPHA: f32 = 0.15;
const FRAME_PAD: f32 = 6.0;

/// The menu's Options and Quit items.
pub(crate) fn menu_requests(mut events: MessageReader<crate::game::GameEvent>, mut req: ResMut<OptionsRequest>) {
    for crate::game::GameEvent(e) in events.read() {
        match e {
            sammy_core::sim::Event::OpenOptions => req.open = Some(OptionsMode::Menu),
            sammy_core::sim::Event::Quit => req.quit = true,
            _ => {}
        }
    }
}

pub(crate) fn open_on_esc(
    controls: Res<crate::platform::input::Controls>,
    game: Res<Game>,
    state: Res<OptionsState>,
    mut req: ResMut<OptionsRequest>,
) {
    // Esc skips the intro instead (and its release is awaited, Intro2.wdl:209).
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

pub(crate) fn img(files: &GameFiles, images: &mut Assets<Image>, name: &str) -> Handle<Image> {
    load_ui_image(files, images, name).map(|(h, _)| h).unwrap_or_default()
}

pub(crate) fn at(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node { position_type: PositionType::Absolute, left: px(x), top: px(y), width: px(w), height: px(h), ..default() }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn open_close(
    mut commands: Commands,
    mut req: ResMut<OptionsRequest>,
    mut state: ResMut<OptionsState>,
    mut menu: ResMut<crate::ui::menu::MenuState>,
    mut time: ResMut<Time<Virtual>>,
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<crate::platform::input::Controls>,
    files: Res<GameFiles>,
    mut images: ResMut<Assets<Image>>,
    roots: Query<Entity, With<OptionsRoot>>,
    dialogs: Query<Entity, With<DialogRoot>>,
    strings: Option<Res<WdlStrings>>,
) {
    if let Some(mode) = req.open.take() {
        state.mode = Some(mode);
        state.dirty = true;
        state.help_page = state.help_page.max(1);
        // Debug: SAMMY_OPTIONS=audio opens on the audio/video tab.
        if std::env::var("SAMMY_OPTIONS").is_ok_and(|v| v == "audio") {
            state.tab = Tab::Audio;
        }
        if mode != OptionsMode::Menu {
            time.pause(); // `Freeze_mode = 1`
        }
        let mut im = |n: &str| img(&files, &mut images, n);
        let (bg, resume, back, quit, restart) = (
            im("fondopt1.pcx"),
            (im("resum1.pcx"), im("resum2.pcx")),
            (im("back1.pcx"), im("back2.pcx")),
            (im("quit1.pcx"), im("quit2.pcx")),
            (im("restart2.pcx"), im("restart3.pcx")),
        );
        let tabs = [
            (Tab::Audio, TAB_AUDIO, im("audio1.pcx"), im("audio2.pcx")),
            (Tab::Help, TAB_HELP, im("help1.pcx"), im("help2.pcx")),
            (Tab::Credits, TAB_CREDITS, im("credits1.pcx"), im("credits2.pcx")),
        ];
        commands
            .spawn((
                OptionsRoot,
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, OVERLAY_ALPHA)),
                GlobalZIndex(Z_OVERLAY),
            ))
            .with_children(|root| {
                root.spawn((Node { width: px(W), height: px(H), ..default() }, ImageNode::new(bg))).with_children(|c| {
                    for (tab, area, n, hv) in tabs {
                        c.spawn((area.node(), RelativeCursorPosition::default(), ImageNode::new(n.clone()), ButtonImages { normal: n, hover: hv, lit: false }, Action::Tab(tab)));
                    }
                    let mut button = |(n, hv): (Handle<Image>, Handle<Image>), area: Area, action: Action| {
                        c.spawn((area.node(), RelativeCursorPosition::default(), ImageNode::new(n.clone()), ButtonImages { normal: n, hover: hv, lit: false }, action));
                    };
                    if mode != OptionsMode::Menu {
                        button(resume, BTN_RESUME, Action::Resume);
                    }
                    button(back, BTN_BACK, Action::Back);
                    button(quit, BTN_QUIT, Action::Quit);
                    if mode == OptionsMode::Level {
                        button(restart, BTN_RESTART, Action::Restart);
                    }
                    c.spawn((TabContent, at(0.0, 0.0, W, H)));
                });
            });
    }
    if std::mem::take(&mut req.quit) && state.dialog.is_none() {
        state.dialog = Some(Question::Quit);
    }
    // Esc / Enter close the overlay (`Fin_MenuOptions`); Esc cancels a dialog first.
    // Pad: Start closes too, B backs out.
    let esc = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter) || controls.options || controls.back;
    if esc && state.dialog.is_some() {
        state.dialog = None;
    } else if esc && state.mode.is_some() && roots.iter().next().is_some() && !state.dirty {
        close(&mut commands, &mut state, &mut time, &roots);
    }
    // Yes/No dialog panel.
    match (state.dialog, dialogs.iter().next()) {
        (Some(q), None) => {
            let mut im = |n: &str| img(&files, &mut images, n);
            let (bg, yes, no) = (im("fondyn.pcx"), (im("oui1.pcx"), im("oui2.pcx")), (im("non1.pcx"), im("non2.pcx")));
            let text = strings.as_ref().map(|s| s.get(if q == Question::Quit { "QuitYesNo_Str" } else { "RestartYesNo_Str" }).to_vec()).unwrap_or_default();
            commands
                .spawn((
                    DialogRoot,
                    Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    GlobalZIndex(Z_DIALOG),
                ))
                .with_children(|root| {
                    root.spawn(Node { width: px(W), height: px(H), ..default() }).with_children(|c| {
                        c.spawn((DIALOG_PLATE.node(), ImageNode::new(bg)));
                        c.spawn((Node { position_type: PositionType::Absolute, left: px(DIALOG_TEXT_POS.0), top: px(DIALOG_TEXT_POS.1), ..default() }, BitmapText::new(FontKind::Small, text)));
                        c.spawn((DIALOG_YES.node(), RelativeCursorPosition::default(), ImageNode::new(yes.0.clone()), ButtonImages { normal: yes.0, hover: yes.1, lit: false }, Action::Yes));
                        c.spawn((DIALOG_NO.node(), RelativeCursorPosition::default(), ImageNode::new(no.0.clone()), ButtonImages { normal: no.0, hover: no.1, lit: false }, Action::No));
                    });
                });
        }
        (None, Some(e)) => commands.entity(e).despawn(),
        _ => {}
    }
    menu.modal = state.open();
}

pub(crate) fn close(commands: &mut Commands, state: &mut OptionsState, time: &mut Time<Virtual>, roots: &Query<Entity, With<OptionsRoot>>) {
    for e in roots {
        commands.entity(e).despawn();
    }
    state.mode = None;
    state.dialog = None;
    time.unpause();
}

/// (Re)builds the selected tab's content: sliders, help page or credits.
pub(crate) fn build_tab(
    mut commands: Commands,
    mut state: ResMut<OptionsState>,
    files: Res<GameFiles>,
    mut images: ResMut<Assets<Image>>,
    strings: Option<Res<WdlStrings>>,
    content: Query<Entity, With<TabContent>>,
    mut tabs: Query<(&Action, &mut ButtonImages)>,
) {
    if !state.dirty {
        return;
    }
    let Ok(root) = content.single() else { return };
    state.dirty = false;
    for (a, mut b) in &mut tabs {
        if let Action::Tab(t) = a {
            b.lit = *t == state.tab;
        }
    }
    commands.entity(root).despawn_children();
    let mut im = |n: &str| img(&files, &mut images, n);
    let text = |name: &str| strings.as_ref().map(|s| s.get(name).to_vec()).unwrap_or_default();
    let tab = state.tab;
    let page = state.help_page;
    let (thumb, volmus, prev, next) = (im("butvol1.pcx"), im("volmus.pcx"), (im("flechga1.pcx"), im("flechga2.pcx")), (im("flechdr1.pcx"), im("flechdr2.pcx")));
    let picture = (page != HELP_NO_PICTURE_PAGE).then(|| im(&format!("help0{page}.pcx")));
    let team = im("lateam.pcx");
    commands.entity(root).with_children(|c| match tab {
        Tab::Audio => {
            // The music/sound art and the "Rozdzielczość grafiki" heading; the old
            // resolution slider below it is replaced by the display rows.
            c.spawn((VOLMUS.node(), ImageNode { image: volmus, rect: Some(Rect::new(0.0, 0.0, VOLMUS.w, VOLMUS.h)), ..default() }));
            for (slider, y) in [(Slider::Music, SLIDER_MUSIC_Y), (Slider::Sound, SLIDER_SOUND_Y)] {
                c.spawn((at(SLIDER_X, y, TRACK, SLIDER_H), RelativeCursorPosition::default(), slider)).with_children(|t| {
                    t.spawn((at(0.0, 0.0, THUMB, SLIDER_H), ImageNode::new(thumb.clone()), SliderThumb(slider)));
                });
            }
            for (choice, y) in [(Choice::Mode, ROW_MODE_Y), (Choice::Size, ROW_SIZE_Y), (Choice::UiSize, ROW_UI_SIZE_Y)] {
                c.spawn((
                    Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..at(ROW_X, y, ROW_W, ROW_H) },
                    RelativeCursorPosition::default(),
                    choice,
                ))
                .with_children(|r| {
                    r.spawn((Node { position_type: PositionType::Relative, ..default() }, BitmapText::new(FontKind::Help, Vec::<u8>::new()).narrow()));
                });
            }
        }
        Tab::Help => {
            c.spawn((Node { position_type: PositionType::Absolute, left: px(HELP_TEXT_POS.0), top: px(HELP_TEXT_POS.1), ..default() }, BitmapText::new(FontKind::Help, text(&format!("Help{page}_Str"))).narrow()));
            if let Some(p) = picture {
                c.spawn((HELP_PICTURE.node(), ImageNode::new(p)));
            }
            if page > 1 {
                c.spawn((HELP_PREV.node(), RelativeCursorPosition::default(), ImageNode::new(prev.0.clone()), ButtonImages { normal: prev.0, hover: prev.1, lit: false }, Action::PrevPage));
            }
            if page < HELP_LAST_PAGE {
                c.spawn((HELP_NEXT.node(), RelativeCursorPosition::default(), ImageNode::new(next.0.clone()), ButtonImages { normal: next.0, hover: next.1, lit: false }, Action::NextPage));
            }
        }
        Tab::Credits => {
            c.spawn((CREDITS_TEAM.node(), ImageNode::new(team)));
            c.spawn((Node { position_type: PositionType::Absolute, left: px(CREDITS_THANKS_POS.0), top: px(CREDITS_THANKS_POS.1), ..default() }, BitmapText::new(FontKind::Help, text("Thanksto_Str")).narrow()));
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn buttons(
    mut commands: Commands,
    mut state: ResMut<OptionsState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut q: Query<(Entity, &RelativeCursorPosition, &Action, &ButtonImages, &mut ImageNode)>,
    (focus, controls): (Res<Focus>, Res<crate::platform::input::Controls>),
    mut sfx: crate::platform::audio::Sfx,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<OptionsRoot>>,
    (mut game, storage, mut exit): (ResMut<Game>, Res<crate::platform::storage::Storage>, MessageWriter<AppExit>),
) {
    // Hover shows the "2" image (the selected tab / device stays lit).
    let mut fired = None;
    for (e, rel, action, imgs, mut node) in &mut q {
        let focused = focus.0 == Some(e) && controls.pad;
        let want = if rel.cursor_over || imgs.lit || focused { &imgs.hover } else { &imgs.normal };
        if node.image != *want {
            node.image = want.clone();
        }
        if (rel.cursor_over && mouse.just_pressed(MouseButton::Left)) || (focus.0 == Some(e) && controls.accept) {
            fired = Some(*action);
        }
    }
    let Some(action) = fired else { return };
    // Dialog buttons only while the dialog is up, overlay buttons only without it.
    if state.dialog.is_some() != matches!(action, Action::Yes | Action::No) {
        return;
    }
    match action {
        Action::Tab(t) => {
            sfx.play_vol(if t == Tab::Credits { "tigre_son" } else { "Ding2" }, UI_SFX_VOL);
            state.tab = t;
            state.dirty = true;
        }
        Action::PrevPage | Action::NextPage => {
            sfx.play_vol("Ding1", UI_SFX_VOL);
            state.help_page = if action == Action::PrevPage { state.help_page.saturating_sub(1).max(1) } else { (state.help_page + 1).min(HELP_LAST_PAGE) };
            state.dirty = true;
        }
        Action::Resume => {
            sfx.play_vol("Ding2", UI_SFX_VOL);
            close(&mut commands, &mut state, &mut time, &roots);
        }
        Action::Back => {
            sfx.play_vol("Ding2", UI_SFX_VOL);
            let mode = state.mode;
            close(&mut commands, &mut state, &mut time, &roots);
            // In a game: abandon it (stored like a lost game, no lose screen) and go to the menu.
            if mode != Some(OptionsMode::Menu) {
                game.abandon();
            }
        }
        Action::Quit => {
            sfx.play_vol("Ding2", UI_SFX_VOL);
            state.dialog = Some(Question::Quit);
        }
        Action::Restart => state.dialog = Some(Question::Restart),
        Action::Yes => answer(true, &mut commands, &mut state, &mut time, &roots, &mut game, &storage, &mut exit),
        Action::No => answer(false, &mut commands, &mut state, &mut time, &roots, &mut game, &storage, &mut exit),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn answer(
    yes: bool,
    commands: &mut Commands,
    state: &mut OptionsState,
    time: &mut Time<Virtual>,
    roots: &Query<Entity, With<OptionsRoot>>,
    game: &mut Game,
    storage: &crate::platform::storage::Storage,
    exit: &mut MessageWriter<AppExit>,
) {
    let q = state.dialog.take();
    if !yes {
        return;
    }
    match q {
        Some(Question::Quit) => {
            // `QuitSammy`: save, then exit.
            crate::platform::save::write(storage, &game.save);
            exit.write(AppExit::Success);
        }
        Some(Question::Restart) => {
            // `OuiRestartLevel`: close, `BallesJoueur1 = 0` → lose screen, same level again.
            close(commands, state, time, roots);
            game.give_up();
        }
        None => {}
    }
}

/// Slider dragging (`HSLIDER`, track 227 px with a 32-px thumb); left/right on a
/// focused slider steps it by 5.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sliders(
    mut game: ResMut<Game>,
    storage: Res<crate::platform::storage::Storage>,
    mouse: Res<ButtonInput<MouseButton>>,
    (focus, controls): (Res<Focus>, Res<crate::platform::input::Controls>),
    tracks: Query<(Entity, &Slider, &RelativeCursorPosition)>,
    mut thumbs: Query<(&SliderThumb, &mut Node)>,
    mut volume: ResMut<GlobalVolume>,
    mut dragging: Local<Option<u8>>,
) {
    if !mouse.pressed(MouseButton::Left) {
        *dragging = None;
    }
    let mut changed = false;
    for (e, slider, rel) in &tracks {
        let id = *slider as u8;
        let mut value = match slider {
            Slider::Music => game.save.music,
            Slider::Sound => game.save.sound,
        };
        if mouse.just_pressed(MouseButton::Left) && rel.cursor_over {
            *dragging = Some(id);
        }
        if *dragging == Some(id)
            && let Some(p) = rel.normalized
        {
            value = ((p.x + 0.5).clamp(0.0, 1.0) * SLIDER_MAX).round();
        } else if focus.0 == Some(e)
            && let Some(d) = controls.nav_dir().filter(|d| d.x != 0)
        {
            value = (value + SLIDER_STEP * d.x as f32).clamp(0.0, SLIDER_MAX);
            changed = true;
        } else {
            continue;
        }
        match slider {
            Slider::Music => game.save.music = value,
            Slider::Sound => {
                game.save.sound = value;
                volume.volume = bevy::audio::Volume::Linear(game.save.sound / SLIDER_MAX);
            }
        }
    }
    if mouse.just_released(MouseButton::Left) || changed {
        crate::platform::save::write(&storage, &game.save);
    }
    for (thumb, mut node) in &mut thumbs {
        let f = match thumb.0 {
            Slider::Music => game.save.music / SLIDER_MAX,
            Slider::Sound => game.save.sound / SLIDER_MAX,
        };
        node.left = px(f * (TRACK - THUMB));
    }
}

/// Display rows: a click on the left half steps back, on the right half forward; on a
/// focused row left/right do the same.
pub(crate) fn choices(
    mut game: ResMut<Game>,
    storage: Res<crate::platform::storage::Storage>,
    mouse: Res<ButtonInput<MouseButton>>,
    (focus, controls): (Res<Focus>, Res<crate::platform::input::Controls>),
    monitors: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
    rows: Query<(Entity, &Choice, &RelativeCursorPosition)>,
    mut sfx: crate::platform::audio::Sfx,
) {
    use crate::platform::save::DisplayMode;
    let monitor = monitors.single().ok();
    for (e, choice, rel) in &rows {
        let step = if rel.cursor_over && mouse.just_pressed(MouseButton::Left) {
            rel.normalized.map(|p| if p.x < 0.0 { -1 } else { 1 })
        } else if focus.0 == Some(e) {
            controls.nav_dir().filter(|d| d.x != 0).map(|d| d.x).or(controls.accept.then_some(1))
        } else {
            None
        };
        let Some(step) = step else { continue };
        let d = &mut game.save.display;
        let cycle = |i: usize, n: usize| (i as i32 + step).rem_euclid(n as i32) as usize;
        match choice {
            Choice::Mode => {
                let modes = [DisplayMode::Windowed, DisplayMode::Borderless, DisplayMode::Fullscreen];
                let i = modes.iter().position(|m| *m == d.mode).unwrap_or(0);
                d.mode = modes[cycle(i, modes.len())];
            }
            Choice::Size => {
                let list = crate::platform::display::sizes(d.mode, monitor);
                if list.is_empty() {
                    continue;
                }
                let cur = crate::platform::display::current_size(d, monitor).unwrap_or(list[0]);
                let i = list.iter().position(|s| *s == cur).unwrap_or(0);
                let next = list[cycle(i, list.len())];
                match d.mode {
                    DisplayMode::Windowed => d.window = next,
                    DisplayMode::Fullscreen => d.fullscreen = Some(next),
                    DisplayMode::Borderless => {}
                }
            }
            Choice::UiSize => d.ui_size = (UI_SIZE_MAX as usize - cycle(UI_SIZE_MAX as usize - d.ui_size.min(UI_SIZE_MAX) as usize, UI_SIZE_MAX as usize + 1)) as u8,
        }
        sfx.play_vol("Toc2", UI_SFX_VOL);
        crate::platform::save::write(&storage, &game.save);
        return;
    }
}

pub(crate) fn choice_labels(
    game: Res<Game>,
    monitors: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
    rows: Query<(&Choice, &Children)>,
    mut texts: Query<&mut BitmapText>,
) {
    use crate::platform::save::DisplayMode;
    let d = &game.save.display;
    let monitor = monitors.single().ok();
    for (choice, children) in &rows {
        let label = match choice {
            Choice::Mode => format!("<  Tryb: {}  >", match d.mode {
                DisplayMode::Windowed => "okno",
                DisplayMode::Borderless => "bez ramki",
                DisplayMode::Fullscreen => "pełny ekran",
            }),
            Choice::Size => match (d.mode, crate::platform::display::current_size(d, monitor)) {
                (DisplayMode::Borderless, Some([w, h])) => format!("Pulpit {w} x {h}"),
                (_, Some([w, h])) => format!("<  {w} x {h}  >"),
                _ => String::new(),
            },
            Choice::UiSize => format!("<  Interfejs: {}  >", ["duży", "średni", "mały"][d.ui_size.min(UI_SIZE_MAX) as usize]),
        };
        let bytes = crate::ui::text::cp1250(&label);
        for c in children.iter() {
            if let Ok(mut t) = texts.get_mut(c)
                && t.bytes != bytes
            {
                t.bytes = bytes.clone();
            }
        }
    }
}

/// Moves the focus with the D-pad / stick / arrows: to the nearest focusable in that
/// direction (dialog buttons only while a dialog is up). Sliders and display rows use
/// left/right themselves while focused.
#[allow(clippy::type_complexity)]
pub(crate) fn focus_nav(
    state: Res<OptionsState>,
    controls: Res<crate::platform::input::Controls>,
    mut focus: ResMut<Focus>,
    items: Query<(Entity, &ComputedNode, &bevy::ui::UiGlobalTransform, &InheritedVisibility, Option<&Action>, Has<Slider>, Has<Choice>), Or<(With<Action>, With<Slider>, With<Choice>)>>,
) {
    if !state.open() {
        focus.0 = None;
        return;
    }
    let dialog = state.dialog.is_some();
    let list: Vec<(Entity, Vec2, bool)> = items
        .iter()
        .filter(|(_, n, _, v, a, ..)| v.get() && n.size().x > 0.0 && dialog == matches!(a, Some(Action::Yes | Action::No)))
        .map(|(e, _, t, _, _, slider, choice)| (e, t.translation, slider || choice))
        .collect();
    let current = focus.0.and_then(|f| list.iter().find(|i| i.0 == f));
    let Some(&(_, at, horizontal)) = current else {
        // Nothing focused yet: the first pad press picks the top-left item (or "Yes").
        if controls.pad && (controls.nav.is_some() || controls.accept) {
            focus.0 = list.iter().min_by(|a, b| (a.1.y + a.1.x * 0.5).total_cmp(&(b.1.y + b.1.x * 0.5))).map(|i| i.0);
        }
        return;
    };
    let Some(d) = controls.nav_dir() else { return };
    if horizontal && d.x != 0 {
        return;
    }
    // Screen y grows downwards.
    let dir = Vec2::new(d.x as f32, -d.y as f32);
    focus.0 = list
        .iter()
        .filter(|i| i.0 != focus.0.unwrap())
        .filter_map(|i| {
            let v = i.1 - at;
            let along = v.dot(dir);
            (along > crate::ui::menu::PAD_MIN_ALONG).then(|| (i.0, along + crate::ui::menu::PAD_CROSS_WEIGHT * (v - dir * along).length()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|i| i.0)
        .or(focus.0);
}

/// A frame around the focused item while the pad is in use.
pub(crate) fn focus_frame(
    mut commands: Commands,
    focus: Res<Focus>,
    controls: Res<crate::platform::input::Controls>,
    items: Query<(&ComputedNode, &bevy::ui::UiGlobalTransform)>,
    mut frame: Query<(&mut Node, &mut Visibility), With<FocusFrame>>,
) {
    let target = focus.0.filter(|_| controls.pad).and_then(|e| items.get(e).ok());
    let Ok((mut node, mut vis)) = frame.single_mut() else {
        commands.spawn((
            FocusFrame,
            Node { position_type: PositionType::Absolute, border: UiRect::all(px(FRAME_BORDER)), ..default() },
            BorderColor::all(Color::srgb(FRAME_RGB.0, FRAME_RGB.1, FRAME_RGB.2)),
            BackgroundColor(Color::srgba(FRAME_RGB.0, FRAME_RGB.1, FRAME_RGB.2, FRAME_FILL_ALPHA)),
            GlobalZIndex(Z_FOCUS_FRAME),
            Visibility::Hidden,
        ));
        return;
    };
    let Some((n, t)) = target else {
        *vis = Visibility::Hidden;
        return;
    };
    let k = n.inverse_scale_factor();
    let size = n.size() * k + Vec2::splat(FRAME_PAD);
    let c = t.translation * k;
    (node.left, node.top, node.width, node.height) = (px(c.x - size.x / 2.0), px(c.y - size.y / 2.0), px(size.x), px(size.y));
    *vis = Visibility::Inherited;
}

/// Yes/No keys: J/T/Y = yes, N = no (options.wdl:687-702, port choice).
#[allow(clippy::too_many_arguments)]
pub(crate) fn dialog_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<OptionsState>,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<OptionsRoot>>,
    mut game: ResMut<Game>,
    storage: Res<crate::platform::storage::Storage>,
    mut exit: MessageWriter<AppExit>,
) {
    if state.dialog.is_none() {
        return;
    }
    if keys.any_just_pressed([KeyCode::KeyJ, KeyCode::KeyT, KeyCode::KeyY]) {
        answer(true, &mut commands, &mut state, &mut time, &roots, &mut game, &storage, &mut exit);
    } else if keys.just_pressed(KeyCode::KeyN) {
        answer(false, &mut commands, &mut state, &mut time, &roots, &mut game, &storage, &mut exit);
    }
}
