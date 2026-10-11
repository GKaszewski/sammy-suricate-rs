#[allow(unused_imports)]
use super::*;

#[derive(Component)]
pub struct DeathTile;

#[derive(Component)]
pub struct LavaTile;

/// A small heart ([`sammy_core::health::SmallHeart`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct SmallHeart {
    #[deref]
    pub(crate) heart: sammy_core::health::SmallHeart,
    pub(crate) scale: Vec3,
}

#[derive(Component, Deref)]
pub struct BigHeart(pub(crate) sammy_core::health::BigHeart);

/// Boost pickup ([`sammy_core::health::BoostPickup`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct BoostPickup {
    #[deref]
    pub(crate) boost: sammy_core::health::BoostPickup,
    pub(crate) scale: Vec3,
}

#[derive(Component, Deref, DerefMut)]
pub struct Checkpoint(pub(crate) sammy_core::health::Checkpoint);

/// Running death/respawn sequence on Sammy ([`sammy_core::health::Dying`]).
#[derive(Component, Deref, DerefMut)]
pub struct Dying(pub(crate) sammy_core::health::Dying);
