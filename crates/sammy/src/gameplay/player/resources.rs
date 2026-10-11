#[allow(unused_imports)]
use super::*;

/// [`sammy_core::player::PlayerTuning`] as a resource.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct PlayerTuning(pub sammy_core::player::PlayerTuning);

/// Collision geometry of the current level.
#[derive(Resource, Clone)]
pub struct LevelCollision(pub Arc<Wmb>);

/// [`sammy_core::physics::JumpParams`] as a resource.
#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
pub struct JumpParams(pub sammy_core::physics::JumpParams);

/// Sammy touched an entity while moving (EVENT_ENTITY on Sammy / EVENT_IMPACT on it).
#[derive(Message, Clone, Copy)]
pub struct PlayerTouched(pub Entity);
