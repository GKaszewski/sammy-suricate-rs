//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): a scripted Sammy walks
//! the path-node loop of the menu scene while the A5, publisher and Suricate logos fade
//! in front of the camera; at node key 999 (or a skip key) the camera glides to the
//! menu pose.

use formats::wmb::Object;

use crate::angle::ang;

/// `DisplayLogoA5` (Intro2.wdl:924-1181): one entity morphed through three logos.
#[derive(Clone, Debug, Default)]
pub struct Logo {
    step: usize,
    /// Ticks spent in the current step.
    t: f32,
    /// `Skill2`: distance ahead of the camera; `Skill6`: fade counter.
    pub dist: f32,
    s6: f32,
    pub alpha: f32,
    /// 0 none, 1 A5 sprite, 2 publisher, 3 Suricate.
    pub shown: u8,
}

/// The scripted Sammy (`InitPlayerPatrol` :231-329, `MoveSammyonPath` :412-456).
#[derive(Clone, Debug)]
pub struct Walker {
    pub pos: [f32; 3],
    pub pan: f32,
    pub vel: [f32; 3],
    dest_pan: f32,
    /// Skill2 `SpeedBalle` (15) and Skill5 `BlobZoneScan` (600).
    speed: f32,
    range: f32,
    /// `PlayerPatrolEtat` and the scanned node (x, y, pan).
    etat: u8,
    target: Option<[f32; 3]>,
    /// `PlayerKey` of the last node scanned.
    pub key: f32,
    pub walking: bool,
}

/// `InitPathPlayer` (:392-398): invisible node, Skill4 = key, pan = way to the next.
#[derive(Clone, Debug)]
pub struct PathNode {
    pos: [f32; 3],
    pan: f32,
    key: f32,
}

/// Walk speed (Skill2 `SpeedBalle`) and scan range (Skill5 `BlobZoneScan`) defaults.
const WALKER_SPEED: f32 = 15.0;
const WALKER_RANGE: f32 = 600.0;

impl Walker {
    pub fn from_object(obj: &Object) -> Self {
        Self {
            pos: obj.origin,
            pan: obj.angles[0],
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
fn scan(nodes: &[PathNode], from: [f32; 3], pan: f32, half: f32, range: f32) -> Option<&PathNode> {
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
    fn hit(&mut self, n: &PathNode) {
        self.target = Some([n.pos[0], n.pos[1], n.pan]);
        self.etat = 1;
        self.key = n.key;
    }

    /// `MoveSammyonPath` over `t` ticks (per frame, rates × `time` as in the script).
    pub fn step(&mut self, nodes: &[PathNode], t: f32) {
        // PlayerPatrol.
        match self.target {
            None => {
                if let Some(n) = scan(nodes, self.pos, self.pan, 30.0, self.range) {
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
        self.pan += ang(self.dest_pan - self.pan) * (0.25 * t).min(1.0);
        let a = self.pan.to_radians();
        self.vel = [a.cos() * self.speed, a.sin() * self.speed, 0.0];
        self.pos[0] += self.vel[0] * t;
        self.pos[1] += self.vel[1] * t;
        // The per-tick path scan (cone 60, range BlobZoneScan, id 30).
        if let Some(n) = scan(nodes, self.pos, self.pan, 30.0, self.range) {
            self.hit(n);
        }
    }
}

/// One tick of `DisplayLogoA5` (docs/menu.md §2.2). The logo also drives the fog: the
/// intro loop's own fog changes race with it in the original [U]; the logo's are kept.
/// Returns the new fog (percent) when the logo sets it.
pub fn logo_tick(l: &mut Logo, fog: &mut f32) {
    l.t += 1.0;
    let next = |l: &mut Logo| {
        l.step += 1;
        l.t = 0.0;
    };
    match l.step {
        // Morph through the three files (1 tick each), fog 90.
        0 => {
            (l.dist, l.s6, l.alpha, l.shown) = (50.0, 100.0, 0.0, 1);
            *fog = 90.0;
            if l.t >= 3.0 {
                next(l);
            }
        }
        // A5 fade-in: Skill6 100 → 3 by 4, slides out to 285.
        1 => {
            l.s6 -= 4.0;
            l.dist = (l.dist + 10.0).min(285.0);
            l.alpha = 100.0 - l.s6;
            *fog = 90.0 - l.alpha * 0.25;
            if l.s6 <= 3.0 {
                next(l);
            }
        }
        // Hold 24.
        2 => {
            l.alpha = 100.0;
            l.dist = (l.dist + 5.0).min(285.0);
            *fog -= *fog * 0.05;
            if l.t >= 24.0 {
                l.s6 = 31.0;
                next(l);
            }
        }
        // Fade-out 30.
        3 => {
            l.alpha = l.s6 * 3.125;
            l.s6 -= 1.0;
            *fog -= *fog * 0.25;
            if l.s6 < 1.0 {
                next(l);
            }
        }
        // Gap 8, invisible, fog off.
        4 => {
            (l.alpha, l.shown) = (0.0, 0);
            *fog = 0.0;
            if l.t >= 8.0 {
                l.shown = 2;
                next(l);
            }
        }
        // Publisher: in +10, hold 32 + 24, out -20.
        5 => {
            l.alpha = (l.alpha + 10.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        6 => {
            if l.t >= 56.0 {
                next(l);
            }
        }
        7 => {
            l.alpha -= 20.0;
            if l.alpha <= 4.0 || l.t >= 16.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        // Suricate: gap 32, in +5, hold 64 + 24, out -15, removed.
        8 => {
            if l.t >= 32.0 {
                l.shown = 3;
                next(l);
            }
        }
        9 => {
            l.alpha = (l.alpha + 5.0).min(100.0);
            if l.alpha >= 100.0 || l.t >= 64.0 {
                next(l);
            }
        }
        10 => {
            if l.t >= 88.0 {
                next(l);
            }
        }
        11 => {
            l.alpha -= 15.0;
            if l.alpha <= 0.0 || l.t >= 32.0 {
                (l.alpha, l.shown) = (0.0, 0);
                next(l);
            }
        }
        _ => {}
    }
}

/// `MoveMenu` (interfac.wdl:625-653): position and angles ease 0.2 per tick toward the
/// menu pose; true once within 2 quants in X and Z (then snap exactly).
pub fn glide(pos: &mut [f32; 3], pan: &mut f32, tilt: &mut f32, to: ([f32; 3], f32, f32), t: f32) -> bool {
    let k = (GLIDE_RATE * t).min(1.0);
    for (p, to) in pos.iter_mut().zip(to.0) {
        *p += (to - *p) * k;
    }
    *pan += ang(to.1 - *pan) * k;
    *tilt += (to.2 - *tilt) * k;
    if (pos[0] - to.0[0]).abs() < GLIDE_DONE && (pos[2] - to.0[2]).abs() < GLIDE_DONE {
        (*pos, *pan, *tilt) = to;
        return true;
    }
    false
}

const GLIDE_RATE: f32 = 0.2;
const GLIDE_DONE: f32 = 2.0;
/// While gliding the fog clears by this much per tick.
pub const GLIDE_FOG_CLEAR: f32 = 5.0;
/// The intro ends at the first node with this key.
pub const END_KEY: f32 = 999.0;
/// `CameraPosOrigine` / `CameraAngleOrigine` of the menu (`InitGame2`).
pub const MENU_ORIGIN: ([f32; 3], f32, f32) = ([1.0, -369.0, 115.0], 90.0, 1.0);
/// `Camera_PlusTilt` during the intro.
pub const PLUS_TILT: f32 = 50.0;
/// After the first 12 ticks the follow frictions become 0.1 / 0.5.
pub const FRICTION_AT: f32 = 12.0;
pub const FRICTION: (f32, f32) = (0.1, 0.5);
