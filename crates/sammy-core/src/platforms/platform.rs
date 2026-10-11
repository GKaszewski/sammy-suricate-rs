//! Platforms linked to an engine.

use super::*;

/// A platform or body following an engine (`SetLinkMoteur`).
#[derive(Clone, Debug)]
pub struct Platform {
    pub link: f32,
    pub offset: [f32; 3],
    /// Motion this frame (carried onto riders).
    pub delta: [f32; 3],
}

impl Platform {
    /// `InitPlatForm` (link 0 = unlinked) / `InitBodyPlatForm` (link 0 = default).
    pub fn from_object(obj: &Object, body: bool) -> Self {
        let link = if obj.skill(1) == 0.0 && body { DEFAULT_LINK } else { obj.skill(1) };
        Self { link, offset: [0.0; 3], delta: [0.0; 3] }
    }

    /// Attaches to an engine: the offset from it is kept from now on.
    pub fn attach(&mut self, origin: [f32; 3], engine: &Engine) {
        self.offset = [0, 1, 2].map(|i| origin[i] - engine.pos[i]);
    }

    /// Follows the engine; `origin` is the platform brush's position.
    pub fn follow(&mut self, origin: &mut [f32; 3], engine: &Engine) {
        let target = [0, 1, 2].map(|i| engine.pos[i] + self.offset[i]);
        self.delta = [0, 1, 2].map(|i| target[i] - origin[i]);
        *origin = target;
    }
}
