//! Per-level player tuning (`InitGame4` / `InitGame5`).

use formats::wmb::Object;

/// Per-level player tuning (InitLvl.wdl defaults; `InitGame5` turning and slopes,
/// `InitGame4` knockback; docs/level-objects.md §2.9).
#[derive(Clone, Copy, Debug)]
pub struct PlayerTuning {
    /// `SpeedRotationPlayer` / `SpeedRotationPlayerMove` (degrees per tick).
    pub turn_stand: f32,
    pub turn_move: f32,
    pub slope_angle_min: f32,
    pub acc_pente: f32,
    pub force_pente: f32,
    pub amorti_pente_end: f32,
    pub big_slope: f32,
    /// `IntensiteBlobBounce` / `AmortiBouncePlayer`.
    pub bounce: f32,
    pub amorti_bounce: f32,
}

impl Default for PlayerTuning {
    fn default() -> Self {
        Self {
            turn_stand: 10.0,
            turn_move: 7.0,
            slope_angle_min: 0.92,
            acc_pente: 1.5,
            force_pente: 100.0,
            amorti_pente_end: 0.3,
            big_slope: 10.0,
            bounce: 30.0,
            amorti_bounce: 10.0,
        }
    }
}

impl PlayerTuning {
    /// `InitGame5` (mm1.wdl:1912-1924): Skill1/2 turning, Skill4-8 slopes (0 keeps).
    pub fn apply_init_game5(&mut self, obj: &Object) {
        for (v, n) in [
            (&mut self.turn_stand, 1),
            (&mut self.turn_move, 2),
            (&mut self.slope_angle_min, 4),
            (&mut self.acc_pente, 5),
            (&mut self.force_pente, 6),
            (&mut self.amorti_pente_end, 7),
            (&mut self.big_slope, 8),
        ] {
            if obj.skill(n) != 0.0 {
                *v = obj.skill(n);
            }
        }
    }

    /// `InitGame4` (mm1.wdl:1891-1910): Skill5/6 knockback strength and decay.
    pub fn apply_init_game4(&mut self, obj: &Object) {
        if obj.skill(5) != 0.0 {
            self.bounce = obj.skill(5);
        }
        if obj.skill(6) != 0.0 {
            self.amorti_bounce = obj.skill(6);
        }
    }
}
