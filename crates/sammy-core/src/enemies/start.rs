//! Level triggers that wake lions up.

use super::*;
use crate::time::Ticks;

/// `InitStartEnnemy` (mm1.wdl:5126-5170): every 8 ticks, wake the first lion of id
/// Skill1 within Skill2 into state Skill3 with new react/attack ranges; one-shot unless
/// Skill4.
#[derive(Clone, Debug)]
pub struct StartTrigger {
    pub(super) pos: [f32; 3],
    pub(super) s: [f32; 8],
    pub(super) timer: Ticks,
    pub(super) active: bool,
}

impl StartTrigger {
    pub fn from_object(obj: &Object) -> Self {
        let mut s = [0.0; 8];
        s.copy_from_slice(&obj.skills[..8]);
        Self { pos: obj.origin, s, timer: Ticks::ZERO, active: true }
    }

    /// One AI tick.
    pub fn tick(&mut self, enemies: &mut [&mut Enemy]) {
        if !self.active {
            return;
        }
        self.timer -= Ticks(1.0);
        if self.timer > Ticks::ZERO {
            return;
        }
        self.timer = Ticks(8.0);
        let (p, s) = (self.pos, self.s);
        let hit = enemies.iter_mut().find(|e| e.class as f32 == s[0] && e.alive() && (0..3).map(|i| (e.pos[i] - p[i]).powi(2)).sum::<f32>().sqrt() <= s[1]);
        if let Some(e) = hit {
            e.etat = s[2] as i32;
            if e.etat == 1 && e.class == LION {
                e.anim_step = 0.0;
                e.anim_speed = 0.82 * e.speed;
            }
            if s[4] != 0.0 {
                e.react_range = s[4];
            }
            if s[5] != 0.0 {
                e.attack_range = s[5];
            }
            if s[3] == 0.0 {
                self.active = false;
            } else {
                self.timer = Ticks(s[3]);
            }
        }
    }
}
