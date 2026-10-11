#[allow(unused_imports)]
use super::*;

/// `StartExtro`: after a short wait the camera target becomes this wasp, overriding any
/// trigger that fired on the first ticks.
#[derive(Component)]
pub struct StartExtroPov(pub(crate) f32);

/// One of the extro's triggers ([`sammy_core::extro::Trigger`]).
#[derive(Component, Deref, DerefMut)]
pub struct ExtroTrigger(pub(crate) sammy_core::extro::Trigger);

/// Sammy is in a scripted animation (`PlayerEtat` 2/3 forced by `SetAnimPlayer`):
/// no input, no death. `salut` waves once, then loops "loopjambes".
#[derive(Component)]
pub struct ScriptedAnim {
    pub(crate) salut: bool,
}

#[derive(Component)]
pub struct FadePanel;
