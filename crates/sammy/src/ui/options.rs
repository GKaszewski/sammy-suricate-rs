//! Options overlay and Yes/No dialog (options.wdl, paninter.wdl; docs/menu.md §4-5),
//! drawn on a centred 640×480 canvas. Opens with Esc in a level or from the menu item;
//! the game is frozen while it is open.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::flow::{CurrentLevel, PendingLoad, Phase};
use crate::gameplay::state::GameState;
use crate::ui::hud::load_ui_image;
use crate::platform::save::SaveGame;
use crate::ui::text::{BitmapText, FontKind, WdlStrings};
use crate::platform::vfs::GameFiles;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsRequest>()
            .init_resource::<OptionsState>()
            .init_resource::<Focus>()
            .add_systems(Update, (open_on_esc, open_close, build_tab, focus_nav, buttons, sliders, choices, choice_labels, dialog_keys, focus_frame).chain());
    }
}

/// `MenuOptionsGame`: 0 front-end, 1 level, 2 extro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptionsMode {
    Menu,
    Level,
    Extro,
}

/// Requests from other systems (menu items, keys).
#[derive(Resource, Default)]
pub struct OptionsRequest {
    pub open: Option<OptionsMode>,
    /// Menu "Quit" item: the Yes/No dialog without the overlay.
    pub quit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Tab {
    Audio,
    #[default]
    Help,
    Credits,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Question {
    Quit,
    Restart,
}

#[derive(Resource, Default)]
pub struct OptionsState {
    pub mode: Option<OptionsMode>,
    /// `LastOptionChoose`: Help first (5), then remembered.
    tab: Tab,
    help_page: u32,
    dialog: Option<Question>,
    dirty: bool,
}

impl OptionsState {
    pub fn open(&self) -> bool {
        self.mode.is_some() || self.dialog.is_some()
    }
}

#[derive(Component)]
struct OptionsRoot;
#[derive(Component)]
struct TabContent;
#[derive(Component)]
struct DialogRoot;

/// What a button does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Tab(Tab),
    Resume,
    Back,
    Quit,
    Restart,
    PrevPage,
    NextPage,
    Yes,
    No,
}

/// Normal and hover images (A4 `BUTTON` off/over).
#[derive(Component)]
struct ButtonImages {
    normal: Handle<Image>,
    hover: Handle<Image>,
    /// Shown lit regardless of hover (selected tab).
    lit: bool,
}

#[derive(Component, Clone, Copy)]
enum Slider {
    Music,
    Sound,
}

/// Port display settings, drawn as `<  value  >` rows in the old video slider's place.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Choice {
    Mode,
    Size,
    UiSize,
}

/// Gamepad / keyboard focus inside the overlay (shown while the pad is in use).
#[derive(Resource, Default)]
struct Focus(Option<Entity>);

#[derive(Component)]
struct FocusFrame;

#[derive(Component)]
struct SliderThumb(Slider);

const TRACK: f32 = 227.0;
const THUMB: f32 = 32.0;

fn open_on_esc(
    controls: Res<crate::platform::input::Controls>,
    current: Res<CurrentLevel>,
    state: Res<OptionsState>,
    mut req: ResMut<OptionsRequest>,
    intro: Res<crate::ui::intro::Intro>,
) {
    // Esc skips the intro instead (and its release is awaited, Intro2.wdl:209).
    if !controls.options || state.open() || intro.active {
        return;
    }
    req.open = match current.phase {
        Phase::Menu => Some(OptionsMode::Menu),
        Phase::Playing if current.slot == Some(crate::flow::EXTRO_SLOT) => Some(OptionsMode::Extro),
        Phase::Playing => Some(OptionsMode::Level),
        _ => None,
    };
}

fn img(files: &GameFiles, images: &mut Assets<Image>, name: &str) -> Handle<Image> {
    load_ui_image(files, images, name).map(|(h, _)| h).unwrap_or_default()
}

fn at(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node { position_type: PositionType::Absolute, left: px(x), top: px(y), width: px(w), height: px(h), ..default() }
}

#[allow(clippy::too_many_arguments)]
fn open_close(
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
            (Tab::Audio, 41.0, 14.0, 218.0, 80.0, im("audio1.pcx"), im("audio2.pcx")),
            (Tab::Help, 41.0, 105.0, 226.0, 47.0, im("help1.pcx"), im("help2.pcx")),
            (Tab::Credits, 41.0, 152.0, 226.0, 46.0, im("credits1.pcx"), im("credits2.pcx")),
        ];
        commands
            .spawn((
                OptionsRoot,
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                GlobalZIndex(50),
            ))
            .with_children(|root| {
                root.spawn((Node { width: px(640.0), height: px(480.0), ..default() }, ImageNode::new(bg))).with_children(|c| {
                    for (tab, x, y, w, h, n, hv) in tabs {
                        c.spawn((at(x, y, w, h), RelativeCursorPosition::default(), ImageNode::new(n.clone()), ButtonImages { normal: n, hover: hv, lit: false }, Action::Tab(tab)));
                    }
                    let mut button = |(n, hv): (Handle<Image>, Handle<Image>), x: f32, y: f32, w: f32, h: f32, action: Action| {
                        c.spawn((at(x, y, w, h), RelativeCursorPosition::default(), ImageNode::new(n.clone()), ButtonImages { normal: n, hover: hv, lit: false }, action));
                    };
                    if mode != OptionsMode::Menu {
                        button(resume, 0.0, 335.0, 306.0, 42.0, Action::Resume);
                    }
                    button(back, 0.0, 377.0, 306.0, 40.0, Action::Back);
                    button(quit, 0.0, 417.0, 306.0, 41.0, Action::Quit);
                    if mode == OptionsMode::Level {
                        button(restart, 48.0, 268.0, 204.0, 50.0, Action::Restart);
                    }
                    c.spawn((TabContent, at(0.0, 0.0, 640.0, 480.0)));
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
                    GlobalZIndex(60),
                ))
                .with_children(|root| {
                    root.spawn(Node { width: px(640.0), height: px(480.0), ..default() }).with_children(|c| {
                        c.spawn((at(223.0, 135.0, 199.0, 68.0), ImageNode::new(bg)));
                        c.spawn((Node { position_type: PositionType::Absolute, left: px(228.0), top: px(138.0), ..default() }, BitmapText::new(FontKind::Small, text)));
                        c.spawn((at(235.0, 171.0, 68.0, 28.0), RelativeCursorPosition::default(), ImageNode::new(yes.0.clone()), ButtonImages { normal: yes.0, hover: yes.1, lit: false }, Action::Yes));
                        c.spawn((at(341.0, 171.0, 68.0, 28.0), RelativeCursorPosition::default(), ImageNode::new(no.0.clone()), ButtonImages { normal: no.0, hover: no.1, lit: false }, Action::No));
                    });
                });
        }
        (None, Some(e)) => commands.entity(e).despawn(),
        _ => {}
    }
    menu.modal = state.open();
}

fn close(commands: &mut Commands, state: &mut OptionsState, time: &mut Time<Virtual>, roots: &Query<Entity, With<OptionsRoot>>) {
    for e in roots {
        commands.entity(e).despawn();
    }
    state.mode = None;
    state.dialog = None;
    time.unpause();
}

/// (Re)builds the selected tab's content: sliders, help page or credits.
fn build_tab(
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
    let picture = (page != 2).then(|| im(&format!("help0{page}.pcx")));
    let team = im("lateam.pcx");
    commands.entity(root).with_children(|c| match tab {
        Tab::Audio => {
            // The music/sound art and the "Rozdzielczość grafiki" heading; the old
            // resolution slider below it is replaced by the display rows.
            c.spawn((at(347.0, 17.0, 266.0, 249.0), ImageNode { image: volmus, rect: Some(Rect::new(0.0, 0.0, 266.0, 249.0)), ..default() }));
            for (slider, y) in [(Slider::Music, 60.0), (Slider::Sound, 161.0)] {
                c.spawn((at(365.0, y, TRACK, 29.0), RelativeCursorPosition::default(), slider)).with_children(|t| {
                    t.spawn((at(0.0, 0.0, THUMB, 29.0), ImageNode::new(thumb.clone()), SliderThumb(slider)));
                });
            }
            for (choice, y) in [(Choice::Mode, 272.0), (Choice::Size, 302.0), (Choice::UiSize, 346.0)] {
                c.spawn((
                    Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..at(352.0, y, 256.0, 24.0) },
                    RelativeCursorPosition::default(),
                    choice,
                ))
                .with_children(|r| {
                    r.spawn((Node { position_type: PositionType::Relative, ..default() }, BitmapText::new(FontKind::Help, Vec::<u8>::new()).narrow()));
                });
            }
        }
        Tab::Help => {
            c.spawn((Node { position_type: PositionType::Absolute, left: px(334.0), top: px(20.0), ..default() }, BitmapText::new(FontKind::Help, text(&format!("Help{page}_Str"))).narrow()));
            if let Some(p) = picture {
                c.spawn((at(330.0, 297.0, 297.0, 113.0), ImageNode::new(p)));
            }
            if page > 1 {
                c.spawn((at(330.0, 408.0, 65.0, 62.0), RelativeCursorPosition::default(), ImageNode::new(prev.0.clone()), ButtonImages { normal: prev.0, hover: prev.1, lit: false }, Action::PrevPage));
            }
            if page < 7 {
                c.spawn((at(560.0, 408.0, 65.0, 62.0), RelativeCursorPosition::default(), ImageNode::new(next.0.clone()), ButtonImages { normal: next.0, hover: next.1, lit: false }, Action::NextPage));
            }
        }
        Tab::Credits => {
            c.spawn((at(340.0, 25.0, 282.0, 294.0), ImageNode::new(team)));
            c.spawn((Node { position_type: PositionType::Absolute, left: px(334.0), top: px(320.0), ..default() }, BitmapText::new(FontKind::Help, text("Thanksto_Str")).narrow()));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn buttons(
    mut commands: Commands,
    mut state: ResMut<OptionsState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut q: Query<(Entity, &RelativeCursorPosition, &Action, &ButtonImages, &mut ImageNode)>,
    (focus, controls): (Res<Focus>, Res<crate::platform::input::Controls>),
    mut sfx: crate::platform::audio::Sfx,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<OptionsRoot>>,
    (mut save, mut game, current, mut pending, mut exit): (ResMut<SaveGame>, ResMut<GameState>, Res<CurrentLevel>, ResMut<PendingLoad>, MessageWriter<AppExit>),
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
            sfx.play_vol(if t == Tab::Credits { "tigre_son" } else { "Ding2" }, 50.0);
            state.tab = t;
            state.dirty = true;
        }
        Action::PrevPage | Action::NextPage => {
            sfx.play_vol("Ding1", 50.0);
            state.help_page = if action == Action::PrevPage { state.help_page.saturating_sub(1).max(1) } else { (state.help_page + 1).min(7) };
            state.dirty = true;
        }
        Action::Resume => {
            sfx.play_vol("Ding2", 50.0);
            close(&mut commands, &mut state, &mut time, &roots);
        }
        Action::Back => {
            sfx.play_vol("Ding2", 50.0);
            let mode = state.mode;
            close(&mut commands, &mut state, &mut time, &roots);
            // In a game: abandon it (stored like a lost game, no lose screen) and go to the menu.
            if mode != Some(OptionsMode::Menu) {
                if let Some(slot) = current.slot {
                    let checkpoint = game.checkpoint.then_some(game.start).flatten().map(|(p, _)| p);
                    save.record_game(slot, game.score, checkpoint, false, None);
                    crate::platform::save::write(&save);
                }
                pending.0 = Some(crate::ui::menu::MENU_FILE.to_owned());
            }
        }
        Action::Quit => {
            sfx.play_vol("Ding2", 50.0);
            state.dialog = Some(Question::Quit);
        }
        Action::Restart => state.dialog = Some(Question::Restart),
        Action::Yes => answer(true, &mut commands, &mut state, &mut time, &roots, &save, &mut game, &mut exit),
        Action::No => answer(false, &mut commands, &mut state, &mut time, &roots, &save, &mut game, &mut exit),
    }
}

#[allow(clippy::too_many_arguments)]
fn answer(
    yes: bool,
    commands: &mut Commands,
    state: &mut OptionsState,
    time: &mut Time<Virtual>,
    roots: &Query<Entity, With<OptionsRoot>>,
    save: &SaveGame,
    game: &mut GameState,
    exit: &mut MessageWriter<AppExit>,
) {
    let q = state.dialog.take();
    if !yes {
        return;
    }
    match q {
        Some(Question::Quit) => {
            // `QuitSammy`: save, then exit.
            crate::platform::save::write(save);
            exit.write(AppExit::Success);
        }
        Some(Question::Restart) => {
            // `OuiRestartLevel`: close, `BallesJoueur1 = 0` → lose screen, same level again.
            close(commands, state, time, roots);
            game.lives = 0;
            game.lost = true;
        }
        None => {}
    }
}

/// Slider dragging (`HSLIDER`, track 227 px with a 32-px thumb); left/right on a
/// focused slider steps it by 5.
#[allow(clippy::too_many_arguments)]
fn sliders(
    mut save: ResMut<SaveGame>,
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
            Slider::Music => save.music,
            Slider::Sound => save.sound,
        };
        if mouse.just_pressed(MouseButton::Left) && rel.cursor_over {
            *dragging = Some(id);
        }
        if *dragging == Some(id)
            && let Some(p) = rel.normalized
        {
            value = ((p.x + 0.5).clamp(0.0, 1.0) * 100.0).round();
        } else if focus.0 == Some(e)
            && let Some(d) = controls.nav_dir().filter(|d| d.x != 0)
        {
            value = (value + 5.0 * d.x as f32).clamp(0.0, 100.0);
            changed = true;
        } else {
            continue;
        }
        match slider {
            Slider::Music => save.music = value,
            Slider::Sound => {
                save.sound = value;
                volume.volume = bevy::audio::Volume::Linear(save.sound / 100.0);
            }
        }
    }
    if mouse.just_released(MouseButton::Left) || changed {
        crate::platform::save::write(&save);
    }
    for (thumb, mut node) in &mut thumbs {
        let f = match thumb.0 {
            Slider::Music => save.music / 100.0,
            Slider::Sound => save.sound / 100.0,
        };
        node.left = px(f * (TRACK - THUMB));
    }
}

/// Display rows: a click on the left half steps back, on the right half forward; on a
/// focused row left/right do the same.
fn choices(
    mut save: ResMut<SaveGame>,
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
        let d = &mut save.display;
        let cycle = |i: usize, n: usize| (i as i32 + step).rem_euclid(n as i32) as usize;
        match choice {
            Choice::Mode => {
                let modes = [DisplayMode::Windowed, DisplayMode::Borderless, DisplayMode::Fullscreen];
                let i = modes.iter().position(|m| *m == d.mode).unwrap_or(0);
                d.mode = modes[cycle(i, 3)];
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
            Choice::UiSize => d.ui_size = (2 - cycle(2 - d.ui_size.min(2) as usize, 3)) as u8,
        }
        sfx.play_vol("Toc2", 50.0);
        crate::platform::save::write(&save);
        return;
    }
}

fn choice_labels(
    save: Res<SaveGame>,
    monitors: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
    rows: Query<(&Choice, &Children)>,
    mut texts: Query<&mut BitmapText>,
) {
    use crate::platform::save::DisplayMode;
    let d = &save.display;
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
            Choice::UiSize => format!("<  Interfejs: {}  >", ["duży", "średni", "mały"][d.ui_size.min(2) as usize]),
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
fn focus_nav(
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
            (along > 1.0).then(|| (i.0, along + 2.0 * (v - dir * along).length()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|i| i.0)
        .or(focus.0);
}

/// A frame around the focused item while the pad is in use.
fn focus_frame(
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
            Node { position_type: PositionType::Absolute, border: UiRect::all(px(2.0)), ..default() },
            BorderColor::all(Color::srgb(1.0, 0.85, 0.2)),
            BackgroundColor(Color::srgba(1.0, 0.85, 0.2, 0.15)),
            GlobalZIndex(70),
            Visibility::Hidden,
        ));
        return;
    };
    let Some((n, t)) = target else {
        *vis = Visibility::Hidden;
        return;
    };
    let k = n.inverse_scale_factor();
    let size = n.size() * k + Vec2::splat(6.0);
    let c = t.translation * k;
    (node.left, node.top, node.width, node.height) = (px(c.x - size.x / 2.0), px(c.y - size.y / 2.0), px(size.x), px(size.y));
    *vis = Visibility::Inherited;
}

/// Yes/No keys: J/T/Y = yes, N = no (options.wdl:687-702, port choice).
#[allow(clippy::too_many_arguments)]
fn dialog_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<OptionsState>,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<OptionsRoot>>,
    save: Res<SaveGame>,
    mut game: ResMut<GameState>,
    mut exit: MessageWriter<AppExit>,
) {
    if state.dialog.is_none() {
        return;
    }
    if keys.any_just_pressed([KeyCode::KeyJ, KeyCode::KeyT, KeyCode::KeyY]) {
        answer(true, &mut commands, &mut state, &mut time, &roots, &save, &mut game, &mut exit);
    } else if keys.just_pressed(KeyCode::KeyN) {
        answer(false, &mut commands, &mut state, &mut time, &roots, &save, &mut game, &mut exit);
    }
}
