//! Options overlay and Yes/No dialog (options.wdl, paninter.wdl; docs/menu.md §4-5),
//! drawn on a centred 640×480 canvas. Opens with Esc in a level or from the menu item;
//! the game is frozen while it is open.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::flow::{CurrentLevel, PendingLoad, Phase};
use crate::game::GameState;
use crate::hud::load_ui_image;
use crate::save::SaveGame;
use crate::ui_text::{BitmapText, FontKind, WdlStrings};
use crate::vfs::GameFiles;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsRequest>()
            .init_resource::<OptionsState>()
            .add_systems(Update, (open_on_esc, open_close, build_tab, buttons, sliders, dialog_keys).chain());
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
    Keyboard,
    Joypad,
    Yes,
    No,
}

/// Normal and hover images (A4 `BUTTON` off/over).
#[derive(Component)]
struct ButtonImages {
    normal: Handle<Image>,
    hover: Handle<Image>,
    /// Shown lit regardless of hover (selected tab, current input device).
    lit: bool,
}

#[derive(Component, Clone, Copy)]
enum Slider {
    Music,
    Sound,
    Video,
}

#[derive(Component)]
struct SliderThumb(Slider);

const TRACK: f32 = 227.0;
const THUMB: f32 = 32.0;

fn open_on_esc(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<CurrentLevel>,
    state: Res<OptionsState>,
    mut req: ResMut<OptionsRequest>,
) {
    if !keys.just_pressed(KeyCode::Escape) || state.open() {
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
    mut menu: ResMut<crate::menu::MenuState>,
    mut time: ResMut<Time<Virtual>>,
    keys: Res<ButtonInput<KeyCode>>,
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
    let esc = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter);
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
    save: Res<SaveGame>,
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
    let (thumb, volmus, keyb, pad, prev, next) =
        (im("butvol1.pcx"), im("volmus.pcx"), (im("butkey1.pcx"), im("butkey2.pcx")), (im("butpad1.pcx"), im("butpad2.pcx")), (im("flechga1.pcx"), im("flechga2.pcx")), (im("flechdr1.pcx"), im("flechdr2.pcx")));
    let picture = (page != 2).then(|| im(&format!("help0{page}.pcx")));
    let (team, hbarvide) = (im("lateam.pcx"), im("hbarvide.pcx"));
    commands.entity(root).with_children(|c| match tab {
        Tab::Audio => {
            c.spawn((at(347.0, 17.0, 266.0, 296.0), ImageNode::new(volmus)));
            for (slider, y) in [(Slider::Music, 60.0), (Slider::Sound, 161.0), (Slider::Video, 264.0)] {
                c.spawn((at(365.0, y, TRACK, 29.0), RelativeCursorPosition::default(), slider)).with_children(|t| {
                    t.spawn((at(0.0, 0.0, THUMB, 29.0), ImageNode::new(thumb.clone()), SliderThumb(slider)));
                });
            }
            // Resolution label: 84-px window over the six-label strip.
            c.spawn((at(444.0, 300.0, 84.0, 20.0), ImageNode { image: hbarvide, rect: Some(Rect::new(0.0, 0.0, 84.0, 20.0)), ..default() }, SliderThumb(Slider::Video)));
            c.spawn((at(400.0, 340.0, 64.0, 64.0), RelativeCursorPosition::default(), ImageNode::new(keyb.0.clone()), ButtonImages { normal: keyb.0, hover: keyb.1, lit: !save.joypad }, Action::Keyboard));
            c.spawn((at(495.0, 340.0, 64.0, 64.0), RelativeCursorPosition::default(), ImageNode::new(pad.0.clone()), ButtonImages { normal: pad.0, hover: pad.1, lit: save.joypad }, Action::Joypad));
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
    mut q: Query<(&RelativeCursorPosition, &Action, &ButtonImages, &mut ImageNode)>,
    mut sfx: crate::audio::Sfx,
    mut time: ResMut<Time<Virtual>>,
    roots: Query<Entity, With<OptionsRoot>>,
    (mut save, mut game, current, mut pending, mut exit): (ResMut<SaveGame>, ResMut<GameState>, Res<CurrentLevel>, ResMut<PendingLoad>, MessageWriter<AppExit>),
) {
    // Hover shows the "2" image (the selected tab / device stays lit).
    let mut fired = None;
    for (rel, action, imgs, mut node) in &mut q {
        let want = if rel.cursor_over || imgs.lit { &imgs.hover } else { &imgs.normal };
        if node.image != *want {
            node.image = want.clone();
        }
        if rel.cursor_over && mouse.just_pressed(MouseButton::Left) {
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
        Action::Keyboard | Action::Joypad => {
            save.joypad = action == Action::Joypad;
            crate::save::write(&save);
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
                    crate::save::write(&save);
                }
                pending.0 = Some(crate::menu::MENU_FILE.to_owned());
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
            crate::save::write(save);
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

/// Slider dragging (`HSLIDER`, track 227 px with a 32-px thumb).
#[allow(clippy::too_many_arguments)]
fn sliders(
    mut save: ResMut<SaveGame>,
    mouse: Res<ButtonInput<MouseButton>>,
    tracks: Query<(&Slider, &RelativeCursorPosition)>,
    mut thumbs: Query<(&SliderThumb, &mut Node, Option<&mut ImageNode>)>,
    mut windows: Query<&mut Window>,
    mut volume: ResMut<GlobalVolume>,
    mut video: Local<f32>,
    mut dragging: Local<Option<u8>>,
) {
    if *video == 0.0 {
        *video = 3.0;
    }
    if !mouse.pressed(MouseButton::Left) {
        *dragging = None;
    }
    for (slider, rel) in &tracks {
        let id = *slider as u8;
        if mouse.just_pressed(MouseButton::Left) && rel.cursor_over {
            *dragging = Some(id);
        }
        if *dragging != Some(id) {
            continue;
        }
        let Some(p) = rel.normalized else { continue };
        let f = (p.x + 0.5).clamp(0.0, 1.0);
        match slider {
            Slider::Music => save.music = (f * 100.0).round(),
            Slider::Sound => {
                save.sound = (f * 100.0).round();
                volume.volume = bevy::audio::Volume::Linear(save.sound / 100.0);
            }
            Slider::Video => {
                // 1..5 → 512×384 .. 1280×960 (applied at once; the original waited for close).
                let v = (1.0 + f * 4.0).round();
                if v != *video {
                    *video = v;
                    let (w, h) = [(512, 384), (640, 480), (800, 600), (1024, 768), (1280, 960)][v as usize - 1];
                    if let Ok(mut win) = windows.single_mut() {
                        win.resolution.set(w as f32, h as f32);
                    }
                }
            }
        }
    }
    if mouse.just_released(MouseButton::Left) {
        crate::save::write(&save);
    }
    for (thumb, mut node, img) in &mut thumbs {
        let f = match thumb.0 {
            Slider::Music => save.music / 100.0,
            Slider::Sound => save.sound / 100.0,
            Slider::Video => (*video - 1.0) / 4.0,
        };
        match img {
            // The resolution label window over `hbarvide.pcx` (six 84-px labels, shift 83).
            Some(mut i) if node.width == px(84.0) => {
                let x = (*video) * 83.0;
                i.rect = Some(Rect::new(x, 0.0, x + 84.0, 20.0));
            }
            _ => node.left = px(f * (TRACK - THUMB)),
        }
    }
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
