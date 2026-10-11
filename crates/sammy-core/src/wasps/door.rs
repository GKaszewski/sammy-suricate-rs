//! Weak doors that wasps break, and their planks.

use super::*;

/// A door wasps can break (`InitDoorWeak`).
#[derive(Clone, Copy, Debug)]
pub struct WeakDoor {
    pub hp: f32,
}

/// What a damaged weak door turns into.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorDamage {
    Untouched,
    /// A more broken prefab.
    Morph(&'static str),
    /// Gone: planks fly out.
    Destroyed,
}

/// Sammy's id as a weak door's Skill1 (meaning "use Skill2").
pub(super) const SAMMY_ID: f32 = 3333.0;
/// Planks a destroyed door breaks into.
pub const PLANKS: [&str; 6] = ["plank1.mdl", "plank2.mdl", "plank3.mdl", "plank1.mdl", "plank2.mdl", "plank3.mdl"];

impl WeakDoor {
    pub fn from_object(obj: &Object) -> Self {
        let hp = if obj.skill(1) == 0.0 || obj.skill(1) == SAMMY_ID { obj.skill(2) } else { obj.skill(1) };
        Self { hp }
    }

    /// A wasp of `poison` hit it.
    pub fn hit(&mut self, poison: f32) {
        self.hp = (self.hp - poison).trunc();
    }

    /// `ExplodeDoorWeak`: HP 3/2/1 show doorwk2/3/4, 0 or less breaks it.
    pub fn damage(&self) -> DoorDamage {
        match self.hp as i32 {
            1 => DoorDamage::Morph("doorwk4.wmb"),
            2 => DoorDamage::Morph("doorwk3.wmb"),
            3 => DoorDamage::Morph("doorwk2.wmb"),
            4.. => DoorDamage::Untouched,
            _ => DoorDamage::Destroyed,
        }
    }
}

/// A plank of a broken door (simplified `LaunchPlank`): ballistic tumble with bounces,
/// settle flat, then vanish.
#[derive(Clone, Debug)]
pub struct Plank {
    pub(super) v: [f32; 3],
    pub(super) spin: [f32; 2],
    pub angles: [f32; 3],
    pub pos: [f32; 3],
    pub(super) still: f32,
}

pub(super) const PLANK_GRAVITY: f32 = 1.5;
pub(super) const PLANK_STILL: f32 = 24.0;

impl Plank {
    /// One plank around a door at `origin`.
    pub fn burst(origin: [f32; 3], rng: &mut Rng) -> Self {
        let pos = [origin[0] + rng.random(100.0) - 50.0, origin[1] + rng.random(100.0) - 50.0, origin[2] + rng.random(300.0)];
        let angles = [rng.random(360.0), 0.0, 0.0];
        Self {
            v: [rng.random(40.0) - 20.0, rng.random(40.0) - 20.0, 30.0],
            spin: [30.0 * rng.random(2.0) - 1.0, 30.0 * rng.random(2.0) - 1.0],
            angles,
            pos,
            still: PLANK_STILL,
        }
    }

    /// One frame; returns (bounce sound, gone).
    pub fn step(&mut self, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, t: f32) -> (bool, bool) {
        self.v[2] -= PLANK_GRAVITY * t;
        let target = [0, 1, 2].map(|i| self.pos[i] + self.v[i] * t);
        let hit = trace(self.pos, target);
        self.pos = hit.end;
        let mut sound = false;
        if hit.hit() && hit.normal[2] > 0.5 {
            sound = self.v[2].abs() > 6.0;
            self.v[2] = -self.v[2] * 0.5;
            self.v[0] *= 0.6;
            self.v[1] *= 0.6;
        }
        let speed = self.v[0].abs() + self.v[1].abs();
        if speed < 2.0 && hit.hit() {
            self.still -= t;
            self.angles[1] += (90.0 - self.angles[1]) * (0.5 * t).min(1.0);
        } else {
            self.angles[1] += self.spin[0] * t;
            self.angles[2] += self.spin[1] * t;
        }
        (sound, self.still <= 0.0)
    }
}
