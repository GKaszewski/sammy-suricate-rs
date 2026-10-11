#[allow(unused_imports)]
use super::*;

/// The scripted Sammy ([`sammy_core::intro::Walker`]).
#[derive(Component, Deref, DerefMut)]
pub struct Walker(pub sammy_core::intro::Walker);

/// A path node of the intro walk ([`sammy_core::intro::PathNode`]).
#[derive(Component, Deref)]
pub struct PathNode(pub(crate) sammy_core::intro::PathNode);

#[derive(Component)]
pub(crate) struct LogoView(pub(crate) u8);

#[derive(Component)]
pub(crate) struct FogQuad;
