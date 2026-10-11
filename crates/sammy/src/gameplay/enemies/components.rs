#[allow(unused_imports)]
use super::*;

/// A call ring spawned by a calling lion: (delay ticks, z offset, skin).
#[derive(Component)]
pub struct CallRing {
    pub delay: f32,
    pub scale: f32,
    pub frame: f32,
    pub origin: [f32; 3],
    pub skin: u32,
}

/// Patrol path node ([`sammy_core::enemies::PathNode`]).
#[derive(Component, Deref)]
pub struct PathNode(pub sammy_core::enemies::PathNode);

/// An enemy ([`sammy_core::enemies::Enemy`]).
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Enemy(pub sammy_core::enemies::Enemy);

/// Lions fading on teleport pads get their own material copy while translucent.
#[derive(Component)]
pub struct FadeMaterial(pub(crate) Handle<StandardMaterial>);

/// `InitStartEnnemy` ([`sammy_core::enemies::StartTrigger`]).
#[derive(Component, Deref, DerefMut)]
pub struct StartTrigger(pub(crate) sammy_core::enemies::StartTrigger);

#[derive(Component)]
pub struct PlayerBoxOffset(pub(crate) f32);
