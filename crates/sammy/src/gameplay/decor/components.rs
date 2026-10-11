#[allow(unused_imports)]
use super::*;

/// A sky dome following the camera ([`sammy_core::decor::SkyFollow`]).
#[derive(Component, Deref, DerefMut)]
pub struct SkyFollow(pub(crate) sammy_core::decor::SkyFollow);

/// Snaps a sprite onto the floor below its editor origin once collision is loaded.
#[derive(Component)]
pub struct SpriteGround {
    pub origin: [f32; 3],
    /// Height offset above the floor (half sprite height + action offset).
    pub dz: f32,
}
