//! Sammy: `InitMarche` (player.wdl:1-260), the per-tick movement of
//! `HandleSpeedPlayer` (player.wdl:1115-1488), `PlayerJump` / `FinJump`
//! (player.wdl:385-520), slopes (`handlePente`), knockback (`Player_Bong`), boost
//! (`HandleBoostPlayer`) and the U-turn (`RetourneSammy`).

use formats::wmb::Object;

use crate::anim::Anim;
use crate::collision::{Hull, Space, floor_below};
use crate::game::GameState;
use crate::id::Id;
use crate::input::Controls;
use crate::physics::JumpParams;

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
pub const MAX_STEP: Ticks = Ticks(4.0);

mod animate;
mod intent;
mod jump;
mod slope;
mod state;
mod tuning;
mod walk;

pub use self::state::{State, Touch};
pub use self::tuning::PlayerTuning;
use self::intent::Intent;
use crate::time::Ticks;
use crate::space::Quants;
use crate::angle::Degrees;

#[derive(Clone, Debug)]
pub struct Player {
    /// Native position (quants).
    pub pos: [f32; 3],
    /// Degrees, counter-clockwise from +X.
    pub pan: Degrees,
    /// Walk speed in quants/tick (Skill2 `SpeedBalle`).
    pub speed: f32,
    /// `BlobVspeed` (quants/tick) and `BlobFricVspeed`.
    pub vz: f32,
    pub fric: f32,
    pub grounded: bool,
    /// Entity Sammy stands on (None = level geometry).
    pub ground_entity: Option<Id>,
    /// `BlobZoneScan` (Skill5): pickup scan radius.
    pub scan_radius: Quants,
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
    pub t: Ticks,
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
        Self::at(obj.origin, obj.angles[0], speed, Quants(scan_radius))
    }

    pub fn at(pos: [f32; 3], pan: f32, speed: f32, scan_radius: Quants) -> Self {
        Self {
            pos,
            pan: Degrees(pan),
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
        self.pan = Degrees(pan);
        self.vz = 0.0;
        self.grounded = true;
        self.snapped = false;
    }

    /// Model orientation: pan plus the slope lean (upright while dancing).
    pub fn angles(&self) -> [f32; 3] {
        if self.state == State::Win { [self.pan.0, 0.0, 0.0] } else { [self.pan.0, self.tilt, self.roll] }
    }

    /// One frame of `HandleSpeedPlayer`.
    pub fn step<S: Space>(&mut self, game: &mut GameState, mut anim: Option<&mut Anim>, f: &Frame<S>, out: &mut StepOut) {
        let t = f.t.min(MAX_STEP).get();
        let start_pos = self.pos;
        self.snap_once(f.space);
        let mut intent = Intent::read(f, self.state);
        if f.won && self.grounded {
            return self.dance(anim, t);
        }
        self.boost(game, f, &mut intent, t);
        self.u_turn(f, anim.as_deref_mut(), &mut out.sounds, t);
        intent.restrict_to(self.state);
        let slope = self.walk(&intent, f, out, t);
        self.slide(f, t);
        self.knockback(f, t);
        self.jump(&intent, slope, f, anim.as_deref_mut());
        let landed = self.fall(f, anim.as_deref_mut(), &mut out.sounds, t);
        self.land(landed, anim.as_deref_mut());
        self.animate(&intent, anim);
        if t > 0.0 {
            self.vel = [0, 1, 2].map(|i| (self.pos[i] - start_pos[i]) / t);
        }
    }

    /// After a spawn or teleport, Sammy stands on the floor below him.
    fn snap_once<S: Space>(&mut self, space: &S) {
        if !self.snapped {
            if let Some((fl, ..)) = floor_below(space, self.pos) {
                self.pos[2] = fl + BLOB_HAUT;
            }
            self.snapped = true;
        }
    }
}
