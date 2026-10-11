//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X at Skill2 quants/tick through hidden
//! rails in the level. On impact it either reflects (Flag3) or turns by Skill4/5/7 and
//! pauses Skill3 ticks. Platforms and bodies with the same link id (Skill1) copy the
//! engine's motion at a fixed offset, carrying whatever stands on them. An engine with
//! Skill6 is locked until Sammy stands on its platform holding a matching key.

use formats::trace::Trace;
use formats::wmb::Object;

use crate::angle::ang;

/// Default link id when Skill1 is 0 (`8465`).
pub const DEFAULT_LINK: f32 = 8465.0;
/// The engine carrying a Shift-boosting rider runs this much faster.
pub const BOOST_FACTOR: f32 = 3.0;
/// The plain wooden platform an unlocked platform turns into.
pub const UNLOCKED_PLATFORM: &str = "platfor1.wmb";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EngineState {
    Moving,
    Paused(f32),
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub link: f32,
    pub pos: [f32; 3],
    angles: [f32; 3],
    /// Skill2: quants per tick along local +X.
    speed: f32,
    /// Skill3: pause after an impact.
    pause: f32,
    /// Skill4/5/7: pan/tilt/roll change on impact.
    turn: [f32; 3],
    /// Flag3: reflect off walls instead of turning.
    reflect: bool,
    /// Skill6: key colour that unlocks it (0 = always running); Flag7 = exact colour.
    lock: u32,
    lock_exact: bool,
    pub unlocked: bool,
    state: EngineState,
    /// Motion this frame.
    pub delta: [f32; 3],
}

fn forward([pan, tilt, _]: [f32; 3]) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

impl Engine {
    pub fn from_object(obj: &Object) -> Self {
        Self {
            link: if obj.skill(1) == 0.0 { DEFAULT_LINK } else { obj.skill(1) },
            pos: obj.origin,
            angles: obj.angles,
            speed: obj.skill(2),
            pause: obj.skill(3),
            turn: [obj.skill(4), obj.skill(5), obj.skill(7)],
            reflect: obj.flag(3),
            lock: obj.skill(6) as u32,
            lock_exact: obj.flag(7),
            unlocked: obj.skill(6) == 0.0,
            state: EngineState::Moving,
            delta: [0.0; 3],
        }
    }

    /// Whether `key` starts a locked engine.
    pub fn unlocks(&self, key: u32) -> bool {
        crate::keys::fits(key, self.lock, self.lock_exact)
    }

    /// One frame; `trace` sweeps a point through the level (the engine is a tiny sprite).
    pub fn step(&mut self, boosted: bool, t: f32, trace: impl Fn([f32; 3], [f32; 3]) -> Trace) {
        let speed = if boosted { self.speed * BOOST_FACTOR } else { self.speed };
        self.delta = [0.0; 3];
        if !self.unlocked {
            return;
        }
        self.state = match self.state {
            EngineState::Paused(left) if left - t > 0.0 => EngineState::Paused(left - t),
            EngineState::Paused(_) => EngineState::Moving,
            EngineState::Moving => {
                let dir = forward(self.angles);
                let target = [0, 1, 2].map(|i| self.pos[i] + dir[i] * speed * t);
                let tr = trace(self.pos, target);
                if tr.start_solid {
                    EngineState::Moving
                } else {
                    self.delta = [0, 1, 2].map(|i| tr.end[i] - self.pos[i]);
                    self.pos = tr.end;
                    if !tr.hit() {
                        EngineState::Moving
                    } else if self.reflect {
                        // `To_angle Angles,BOUNCE`: face along the reflected direction.
                        let n = tr.normal;
                        let d = dir[0] * n[0] + dir[1] * n[1] + dir[2] * n[2];
                        let r = [0, 1, 2].map(|i| dir[i] - 2.0 * d * n[i]);
                        self.angles[0] = r[1].atan2(r[0]).to_degrees();
                        EngineState::Moving
                    } else {
                        // `moteurBonus_Hit`; roll uses the *new tilt* (original bug, mm1.wdl:2591).
                        self.angles[0] = ang(self.angles[0] + self.turn[0]);
                        self.angles[1] = ang(self.angles[1] + self.turn[1]);
                        self.angles[2] = ang(self.angles[1] + self.turn[2]);
                        if self.pause > 0.0 { EngineState::Paused(self.pause) } else { EngineState::Moving }
                    }
                }
            }
        };
    }
}

/// A platform or body following an engine (`SetLinkMoteur`).
#[derive(Clone, Debug)]
pub struct Platform {
    pub link: f32,
    pub offset: [f32; 3],
    /// Motion this frame (carried onto riders).
    pub delta: [f32; 3],
}

impl Platform {
    /// `InitPlatForm` (link 0 = unlinked) / `InitBodyPlatForm` (link 0 = default).
    pub fn from_object(obj: &Object, body: bool) -> Self {
        let link = if obj.skill(1) == 0.0 && body { DEFAULT_LINK } else { obj.skill(1) };
        Self { link, offset: [0.0; 3], delta: [0.0; 3] }
    }

    /// Attaches to an engine: the offset from it is kept from now on.
    pub fn attach(&mut self, origin: [f32; 3], engine: &Engine) {
        self.offset = [0, 1, 2].map(|i| origin[i] - engine.pos[i]);
    }

    /// Follows the engine; `origin` is the platform brush's position.
    pub fn follow(&mut self, origin: &mut [f32; 3], engine: &Engine) {
        let target = [0, 1, 2].map(|i| engine.pos[i] + self.offset[i]);
        self.delta = [0, 1, 2].map(|i| target[i] - origin[i]);
        *origin = target;
    }
}

/// A teleport pad.
#[derive(Clone, Debug)]
pub struct Teleport {
    /// Skill1-3: destination; Skill4: pan on arrival.
    pub dest: [f32; 3],
    pub pan: f32,
    /// Flag2: lions are teleported too.
    pub lions: bool,
    /// `Skill12`: busy while someone is being teleported (shared by Sammy and lions).
    pub busy: bool,
}

impl Teleport {
    pub fn from_object(obj: &Object) -> Self {
        Self { dest: [obj.skill(1), obj.skill(2), obj.skill(3)], pan: obj.skill(4), lions: obj.flag(2), busy: false }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum TeleportStage {
    /// `waitt 4` while Sammy settles onto the pad.
    Settle(f32),
    FadeOut,
    FadeIn,
}

/// Ticks Sammy settles on a pad before fading.
const SETTLE_TICKS: f32 = 4.0;
/// Fade speed in alpha percent per tick.
const FADE_SPEED: f32 = 10.0;
/// After the jump the camera is put this far from the destination (x, y, z).
pub const CAMERA_SNAP: [f32; 3] = [32.0, 32.0, 150.0];

/// Sammy being teleported (input locked).
#[derive(Clone, Debug)]
pub struct Teleporting {
    pub dest: [f32; 3],
    pub pan: f32,
    stage: TeleportStage,
    /// Sammy's opacity (percent).
    pub alpha: f32,
}

/// What a teleport step asks of the frontend.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TeleportStep {
    Fading,
    /// Faded out: move Sammy to `dest`, facing `pan`, and snap the camera.
    Jump,
    /// Faded back in: Sammy can move again and the pad is free.
    Done,
}

impl Teleporting {
    pub fn new(pad: &Teleport) -> Self {
        Self { dest: pad.dest, pan: pad.pan, stage: TeleportStage::Settle(SETTLE_TICKS), alpha: 100.0 }
    }

    pub fn step(&mut self, t: f32) -> TeleportStep {
        let mut out = TeleportStep::Fading;
        self.stage = match self.stage {
            TeleportStage::Settle(left) if left - t > 0.0 => TeleportStage::Settle(left - t),
            TeleportStage::Settle(_) => TeleportStage::FadeOut,
            TeleportStage::FadeOut => {
                self.alpha = (self.alpha - FADE_SPEED * t).max(0.0);
                if self.alpha <= 0.0 {
                    out = TeleportStep::Jump;
                    TeleportStage::FadeIn
                } else {
                    TeleportStage::FadeOut
                }
            }
            TeleportStage::FadeIn => {
                self.alpha = (self.alpha + FADE_SPEED * t).min(100.0);
                if self.alpha >= 100.0 {
                    out = TeleportStep::Done;
                }
                TeleportStage::FadeIn
            }
        };
        out
    }
}
