#[allow(unused_imports)]
use super::*;

/// A door ([`sammy_core::doors::Door`]) on a brush entity.
#[derive(Component, Deref, DerefMut)]
pub struct Door(pub sammy_core::doors::Door);

/// The exit tile ([`sammy_core::doors::ExitTile`]).
#[derive(Component, Default, Deref, DerefMut)]
pub struct ExitTile(pub sammy_core::doors::ExitTile);
