#[allow(unused_imports)]
use super::*;

/// An engine ([`sammy_core::platforms::Engine`]).
#[derive(Component, Deref, DerefMut)]
pub struct Engine(pub sammy_core::platforms::Engine);

/// A platform or body ([`sammy_core::platforms::Platform`]) and the engine it follows.
#[derive(Component, Deref, DerefMut)]
pub struct Platform {
    #[deref]
    pub platform: sammy_core::platforms::Platform,
    pub(crate) engine: Option<Entity>,
}

/// A teleport pad ([`sammy_core::platforms::Teleport`]).
#[derive(Component, Deref, DerefMut)]
pub struct Teleport(pub sammy_core::platforms::Teleport);

/// Sammy is being teleported from `pad` (input locked).
#[derive(Component, Deref, DerefMut)]
pub struct Teleporting {
    pub(crate) pad: Entity,
    #[deref]
    pub(crate) state: sammy_core::platforms::Teleporting,
}
