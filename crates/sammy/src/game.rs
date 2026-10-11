//! Global game state (the WDL globals of skills.wdl that the ported actions share).

use bevy::prelude::*;
use formats::wmb::Object;

use crate::audio::Sfx;

/// Colour key value granted on reaching the score goal (opens every door).
pub const MASTER_KEY: u32 = 255;
/// `WindowHeartsOrg`: full health.
pub const MAX_HEARTS: f32 = 48.0;
/// `MaxBalles`: most lives Sammy can hold.
pub const MAX_LIVES: i32 = 19;

/// The reset scan (id 1000) sent by `RestartSammy`: level-placed keys and
/// checkpoints come back, runtime keys disappear.
#[derive(Message, Clone, Copy)]
pub struct ResetScan;

#[derive(Resource, Debug)]
pub struct GameState {
    /// `ScoreJoueur1`.
    pub score: i32,
    /// `ScoreGoal` (InitGame Skill7).
    pub goal: i32,
    /// `PlayerKey`: colour bitmask of the carried key (0 = none, 255 = master).
    pub key: u32,
    /// `PlayerOldKey != 0`: an ejected key is still bouncing, pickups are blocked.
    pub key_in_flight: bool,
    /// `BallesJoueur1`.
    pub lives: i32,
    pub goal_reached: bool,
    /// `partieGagnee`.
    pub won: bool,
    /// Out of lives (`BallesJoueur1 == 0` after a death).
    pub lost: bool,
    /// Ticks left of the score-panel blink (`WarnScoreGoal`).
    pub warn_score: f32,
    /// `WindowHearts`: health 0..48.
    pub hearts: f32,
    /// `Player1Start` / `Player1AngleStart`: respawn point (spawn or checkpoint).
    pub start: Option<([f32; 3], f32)>,
    /// `DoRestart`: a checkpoint was taken (its position is saved with the level).
    pub checkpoint: bool,
    /// `EyeOfTruth` ticks left (lion range rings visible while > 0).
    pub eye_ticks: f32,
    /// `DureeEyeTruthOrg` (256; InitGame7 Skill6).
    pub eye_duration: f32,
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
            score: 0,
            goal: 0,
            key: 0,
            key_in_flight: false,
            lives: 1,
            goal_reached: false,
            won: false,
            lost: false,
            warn_score: 0.0,
            hearts: MAX_HEARTS,
            start: None,
            checkpoint: false,
            eye_ticks: 0.0,
            boost: crate::actions::player::BOOST_MAX,
            eye_duration: 256.0,
            display_range: 1.0,
            ammo: 0,
            ammo_type: 0,
            ammo_poison: 0.0,
        }
    }
}

impl GameState {
    /// `InitGame` (mm1.wdl:1740): Skill7 goal, Skill8 starting score.
    pub fn apply_init_game(&mut self, obj: &Object) {
        self.goal = obj.skills[6] as i32;
        self.score = obj.skills[7] as i32;
    }

    /// `InitGame2` (mm1.wdl:1815): Skill1 starting lives.
    pub fn apply_init_game2(&mut self, obj: &Object) {
        if obj.skills[0] != 0.0 {
            self.lives = obj.skills[0].abs() as i32;
        }
    }

    /// `InitGame7` (mm1.wdl): Skill6 Eye of Truth duration, Skill7 range display mode.
    pub fn apply_init_game7(&mut self, obj: &Object) {
        if obj.skills[5] != 0.0 {
            self.eye_duration = obj.skills[5];
        }
        if obj.skills[6] != 0.0 {
            self.display_range = obj.skills[6];
        }
    }

    /// Debug: SAMMY_KEY=<colour> starts with a key (255 = master).
    pub fn apply_debug_env(&mut self) {
        if let Some(k) = std::env::var("SAMMY_KEY").ok().and_then(|v| v.parse().ok()) {
            self.key = k;
        }
    }

    /// `AddPoints` (mm1.wdl:1440): reaching the goal grants the master key.
    pub fn add_points(&mut self, points: i32, sfx: &mut Sfx) {
        self.score = (self.score + points).max(0);
        if !self.goal_reached && self.score >= self.goal {
            self.goal_reached = true;
            self.key = MASTER_KEY;
            sfx.play("ScoreReach_son");
        }
    }
}

/// WDL `random(x)`: uniform in [0, x). Small xorshift so runs are reproducible
/// with `SAMMY_SEED`.
#[derive(Resource)]
pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        let seed = std::env::var("SAMMY_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
        });
        Self(seed | 1)
    }
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn random(&mut self, x: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32 * x
    }
}
