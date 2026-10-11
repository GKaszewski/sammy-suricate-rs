//! Extro level (`Extro.wdl`, docs/level-objects.md §2.14): the `pov_Syn` camera target,
//! id-25 scanners other than Sammy, and the scripted triggers `InitSetPovSyn`,
//! `InitSetAnimPlayer` and `InitSetFinExtro` (mm1.wdl:5395-5562).

use formats::wmb::Object;

use crate::anim::Anim;
use crate::id::Id;
use crate::levels::LevelSlot;
use crate::time::Ticks;
use crate::time::TICKS_PER_SEC;

/// `IdPlayer` (Sammy's Skill1 in scans).
pub const ID_PLAYER: f32 = 3333.0;
/// `PlayerAnimSpeedLoopjambes` (mm1.wdl:52).
const ANIM_SPEED_LOOPJAMBES: f32 = 2.0;
/// `RangeScanDoor`: radius of the id-25 scans.
pub const RANGE_SCAN_25: f32 = 500.0;
/// `StartExtro` (Extro.wdl:155-159): after `waitt 2` the camera follows this wasp.
pub const START_POV_DELAY: f32 = 2.0;
/// `SetFinExtro`: fog grows 6 % per tick; at 100 the extro ends.
pub const FADE_SPEED: f32 = 6.0;
/// After the extro "Play" resumes this level (Extro.wdl:114).
pub const RESUME_LEVEL: LevelSlot = LevelSlot(20);
/// The "salut" scene (SetAnimPlayer Skill3 = 12) waves once; others loop "loopjambes".
const SALUT: f32 = 12.0;

/// What `pov_Syn` is this frame: (entity, position, pan, velocity).
#[derive(Clone, Copy, Debug, Default)]
pub struct Pov {
    /// Enemy to follow; `None` = Sammy.
    pub target: Option<Id>,
    /// Resolved this frame.
    pub entity: Option<Id>,
    pub pos: [f32; 3],
    pub pan: f32,
    pub vel: [f32; 3],
}

/// A candidate camera target: (entity, position, pan, velocity).
pub type Body = (Id, [f32; 3], f32, [f32; 3]);

impl Pov {
    /// Resolves the target: the chosen enemy while it exists, else Sammy, else the
    /// intro's walker (only when there is no Sammy).
    pub fn resolve(&mut self, sammy: Option<Body>, walker: Option<Body>, target: Option<Body>) {
        if sammy.is_none()
            && let Some((e, pos, pan, vel)) = walker
        {
            (self.entity, self.pos, self.pan, self.vel) = (Some(e), pos, pan, vel);
        }
        if let Some((e, pos, pan, vel)) = target.or(sammy) {
            (self.entity, self.pos, self.pan, self.vel) = (Some(e), pos, pan, vel);
        }
    }
}

/// Positions doing an id-25 scan this frame (Sammy, plus Flag5 lions and wasp enemies):
/// they trigger `SetCamera` zones and `InitTitle` signs.
#[derive(Clone, Debug, Default)]
pub struct Scanners(pub Vec<[f32; 3]>);

impl Scanners {
    /// Nearest scanner and its distance to `p`.
    pub fn nearest(&self, p: [f32; 3]) -> Option<([f32; 3], f32)> {
        self.0.iter().map(|s| (*s, (0..3).map(|i| (s[i] - p[i]).powi(2)).sum::<f32>().sqrt())).min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriggerKind {
    /// `SetPovSyn`: the camera cuts to the entity found.
    PovSyn,
    /// `SetAnimPlayer`: Sammy plays a scripted animation.
    AnimPlayer,
    /// `SetFinExtro`: fade to black and end.
    FinExtro,
}

/// One of the extro's invisible sphere-scan triggers (all shipped ones are one-shot).
#[derive(Clone, Debug)]
pub struct Trigger {
    pub kind: TriggerKind,
    pub pos: [f32; 3],
    /// Skill1 id to find, Skill2 radius, Skill3 subtype / scene, Skill5 speed, Skill6 step.
    s: [f32; 8],
    timer: Ticks,
    pub done: bool,
}

/// What a trigger looks for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Wanted {
    Sammy,
    /// Enemy of this class (and spec, if given).
    Enemy { class: f32, spec: Option<f32> },
}

/// Sammy's scripted animation (`SetAnimPlayer`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ScriptedScene {
    pub salut: bool,
    pub step: f32,
    pub speed: f32,
}

impl Trigger {
    pub fn from_object(kind: TriggerKind, obj: &Object) -> Self {
        let mut s = [0.0; 8];
        s.copy_from_slice(&obj.skills[..8]);
        Self { kind, pos: obj.origin, s, timer: Ticks::ZERO, done: false }
    }

    /// One tick: whether it scans now (SetPovSyn / SetAnimPlayer every 2 ticks,
    /// SetFinExtro every 4).
    pub fn scans(&mut self) -> bool {
        if self.done {
            return false;
        }
        self.timer -= Ticks(1.0);
        if self.timer > Ticks::ZERO {
            return false;
        }
        self.timer = if self.kind == TriggerKind::FinExtro { Ticks(4.0) } else { Ticks(2.0) };
        true
    }

    pub fn wanted(&self) -> Wanted {
        if self.s[0] == ID_PLAYER { Wanted::Sammy } else { Wanted::Enemy { class: self.s[0], spec: (self.s[2] != 0.0).then_some(self.s[2]) } }
    }

    pub fn within(&self, q: [f32; 3]) -> bool {
        (0..3).map(|i| (q[i] - self.pos[i]).powi(2)).sum::<f32>().sqrt() <= self.s[1]
    }

    /// `SetAnimPlayer`'s scene.
    pub fn scene(&self) -> ScriptedScene {
        ScriptedScene { salut: self.s[2] == SALUT, step: self.s[5], speed: self.s[4] }
    }
}

impl ScriptedScene {
    /// Starts it: "salut" once, else "loopjambes" looping.
    pub fn start(&self, anim: &mut Anim) {
        anim.play(if self.salut { "salut" } else { "loopjambes" }, !self.salut, self.step);
        anim.set_speed_pct(self.speed, TICKS_PER_SEC);
    }
}

/// After "salut", "loopjambes" loops at its default speed (anims.wdl:485-496). Returns
/// whether the wave is still playing.
pub fn scripted_anim(salut: bool, anim: &mut Anim) -> bool {
    if salut && anim.finished() {
        anim.play("loopjambes", true, 0.0);
        anim.set_speed_pct(ANIM_SPEED_LOOPJAMBES, TICKS_PER_SEC);
        return false;
    }
    salut
}

/// A hard camera cut onto `at`: `dist` behind at the world-fixed back angle, `height` above.
pub fn cut_camera(at: [f32; 3], back: f32, dist: f32, height: f32) -> [f32; 3] {
    let a = back.to_radians();
    [at[0] + a.cos() * dist, at[1] + a.sin() * dist, at[2] + height]
}
