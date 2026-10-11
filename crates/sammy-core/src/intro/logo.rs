//! The intro's logos fading in and out.

/// `DisplayLogoA5` (Intro2.wdl:924-1181): one entity morphed through three logos.
#[derive(Clone, Debug, Default)]
pub struct Logo {
    pub(super) step: usize,
    /// Ticks spent in the current step.
    pub(super) t: f32,
    /// `Skill2`: distance ahead of the camera; `Skill6`: fade counter.
    pub dist: f32,
    pub(super) s6: f32,
    pub alpha: f32,
    /// 0 none, 1 A5 sprite, 2 publisher, 3 Suricate.
    pub shown: u8,
}

/// One tick of `DisplayLogoA5` (docs/menu.md §2.2). The logo also drives the fog: the
/// intro loop's own fog changes race with it in the original [U]; the logo's are kept.
/// Returns the new fog (percent) when the logo sets it.
pub fn logo_tick(l: &mut Logo, fog: &mut f32) {
    l.t += 1.0;
    let next = |l: &mut Logo| {
        l.step += 1;
        l.t = 0.0;
    };
    match l.step {
        // Morph through the three files (1 tick each), fog 90.
        0 => {
            (l.dist, l.s6, l.alpha, l.shown) = (50.0, 100.0, 0.0, 1);
            *fog = 90.0;
            if l.t >= 3.0 {
                next(l);
            }
        }
        // A5 fade-in: Skill6 100 → 3 by 4, slides out to 285.
        1 => {
            l.s6 -= 4.0;
            l.dist = (l.dist + 10.0).min(285.0);
            l.alpha = 100.0 - l.s6;
            *fog = 90.0 - l.alpha * 0.25;
            if l.s6 <= 3.0 {
                next(l);
            }
        }
        // Hold 24.
        2 => {
            l.alpha = 100.0;
            l.dist = (l.dist + 5.0).min(285.0);
            *fog -= *fog * 0.05;
            if l.t >= 24.0 {
                l.s6 = 31.0;
                next(l);
            }
        }
        // Fade-out 30.
        3 => {
            l.alpha = l.s6 * 3.125;
            l.s6 -= 1.0;
            *fog -= *fog * 0.25;
            if l.s6 < 1.0 {
                next(l);
            }
        }
        // Gap 8, invisible, fog off.
        4 => {
            (l.alpha, l.shown) = (0.0, 0);
            *fog = 0.0;
            if l.t >= 8.0 {
                l.shown = 2;
                next(l);
            }
        }
        // Publisher: in +10, hold 32 + 24, out -20.
        5 => {
            l.alpha = (l.alpha + 10.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        6 => {
            if l.t >= 56.0 {
                next(l);
            }
        }
        7 => {
            l.alpha -= 20.0;
            if l.alpha <= 4.0 || l.t >= 16.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        // Suricate: gap 32, in +5, hold 64 + 24, out -15, removed.
        8 => {
            if l.t >= 32.0 {
                l.shown = 3;
                next(l);
            }
        }
        9 => {
            l.alpha = (l.alpha + 5.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        10 => {
            if l.t >= 88.0 {
                next(l);
            }
        }
        11 => {
            l.alpha -= 15.0;
            if l.alpha <= 0.0 || l.t >= 32.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        _ => {}
    }
}
