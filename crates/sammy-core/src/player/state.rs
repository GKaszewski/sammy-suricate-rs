//! What Sammy is doing, and what he can bump into.

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
