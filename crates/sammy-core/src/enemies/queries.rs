//! What an enemy's state says: class, rings, speeds, liveness.

use super::*;

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
