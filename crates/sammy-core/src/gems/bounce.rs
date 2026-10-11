//! A flying gem's frame: ballistic motion, bounces, getting unstuck, coming to rest.

use super::*;
use crate::time::Ticks;

impl FlyingGem {
    /// One frame (`t` ticks, capped at 1 like the original `mintime`). `trace` sweeps a
    /// point against the level and map entities; sounds go to `sounds`.
    pub fn step(&mut self, collectable: bool, jump: &JumpParams, rng: &mut Rng, t: Ticks, trace: impl Fn([f32; 3], [f32; 3]) -> Trace, sounds: &mut Vec<&'static str>) -> FlightStep {
        let t = t.get();
        let mut out = FlightStep::default();
        let floor = {
            let tr = trace(self.pos, [self.pos[0], self.pos[1], self.pos[2] - FLOOR_PROBE]);
            if tr.hit() { tr.end[2] } else { self.pos[2] - FLOOR_PROBE }
        };
        let thrown = matches!(self.kind, FlightKind::Thrown { .. });
        if self.vz == 0.0 && self.pos[2] - floor > HEIGHT_GEM + 1.0 {
            self.vz = jump.gravity;
            self.fric = jump.acc_gravity;
        }
        if thrown && self.pos[2] > self.ceiling {
            self.vz = jump.gravity;
            self.fric = jump.acc_gravity;
        }
        let mut dz = 0.0;
        if self.vz != 0.0 {
            self.vz -= self.fric * t;
            if self.pos[2] < floor + HEIGHT_GEM {
                // Touchdown = one bounce; collectable from the 5th on.
                self.bounces += 1;
                let decorative = self.kind == FlightKind::Launched(Fate::Decorative);
                if self.bounces == COLLECTABLE_BOUNCE && !decorative && !collectable {
                    out.collectable = true;
                }
                let (min_speed, keep, loud) = if thrown { (2.0, 0.95, 7.0) } else { (0.5, 0.9, 5.0) };
                if self.speed > min_speed {
                    if self.vz_org > loud {
                        sounds.push(BOUNCE_SOUNDS[(self.bounces.clamp(1, 4) - 1) as usize]);
                    }
                    if thrown {
                        self.speed -= 5.0;
                    }
                    self.vz_org *= keep;
                    self.vz = self.vz_org - self.fric * t;
                    dz = self.vz * t;
                } else {
                    dz = floor + HEIGHT_GEM - self.pos[2];
                    self.vz = 0.0;
                }
            } else {
                dz = self.vz * t;
            }
        }
        self.good_timer -= t;
        if self.good_timer <= 0.0 {
            self.good_timer = GOOD_POS_PERIOD;
            self.last_good = self.pos;
        }
        let (s, c) = self.pan.to_radians().sin_cos();
        let target = [self.pos[0] + c * self.speed * t, self.pos[1] + s * self.speed * t, self.pos[2] + dz];
        let hit = trace(self.pos, target);
        let before = self.pos;
        if hit.start_solid {
            self.pos = self.last_good;
            out.done = true;
        } else {
            self.pos = hit.end;
        }
        // Gem_Hit: reflect off walls while not yet collectable.
        if hit.hit() && hit.normal[2].abs() < 0.7 && !collectable {
            let n = hit.normal;
            let d = c * n[0] + s * n[1];
            self.pan = (s - 2.0 * d * n[1]).atan2(c - 2.0 * d * n[0]).to_degrees();
            self.speed = (self.speed - 8.0).max(0.1);
        }
        let moved_z = self.pos[2] - before[2];
        if thrown {
            if dz != 0.0 && moved_z.abs() < 1e-3 {
                if self.pos[2] > floor + HEIGHT_GEM {
                    // Stuck in the air: hop off in a random direction.
                    self.stuck += 1.0;
                    if self.stuck > 24.0 {
                        out.done = true;
                    }
                    sounds.push("ding3");
                    self.speed = 30.0;
                    self.vz = GEM_VSPEED;
                    self.vz_org = GEM_VSPEED;
                    self.pan = rng.random(360.0);
                    // Port: the hop and the snap to rest height are swept too, so a gem
                    // under a low ceiling can't be set inside it (and fall out of the level).
                    self.pos = trace(self.pos, [self.pos[0], self.pos[1], self.pos[2] + STUCK_HOP]).end;
                } else {
                    self.pos = trace(self.pos, [self.pos[0], self.pos[1], floor + HEIGHT_GEM]).end;
                    out.done = true;
                }
            }
            self.speed -= 0.5 * t;
            if self.speed < 0.1 {
                self.speed = 0.1;
                if self.vz == 0.0 {
                    out.done = true;
                }
            }
        } else {
            self.speed -= 0.05 * t;
            let blocked = (self.pos[0] - before[0]).abs() + (self.pos[1] - before[1]).abs() < 1.0;
            self.vz_org -= if blocked { 0.5 } else { 0.02 } * t;
            if blocked && self.vz_org.abs() < jump.acc_gravity.abs() {
                out.done = true;
            }
            if self.speed < 1.0 {
                self.speed = 1.0;
                if self.vz_org.abs() < jump.acc_gravity.abs() {
                    out.done = true;
                }
            }
        }
        out
    }
}
