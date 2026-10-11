//! Sliding down slopes and leaning into them (`handlePente`), and knockback.

use super::{BLOB_HAUT, Frame, HULL, Player};
use crate::collision::{Space, floor_below, slide_move};
use crate::space::Pos;

/// A slide only starts within this much above the normal standing height.
const SLOPE_REACH: f32 = 32.0;
/// Downhill pull on a slope.
const SLOPE_DOWN_FORCE: f32 = -20.0;
/// Leaning into a slope: only for real slides, limited, easing 10 % per tick.
const LEAN_MIN_SLIDE: f32 = 4.0;
const LEAN_MAX_TILT: f32 = 12.0;
const LEAN_MAX_ROLL: f32 = 6.0;
const LEAN_EASE: f32 = 0.1;
const LEAN_RELAX: f32 = 0.6;
/// Slide components below this stop.
const SLIDE_EPSILON: f32 = 0.01;
/// Knockback decay: `AmortiBouncePlayer` × 80 / (fps + 1) at the reference 50 fps.
const KNOCK_DECAY_SCALE: f32 = 80.0 / 51.0;

impl Player {
    /// `handlePente`: on floors steeper than ~23° Sammy slides downhill and leans;
    /// elsewhere the slide and the lean die out.
    pub(super) fn slide<S: Space>(&mut self, f: &Frame<S>, t: f32) {
        let tuning = f.tuning;
        let me = Some(f.me);
        if let Some((fl, _, n)) = floor_below(f.space, self.pos.get())
            && n[2] > 0.0
            && n[2] <= tuning.slope_angle_min
            && self.pos[2] - fl <= BLOB_HAUT + SLOPE_REACH
        {
            let force = [n[0] * tuning.force_pente, n[1] * tuning.force_pente, SLOPE_DOWN_FORCE];
            let v2 = 1.0 - n[2];
            for (p, f) in self.pente.iter_mut().zip(force) {
                *p += tuning.acc_pente * f.signum() * v2;
                if p.abs() > f.abs() {
                    *p = f;
                }
            }
            if self.pente[0].abs() > LEAN_MIN_SLIDE || self.pente[1].abs() > LEAN_MIN_SLIDE {
                self.lean(n, t);
            }
            let d = self.pente.map(|x| x * t);
            self.pos = Pos(slide_move(f.space, HULL, self.pos.get(), d, me).0);
        } else {
            self.tilt -= LEAN_RELAX * self.tilt * t.min(1.0);
            self.roll -= LEAN_RELAX * self.roll * t.min(1.0);
            let k = (1.0 - tuning.amorti_pente_end * t).max(0.0);
            self.pente = self.pente.map(|x| if x.abs() < SLIDE_EPSILON { 0.0 } else { x * k });
            if self.pente != [0.0; 3] {
                let d = self.pente.map(|x| x * t);
                self.pos = Pos(slide_move(f.space, HULL, self.pos.get(), [d[0], d[1], 0.0], me).0);
            }
        }
    }

    /// Eases the visual tilt and roll toward the floor normal `n`.
    fn lean(&mut self, n: [f32; 3], t: f32) {
        let (sp, cp) = (-self.pan).radians().sin_cos();
        let local = [n[0] * cp - n[1] * sp, n[0] * sp + n[1] * cp];
        let want_tilt = -local[0].asin().to_degrees();
        let want_roll = -local[1].asin().to_degrees();
        if self.tilt.abs() < LEAN_MAX_TILT {
            self.tilt += LEAN_EASE * (want_tilt - self.tilt) * t;
        }
        if self.roll.abs() < LEAN_MAX_ROLL {
            self.roll += LEAN_EASE * (want_roll - self.roll) * t;
        }
    }

    /// Knockback (`BounceForce1`): a separate move, decaying by `AmortiBouncePlayer`.
    pub(super) fn knockback<S: Space>(&mut self, f: &Frame<S>, t: f32) {
        if self.knock == [0.0; 3] {
            return;
        }
        let k = self.knock.map(|v| v * t);
        self.pos = Pos(slide_move(f.space, HULL, self.pos.get(), [k[0], k[1], 0.0], Some(f.me)).0);
        let dec = f.tuning.amorti_bounce * KNOCK_DECAY_SCALE * t;
        self.knock = self.knock.map(|v| if v.abs() <= dec { 0.0 } else { v - dec * v.signum() });
    }
}
