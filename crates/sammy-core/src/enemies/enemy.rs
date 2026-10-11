//! An enemy's state, its patrol nodes and what the frontend reads from it.

use super::*;

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
    pub alpha: f32,
    /// Editor/script `passable` (wasp enemies): never blocks movers.
    pub(super) passable: bool,
    /// Wasp enemy: keeps its editor height instead of snapping to the floor.
    pub(super) flying_init: bool,
    /// `IdWaspSpec` of wasp enemies (`InitWaspEnnemy`), 0 for ground enemies.
    pub wasp_spec: u32,
    /// Hidden while blinking out after death.
    pub hidden: bool,
}

impl Enemy {
    /// One-line state for debug logs.
    pub fn debug_line(&self) -> String {
        format!(
            "etat {} pos {:?} pan {:.0} res {:.1} attack {:.0} react {:.0} fear {:.1}",
            self.etat,
            self.pos.map(|v| v.round()),
            self.pan,
            self.resistance,
            self.attack_range,
            self.react_range,
            self.attacked
        )
    }

    /// `DisplayEnnemyRange` (lion.wdl:1586-1653): (skin, frame %, radius) of the
    /// range ring while the Eye of Truth is active, or None when hidden.
    pub fn ring(&self, display_mode: f32) -> Option<(u32, f32, f32)> {
        if self.etat == 7 || self.class == 0 {
            return None;
        }
        let d = self.dist.max(1.0);
        let calm = |e: &Enemy| {
            let skin = if d < e.attack_range + (e.react_range - e.attack_range) * 0.5 { 2 } else { 1 };
            (skin, (100.0 / (e.attack_range / d)).clamp(60.0, 99.0), e.attack_range)
        };
        if display_mode > 1.0 {
            return Some(if self.etat == 2 { self.chase_ring(d) } else { calm(self) });
        }
        if self.class != LION || !self.avu {
            return None;
        }
        if self.etat == 2 {
            return Some(self.chase_ring(d));
        }
        (self.etat == 1 || self.random_org2 != 0.0).then(|| calm(self))
    }

    pub(super) fn chase_ring(&self, d: f32) -> (u32, f32, f32) {
        let mut radius = self.attack_range;
        let mut skin = 2;
        if self.avu {
            skin = 3;
            if self.attack_range < d {
                radius = d + 128.0;
            }
        }
        (skin, (100.0 / (radius / d)).clamp(55.0, 99.0), radius)
    }

    pub fn alive(&self) -> bool {
        !matches!(self.etat, 6 | 7) && (self.class == LION || self.class == ELEPHANT)
    }

    /// `IdEnnemy`.
    pub fn class(&self) -> u32 {
        self.class
    }

    /// `IdEnnemySpec` (1-3 ground enemies, 101-107 wasps).
    pub fn spec_id(&self) -> u32 {
        self.spec
    }

    /// Flag5: scans id 25 every tick (camera zones, credit titles).
    pub fn scans_25(&self) -> bool {
        self.flag(5) && self.initialised && !self.gone()
    }

    /// Wasps hitting it leave a visible corpse (`Lion_Hit` sets `wasptat` 1004).
    pub fn keeps_wasp_corpses(&self) -> bool {
        self.class == LION
    }

    pub fn is_class_target(&self) -> bool {
        self.class == LION || self.class == ELEPHANT
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn floor(&self) -> f32 {
        self.floor
    }

    /// Dead and gone (blinked out): no shadow, no ring.
    pub fn gone(&self) -> bool {
        self.class == 0 || (self.etat == 7 && self.dying >= 33.0)
    }

    pub(super) fn flag(&self, n: u32) -> bool {
        self.flags & (1 << (n - 1)) != 0
    }

    pub(super) fn walk_speed(&self) -> f32 {
        0.82 * self.speed * if self.spec == 3 { 3.0 } else { 1.0 }
    }

    pub(super) fn run_speed(&self) -> f32 {
        0.3 * self.speed * self.accel_attack
    }

    pub(super) fn set_state(&mut self, etat: i32, anim_speed: f32) {
        self.etat = etat;
        self.anim_step = 0.0;
        self.anim_speed = anim_speed;
    }
}

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
