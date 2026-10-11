//! The scene's dynamic lights.

use super::*;

/// A frame's lights and counts, carried through the draw passes.
pub(super) struct FrameLights {
    pub(super) lights: SceneLights,
    pub(super) stats: Stats,
}

/// The scene's dynamic lights (position, range, colour 0..1), nearest the camera first.
pub(super) struct SceneLights(pub(super) Vec<(Vec3, f32, [f32; 3])>);

impl SceneLights {
    pub(super) fn new(sim: &Sim, cam: Vec3, extra: &[(Vec3, f32, [f32; 3])]) -> Self {
        let w = &sim.world;
        let mut list: Vec<(Vec3, f32, [f32; 3])> = w
            .lights
            .iter()
            .filter(|(_, l)| l.range > sammy_core::space::Quants::ZERO)
            .filter_map(|(id, l)| w.bodies.get(id).map(|b| (Vec3::from(b.pos.get()), l.range.get(), l.color.map(|c| c / 255.0))))
            .chain(extra.iter().copied())
            .collect();
        list.sort_by(|a, b| (a.0 - cam).length_squared().total_cmp(&(b.0 - cam).length_squared()));
        Self(list)
    }

    /// The lights reaching a sphere (at most [`MAX_LIGHTS`]); the shaders skip the rest.
    pub(super) fn reaching(&self, (c, r): Sphere) -> Lights {
        let mut out = Lights::default();
        for (p, range, color) in self.0.iter().filter(|(p, range, _)| p.distance(c) < range + r).take(MAX_LIGHTS) {
            out.data[out.count * 2] = [p.x, p.y, p.z, *range];
            out.data[out.count * 2 + 1] = [color[0], color[1], color[2], 0.0];
            out.count += 1;
        }
        out
    }
}
