#[allow(unused_imports)]
use super::*;

/// `pov_Syn` ([`sammy_core::extro::Pov`]): whom the camera follows and whom enemies
/// measure distance and line of sight to.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Pov(pub sammy_core::extro::Pov);

/// This frame's id-25 scanners ([`sammy_core::extro::Scanners`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Scanners(pub sammy_core::extro::Scanners);

/// The triggers' own 16 Hz clock: their scan loops are not gated by `PartieEncours`, so
/// they already run during the level-start sweep (mm1.wdl:5411-5416).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TriggerClock(pub(crate) sammy_core::time::TickClock);

/// Fade to black after `SetFinExtro`, in fog percent; at 100 the extro ends.
#[derive(Resource)]
pub struct FinExtro(pub f32);
