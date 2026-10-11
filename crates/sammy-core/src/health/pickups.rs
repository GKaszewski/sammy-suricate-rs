//! Hearts, boosts and checkpoints.

use super::*;

/// A small heart (health).
#[derive(Clone, Debug)]
pub struct SmallHeart {
    /// Skill2: health restored.
    pub amount: f32,
    /// Health was full: ducked away for this many more ticks.
    pub away: Option<f32>,
    /// Snap to the floor (Flag6 keeps the WED height).
    pub snap: bool,
}

impl SmallHeart {
    pub fn from_object(obj: &Object) -> Self {
        Self { amount: if obj.skill(2) == 0.0 { SMALL_HEART_DEFAULT } else { obj.skill(2) }, away: None, snap: !obj.flag(6) }
    }
}

/// A big heart (extra lives, Skill2).
#[derive(Clone, Copy, Debug)]
pub struct BigHeart {
    pub lives: i32,
}

impl BigHeart {
    pub fn from_object(obj: &Object) -> Self {
        Self { lives: obj.skill(2) as i32 }
    }
}

/// A boost pickup.
#[derive(Clone, Debug)]
pub struct BoostPickup {
    /// Skill2: boost units refilled.
    pub amount: f32,
    /// Boost was full: away for this many more ticks.
    pub away: Option<f32>,
}

impl BoostPickup {
    pub fn from_object(obj: &Object) -> Self {
        Self { amount: if obj.skill(2) == 0.0 { BOOST_DEFAULT } else { obj.skill(2) }, away: None }
    }
}

/// A checkpoint (`InitRestart`).
#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub home: [f32; 3],
    /// Skill8: Sammy's pan after respawning here.
    pub pan: f32,
    /// Skill2-4 spin speeds (Flag3).
    pub spin: Option<[f32; 3]>,
    pub angles: [f32; 3],
}

impl Checkpoint {
    pub fn from_object(obj: &Object) -> Self {
        Self { home: obj.origin, pan: obj.skill(8), spin: obj.flag(3).then_some([obj.skill(2), obj.skill(3), obj.skill(4)]), angles: obj.angles }
    }

    /// Where Sammy restarts once it is taken.
    pub fn respawn(&self) -> ([f32; 3], f32) {
        ([self.home[0], self.home[1], self.home[2] + CHECKPOINT_RESPAWN_HEIGHT], self.pan)
    }
}

/// Picking up a small heart: true if it heals (else it ducks away).
pub fn take_small_heart(heart: &mut SmallHeart, game: &mut GameState) -> bool {
    if game.hearts >= MAX_HEARTS {
        heart.away = Some(HEART_AWAY);
        return false;
    }
    game.hearts = (game.hearts + heart.amount).min(MAX_HEARTS);
    true
}

/// Picking up a big heart: true if it gives lives.
pub fn take_big_heart(heart: &BigHeart, game: &mut GameState) -> bool {
    if game.lives >= MAX_LIVES {
        return false;
    }
    game.lives = (game.lives + heart.lives).min(MAX_LIVES);
    true
}

/// Picking up a boost: true if it refills (else it goes away while the boost is full).
pub fn take_boost(boost: &mut BoostPickup, game: &mut GameState) -> bool {
    if game.boost <= 0.0 {
        boost.away = Some(BOOST_AWAY);
        return false;
    }
    game.boost = (game.boost - boost.amount).max(0.0);
    true
}

/// Taking a checkpoint makes it the respawn point.
pub fn take_checkpoint(c: &Checkpoint, game: &mut GameState) {
    game.start = Some(c.respawn());
    game.checkpoint = true;
}
