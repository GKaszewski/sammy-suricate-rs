//! Help signs: `InitTitle` / `Title_Hit` (Intro2.wdl:492-760). A sprite that fades in
//! when an id-25 scanner comes within range, stays while it is near, lingers, fades
//! out, and can be dismissed with Enter. One-shot.

use formats::wmb::Object;
use crate::time::Ticks;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    Waiting,
    FadeIn,
    Near,
    Linger(Ticks),
    FadeOut,
    Done,
}

/// The trigger centre is this far below the sign.
const CENTER_DROP: f32 = 100.0;
/// Trigger distance unless frac(Skill1)×1000 gives one.
const DEFAULT_RANGE: f32 = 300.0;

#[derive(Clone, Debug)]
pub struct Title {
    /// Trigger centre.
    pub center: [f32; 3],
    /// Trigger distance.
    range: f32,
    /// Target alpha in percent (Skill8).
    alpha_max: f32,
    /// Fade-in ticks, lifetime after leaving, fade-out ticks (Skill5/6/7).
    fade_in: f32,
    life: Ticks,
    fade_out: f32,
    /// Opacity in percent.
    pub alpha: f32,
    phase: Phase,
}

impl Title {
    pub fn from_object(obj: &Object) -> Self {
        let range = ((obj.skill(1).fract() * 1000.0) + 0.5).floor();
        let or1 = |v: f32| if v == 0.0 { 1.0 } else { v };
        Self {
            center: [obj.origin[0], obj.origin[1], obj.origin[2] - CENTER_DROP],
            range: if range == 0.0 { DEFAULT_RANGE } else { range },
            alpha_max: if obj.skill(8) == 0.0 { 100.0 } else { obj.skill(8) },
            fade_in: or1(obj.skill(5)),
            life: Ticks(or1(obj.skill(6))),
            fade_out: or1(obj.skill(7)),
            alpha: 0.0,
            phase: Phase::Waiting,
        }
    }

    /// One frame; `nearest` is the nearest scanner's distance. Returns whether shown.
    pub fn step(&mut self, nearest: Option<f32>, enter: bool, t: Ticks) -> bool {
        let near = nearest.is_some_and(|d| d <= self.range);
        self.phase = match self.phase {
            Phase::Waiting if near => Phase::FadeIn,
            Phase::FadeIn | Phase::Near | Phase::Linger(_) if enter => Phase::FadeOut,
            Phase::FadeIn => {
                self.alpha += self.alpha_max / self.fade_in * t;
                if self.alpha >= self.alpha_max {
                    self.alpha = self.alpha_max;
                    Phase::Near
                } else {
                    Phase::FadeIn
                }
            }
            Phase::Near if !near => Phase::Linger(self.life),
            Phase::Linger(left) if left - t <= Ticks::ZERO => Phase::FadeOut,
            Phase::Linger(left) => Phase::Linger(left - t),
            Phase::FadeOut => {
                self.alpha -= self.alpha_max / self.fade_out * t;
                if self.alpha <= 0.0 { Phase::Done } else { Phase::FadeOut }
            }
            other => other,
        };
        !matches!(self.phase, Phase::Waiting | Phase::Done)
    }
}
