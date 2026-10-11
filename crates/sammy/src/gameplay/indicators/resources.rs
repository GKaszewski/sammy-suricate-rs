#[allow(unused_imports)]
use super::*;

/// Additive, unlit ring materials per skin (1..=3).
#[derive(Resource)]
pub struct RingMaterials(pub(crate) Vec<Handle<StandardMaterial>>);
