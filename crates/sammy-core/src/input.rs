//! The input snapshot the game reads each frame, from any device.

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Controls {
    /// Right(+)/left(-) and forward(+)/back(-), -1..1 (analog on a stick).
    pub turn: f32,
    pub forward: f32,
    pub jump: bool,
    pub jump_hit: bool,
    pub fire_hit: bool,
    pub boost: bool,
    pub uturn_hit: bool,
    /// Enter / pad Back: dismiss hints, skip intros.
    pub confirm: bool,
    /// Esc / pad Start: open or close the options.
    pub options: bool,
    /// Pad B: back out of a menu level or dialog.
    pub back: bool,
    /// Pad A in menus (activate the focused item).
    pub accept: bool,
    /// A focus move this frame ([x right, y up]), from the D-pad or the stick.
    pub nav: Option<[i32; 2]>,
    /// Any key or pad button was pressed this frame.
    pub any: bool,
    /// The last input came from a gamepad (the menus show focus instead of the cursor).
    pub pad: bool,
}

