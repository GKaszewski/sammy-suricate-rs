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
