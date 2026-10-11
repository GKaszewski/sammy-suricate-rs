//! Where everything sits on the plate, and what to draw this frame.

use super::*;

/// Tab buttons.
pub(super) const TAB_AUDIO: [f32; 4] = [41.0, 14.0, 218.0, 80.0];
pub(super) const TAB_HELP: [f32; 4] = [41.0, 105.0, 226.0, 47.0];
pub(super) const TAB_CREDITS: [f32; 4] = [41.0, 152.0, 226.0, 46.0];
/// Resume / back / quit / restart buttons.
pub(super) const BTN_RESUME: [f32; 4] = [0.0, 335.0, 306.0, 42.0];
pub(super) const BTN_BACK: [f32; 4] = [0.0, 377.0, 306.0, 40.0];
pub(super) const BTN_QUIT: [f32; 4] = [0.0, 417.0, 306.0, 41.0];
pub(super) const BTN_RESTART: [f32; 4] = [48.0, 268.0, 204.0, 50.0];
/// Yes/No dialog: plate, text origin and the two buttons.
pub(super) const DIALOG_PLATE: [f32; 4] = [223.0, 135.0, 199.0, 68.0];
pub(super) const DIALOG_TEXT_POS: [f32; 2] = [228.0, 138.0];
pub(super) const DIALOG_YES: [f32; 4] = [235.0, 171.0, 68.0, 28.0];
pub(super) const DIALOG_NO: [f32; 4] = [341.0, 171.0, 68.0, 28.0];
/// Audio tab: music/sound art (and the sub-rect of `volmus` it shows).
pub(super) const VOLMUS: [f32; 4] = [347.0, 17.0, 266.0, 249.0];
/// Slider tracks (`HSLIDER`, 227 px with a 32-px thumb): x, thumb height, rows' y.
pub(super) const TRACK: f32 = 227.0;
pub(super) const THUMB: f32 = 32.0;
pub(super) const SLIDER_X: f32 = 365.0;
pub(super) const SLIDER_H: f32 = 29.0;
pub(super) const SLIDER_MUSIC_Y: f32 = 60.0;
pub(super) const SLIDER_SOUND_Y: f32 = 161.0;
/// Slider range (percent) and the pad step.
pub(super) const SLIDER_MAX: f32 = 100.0;
pub(super) const SLIDER_STEP: f32 = 5.0;
/// Display rows: x, size and the three rows' y.
pub(super) const ROW_X: f32 = 352.0;
pub(super) const ROW_W: f32 = 256.0;
pub(super) const ROW_H: f32 = 24.0;
pub(super) const ROW_MODE_Y: f32 = 272.0;
pub(super) const ROW_SIZE_Y: f32 = 302.0;
pub(super) const ROW_UI_SIZE_Y: f32 = 346.0;
/// Highest `ui_size` setting (0 large, 1 medium, 2 small).
pub(super) const UI_SIZE_MAX: u8 = 2;
/// Help tab: text origin, picture, previous / next page buttons.
pub(super) const HELP_TEXT_POS: [f32; 2] = [334.0, 20.0];
pub(super) const HELP_PICTURE: [f32; 4] = [330.0, 297.0, 297.0, 113.0];
pub(super) const HELP_PREV: [f32; 4] = [330.0, 408.0, 65.0, 62.0];
pub(super) const HELP_NEXT: [f32; 4] = [560.0, 408.0, 65.0, 62.0];
/// Help pages 1-7; page 2 has no picture.
pub(super) const HELP_LAST_PAGE: u32 = 7;
pub(super) const HELP_NO_PICTURE_PAGE: u32 = 2;
/// Credits tab: team picture and the "Thanksto" text origin.
pub(super) const CREDITS_TEAM: [f32; 4] = [340.0, 25.0, 282.0, 294.0];
pub(super) const CREDITS_THANKS_POS: [f32; 2] = [334.0, 320.0];
/// Volume of the button / row sounds.
pub(super) const UI_SFX_VOL: f32 = 50.0;
/// The screen behind the overlay is dimmed this much.
pub const OVERLAY_ALPHA: f32 = 0.7;
/// Focus frame: border, glow colour (RGB), fill alpha and padding around the item.
pub const FRAME_BORDER: f32 = 2.0;
pub const FRAME_RGB: [f32; 3] = [1.0, 0.85, 0.2];
pub const FRAME_FILL_ALPHA: f32 = 0.15;
pub const FRAME_PAD: f32 = 6.0;
impl Options {
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
    pub(super) fn controls(&self, display: Option<&DisplayInfo>) -> Vec<(Control, [f32; 4])> {
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
    pub(super) fn button_images(a: Action) -> (&'static str, &'static str) {
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

}
