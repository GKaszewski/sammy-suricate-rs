//! An enemy's state, its patrol nodes and what the frontend reads from it.

use super::*;
use crate::percent::Percent;

/// Patrol path node (`InitPath`, lion.wdl:774-780).
#[derive(Clone, Debug)]
pub struct PathNode {
    pub(super) id: f32,
    pub(super) next: f32,
    pub(super) pos: [f32; 3],
    pub(super) pan: f32,
    pub(super) tilt: f32,
}

impl PathNode {
    pub fn from_object(obj: &Object) -> Self {
        Self { id: obj.skill(1), next: obj.skill(2), pos: obj.origin, pan: obj.angles[0], tilt: obj.angles[1] }
    }
}

#[derive(Clone, Debug)]
pub struct Enemy {
    // --- configuration (§1.1-1.2)
    pub(super) class: u32,
    /// 1 lion family, 2 elephant, 3 scorpion.
    pub(super) spec: u32,
    pub(super) weight: f32,
    pub(super) head_vertex: usize,
    pub(super) resistance: f32,
    pub(super) resistance_org: f32,
    pub(super) speed: f32,
    /// `BounceForce`: push multiplier given to whoever touches this enemy.
    pub(super) push_force: f32,
    pub(super) attack_range: f32,
    pub(super) attack_range_bis: f32,
    pub(super) react_range: f32,
    pub(super) path_id: f32,
    pub(super) path_range: f32,
    pub(super) random_org: f32,
    pub(super) random_org2: f32,
    pub(super) accel_attack: f32,
    pub(super) time_for_nothing: f32,
    pub(super) flags: u32,
    pub(super) scale: f32,
    // --- runtime (§1.4)
    pub pos: [f32; 3],
    pub(super) prev_pos: [f32; 3],
    pub pan: f32,
    pub(super) prev_pan: f32,
    pub(super) tilt: f32,
    pub(super) floor: f32,
    pub(super) hover: f32,
    pub(super) initialised: bool,
    pub(super) etat: i32,
    pub(super) etat_bis: i32,
    pub(super) guard_stuck: f32,
    pub(super) delta_pan: f32,
    pub(super) dest_pan: f32,
    pub(super) ball_accel: f32,
    pub(super) anim_step: f32,
    pub(super) anim_speed: f32,
    pub(super) random_time: f32,
    pub(super) random_time2: f32,
    pub(super) target: [f32; 3],
    pub(super) target_pan: f32,
    pub(super) target_tilt: f32,
    pub(super) time_without_patrol: f32,
    pub attacked: f32,
    pub(super) bounce: [f32; 3],
    pub(super) vspeed: f32,
    pub(super) fric_vspeed: f32,
    pub(super) vertical: f32,
    /// `EnnemySpeed`: last velocity in quants/tick.
    pub vel: [f32; 3],
    /// Swipe not yet landed (`BaffeDonne == -999`).
    pub(super) swipe_pending: bool,
    /// Cumulative blink time in state 7.
    pub(super) dying: f32,
    /// World position of the head vertex (wasps home on it).
    pub muzzle: Option<[f32; 3]>,
    /// Model-space bbox (native, scaled) for the collision box.
    pub(super) half: [f32; 3],
    pub(super) center_z: f32,
    pub(super) enable_block: bool,
    /// Result of the last line-of-sight test (`EnnemyAvuPlayer`).
    pub(super) avu: bool,
    /// Range-ring height offset (`CircleZ = 8 + random(12)`).
    pub circle_z: f32,
    /// Distance to Sammy from the last `TestDistLionPlayer`.
    pub dist: f32,
    /// Entity the gravity trace stands on (platform, teleport pad...).
    pub(super) floor_entity: Option<Id>,
    /// Teleport pad in use: (pad, destination, fading in?).
    pub(super) teleport: Option<(Id, [f32; 3], bool)>,
    /// Opacity in percent (teleport fades).
    pub alpha: Percent,
    /// Editor/script `passable` (wasp enemies): never blocks movers.
    pub(super) passable: bool,
    /// Wasp enemy: keeps its editor height instead of snapping to the floor.
    pub(super) flying_init: bool,
    /// `IdWaspSpec` of wasp enemies (`InitWaspEnnemy`), 0 for ground enemies.
    pub wasp_spec: u32,
    /// Hidden while blinking out after death.
    pub hidden: bool,
}
