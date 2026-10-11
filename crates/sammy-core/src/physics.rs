//! Vertical physics globals shared by Sammy, gems and wasps.

use formats::wmb::Object;

/// Jump and fall parameters (InitLvl.wdl:60-61 defaults, `InitGame3` overrides,
/// mm1.wdl:1846).
#[derive(Clone, Copy, Debug)]
pub struct JumpParams {
    /// `BlobBallVerticalSpeed`: jump impulse.
    pub jump_speed: f32,
    /// `BlobBallVerticalFriction`: deceleration during a jump.
    pub jump_fric: f32,
    /// `Gravite`: initial vertical speed when walking off a ledge.
    pub gravity: f32,
    /// `AccGravite`: deceleration when falling off a ledge.
    pub acc_gravity: f32,
    /// `BounceOnEnnemy` (InitGame7 Skill1, default 0): stomping a lion re-jumps × this.
    pub bounce_on_enemy: f32,
}

impl Default for JumpParams {
    fn default() -> Self {
        Self { jump_speed: 30.0, jump_fric: 3.0, gravity: -2.0, acc_gravity: 1.5, bounce_on_enemy: 0.0 }
    }
}

impl JumpParams {
    /// `InitGame3`: Skill1 jump speed, Skill2 jump friction, Skill5 gravity, Skill6
    /// gravity acceleration (0 keeps the default).
    pub fn apply_init_game3(&mut self, obj: &Object) {
        for (v, n) in [(&mut self.jump_speed, 1), (&mut self.jump_fric, 2), (&mut self.gravity, 5), (&mut self.acc_gravity, 6)] {
            if obj.skill(n) != 0.0 {
                *v = obj.skill(n);
            }
        }
    }
}
