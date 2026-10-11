#[allow(unused_imports)]
use super::*;

#[derive(Component)]
pub(crate) struct OptionsRoot;

#[derive(Component)]
pub(crate) struct TabContent;

#[derive(Component)]
pub(crate) struct DialogRoot;

/// What a button does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
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
pub(crate) struct ButtonImages {
    pub(crate) normal: Handle<Image>,
    pub(crate) hover: Handle<Image>,
    /// Shown lit regardless of hover (selected tab).
    pub(crate) lit: bool,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum Slider {
    Music,
    Sound,
}

/// Port display settings, drawn as `<  value  >` rows in the old video slider's place.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Choice {
    Mode,
    Size,
    UiSize,
}

#[derive(Component)]
pub(crate) struct FocusFrame;

#[derive(Component)]
pub(crate) struct SliderThumb(pub(crate) Slider);
