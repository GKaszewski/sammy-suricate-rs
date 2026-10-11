#[allow(unused_imports)]
use super::*;

#[derive(Component, Deref, DerefMut)]
pub struct Title(pub(crate) sammy_core::titles::Title);
