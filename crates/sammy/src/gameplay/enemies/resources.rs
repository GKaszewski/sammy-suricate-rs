#[allow(unused_imports)]
use super::*;

/// [`sammy_core::enemies::LionGlobals`] as a resource.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct LionGlobals(pub sammy_core::enemies::LionGlobals);

/// [`sammy_core::enemies::FriendCalls`] as a resource (not reset per level).
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct FriendCalls(pub sammy_core::enemies::FriendCalls);

/// The 16 Hz AI clock ([`sammy_core::enemies::AiClock`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct AiClock(pub sammy_core::enemies::AiClock);

/// A lion started a chase: Sammy gets the orange warning light (`Warning_attack`).
#[derive(Message, Clone, Copy)]
pub struct WarningAttack;
