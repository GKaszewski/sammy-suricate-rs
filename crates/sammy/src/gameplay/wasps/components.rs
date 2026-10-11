#[allow(unused_imports)]
use super::*;

/// An ammo pickup ([`sammy_core::wasps::AmmoPickup`]).
#[derive(Component, Deref)]
pub struct AmmoPickup(pub sammy_core::wasps::AmmoPickup);

/// Spin of an idle pickup.
#[derive(Component)]
pub struct Spinning {
    pub(crate) angles: [f32; 3],
}

/// A flying wasp ([`sammy_core::wasps::Wasp`]).
#[derive(Component, Deref, DerefMut)]
pub struct Wasp(pub sammy_core::wasps::Wasp);

/// Shots still to fire in a fan ([`sammy_core::wasps::PendingShots`]).
#[derive(Component, Deref, DerefMut)]
pub struct PendingShots(pub(crate) sammy_core::wasps::PendingShots);

#[derive(Component, Deref, DerefMut)]
pub struct WeakDoor(pub(crate) sammy_core::wasps::WeakDoor);

#[derive(Component, Deref, DerefMut)]
pub struct Plank(pub(crate) sammy_core::wasps::Plank);

/// A hive ([`sammy_core::wasps::Hive`]).
#[derive(Component, Deref, DerefMut)]
pub struct Hive(pub(crate) sammy_core::wasps::Hive);

/// A wasp launched by a hive ([`sammy_core::wasps::HiveWasp`]).
#[derive(Component, Deref, DerefMut)]
pub struct HiveWasp(pub(crate) sammy_core::wasps::HiveWasp);

/// Remaining lifetime of a hive-emitted pickup (fades over its last 16 ticks).
#[derive(Component)]
pub struct PickupLifetime(pub(crate) f32);
