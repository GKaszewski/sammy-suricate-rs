//! Dying on hazards and respawning.

use super::*;
use crate::time::Ticks;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DeathKind {
    /// Deadly tile: lose a life, fade out, respawn (no death animation).
    Tile,
    /// Out of health: death animation, lose a life, blink, respawn.
    Kaputt,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Stage {
    Dying,
    FadeOut,
    Blink,
    FadeIn,
    GameOver,
}

/// The death animation's sound plays this many ticks in.
pub(super) const KAPUTT_SOUND_AT: f32 = 8.0;
/// The death animation ends at the latest after this many ticks.
pub(super) const KAPUTT_MAX: f32 = 64.0;
/// Speeds of the death animations (percent per tick).
pub(super) const KAPUTT_ANIM_SPEED: f32 = 3.0;
pub(super) const FINKAPUTT_ANIM_SPEED: f32 = 2.0;
/// The camera's look-down while dying: PlusTilt shrinks by 75 % per tick.
pub(super) const LOOK_DOWN_RATE: f32 = 0.75;
/// `Camera_PlusTilt -= Camera_PlusTilt * 0.75 * time` on a PlusTilt scale `k`.
pub fn look_down(k: f32, t: f32) -> f32 {
    (k - k * LOOK_DOWN_RATE * t).max(0.0)
}

/// Death-tile fade-out and respawn fade-in speeds (alpha percent per tick).
pub(super) const FADE_OUT_SPEED: f32 = 2.0;
pub(super) const FADE_IN_SPEED: f32 = 10.0;
/// Blink after a death animation: ticks, and the flicker period over time.
pub(super) const BLINK_TICKS: f32 = 46.0;
pub(super) fn blink_period(ticks: f32) -> f32 {
    if ticks < 16.0 {
        4.0
    } else if ticks < 30.0 {
        3.0
    } else {
        2.0
    }
}
/// The dropped key leaves this high above Sammy and this fast upwards.
pub(super) const DROP_HEIGHT: f32 = 30.0;
pub(super) const DROP_VZ: f32 = 40.0;

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
    pub(super) stage: Stage,
    pub(super) ticks: f32,
    /// Sammy's opacity in percent (`my.alpha`).
    pub alpha: f32,
    pub(super) sound_played: bool,
}

/// Which death (if any) starts this frame: a death tile under Sammy, or no health left
/// (lava burns it meanwhile).
pub fn check_death(game: &mut GameState, on_death_tile: bool, on_lava: bool, t: Ticks) -> Option<DeathKind> {
    let t = t.get();
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
    pub fn step(&mut self, game: &mut GameState, anim: Option<&mut Anim>, victim: Victim, t: Ticks, out: &mut Vec<DeathEvent>) {
        let t = t.get();
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
    pub(super) fn respawn(&mut self, game: &mut GameState, out: &mut Vec<DeathEvent>) {
        game.hearts = MAX_HEARTS;
        out.push(DeathEvent::Respawn(game.start));
        self.ticks = 0.0;
        self.stage = Stage::FadeIn;
    }
}
