#[allow(unused_imports)]
use super::*;

/// A diamond ([`sammy_core::pickups::Diamond`]).
#[derive(Component, Deref)]
pub struct Diamond(pub sammy_core::pickups::Diamond);

#[derive(Component)]
pub struct KeyGem {
    pub color: u32,
}

/// Native position of a pickup (kept separately so ground snapping and bouncing
/// work in Acknex units).
#[derive(Component)]
pub struct PickupPos(pub [f32; 3]);

/// Snap to `floor + HEIGHT_GEM` on the first frame (`sonar my,2000`).
#[derive(Component)]
pub struct SnapToGround;

/// Picked up: shrink ×0.8 per tick, then despawn (`Bonus3_Hit`, `ScaleDownGem`).
/// Entities that can come back (level keys, checkpoints) are hidden instead.
#[derive(Component)]
pub struct Shrinking;

/// Collected and waiting for the reset scan (`GemWaitForNewStart`).
#[derive(Component)]
pub struct Collected;

/// A key placed in the level (WDL Flag3): restored at its home on each respawn.
#[derive(Component)]
pub struct LevelKey {
    pub(crate) home: Option<[f32; 3]>,
    pub(crate) scale: Vec3,
}

impl LevelKey {
    pub fn at(home: [f32; 3], scale: Vec3) -> Self {
        Self { home: Some(home), scale }
    }
}

/// Flag5 gems hover at their WED height above the floor (+HeightGem) instead of 25.
#[derive(Component)]
pub struct HoverFromWed;

/// Restored by the reset scan instead of being despawned after shrinking.
#[derive(Component)]
pub struct Respawnable;
