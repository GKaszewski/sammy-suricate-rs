#[allow(unused_imports)]
use super::*;

/// Gem sprite assets by colour, for keys created at runtime (ejected/mixed keys).
#[derive(Resource, Default)]
pub struct GemSprites(pub HashMap<u32, (Handle<Mesh>, Handle<StandardMaterial>, u32)>);
