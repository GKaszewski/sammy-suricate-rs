//! Keyboard + gamepad merged into game actions, once per frame.
//!
//! Keyboard (original): arrows move, Ctrl jump, Space fire, Shift boost, Alt U-turn,
//! Enter confirm, Esc options. Gamepad (Xbox layout): left stick / D-pad move, A jump,
//! X fire, RT/RB boost, B U-turn, Start options, Back confirm (hints, intro skip).
//! In menus the D-pad / stick move the focus, A activates and B goes back.

use bevy::input::InputSystems;
use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Controls>().add_systems(PreUpdate, read_controls.after(InputSystems));
    }
}

const DEADZONE: f32 = 0.25;
/// Stick nav: first repeat after this many seconds, then every `NAV_REPEAT`.
const NAV_DELAY: f32 = 0.4;
const NAV_REPEAT: f32 = 0.15;

#[derive(Resource, Default, Debug)]
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
    /// A focus move this frame (x right, y up), from the D-pad or the stick.
    pub nav: Option<IVec2>,
    /// Any key or pad button was pressed this frame.
    pub any: bool,
    /// The last input came from a gamepad (the menus show focus instead of the cursor).
    pub pad: bool,
}

#[derive(Default)]
struct NavRepeat {
    dir: Option<IVec2>,
    timer: f32,
}

fn read_controls(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    pads: Query<&Gamepad>,
    mut c: ResMut<Controls>,
    mut repeat: Local<NavRepeat>,
) {
    let key = |k: &[KeyCode]| k.iter().any(|k| keys.pressed(*k));
    let hit = |k: &[KeyCode]| k.iter().any(|k| keys.just_pressed(*k));
    let axis = |pos: KeyCode, neg: KeyCode| keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32;
    let mut out = Controls {
        turn: axis(KeyCode::ArrowRight, KeyCode::ArrowLeft),
        forward: axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
        jump: key(&[KeyCode::ControlLeft, KeyCode::ControlRight]),
        jump_hit: hit(&[KeyCode::ControlLeft, KeyCode::ControlRight]),
        fire_hit: hit(&[KeyCode::Space]),
        boost: key(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        uturn_hit: hit(&[KeyCode::AltLeft, KeyCode::AltRight]),
        confirm: hit(&[KeyCode::Enter, KeyCode::NumpadEnter]),
        options: hit(&[KeyCode::Escape]),
        any: keys.get_just_pressed().next().is_some(),
        pad: c.pad,
        ..default()
    };
    // Keyboard arrows also navigate the menus.
    let mut nav = None;
    for (k, d) in [(KeyCode::ArrowLeft, IVec2::NEG_X), (KeyCode::ArrowRight, IVec2::X), (KeyCode::ArrowUp, IVec2::Y), (KeyCode::ArrowDown, IVec2::NEG_Y)] {
        if keys.just_pressed(k) {
            nav = Some(d);
        }
    }
    if out.any || mouse.get_just_pressed().next().is_some() || motion.delta.length_squared() > 4.0 {
        out.pad = false;
    }
    let mut stick_dir = None;
    for pad in &pads {
        let stick = pad.left_stick();
        let dpad = pad.dpad();
        let (sx, sy) = (dz(stick.x), dz(stick.y));
        out.turn = (out.turn + sx + dpad.x).clamp(-1.0, 1.0);
        out.forward = (out.forward + sy + dpad.y).clamp(-1.0, 1.0);
        out.jump |= pad.pressed(GamepadButton::South);
        out.jump_hit |= pad.just_pressed(GamepadButton::South);
        out.accept |= pad.just_pressed(GamepadButton::South);
        out.fire_hit |= pad.just_pressed(GamepadButton::West);
        out.boost |= pad.pressed(GamepadButton::RightTrigger) || pad.pressed(GamepadButton::RightTrigger2);
        out.uturn_hit |= pad.just_pressed(GamepadButton::East);
        out.back |= pad.just_pressed(GamepadButton::East);
        out.options |= pad.just_pressed(GamepadButton::Start);
        out.confirm |= pad.just_pressed(GamepadButton::Select);
        let pressed = pad.get_just_pressed().next().is_some();
        out.any |= pressed;
        for (b, d) in [
            (GamepadButton::DPadLeft, IVec2::NEG_X),
            (GamepadButton::DPadRight, IVec2::X),
            (GamepadButton::DPadUp, IVec2::Y),
            (GamepadButton::DPadDown, IVec2::NEG_Y),
        ] {
            if pad.just_pressed(b) {
                nav = Some(d);
            }
        }
        if stick.length() > 0.5 {
            stick_dir = Some(if stick.x.abs() > stick.y.abs() { IVec2::new(stick.x.signum() as i32, 0) } else { IVec2::new(0, stick.y.signum() as i32) });
        }
        if pressed || stick.length() > 0.5 || dpad != Vec2::ZERO {
            out.pad = true;
        }
    }
    // Stick navigation with key-repeat timing.
    match stick_dir {
        Some(d) if repeat.dir != Some(d) => {
            *repeat = NavRepeat { dir: Some(d), timer: NAV_DELAY };
            nav = nav.or(Some(d));
        }
        Some(d) => {
            repeat.timer -= time.delta_secs();
            if repeat.timer <= 0.0 {
                repeat.timer = NAV_REPEAT;
                nav = nav.or(Some(d));
            }
        }
        None => repeat.dir = None,
    }
    out.nav = nav;
    // Debug: SAMMY_PAD_NAV=right,down,a,b,... presses one pad input every 0.5 s from 3 s on.
    if let Ok(script) = std::env::var("SAMMY_PAD_NAV") {
        let t = time.elapsed_secs() - 3.0;
        let step = (t / 0.5).floor();
        if t >= 0.0 && (t - time.delta_secs()) / 0.5 < step
            && let Some(cmd) = script.split(',').nth(step as usize)
        {
            out.pad = true;
            match cmd {
                "left" => out.nav = Some(IVec2::NEG_X),
                "right" => out.nav = Some(IVec2::X),
                "up" => out.nav = Some(IVec2::Y),
                "down" => out.nav = Some(IVec2::NEG_Y),
                "a" => (out.accept, out.jump_hit, out.any) = (true, true, true),
                "b" => (out.back, out.uturn_hit) = (true, true),
                "start" => out.options = true,
                _ => {}
            }
        }
        out.pad |= t >= 0.0;
    }
    *c = out;
}

fn dz(v: f32) -> f32 {
    if v.abs() < DEADZONE { 0.0 } else { (v - DEADZONE * v.signum()) / (1.0 - DEADZONE) }
}
