//! The intro's walking Sammy and his path nodes.

use super::*;
use crate::angle::Degrees;
use crate::time::Ticks;

/// The scripted Sammy (`InitPlayerPatrol` :231-329, `MoveSammyonPath` :412-456).
#[derive(Clone, Debug)]
pub struct Walker {
    pub pos: [f32; 3],
    pub pan: Degrees,
    pub vel: [f32; 3],
    pub(super) dest_pan: f32,
    /// Skill2 `SpeedBalle` (15) and Skill5 `BlobZoneScan` (600).
    pub(super) speed: f32,
    pub(super) range: f32,
    /// `PlayerPatrolEtat` and the scanned node (x, y, pan).
    pub(super) etat: u8,
    pub(super) target: Option<[f32; 3]>,
    /// `PlayerKey` of the last node scanned.
    pub key: f32,
    pub walking: bool,
}

/// `InitPathPlayer` (:392-398): invisible node, Skill4 = key, pan = way to the next.
#[derive(Clone, Debug)]
pub struct PathNode {
    pub(super) pos: [f32; 3],
    pub(super) pan: f32,
    pub(super) key: f32,
}

/// Walk speed (Skill2 `SpeedBalle`) and scan range (Skill5 `BlobZoneScan`) defaults.
pub(super) const WALKER_SPEED: f32 = 15.0;
pub(super) const WALKER_RANGE: f32 = 600.0;

impl Walker {
    pub fn from_object(obj: &Object) -> Self {
        Self {
            pos: obj.origin,
            pan: Degrees(obj.angles[0]),
            vel: [0.0; 3],
            dest_pan: obj.angles[0],
            speed: if obj.skill(2) != 0.0 { obj.skill(2) } else { WALKER_SPEED },
            range: if obj.skill(5) != 0.0 { obj.skill(5) } else { WALKER_RANGE },
            etat: 0,
            target: None,
            key: 1.0,
            walking: false,
        }
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }
}

impl PathNode {
    pub fn from_object(obj: &Object) -> Self {
        Self { pos: obj.origin, pan: obj.angles[0], key: obj.skill(4) }
    }
}

/// `SCAN` with a horizontal cone: the nearest node within `range` and `half` degrees of
/// `pan` (all hit nodes set the target; the nearest is assumed to win [U]).
pub(super) fn scan(nodes: &[PathNode], from: [f32; 3], pan: f32, half: f32, range: f32) -> Option<&PathNode> {
    nodes
        .iter()
        .filter_map(|n| {
            let (dx, dy) = (n.pos[0] - from[0], n.pos[1] - from[1]);
            let d = dx.hypot(dy);
            (d <= range && (d < 1.0 || ang(dy.atan2(dx).to_degrees() - pan).abs() <= half)).then_some((n, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(n, _)| n)
}

impl Walker {
    pub(super) fn hit(&mut self, n: &PathNode) {
        self.target = Some([n.pos[0], n.pos[1], n.pan]);
        self.etat = 1;
        self.key = n.key;
    }

    /// `MoveSammyonPath` over `t` ticks (per frame, rates × `time` as in the script).
    pub fn step(&mut self, nodes: &[PathNode], t: Ticks) {
        // PlayerPatrol.
        match self.target {
            None => {
                if let Some(n) = scan(nodes, self.pos, self.pan.get(), 30.0, self.range) {
                    self.hit(n);
                }
            }
            Some([tx, ty, tpan]) => {
                let (dx, dy) = (tx - self.pos[0], ty - self.pos[1]);
                let dist = dx.hypot(dy);
                let to = dy.atan2(dx).to_degrees();
                let mut v = 0.0;
                if dist < 100.0 {
                    v = ang(tpan - self.dest_pan) * (0.7 * t).min(1.0);
                    self.etat = 2;
                } else if self.etat == 1 {
                    v = ang(to - self.dest_pan) * (0.5 * t).min(1.0);
                } else if dist > 600.0 && self.etat == 2 {
                    self.etat = 1;
                    v = ang(to - self.dest_pan) * (0.2 * t).min(1.0);
                }
                self.dest_pan += v;
                if dist < 50.0 || (self.etat == 2 && v < 5.0) {
                    self.etat = 0;
                    self.target = None;
                    if let Some(n) = scan(nodes, [tx, ty, self.pos[2]], tpan, 10.0, 600.0) {
                        self.hit(n);
                    }
                }
            }
        }
        // Heading, then full speed ahead (`VitesseRaq.X = SpeedBalle`).
        self.pan += Degrees(ang(self.dest_pan - self.pan.0) * (0.25 * t).min(1.0));
        let a = self.pan.radians();
        self.vel = [a.cos() * self.speed, a.sin() * self.speed, 0.0];
        self.pos[0] += self.vel[0] * t;
        self.pos[1] += self.vel[1] * t;
        // The per-tick path scan (cone 60, range BlobZoneScan, id 30).
        if let Some(n) = scan(nodes, self.pos, self.pan.get(), 30.0, self.range) {
            self.hit(n);
        }
    }
}
