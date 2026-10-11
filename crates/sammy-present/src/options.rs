//! The options overlay and Yes/No dialog (options.wdl, paninter.wdl; docs/menu.md §4-5),
//! for any frontend: a 640×480 plate centred on screen with tabs (audio/video, help,
//! credits), volume sliders, display rows, help pages and the game buttons. [`Options`]
//! holds the state, [`Options::elements`] says what to draw, [`Options::update`] takes
//! the frame's input and returns what the frontend must do ([`Effect`]).

use sammy_core::save::{DisplayMode, SaveGame};

use crate::text::{FontKind, WdlStrings, cp1250};

/// The plate (mode-6 pixels).
pub const W: f32 = 640.0;
pub const H: f32 = 480.0;

/// `MenuOptionsGame`: 0 front-end, 1 level, 2 extro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Menu,
    Level,
    Extro,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    Audio,
    #[default]
    Help,
    Credits,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Question {
    Quit,
    Restart,
}

/// What a button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slider {
    Music,
    Sound,
}

/// Port display settings, drawn as `<  value  >` rows in the old video slider's place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Mode,
    Size,
    UiSize,
}

/// Something the player can point at or focus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Button(Action),
    Slider(Slider),
    Row(Row),
}

/// What to draw: an image (region) or a text, at plate pixels; `control` marks the
/// focusable / clickable ones.
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Image { image: String, src: Option<[f32; 4]>, area: [f32; 4] },
    Text { font: FontKind, narrow: bool, bytes: Vec<u8>, at: [f32; 2], centre_in: Option<[f32; 4]> },
}

/// A drawable with its control (if any).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub element: Element,
    pub control: Option<(Control, [f32; 4])>,
}

/// What the frontend must do after an update.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Effect {
    /// `play_sound name, vol`.
    Sound(&'static str, f32),
    /// The game is frozen while the overlay is up in a level (`Freeze_mode`).
    Pause(bool),
    /// Back to the menu from a game (stored like a lost game).
    Abandon,
    /// `OuiRestartLevel`: no lives left → lose screen, same level again.
    GiveUp,
    /// `QuitSammy`: save, then exit.
    Exit,
    /// The settings changed (write the save; apply volumes / the display).
    SaveChanged,
}

/// The display choices a frontend offers: window sizes for the current mode and the size
/// in use. `None` where the display is fixed (handhelds).
#[derive(Clone, Debug, Default)]
pub struct DisplayInfo {
    pub sizes: Vec<[u32; 2]>,
    pub current: Option<[u32; 2]>,
}

/// Windowed sizes offered (those that fit the monitor).
const WINDOW_SIZES: [[u32; 2]; 10] = [[640, 480], [800, 600], [1024, 768], [1280, 720], [1280, 960], [1366, 768], [1600, 900], [1920, 1080], [2560, 1440], [3840, 2160]];
/// Smallest fullscreen mode offered (the original art size).
const MIN_FULLSCREEN: [u32; 2] = [640, 480];

impl DisplayInfo {
    /// The choices for `d`'s mode on a monitor of `desktop` size with these fullscreen
    /// video modes.
    pub fn new(d: &sammy_core::save::DisplaySettings, desktop: Option<[u32; 2]>, video_modes: &[[u32; 2]]) -> Self {
        let max = desktop.unwrap_or([u32::MAX; 2]);
        let sizes = match d.mode {
            DisplayMode::Windowed => WINDOW_SIZES.iter().copied().filter(|s| s[0] <= max[0] && s[1] <= max[1]).collect(),
            DisplayMode::Fullscreen => {
                let mut v = video_modes.to_vec();
                v.sort();
                v.dedup();
                v.retain(|s| s[0] >= MIN_FULLSCREEN[0] && s[1] >= MIN_FULLSCREEN[1]);
                v
            }
            DisplayMode::Borderless => Vec::new(),
        };
        let current = match d.mode {
            DisplayMode::Windowed => Some(d.window),
            DisplayMode::Borderless => desktop,
            DisplayMode::Fullscreen => d.fullscreen.or(desktop),
        };
        Self { sizes, current }
    }
}

/// One frame's input for the overlay, pointer in plate pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub pointer: Option<[f32; 2]>,
    /// Left button: pressed this frame, held, released this frame.
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
    /// A focus move (x right, y up).
    pub nav: Option<[i32; 2]>,
    /// Pad A / Enter on the focused item.
    pub accept: bool,
    /// Esc, Enter, Start or B: close the overlay or cancel the dialog.
    pub close: bool,
    /// Yes / No keys (J/T/Y, N).
    pub yes: bool,
    pub no: bool,
    /// The pad is in use (focus is shown and used).
    pub pad: bool,
}

/// Tab buttons.
const TAB_AUDIO: [f32; 4] = [41.0, 14.0, 218.0, 80.0];
const TAB_HELP: [f32; 4] = [41.0, 105.0, 226.0, 47.0];
const TAB_CREDITS: [f32; 4] = [41.0, 152.0, 226.0, 46.0];
/// Resume / back / quit / restart buttons.
const BTN_RESUME: [f32; 4] = [0.0, 335.0, 306.0, 42.0];
const BTN_BACK: [f32; 4] = [0.0, 377.0, 306.0, 40.0];
const BTN_QUIT: [f32; 4] = [0.0, 417.0, 306.0, 41.0];
const BTN_RESTART: [f32; 4] = [48.0, 268.0, 204.0, 50.0];
/// Yes/No dialog: plate, text origin and the two buttons.
const DIALOG_PLATE: [f32; 4] = [223.0, 135.0, 199.0, 68.0];
const DIALOG_TEXT_POS: [f32; 2] = [228.0, 138.0];
const DIALOG_YES: [f32; 4] = [235.0, 171.0, 68.0, 28.0];
const DIALOG_NO: [f32; 4] = [341.0, 171.0, 68.0, 28.0];
/// Audio tab: music/sound art (and the sub-rect of `volmus` it shows).
const VOLMUS: [f32; 4] = [347.0, 17.0, 266.0, 249.0];
/// Slider tracks (`HSLIDER`, 227 px with a 32-px thumb): x, thumb height, rows' y.
const TRACK: f32 = 227.0;
const THUMB: f32 = 32.0;
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
const HELP_TEXT_POS: [f32; 2] = [334.0, 20.0];
const HELP_PICTURE: [f32; 4] = [330.0, 297.0, 297.0, 113.0];
const HELP_PREV: [f32; 4] = [330.0, 408.0, 65.0, 62.0];
const HELP_NEXT: [f32; 4] = [560.0, 408.0, 65.0, 62.0];
/// Help pages 1-7; page 2 has no picture.
const HELP_LAST_PAGE: u32 = 7;
const HELP_NO_PICTURE_PAGE: u32 = 2;
/// Credits tab: team picture and the "Thanksto" text origin.
const CREDITS_TEAM: [f32; 4] = [340.0, 25.0, 282.0, 294.0];
const CREDITS_THANKS_POS: [f32; 2] = [334.0, 320.0];
/// Volume of the button / row sounds.
const UI_SFX_VOL: f32 = 50.0;
/// The screen behind the overlay is dimmed this much.
pub const OVERLAY_ALPHA: f32 = 0.7;
/// Focus frame: border, glow colour (RGB), fill alpha and padding around the item.
pub const FRAME_BORDER: f32 = 2.0;
pub const FRAME_RGB: [f32; 3] = [1.0, 0.85, 0.2];
pub const FRAME_FILL_ALPHA: f32 = 0.15;
pub const FRAME_PAD: f32 = 6.0;
/// Background plates.
const PLATE: &str = "fondopt1.pcx";
const DIALOG: &str = "fondyn.pcx";

/// The overlay's state.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub mode: Option<Mode>,
    /// `LastOptionChoose`: Help first (5), then remembered.
    pub tab: Tab,
    help_page: u32,
    pub dialog: Option<Question>,
    /// Focused control (pad / keys).
    pub focus: Option<Control>,
    dragging: Option<Slider>,
}

fn inside(p: [f32; 2], a: [f32; 4]) -> bool {
    p[0] >= a[0] && p[1] >= a[1] && p[0] < a[0] + a[2] && p[1] < a[1] + a[3]
}

fn centre(a: [f32; 4]) -> [f32; 2] {
    [a[0] + a[2] / 2.0, a[1] + a[3] / 2.0]
}

impl Options {
    /// The overlay or the dialog is up.
    pub fn open(&self) -> bool {
        self.mode.is_some() || self.dialog.is_some()
    }

    /// Opens the overlay; returns its effects (a level freezes).
    pub fn show(&mut self, mode: Mode, tab: Option<Tab>) -> Vec<Effect> {
        self.mode = Some(mode);
        self.help_page = self.help_page.max(1);
        if let Some(t) = tab {
            self.tab = t;
        }
        if mode != Mode::Menu { vec![Effect::Pause(true)] } else { Vec::new() }
    }

    /// The menu's Quit item: the Yes/No dialog without the overlay.
    pub fn ask_quit(&mut self) {
        if self.dialog.is_none() {
            self.dialog = Some(Question::Quit);
        }
    }

    fn close(&mut self, out: &mut Vec<Effect>) {
        if self.mode.take().is_some_and(|m| m != Mode::Menu) {
            out.push(Effect::Pause(false));
        }
        self.dialog = None;
        self.focus = None;
    }

    /// Images the overlay may show (to load up front).
    pub fn images() -> Vec<String> {
        let mut v: Vec<String> = [
            PLATE, DIALOG, "resum1.pcx", "resum2.pcx", "back1.pcx", "back2.pcx", "quit1.pcx", "quit2.pcx", "restart2.pcx", "restart3.pcx", "audio1.pcx",
            "audio2.pcx", "help1.pcx", "help2.pcx", "credits1.pcx", "credits2.pcx", "oui1.pcx", "oui2.pcx", "non1.pcx", "non2.pcx", "butvol1.pcx",
            "volmus.pcx", "flechga1.pcx", "flechga2.pcx", "flechdr1.pcx", "flechdr2.pcx", "lateam.pcx",
        ]
        .map(String::from)
        .to_vec();
        v.extend((1..=HELP_LAST_PAGE).filter(|p| *p != HELP_NO_PICTURE_PAGE).map(|p| format!("help0{p}.pcx")));
        v
    }

    /// The overlay's controls with their areas (dialog buttons only while it is up).
    fn controls(&self, display: Option<&DisplayInfo>) -> Vec<(Control, [f32; 4])> {
        let mut v = Vec::new();
        if self.dialog.is_some() {
            v.push((Control::Button(Action::Yes), DIALOG_YES));
            v.push((Control::Button(Action::No), DIALOG_NO));
            return v;
        }
        let Some(mode) = self.mode else { return v };
        v.push((Control::Button(Action::Tab(Tab::Audio)), TAB_AUDIO));
        v.push((Control::Button(Action::Tab(Tab::Help)), TAB_HELP));
        v.push((Control::Button(Action::Tab(Tab::Credits)), TAB_CREDITS));
        if mode != Mode::Menu {
            v.push((Control::Button(Action::Resume), BTN_RESUME));
        }
        v.push((Control::Button(Action::Back), BTN_BACK));
        v.push((Control::Button(Action::Quit), BTN_QUIT));
        if mode == Mode::Level {
            v.push((Control::Button(Action::Restart), BTN_RESTART));
        }
        match self.tab {
            Tab::Audio => {
                v.push((Control::Slider(Slider::Music), [SLIDER_X, SLIDER_MUSIC_Y, TRACK, SLIDER_H]));
                v.push((Control::Slider(Slider::Sound), [SLIDER_X, SLIDER_SOUND_Y, TRACK, SLIDER_H]));
                if display.is_some() {
                    v.push((Control::Row(Row::Mode), [ROW_X, ROW_MODE_Y, ROW_W, ROW_H]));
                    v.push((Control::Row(Row::Size), [ROW_X, ROW_SIZE_Y, ROW_W, ROW_H]));
                }
                v.push((Control::Row(Row::UiSize), [ROW_X, ROW_UI_SIZE_Y, ROW_W, ROW_H]));
            }
            Tab::Help => {
                if self.help_page > 1 {
                    v.push((Control::Button(Action::PrevPage), HELP_PREV));
                }
                if self.help_page < HELP_LAST_PAGE {
                    v.push((Control::Button(Action::NextPage), HELP_NEXT));
                }
            }
            Tab::Credits => {}
        }
        v
    }

    /// Button images (normal, hover).
    fn button_images(a: Action) -> (&'static str, &'static str) {
        match a {
            Action::Tab(Tab::Audio) => ("audio1.pcx", "audio2.pcx"),
            Action::Tab(Tab::Help) => ("help1.pcx", "help2.pcx"),
            Action::Tab(Tab::Credits) => ("credits1.pcx", "credits2.pcx"),
            Action::Resume => ("resum1.pcx", "resum2.pcx"),
            Action::Back => ("back1.pcx", "back2.pcx"),
            Action::Quit => ("quit1.pcx", "quit2.pcx"),
            Action::Restart => ("restart2.pcx", "restart3.pcx"),
            Action::PrevPage => ("flechga1.pcx", "flechga2.pcx"),
            Action::NextPage => ("flechdr1.pcx", "flechdr2.pcx"),
            Action::Yes => ("oui1.pcx", "oui2.pcx"),
            Action::No => ("non1.pcx", "non2.pcx"),
        }
    }

    /// What to draw now: the plate and its tab, then (if up) the dialog. `pointer` (plate
    /// pixels) lights the hovered button; `display` labels the display rows.
    pub fn elements(&self, strings: &WdlStrings, save: &SaveGame, display: Option<&DisplayInfo>, pointer: Option<[f32; 2]>, pad: bool) -> Vec<Item> {
        let mut out = Vec::new();
        let img = |image: &str, area: [f32; 4]| Element::Image { image: image.into(), src: None, area };
        let text = |font, narrow, bytes: Vec<u8>, at| Element::Text { font, narrow, bytes, at, centre_in: None };
        let lit = |c: Control, area: [f32; 4]| pointer.is_some_and(|p| inside(p, area)) || (pad && self.focus == Some(c));
        let controls = self.controls(display);
        if self.mode.is_some() {
            out.push(Item { element: img(PLATE, [0.0, 0.0, W, H]), control: None });
            match self.tab {
                Tab::Audio => {
                    // The music/sound art and the "Rozdzielczość grafiki" heading; the old
                    // resolution slider below it is replaced by the display rows.
                    out.push(Item { element: Element::Image { image: "volmus.pcx".into(), src: Some([0.0, 0.0, VOLMUS[2], VOLMUS[3]]), area: VOLMUS }, control: None });
                }
                Tab::Help => {
                    let page = self.help_page;
                    out.push(Item { element: text(FontKind::Help, true, strings.get(&format!("Help{page}_Str")).to_vec(), HELP_TEXT_POS), control: None });
                    if page != HELP_NO_PICTURE_PAGE {
                        out.push(Item { element: img(&format!("help0{page}.pcx"), HELP_PICTURE), control: None });
                    }
                }
                Tab::Credits => {
                    out.push(Item { element: img("lateam.pcx", CREDITS_TEAM), control: None });
                    out.push(Item { element: text(FontKind::Help, true, strings.get("Thanksto_Str").to_vec(), CREDITS_THANKS_POS), control: None });
                }
            }
        }
        if self.dialog.is_some() {
            out.push(Item { element: img(DIALOG, DIALOG_PLATE), control: None });
            let q = if self.dialog == Some(Question::Quit) { "QuitYesNo_Str" } else { "RestartYesNo_Str" };
            out.push(Item { element: text(FontKind::Small, false, strings.get(q).to_vec(), DIALOG_TEXT_POS), control: None });
        }
        for (c, area) in controls {
            let element = match c {
                // Hover shows the "2" image (the selected tab stays lit).
                Control::Button(a) => {
                    let (normal, hover) = Self::button_images(a);
                    let on = lit(c, area) || a == Action::Tab(self.tab);
                    img(if on { hover } else { normal }, area)
                }
                Control::Slider(s) => {
                    let f = match s {
                        Slider::Music => save.music,
                        Slider::Sound => save.sound,
                    } / SLIDER_MAX;
                    img("butvol1.pcx", [area[0] + f * (TRACK - THUMB), area[1], THUMB, SLIDER_H])
                }
                Control::Row(r) => Element::Text { font: FontKind::Help, narrow: true, bytes: row_label(r, save, display), at: [area[0], area[1]], centre_in: Some(area) },
            };
            out.push(Item { element, control: Some((c, area)) });
        }
        out
    }

    /// The focused control's area (for the focus frame), while the pad is in use.
    pub fn focus_area(&self, display: Option<&DisplayInfo>) -> Option<[f32; 4]> {
        let f = self.focus?;
        self.controls(display).into_iter().find(|(c, _)| *c == f).map(|(_, a)| a)
    }

    /// One frame: hover, clicks, sliders, rows, focus moves, close / cancel. Settings
    /// changes go into `save`.
    pub fn update(&mut self, input: &Input, save: &mut SaveGame, display: Option<&DisplayInfo>) -> Vec<Effect> {
        let mut out = Vec::new();
        if !self.open() {
            self.focus = None;
            return out;
        }
        // Esc / Enter / Start / B close the overlay; they cancel a dialog first.
        if input.close {
            if self.dialog.is_some() {
                self.dialog = None;
                self.focus = None;
            } else {
                self.close(&mut out);
            }
            return out;
        }
        if self.dialog.is_some() && (input.yes || input.no) {
            self.answer(input.yes, &mut out);
            return out;
        }
        let controls = self.controls(display);
        self.move_focus(input, &controls);
        // Sliders: drag with the mouse; left/right on a focused one steps by 5.
        if !input.held && self.dragging.take().is_some() {
            out.push(Effect::SaveChanged);
        }
        for (c, area) in &controls {
            let Control::Slider(s) = c else { continue };
            if input.pressed && input.pointer.is_some_and(|p| inside(p, *area)) {
                self.dragging = Some(*s);
            }
            let value = match s {
                Slider::Music => &mut save.music,
                Slider::Sound => &mut save.sound,
            };
            if self.dragging == Some(*s)
                && let Some(p) = input.pointer
            {
                *value = (((p[0] - area[0]) / area[2]).clamp(0.0, 1.0) * SLIDER_MAX).round();
            } else if input.pad
                && self.focus == Some(*c)
                && let Some(d) = input.nav.filter(|d| d[0] != 0)
            {
                *value = (*value + SLIDER_STEP * d[0] as f32).clamp(0.0, SLIDER_MAX);
                out.push(Effect::SaveChanged);
            }
        }
        // Rows: a click on the left half steps back, on the right half forward; left /
        // right / A on a focused row do the same.
        for (c, area) in &controls {
            let Control::Row(r) = c else { continue };
            let step = if input.pressed && input.pointer.is_some_and(|p| inside(p, *area)) {
                input.pointer.map(|p| if p[0] < area[0] + area[2] / 2.0 { -1 } else { 1 })
            } else if input.pad && self.focus == Some(*c) {
                input.nav.filter(|d| d[0] != 0).map(|d| d[0]).or(input.accept.then_some(1))
            } else {
                None
            };
            if let Some(step) = step {
                step_row(*r, step, save, display);
                out.push(Effect::Sound("Toc2", UI_SFX_VOL));
                out.push(Effect::SaveChanged);
                return out;
            }
        }
        // Buttons: a click, or A on the focused one.
        let fired = controls.iter().find_map(|(c, area)| {
            let Control::Button(a) = c else { return None };
            let clicked = input.pressed && input.pointer.is_some_and(|p| inside(p, *area));
            let accepted = input.pad && input.accept && self.focus == Some(*c);
            (clicked || accepted).then_some(*a)
        });
        if let Some(a) = fired {
            self.act(a, &mut out);
        }
        out
    }

    fn act(&mut self, a: Action, out: &mut Vec<Effect>) {
        match a {
            Action::Tab(t) => {
                out.push(Effect::Sound(if t == Tab::Credits { "tigre_son" } else { "Ding2" }, UI_SFX_VOL));
                self.tab = t;
            }
            Action::PrevPage | Action::NextPage => {
                out.push(Effect::Sound("Ding1", UI_SFX_VOL));
                self.help_page = if a == Action::PrevPage { self.help_page.saturating_sub(1).max(1) } else { (self.help_page + 1).min(HELP_LAST_PAGE) };
            }
            Action::Resume => {
                out.push(Effect::Sound("Ding2", UI_SFX_VOL));
                self.close(out);
            }
            Action::Back => {
                out.push(Effect::Sound("Ding2", UI_SFX_VOL));
                let mode = self.mode;
                self.close(out);
                // In a game: abandon it (stored like a lost game, no lose screen).
                if mode != Some(Mode::Menu) {
                    out.push(Effect::Abandon);
                }
            }
            Action::Quit => {
                out.push(Effect::Sound("Ding2", UI_SFX_VOL));
                self.dialog = Some(Question::Quit);
                self.focus = None;
            }
            Action::Restart => {
                self.dialog = Some(Question::Restart);
                self.focus = None;
            }
            Action::Yes => self.answer(true, out),
            Action::No => self.answer(false, out),
        }
    }

    fn answer(&mut self, yes: bool, out: &mut Vec<Effect>) {
        let q = self.dialog.take();
        self.focus = None;
        if !yes {
            return;
        }
        match q {
            Some(Question::Quit) => out.push(Effect::Exit),
            Some(Question::Restart) => {
                self.close(out);
                out.push(Effect::GiveUp);
            }
            None => {}
        }
    }

    /// The D-pad / stick / arrows move to the nearest control that way; sliders and rows
    /// use left/right themselves while focused.
    fn move_focus(&mut self, input: &Input, controls: &[(Control, [f32; 4])]) {
        if !input.pad {
            return;
        }
        let current = self.focus.and_then(|f| controls.iter().find(|(c, _)| *c == f).copied());
        let Some((cur, area)) = current else {
            // Nothing focused yet: the first press picks the top-left control (or "Yes").
            if input.nav.is_some() || input.accept {
                self.focus = controls.iter().min_by(|a, b| (a.1[1] + a.1[0] * 0.5).total_cmp(&(b.1[1] + b.1[0] * 0.5))).map(|c| c.0);
            }
            return;
        };
        let Some(d) = input.nav else { return };
        if matches!(cur, Control::Slider(_) | Control::Row(_)) && d[0] != 0 {
            return;
        }
        let others = controls.iter().filter(|(c, _)| *c != cur).map(|(c, a)| (*c, centre(*a)));
        self.focus = crate::menu::nearest_in_direction(centre(area), d, others).or(Some(cur));
    }
}

fn row_label(r: Row, save: &SaveGame, display: Option<&DisplayInfo>) -> Vec<u8> {
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

fn step_row(r: Row, step: i32, save: &mut SaveGame, display: Option<&DisplayInfo>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_walks_the_overlay_and_quits_through_the_dialog() {
        let (mut o, mut save) = (Options::default(), SaveGame::default());
        assert_eq!(o.show(Mode::Level, Some(Tab::Credits)), vec![Effect::Pause(true)]);
        let pad = |nav: Option<[i32; 2]>, accept: bool| Input { nav, accept, pad: true, ..Default::default() };
        o.update(&pad(Some([0, -1]), false), &mut save, None);
        assert_eq!(o.focus, Some(Control::Button(Action::Tab(Tab::Audio))));
        // Down to the quit button, through the others.
        for _ in 0..10 {
            o.update(&pad(Some([0, -1]), false), &mut save, None);
        }
        assert_eq!(o.focus, Some(Control::Button(Action::Quit)));
        o.update(&pad(None, true), &mut save, None);
        assert_eq!(o.dialog, Some(Question::Quit));
        o.update(&pad(Some([0, -1]), false), &mut save, None);
        assert_eq!(o.update(&pad(None, true), &mut save, None), vec![Effect::Exit]);
    }

    #[test]
    fn sliders_step_with_the_pad() {
        let (mut o, mut save) = (Options { tab: Tab::Audio, ..Default::default() }, SaveGame::default());
        o.show(Mode::Menu, None);
        o.focus = Some(Control::Slider(Slider::Music));
        let before = save.music;
        let fx = o.update(&Input { nav: Some([-1, 0]), pad: true, ..Default::default() }, &mut save, None);
        assert_eq!(save.music, (before - SLIDER_STEP).max(0.0));
        assert_eq!(fx, vec![Effect::SaveChanged]);
        assert_eq!(o.focus, Some(Control::Slider(Slider::Music)));
    }
}
