//! What the frontend reads from an enemy: its pose, collider and blink.

use super::*;

impl Enemy {
    /// Render pose between the last two AI ticks (`a` = tick fraction): position and
    /// angles.
    pub fn pose(&self, a: f32) -> Option<([f32; 3], [f32; 3])> {
        if !self.initialised {
            return None;
        }
        let p = [0, 1, 2].map(|i| self.prev_pos[i] + (self.pos[i] - self.prev_pos[i]) * a);
        Some((p, [self.prev_pan + ang(self.pan - self.prev_pan) * a, self.tilt, 0.0]))
    }

    /// The collision box once initialised: (centre, half extents, solid).
    pub fn collider(&self) -> Option<([f32; 3], [f32; 3], bool)> {
        let solid = self.class != 0 && !self.passable && self.teleport.is_none() && !matches!(self.etat, 7) && !(self.etat == 6 && self.anim_step > 50.0);
        self.initialised.then_some(([self.pos[0], self.pos[1], self.pos[2] + self.center_z], self.half, solid))
    }

    /// The death blink is running (its visibility is [`Enemy::hidden`]).
    pub fn blinking(&self) -> bool {
        self.etat == 7
    }
}
