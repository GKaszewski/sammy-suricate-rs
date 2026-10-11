//! Jumping (`PlayerJump`), falling and landing (`FinJump`).

use super::intent::Intent;
use super::walk::{SlopeFeel, wrap};
use super::{BLOB_HAUT, Frame, HULL, Player, State, Touch};
use crate::anim::Anim;
use crate::collision::{Space, floor_below};
use crate::space::Pos;
use crate::time::TICKS_PER_SEC;

/// Jumping off a big slope is only allowed within this angle of downhill.
const SLOPE_JUMP_ANGLE: f32 = 45.0;
/// Grounded snap tolerance, and a "real" fall that shows the jump animation.
const GROUND_SNAP: f32 = 1.0;
const FALL_SHOWS_JUMP: f32 = 64.0;
/// Falling faster than this lands hard (slow recovery).
const HARD_LANDING: f32 = -70.0;
/// Standing jump wind-up speed (% of the scene per tick).
const ANIM_DEBUT_JUMP: f32 = 30.0;

impl Player {
    /// Step 3: moving jumps take off at once; standing ones wind up first.
    pub(super) fn jump<S: Space>(&mut self, intent: &Intent, slope: SlopeFeel, f: &Frame<S>, mut anim: Option<&mut Anim>) {
        let mut take_off = false;
        let slope_ok = !slope.big || wrap(slope.pan, self.pan.0).abs() <= SLOPE_JUMP_ANGLE;
        if intent.jump_hit && slope_ok && self.grounded && matches!(self.state, State::Stand | State::Walk | State::Turn) {
            if intent.forward != 0.0 {
                take_off = true;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautB", false, 10.0);
                }
            } else {
                self.state = State::JumpStart;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautA", false, 0.0);
                    a.set_speed_pct(ANIM_DEBUT_JUMP, TICKS_PER_SEC);
                }
            }
        }
        if self.state == State::JumpStart && anim.as_deref().is_none_or(|a| a.finished()) {
            take_off = true;
            if let Some(a) = anim {
                a.play("sautB", false, 0.0);
            }
        }
        if take_off {
            self.state = State::Air;
            self.grounded = false;
            self.vz = f.jump.jump_speed;
            self.fric = f.jump.jump_fric;
        }
    }

    /// Step 4: snapped to the floor when grounded, else ballistic. Returns whether
    /// Sammy landed.
    pub(super) fn fall<S: Space>(&mut self, f: &Frame<S>, anim: Option<&mut Anim>, sounds: &mut Vec<&'static str>, t: f32) -> bool {
        let jump = f.jump;
        let floor = floor_below(f.space, self.pos.get());
        if let Some((fl, ..)) = floor {
            self.floor = fl;
        }
        if self.grounded {
            match floor {
                Some((fl, e, _)) if self.pos[2] - BLOB_HAUT - fl <= GROUND_SNAP => {
                    self.pos[2] = fl + BLOB_HAUT;
                    self.ground_entity = e;
                }
                _ => {
                    // Walked off a ledge (`Gravite`, `AccGravite`).
                    self.grounded = false;
                    self.vz = jump.gravity;
                    self.fric = jump.acc_gravity;
                }
            }
        }
        if self.grounded || self.vz == 0.0 {
            return false;
        }
        self.vz -= self.fric * t;
        let target = [self.pos[0], self.pos[1], self.pos[2] + self.vz * t];
        let hit = f.space.trace(HULL, self.pos.get(), target, Some(f.me));
        let tr = hit.trace;
        // Stomp (Player_Bong :584-618): landing on a lion's head re-jumps.
        if self.vz < 0.0 && tr.hit() && jump.bounce_on_enemy != 0.0 && hit.entity.is_some_and(|e| (f.touch)(e) == Touch::Enemy) {
            self.vz = jump.bounce_on_enemy * jump.jump_speed;
            self.fric = jump.jump_fric;
            self.knock = self.knock.map(|k| k * 2.0);
            sounds.push("Blouit_son");
        }
        if tr.start_solid {
            // Stuck inside geometry (e.g. a closing door): never sink through it.
            self.vz = 0.0;
            self.grounded = true;
        } else {
            if tr.hit() && self.vz > 0.0 {
                self.vz = 0.0; // ceiling cancels the ascent
            }
            self.pos = Pos(tr.end);
        }
        let floor = floor_below(f.space, self.pos.get());
        let floor_z = floor.map(|(fl, ..)| fl);
        if !matches!(self.state, State::Air | State::Dead | State::Scripted) && floor_z.is_none_or(|fl| self.pos[2] - fl > BLOB_HAUT + FALL_SHOWS_JUMP) {
            self.state = State::Air;
            if let Some(a) = anim {
                a.play("sautB", false, 50.0);
            }
        }
        if let Some((fl, e, _)) = floor
            && self.vz <= 0.0
            && self.pos[2] <= fl + BLOB_HAUT
        {
            self.hard_landing = self.vz < HARD_LANDING;
            self.pos[2] = fl + BLOB_HAUT;
            self.ground_entity = e;
            self.grounded = true;
            self.vz = 0.0;
            return true;
        }
        false
    }

    /// Step 5 (`FinJump`): "sautC" once on landing, then back to standing.
    pub(super) fn land(&mut self, landed: bool, mut anim: Option<&mut Anim>) {
        if landed && self.state == State::Air {
            self.state = State::Land;
            if let Some(a) = anim.as_deref_mut() {
                a.play("sautC", false, 10.0);
            }
        }
        if self.state == State::Land && anim.as_deref().is_none_or(|a| a.finished()) {
            self.state = State::Stand;
        }
    }
}
