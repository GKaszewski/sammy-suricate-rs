//! The frame's input: hover, clicks, sliders, rows and focus moves.

use super::*;

impl Options {
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

    pub(super) fn act(&mut self, a: Action, out: &mut Vec<Effect>) {
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

    pub(super) fn answer(&mut self, yes: bool, out: &mut Vec<Effect>) {
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
    pub(super) fn move_focus(&mut self, input: &Input, controls: &[(Control, [f32; 4])]) {
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
