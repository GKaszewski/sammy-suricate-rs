#[allow(unused_imports)]
use super::*;

/// Fading warning light on Sammy (range −5/tick down to 100, then off).
#[derive(Component)]
pub struct WarningFade;

/// Flat shadow decal following an owner at floor + 0.5.
#[derive(Component)]
pub struct GroundShadow {
    pub(crate) owner: Entity,
}

#[derive(Component)]
pub struct HasShadow;

/// Eye of Truth pickup ([`sammy_core::eye::EyePickup`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct EyePickup {
    #[deref]
    pub(crate) eye: sammy_core::eye::EyePickup,
    pub(crate) scale: Vec3,
}

/// Extra hover height added when snapping to the ground.
#[derive(Component)]
pub struct ExtraHeight(pub f32);

/// Range ring drawn around a lion while the Eye of Truth is active.
#[derive(Component)]
pub struct RangeRing {
    pub(crate) owner: Entity,
}

#[derive(Component)]
pub struct HasRing;
