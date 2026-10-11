//! Sammy: `InitMarche` (player.wdl:1-260), the per-tick movement of
//! `HandleSpeedPlayer` (player.wdl:1115-1488), `PlayerJump` / `FinJump`
//! (player.wdl:385-520), slopes (`handlePente`), knockback (`Player_Bong`), boost
//! (`HandleBoostPlayer`) and the U-turn (`RetourneSammy`).

use formats::wmb::Object;

use crate::angle::ang;
use crate::anim::Anim;
use crate::collision::{Hull, Space, floor_below, slide_move};
use crate::game::{BOOST_MAX, GameState};
use crate::id::Id;
use crate::input::Controls;
use crate::physics::JumpParams;
use crate::time::TICKS_PER_SEC;

/// `heightFootPlayer` (mm1.wdl:36) × model scale 8: origin height above the floor.
pub const BLOB_HAUT: f32 = 4.4 * 8.0;
/// Model scale forced by `InitMarche` (player.wdl:34-36).
pub const SCALE: f32 = 8.0;
/// Sammy collides with the "fat" hull (`my.fat = 1`).
pub const HULL: Hull = Hull::Fat;
/// Skill2 `SpeedBalle` default: walk speed in quants per tick.
pub const DEFAULT_SPEED: f32 = 25.0;
/// Skill5 `BlobZoneScan` default: pickup scan radius.
pub const DEFAULT_SCAN_RADIUS: f32 = 104.0;
/// A frame advances at most this many ticks (A4's `time_smooth` clamp).
pub const MAX_STEP: f32 = 4.0;

// Animation speeds in % of the scene per tick (mm1.wdl:39-85, anims.wdl).
const ANIM_STAND: f32 = 2.0;
/// Walking: × forward speed (negative plays backwards).
const ANIM_WALK: f32 = 0.5;
const ANIM_TURN: f32 = 14.0;
const ANIM_DEBUT_JUMP: f32 = 30.0;
const ANIM_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP_FAST: f32 = 18.0;
const ANIM_WIN: f32 = 3.0;
const ANIM_UTURN: f32 = 28.0;
/// The win dance spins this many degrees per tick.
const WIN_SPIN: f32 = 12.0;

/// Slopes: medium ones damp turning, big ones (tuning) strafe and steer downhill.
const MEDIUM_SLOPE: f32 = 2.0;
const SPEED_ROTATION_PENTE: f32 = 0.05;
/// A slide only starts within this much above the normal standing height.
const SLOPE_REACH: f32 = 32.0;
/// Downhill pull on a slope.
const SLOPE_DOWN_FORCE: f32 = -20.0;
/// Leaning into a slope: only for real slides, limited, easing 10 % per tick.
const LEAN_MIN_SLIDE: f32 = 4.0;
const LEAN_MAX_TILT: f32 = 12.0;
const LEAN_MAX_ROLL: f32 = 6.0;
const LEAN_EASE: f32 = 0.1;
const LEAN_RELAX: f32 = 0.6;
/// Slide components below this stop.
const SLIDE_EPSILON: f32 = 0.01;
/// Knockback decay: `AmortiBouncePlayer` × 80 / (fps + 1) at the reference 50 fps.
const KNOCK_DECAY_SCALE: f32 = 80.0 / 51.0;
/// Jumping off a big slope is only allowed within this angle of downhill.
const SLOPE_JUMP_ANGLE: f32 = 45.0;
/// Air control (forward speed factor).
const AIR_CONTROL: f32 = 0.75;
/// Turning: half rate in the air, ×0.6 while boosting.
const AIR_TURN: f32 = 0.5;
const BOOST_TURN: f32 = 0.6;
/// Boost: the engine doubles `Key_Force`; the script keeps turning ×0.75 (net ×1.5).
const BOOST_TURN_FORCE: f32 = 1.5;
const BOOST_FORWARD_FORCE: f32 = 2.0;
/// U-turn easing toward +180: first step ×0.25, then ×0.4, done below 2°.
const UTURN_FIRST: f32 = 0.25;
const UTURN_NEXT: f32 = 0.4;
const UTURN_DONE: f32 = 2.0;
/// Grounded snap tolerance, and a "real" fall that shows the jump animation.
const GROUND_SNAP: f32 = 1.0;
const FALL_SHOWS_JUMP: f32 = 64.0;
/// Falling faster than this lands hard (slow recovery).
const HARD_LANDING: f32 = -70.0;

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

/// `PlayerEtat` / `PlayerAnimEtat`, reduced to what is ported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Stand,
    Walk,
    Turn,
    /// Standing wind-up ("sautA") before leaving the ground.
    JumpStart,
    /// In the air ("sautB", played once).
    Air,
    /// Landing ("sautC", played once).
    Land,
    /// Level won: "Iwin" while spinning (`AnimGagnee`).
    Win,
    /// Dying / respawning (`PlayerEtat == 3`): no input, animation owned by health.
    Dead,
    /// Throwing a wasp: "TirA" wind-up, then "TirB" follow-through (`PlayerEtat == 2`).
    FireA,
    FireB,
    /// Extro `SetAnimPlayer` (`PlayerEtat` 2/3): no input, cannot die, animation scripted.
    Scripted,
    /// Quick 180° turn (`RetourneSammy`, `PlayerEtat == 6`).
    UTurn,
}

/// What Sammy bumped into (decides the knockback).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Touch {
    /// Key gems: no reaction here.
    Gem,
    /// Lions and wasps: handled by the enemy logic.
    Enemy,
    /// Platforms and hives: cancel the knockback.
    Pusher,
    /// Anything else knocks Sammy back.
    Other,
}

#[derive(Clone, Debug)]
pub struct Player {
    /// Native position (quants).
    pub pos: [f32; 3],
    /// Degrees, counter-clockwise from +X.
    pub pan: f32,
    /// Walk speed in quants/tick (Skill2 `SpeedBalle`).
    pub speed: f32,
    /// `BlobVspeed` (quants/tick) and `BlobFricVspeed`.
    pub vz: f32,
    pub fric: f32,
    pub grounded: bool,
    /// Entity Sammy stands on (None = level geometry).
    pub ground_entity: Option<Id>,
    /// `BlobZoneScan` (Skill5): pickup scan radius.
    pub scan_radius: f32,
    /// `BounceForce1`: knockback velocity (quants/tick), decays each tick.
    pub knock: [f32; 3],
    /// Last frame's movement in quants/tick (`PlayerSpeed`), what others get pushed by.
    pub vel: [f32; 3],
    /// Floor height below Sammy (`BlobHautSol`), for his shadow.
    pub floor: f32,
    /// `PlayerPenteX/Y/Z`: slide velocity on slopes (quants/tick).
    pub pente: [f32; 3],
    /// Lean on slopes (visual tilt/roll, degrees).
    tilt: f32,
    roll: f32,
    /// U-turn target pan and whether the first step was taken.
    uturn: Option<(f32, bool)>,
    /// Landing after a fall faster than 70/tick (`ForceChute < -70`).
    pub hard_landing: bool,
    /// Shift-boosting this frame (platforms run faster under him).
    pub boosting: bool,
    pub state: State,
    snapped: bool,
}

/// What Sammy's surroundings and the frame look like.
pub struct Frame<'a, S: Space> {
    pub space: &'a S,
    pub controls: &'a Controls,
    /// Input counts (`PartieEncours == 1`).
    pub input: bool,
    /// The level is won (the win dance starts once grounded).
    pub won: bool,
    pub jump: &'a JumpParams,
    pub tuning: &'a PlayerTuning,
    /// Sammy's own id (ignored by his traces).
    pub me: Id,
    /// Classifies an entity he bumped into.
    pub touch: &'a dyn Fn(Id) -> Touch,
    /// Ticks this frame (clamped to [`MAX_STEP`]).
    pub t: f32,
}

/// What a step produced besides the new state.
#[derive(Default, Debug)]
pub struct StepOut {
    /// Entity Sammy walked into (EVENT_ENTITY / EVENT_IMPACT).
    pub touched: Option<Id>,
    pub sounds: Vec<&'static str>,
}

impl Player {
    /// `InitMarche`: Skill2 walk speed, Skill5 scan radius.
    pub fn from_object(obj: &Object) -> Self {
        let speed = Some(obj.skill(2)).filter(|&s| s > 0.0).unwrap_or(DEFAULT_SPEED);
        let scan_radius = Some(obj.skill(5)).filter(|&s| s > 0.0).unwrap_or(DEFAULT_SCAN_RADIUS);
        Self::at(obj.origin, obj.angles[0], speed, scan_radius)
    }

    pub fn at(pos: [f32; 3], pan: f32, speed: f32, scan_radius: f32) -> Self {
        Self {
            pos,
            pan,
            speed,
            vz: 0.0,
            fric: 0.0,
            grounded: true,
            ground_entity: None,
            scan_radius,
            knock: [0.0; 3],
            vel: [0.0; 3],
            floor: pos[2],
            pente: [0.0; 3],
            tilt: 0.0,
            roll: 0.0,
            uturn: None,
            hard_landing: false,
            boosting: false,
            state: State::Stand,
            snapped: false,
        }
    }

    /// Moves Sammy to a respawn point; he is re-snapped to the floor next frame.
    pub fn teleport(&mut self, pos: [f32; 3], pan: f32) {
        self.pos = pos;
        self.pan = pan;
        self.vz = 0.0;
        self.grounded = true;
        self.snapped = false;
    }

    /// Model orientation: pan plus the slope lean (upright while dancing).
    pub fn angles(&self) -> [f32; 3] {
        if self.state == State::Win { [self.pan, 0.0, 0.0] } else { [self.pan, self.tilt, self.roll] }
    }

    /// One frame of `HandleSpeedPlayer`.
    pub fn step<S: Space>(&mut self, game: &mut GameState, mut anim: Option<&mut Anim>, f: &Frame<S>, out: &mut StepOut) {
        let t = f.t.min(MAX_STEP);
        let (space, tuning, jump) = (f.space, f.tuning, f.jump);
        let me = Some(f.me);
        let start_pos = self.pos;
        if !self.snapped {
            if let Some((fl, ..)) = floor_below(space, self.pos) {
                self.pos[2] = fl + BLOB_HAUT;
            }
            self.snapped = true;
        }

        // Intention: x = right(+)/left(-), y = forward(+)/back(-) (Key_Force).
        let (mut ix, mut iy) = (f.controls.turn, f.controls.forward);
        let (mut ctrl, mut ctrl_hit) = (f.controls.jump, f.controls.jump_hit);
        if !f.input || self.state == State::Dead {
            (ix, iy, ctrl, ctrl_hit) = (0.0, 0.0, false, false);
        }
        if f.won && self.grounded {
            // AnimGagnee: "Iwin" while spinning.
            if self.state != State::Win {
                self.state = State::Win;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("Iwin", true, 0.0);
                    a.set_speed_pct(ANIM_WIN, TICKS_PER_SEC);
                }
            }
            self.pan += WIN_SPIN * t;
            return;
        }

        // Boost (HandleBoostPlayer).
        self.boosting = false;
        if f.controls.boost && f.input && game.boost < BOOST_MAX && self.state != State::Dead {
            game.boost = (game.boost + t).min(BOOST_MAX); // LessBoost: ConsoBoost 1/tick
            ix *= BOOST_TURN_FORCE;
            iy *= BOOST_FORWARD_FORCE;
            self.boosting = true;
        }

        // U-turn (RetourneSammy): in a normal state, pan eases toward +180.
        if f.controls.uturn_hit && f.input && self.grounded && matches!(self.state, State::Stand | State::Walk | State::Turn) {
            self.state = State::UTurn;
            self.uturn = Some((self.pan + 180.0, false));
            out.sounds.push("Blouit_son");
            if let Some(a) = anim.as_deref_mut() {
                a.play("turn", true, 0.0);
                a.set_speed_pct(ANIM_UTURN, TICKS_PER_SEC);
            }
        }
        if let (State::UTurn, Some((dest, started))) = (self.state, self.uturn) {
            let step = ang((dest - self.pan) * if started { UTURN_NEXT } else { UTURN_FIRST });
            if step > UTURN_DONE {
                self.pan += step * t;
                self.uturn = Some((dest, true));
            } else {
                self.uturn = None;
                self.state = State::Stand;
            }
        }

        // Turning and moving only in the normal and jump states (etat 1 / 4).
        if !matches!(self.state, State::Stand | State::Walk | State::Turn | State::Air | State::Land) {
            ix = 0.0;
            iy = 0.0;
        }
        // 1. Turning.
        let mut rate = if iy != 0.0 { if self.boosting { BOOST_TURN * tuning.turn_move } else { tuning.turn_move } } else { tuning.turn_stand };
        if !self.grounded {
            rate *= AIR_TURN;
        }
        let mut angle_force = ix * rate * t;

        // 2. Forward speed; slope steering.
        let mut v = if self.grounded { iy * self.speed * t } else { iy * self.speed * AIR_CONTROL * t };
        let mut strafe = 0.0;
        let big = self.pente[0].abs() > tuning.big_slope || self.pente[1].abs() > tuning.big_slope;
        let medium = self.pente[0].abs() > MEDIUM_SLOPE || self.pente[1].abs() > MEDIUM_SLOPE;
        let slope_pan = self.pente[1].atan2(self.pente[0]).to_degrees();
        if big {
            v *= 0.75;
            strafe = -ix * self.speed * 0.75 * t;
            angle_force = 0.0;
            let wrap = (slope_pan - self.pan + 180.0).rem_euclid(360.0) - 180.0;
            self.pan += wrap * SPEED_ROTATION_PENTE * t;
        } else if medium {
            v *= 0.9;
            strafe = -ix * self.speed * 0.5 * t;
            angle_force *= 0.25;
        }
        self.pan -= angle_force;
        let (s, c) = self.pan.to_radians().sin_cos();
        let (pos, touched) = slide_move(space, HULL, self.pos, [c * v - s * strafe, s * v + c * strafe, 0.0], me);
        self.pos = pos;
        if let Some((e, n)) = touched {
            out.touched = Some(e);
            // Player_Bong (player.wdl:520-697): bumping an entity cancels the slope slide;
            // others knock Sammy back along the contact normal, pushers cancel it.
            match (f.touch)(e) {
                Touch::Gem | Touch::Enemy => {}
                Touch::Pusher => {
                    self.pente = [0.0; 3];
                    self.knock = [0.0; 3];
                }
                Touch::Other => {
                    self.pente = [0.0; 3];
                    self.knock = [n[0] * tuning.bounce, n[1] * tuning.bounce, 0.0];
                }
            }
        }

        // handlePente: on floors steeper than ~23° Sammy slides downhill and leans.
        if let Some((fl, _, n)) = floor_below(space, self.pos)
            && n[2] > 0.0
            && n[2] <= tuning.slope_angle_min
            && self.pos[2] - fl <= BLOB_HAUT + SLOPE_REACH
        {
            let force = [n[0] * tuning.force_pente, n[1] * tuning.force_pente, SLOPE_DOWN_FORCE];
            let v2 = 1.0 - n[2];
            for i in 0..3 {
                self.pente[i] += tuning.acc_pente * force[i].signum() * v2;
                if self.pente[i].abs() > force[i].abs() {
                    self.pente[i] = force[i];
                }
            }
            if self.pente[0].abs() > LEAN_MIN_SLIDE || self.pente[1].abs() > LEAN_MIN_SLIDE {
                let (sp, cp) = (-self.pan).to_radians().sin_cos();
                let local = [n[0] * cp - n[1] * sp, n[0] * sp + n[1] * cp];
                let want_tilt = -local[0].asin().to_degrees();
                let want_roll = -local[1].asin().to_degrees();
                if self.tilt.abs() < LEAN_MAX_TILT {
                    self.tilt += LEAN_EASE * (want_tilt - self.tilt) * t;
                }
                if self.roll.abs() < LEAN_MAX_ROLL {
                    self.roll += LEAN_EASE * (want_roll - self.roll) * t;
                }
            }
            let d = self.pente.map(|x| x * t);
            self.pos = slide_move(space, HULL, self.pos, d, me).0;
        } else {
            self.tilt -= LEAN_RELAX * self.tilt * t.min(1.0);
            self.roll -= LEAN_RELAX * self.roll * t.min(1.0);
            let k = (1.0 - tuning.amorti_pente_end * t).max(0.0);
            self.pente = self.pente.map(|x| if x.abs() < SLIDE_EPSILON { 0.0 } else { x * k });
            if self.pente != [0.0; 3] {
                let d = self.pente.map(|x| x * t);
                self.pos = slide_move(space, HULL, self.pos, [d[0], d[1], 0.0], me).0;
            }
        }
        // Knockback (`BounceForce1`), a separate move, decaying by AmortiBouncePlayer.
        if self.knock != [0.0; 3] {
            let k = self.knock.map(|v| v * t);
            self.pos = slide_move(space, HULL, self.pos, [k[0], k[1], 0.0], me).0;
            let dec = tuning.amorti_bounce * KNOCK_DECAY_SCALE * t;
            self.knock = self.knock.map(|v| if v.abs() <= dec { 0.0 } else { v - dec * v.signum() });
        }

        // 3. Jump (PlayerJump): moving jumps take off at once; standing ones wind up.
        let mut take_off = false;
        let slope_ok = !big || ((slope_pan - self.pan + 180.0).rem_euclid(360.0) - 180.0).abs() <= SLOPE_JUMP_ANGLE;
        if ctrl_hit && slope_ok && self.grounded && matches!(self.state, State::Stand | State::Walk | State::Turn) {
            if iy != 0.0 {
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
            if let Some(a) = anim.as_deref_mut() {
                a.play("sautB", false, 0.0);
            }
        }
        if take_off {
            self.state = State::Air;
            self.grounded = false;
            self.vz = jump.jump_speed;
            self.fric = jump.jump_fric;
        }

        // 4. Vertical motion: snap to the floor when grounded, else ballistic.
        let floor = floor_below(space, self.pos);
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
        let mut landed = false;
        if !self.grounded && self.vz != 0.0 {
            self.vz -= self.fric * t;
            let target = [self.pos[0], self.pos[1], self.pos[2] + self.vz * t];
            let hit = space.trace(HULL, self.pos, target, me);
            let tr = hit.trace;
            // Stomp (Player_Bong :584-618): landing on a lion's head re-jumps.
            if self.vz < 0.0 && tr.hit() && jump.bounce_on_enemy != 0.0 && hit.entity.is_some_and(|e| (f.touch)(e) == Touch::Enemy) {
                self.vz = jump.bounce_on_enemy * jump.jump_speed;
                self.fric = jump.jump_fric;
                self.knock = self.knock.map(|k| k * 2.0);
                out.sounds.push("Blouit_son");
            }
            if tr.start_solid {
                // Stuck inside geometry (e.g. a closing door): never sink through it.
                self.vz = 0.0;
                self.grounded = true;
            } else {
                if tr.hit() && self.vz > 0.0 {
                    self.vz = 0.0; // ceiling cancels the ascent
                }
                self.pos = tr.end;
            }
            let floor = floor_below(space, self.pos);
            let floor_z = floor.map(|(fl, ..)| fl);
            if !matches!(self.state, State::Air | State::Dead | State::Scripted) && floor_z.is_none_or(|fl| self.pos[2] - fl > BLOB_HAUT + FALL_SHOWS_JUMP) {
                self.state = State::Air;
                if let Some(a) = anim.as_deref_mut() {
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
                landed = true;
            }
        }

        // 5. Landing (FinJump): "sautC" once, faster when the player is already steering.
        if landed && self.state == State::Air {
            self.state = State::Land;
            if let Some(a) = anim.as_deref_mut() {
                a.play("sautC", false, 10.0);
            }
        }
        if self.state == State::Land && anim.as_deref().is_none_or(|a| a.finished()) {
            self.state = State::Stand;
        }

        // 6. Ground animation states (anims.wdl:352-497).
        if matches!(self.state, State::Stand | State::Walk | State::Turn) {
            let next = if iy != 0.0 {
                State::Walk
            } else if ix != 0.0 {
                State::Turn
            } else {
                State::Stand
            };
            if let Some(a) = anim.as_deref_mut() {
                let scene = match next {
                    State::Walk => "walkB",
                    State::Turn => "turn",
                    _ => "stand",
                };
                if next != self.state || a.frames.is_empty() || !a.looping {
                    a.play(scene, true, 0.0);
                }
                let pct = match next {
                    State::Walk => ANIM_WALK * self.speed * iy,
                    State::Turn => ANIM_TURN,
                    _ => ANIM_STAND,
                };
                a.set_speed_pct(pct, TICKS_PER_SEC);
            }
            self.state = next;
        }
        if let Some(a) = anim.as_deref_mut() {
            match self.state {
                State::Air => a.set_speed_pct(ANIM_JUMP, TICKS_PER_SEC),
                State::Land => {
                    let pct = if self.hard_landing {
                        ANIM_FIN_JUMP * 0.5
                    } else if iy != 0.0 || ctrl {
                        ANIM_FIN_JUMP_FAST * 2.0
                    } else if ix != 0.0 {
                        ANIM_FIN_JUMP_FAST
                    } else {
                        ANIM_FIN_JUMP
                    };
                    a.set_speed_pct(pct, TICKS_PER_SEC);
                }
                _ => {}
            }
        }

        if t > 0.0 {
            self.vel = [0, 1, 2].map(|i| (self.pos[i] - start_pos[i]) / t);
        }
    }
}
