//! Health, lives and hazards:
//! - `InitTileDeath` / `TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava` / `Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitBoost`
//!   (mm1.wdl:4315), `InitRestart` checkpoints (mm1.wdl:3771)

use formats::wmb::Object;

use crate::anim::Anim;
use crate::game::{GameState, MAX_HEARTS, MAX_LIVES};
use crate::gems::{Fate, Launch};
use crate::time::TICKS_PER_SEC;

/// Lava burns this much health per tick.
pub const LAVA_DAMAGE: f32 = 1.0;
/// `InitSmallHeart` Skill2 default.
const SMALL_HEART_DEFAULT: f32 = 12.0;
/// A heart touched at full health ducks away for this long.
pub const HEART_AWAY: f32 = 48.0;
/// ... shrinking ×0.8 per tick, hidden below 0.1, then growing back ×1.5 per tick.
pub const HEART_AWAY_SHRINK: f32 = 0.8;
pub const HEART_HIDDEN_BELOW: f32 = 0.1;
pub const HEART_REGROW: f32 = 1.5;
/// `InitBoost` Skill2 default (boost units refilled).
const BOOST_DEFAULT: f32 = 1.0;
/// A boost pickup touched while the boost is full: shrinks ×0.9 per tick for 40 ticks,
/// is hidden for 64, then comes back.
pub const BOOST_HIDDEN: f32 = 64.0;
pub const BOOST_AWAY: f32 = BOOST_HIDDEN + 40.0;
pub const BOOST_AWAY_SHRINK: f32 = 0.9;
/// A checkpoint respawns Sammy this far above it.
pub const CHECKPOINT_RESPAWN_HEIGHT: f32 = 150.0;
/// Big hearts spin this many degrees per tick.
pub const BIG_HEART_SPIN: f32 = 10.0;

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DeathKind {
    /// Deadly tile: lose a life, fade out, respawn (no death animation).
    Tile,
    /// Out of health: death animation, lose a life, blink, respawn.
    Kaputt,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Dying,
    FadeOut,
    Blink,
    FadeIn,
    GameOver,
}

/// The death animation's sound plays this many ticks in.
const KAPUTT_SOUND_AT: f32 = 8.0;
/// The death animation ends at the latest after this many ticks.
const KAPUTT_MAX: f32 = 64.0;
/// Speeds of the death animations (percent per tick).
const KAPUTT_ANIM_SPEED: f32 = 3.0;
const FINKAPUTT_ANIM_SPEED: f32 = 2.0;
/// The camera's look-down while dying: PlusTilt shrinks by 75 % per tick.
const LOOK_DOWN_RATE: f32 = 0.75;
/// `Camera_PlusTilt -= Camera_PlusTilt * 0.75 * time` on a PlusTilt scale `k`.
pub fn look_down(k: f32, t: f32) -> f32 {
    (k - k * LOOK_DOWN_RATE * t).max(0.0)
}

/// Death-tile fade-out and respawn fade-in speeds (alpha percent per tick).
const FADE_OUT_SPEED: f32 = 2.0;
const FADE_IN_SPEED: f32 = 10.0;
/// Blink after a death animation: ticks, and the flicker period over time.
const BLINK_TICKS: f32 = 46.0;
fn blink_period(ticks: f32) -> f32 {
    if ticks < 16.0 {
        4.0
    } else if ticks < 30.0 {
        3.0
    } else {
        2.0
    }
}
/// The dropped key leaves this high above Sammy and this fast upwards.
const DROP_HEIGHT: f32 = 30.0;
const DROP_VZ: f32 = 40.0;

/// What Sammy is like when the death starts.
#[derive(Clone, Copy, Debug)]
pub struct Victim {
    pub pos: [f32; 3],
    pub pan: f32,
    /// Movement this frame (quants per tick).
    pub vel: [f32; 3],
}

/// What a death step asks of the frontend.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DeathEvent {
    /// First tick: input off (`PlayerEtat` dead).
    Started,
    /// The held key pops out as a decorative gem.
    DropKey(Launch),
    Sound(&'static str),
    /// The camera looks down at Sammy: apply [`look_down`] to its PlusTilt scale.
    CameraLookDown { t: f32 },
    /// Out of lives: the camera stops tilting and orbits.
    GameOver,
    /// `RestartSammy`: reset scan, back to the respawn point (if any).
    Respawn(Option<([f32; 3], f32)>),
    /// Visible or not during the blink.
    Visible(bool),
    /// Faded back in: Sammy can move again.
    Finished,
}

/// The running death/respawn sequence.
#[derive(Clone, Debug)]
pub struct Dying {
    pub kind: DeathKind,
    stage: Stage,
    ticks: f32,
    /// Sammy's opacity in percent (`my.alpha`).
    pub alpha: f32,
    sound_played: bool,
}

/// Which death (if any) starts this frame: a death tile under Sammy, or no health left
/// (lava burns it meanwhile).
pub fn check_death(game: &mut GameState, on_death_tile: bool, on_lava: bool, t: f32) -> Option<DeathKind> {
    if on_lava {
        game.hearts -= LAVA_DAMAGE * t;
    }
    let kind = if on_death_tile {
        DeathKind::Tile
    } else if game.hearts <= 0.0 {
        DeathKind::Kaputt
    } else {
        return None;
    };
    game.hearts = game.hearts.max(0.0);
    Some(kind)
}

impl Dying {
    pub fn new(kind: DeathKind) -> Self {
        Self { kind, stage: Stage::Dying, ticks: 0.0, alpha: 100.0, sound_played: false }
    }

    /// Whether the sequence is fading Sammy (the frontend applies [`Dying::alpha`]).
    pub fn fading(&self) -> bool {
        matches!(self.stage, Stage::FadeOut | Stage::FadeIn)
    }

    /// One frame of `t` ticks.
    pub fn step(&mut self, game: &mut GameState, anim: Option<&mut Anim>, victim: Victim, t: f32, out: &mut Vec<DeathEvent>) {
        self.ticks += t;
        let mut anim = anim;
        match self.stage {
            Stage::Dying => {
                if self.ticks == t {
                    // ResetSammy: input off; the held key is dropped unless the goal is reached.
                    out.push(DeathEvent::Started);
                    if game.key != 0 && !game.goal_reached {
                        let color = game.key;
                        game.key = 0;
                        let pos = [victim.pos[0], victim.pos[1], victim.pos[2] + DROP_HEIGHT];
                        let hspeed = victim.vel[0].abs() + victim.vel[1].abs();
                        out.push(DeathEvent::DropKey(Launch { color, pos, pan: victim.pan, hspeed, vz: DROP_VZ, fate: Fate::Decorative }));
                    }
                    if self.kind == DeathKind::Kaputt
                        && let Some(a) = anim.as_deref_mut()
                    {
                        a.play("kaputt", false, 0.0);
                        a.set_speed_pct(KAPUTT_ANIM_SPEED, TICKS_PER_SEC);
                    }
                }
                let done = match self.kind {
                    DeathKind::Tile => true,
                    DeathKind::Kaputt => {
                        out.push(DeathEvent::CameraLookDown { t });
                        if self.ticks >= KAPUTT_SOUND_AT && !self.sound_played {
                            out.push(DeathEvent::Sound("Kaputt_son"));
                            self.sound_played = true;
                        }
                        self.ticks >= KAPUTT_MAX || anim.as_deref().is_some_and(|a| a.finished())
                    }
                };
                if done {
                    game.lives -= 1;
                    self.ticks = 0.0;
                    self.stage = if game.lives <= 0 {
                        game.lost = true;
                        out.push(DeathEvent::GameOver);
                        if let Some(a) = anim {
                            a.play("finkaputt", true, 0.0);
                            a.set_speed_pct(FINKAPUTT_ANIM_SPEED, TICKS_PER_SEC);
                        }
                        Stage::GameOver
                    } else if self.kind == DeathKind::Tile {
                        Stage::FadeOut
                    } else {
                        Stage::Blink
                    };
                }
            }
            Stage::FadeOut => {
                self.alpha = (self.alpha - FADE_OUT_SPEED * t).max(0.0);
                if self.alpha <= 0.0 {
                    self.respawn(game, out);
                }
            }
            Stage::Blink => {
                let visible = (self.ticks / blink_period(self.ticks)) as i32 % 2 != 0;
                if self.ticks >= BLINK_TICKS {
                    out.push(DeathEvent::Visible(true));
                    self.alpha = 0.0;
                    self.respawn(game, out);
                } else {
                    out.push(DeathEvent::Visible(visible));
                }
            }
            Stage::FadeIn => {
                self.alpha = (self.alpha + FADE_IN_SPEED * t).min(100.0);
                if self.alpha >= 100.0 {
                    out.push(DeathEvent::Finished);
                }
            }
            Stage::GameOver => {}
        }
    }

    /// `RestartSammy`: full health, back to the respawn point, fade in.
    fn respawn(&mut self, game: &mut GameState, out: &mut Vec<DeathEvent>) {
        game.hearts = MAX_HEARTS;
        out.push(DeathEvent::Respawn(game.start));
        self.ticks = 0.0;
        self.stage = Stage::FadeIn;
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

#[cfg(test)]
mod tests;
