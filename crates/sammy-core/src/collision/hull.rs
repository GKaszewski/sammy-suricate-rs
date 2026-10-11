//! Collision hulls.

/// Clipping hull of a traced object (WMB hulls 0-2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hull {
    /// A point (sprites, particles, line of sight).
    Point,
    /// ±16 cube (`narrow`).
    Narrow,
    /// 64×64 footprint, 48 high (`fat`: Sammy and the enemies).
    Fat,
}

impl Hull {
    /// The WMB clipping hull index.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Extents the hull adds around a traced point, (below/negative, above/positive)
    /// (empirical: see `formats::trace`).
    pub(super) fn extents(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Hull::Point => ([0.0; 3], [0.0; 3]),
            Hull::Narrow => ([16.0; 3], [16.0; 3]),
            Hull::Fat => ([32.0, 32.0, 16.0], [32.0, 32.0, 32.0]),
        }
    }
}
