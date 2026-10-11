#[allow(unused_imports)]
use super::*;

/// The logo shown (1 A5 sprite, 2 publisher, 3 Suricate).
#[derive(Component)]
pub(crate) struct LogoView(pub(crate) u8);

#[derive(Component)]
pub(crate) struct FogQuad;
