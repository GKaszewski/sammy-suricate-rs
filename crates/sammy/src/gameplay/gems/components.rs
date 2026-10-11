#[allow(unused_imports)]
use super::*;

/// The gem can be picked up by Sammy's scan (`InitNewKey` ran).
#[derive(Component)]
pub struct Collectable;

/// A gem in flight ([`sammy_core::gems::FlyingGem`]).
#[derive(Component, Deref, DerefMut)]
pub struct FlyingGem(pub sammy_core::gems::FlyingGem);

/// Remaining lifetime of a runtime gem ([`sammy_core::gems::GemLifetime`]).
#[derive(Component, Deref, DerefMut)]
pub struct GemLifetime(pub sammy_core::gems::GemLifetime);

/// Flag7 gem gravity ([`sammy_core::gems::GemGravity`]).
#[derive(Component, Deref, DerefMut)]
pub struct GemGravity(pub sammy_core::gems::GemGravity);

impl GemGravity {
    pub fn new(rest: f32) -> Self {
        Self(sammy_core::gems::GemGravity { rest })
    }
}

/// Surprise gem ([`sammy_core::gems::Surprise`]).
#[derive(Component, Deref, DerefMut)]
pub struct Surprise(pub sammy_core::gems::Surprise);

impl Surprise {
    pub fn new(add: f32, seed: f32) -> Self {
        Self(sammy_core::gems::Surprise::new(add, seed))
    }
}

/// Decorative gem (fate 0): shrinks and bursts once it stops.
#[derive(Component)]
pub struct Decorative;

/// Boulder emitting gems ([`sammy_core::gems::Boulder`]).
#[derive(Component, Deref, DerefMut)]
pub struct Boulder(pub sammy_core::gems::Boulder);

/// Gem flying from Sammy into a permanent door ([`sammy_core::gems::DoorGem`]).
#[derive(Component, Deref, DerefMut)]
pub struct DoorGem {
    pub door: Entity,
    #[deref]
    pub gem: sammy_core::gems::DoorGem,
}
