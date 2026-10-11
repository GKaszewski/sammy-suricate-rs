//! Global game state (the WDL globals of skills.wdl that the actions share).

use formats::wmb::Object;

use crate::keys::KeyColor;
use crate::points::Points;
use crate::space::Placement;
use crate::time::Ticks;

pub use crate::keys::MASTER_KEY;
/// `WindowHeartsOrg`: full health.
pub const MAX_HEARTS: f32 = 48.0;
/// `MaxBalles`: most lives Sammy can hold.
pub const MAX_LIVES: Lives = Lives(19);

/// A number of lives (`BallesJoueur1`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Lives(pub i32);

impl Lives {
    pub const NONE: Self = Self(0);

    /// The plain number.
    pub const fn get(self) -> i32 {
        self.0
    }

    /// `n` more, up to [`MAX_LIVES`].
    pub fn gain(self, n: i32) -> Self {
        Self((self.0 + n).min(MAX_LIVES.0))
    }

    /// One less.
    pub fn lose_one(self) -> Self {
        Self(self.0 - 1)
    }
}

impl core::fmt::Display for Lives {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// `WindowBoost` when empty (the boost bar counts up while used).
pub const BOOST_MAX: f32 = 150.0;
/// `DureeEyeTruthOrg`: Eye of Truth duration in ticks unless InitGame7 sets it.
pub const EYE_DURATION: Ticks = Ticks(256.0);
/// `EnnemyMustDisplayRange` default.
pub const DISPLAY_RANGE: f32 = 1.0;

/// The WDL globals the actions share (skills.wdl).
#[derive(Clone, Debug)]
pub struct GameState {
    /// `ScoreJoueur1`.
    pub score: Points,
    /// `ScoreGoal` (InitGame Skill7).
    pub goal: Points,
    /// `PlayerKey`: colour bitmask of the carried key (0 = none, 255 = master).
    pub key: KeyColor,
    /// `PlayerOldKey != 0`: an ejected key is still bouncing, pickups are blocked.
    pub key_in_flight: bool,
    /// `BallesJoueur1`.
    pub lives: Lives,
    pub goal_reached: bool,
    /// `partieGagnee`.
    pub won: bool,
    /// Out of lives (`BallesJoueur1 == 0` after a death).
    pub lost: bool,
    /// Ticks left of the score-panel blink (`WarnScoreGoal`).
    pub warn_score: Ticks,
    /// `WindowHearts`: health 0..48.
    pub hearts: f32,
    /// `Player1Start` / `Player1AngleStart`: respawn point (spawn or checkpoint).
    pub start: Option<Placement>,
    /// `DoRestart`: a checkpoint was taken (its position is saved with the level).
    pub checkpoint: bool,
    /// `EyeOfTruth` ticks left (lion range rings visible while > 0).
    pub eye_ticks: Ticks,
    /// `DureeEyeTruthOrg` (256; InitGame7 Skill6).
    pub eye_duration: Ticks,
    /// `EnnemyMustDisplayRange` (1; InitGame7 Skill7).
    pub display_range: f32,
    /// `WindowBoost`: 0 = full, 150 = empty (starts empty).
    pub boost: f32,
    /// `NbrAmmo1`, `WaspCurrentType`, `WaspCurrentPoison`.
    pub ammo: i32,
    pub ammo_type: u32,
    pub ammo_poison: f32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            score: Points::ZERO,
            goal: Points::ZERO,
            key: KeyColor::NONE,
            key_in_flight: false,
            lives: Lives(1),
            goal_reached: false,
            won: false,
            lost: false,
            warn_score: Ticks::ZERO,
            hearts: MAX_HEARTS,
            start: None,
            checkpoint: false,
            eye_ticks: Ticks::ZERO,
            boost: BOOST_MAX,
            eye_duration: EYE_DURATION,
            display_range: DISPLAY_RANGE,
            ammo: 0,
            ammo_type: 0,
            ammo_poison: 0.0,
        }
    }
}

impl GameState {
    /// `InitGame` (mm1.wdl:1740): Skill7 goal, Skill8 starting score.
    pub fn apply_init_game(&mut self, obj: &Object) {
        self.goal = Points(obj.skills[6] as i32);
        self.score = Points(obj.skills[7] as i32);
    }

    /// `InitGame2` (mm1.wdl:1815): Skill1 starting lives.
    pub fn apply_init_game2(&mut self, obj: &Object) {
        if obj.skills[0] != 0.0 {
            self.lives = Lives(obj.skills[0].abs() as i32);
        }
    }

    /// `InitGame7` (mm1.wdl): Skill6 Eye of Truth duration, Skill7 range display mode.
    pub fn apply_init_game7(&mut self, obj: &Object) {
        if obj.skills[5] != 0.0 {
            self.eye_duration = Ticks(obj.skills[5]);
        }
        if obj.skills[6] != 0.0 {
            self.display_range = obj.skills[6];
        }
    }

    /// `AddPoints` (mm1.wdl:1440): reaching the goal grants the master key. Returns
    /// whether the goal was reached just now (`ScoreReach_son`).
    #[must_use]
    pub fn add_points(&mut self, points: Points) -> bool {
        self.score = (self.score + points).max(Points::ZERO);
        if !self.goal_reached && self.score >= self.goal {
            self.goal_reached = true;
            self.key = MASTER_KEY;
            return true;
        }
        false
    }
}
