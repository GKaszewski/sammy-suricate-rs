//! Model shadows on the floor.

use super::*;

impl Scene {
    /// A4 engine model shadows (`my.shadow`): each caster's mesh flattened along the
    /// sun onto the floor below it, 50 % black, each floor pixel darkened once.
    pub(super) fn draw_shadows(&self, gfx: &Gfx, sim: &Sim, cam: &Camera) {
        let Some([pan, tilt]) = self.sun else { return };
        let (p, t) = (pan.to_radians(), tilt.to_radians());
        // Light travels from the sun toward the ground.
        let sun = -Vec3::new(t.cos() * p.cos(), t.cos() * p.sin(), t.sin());
        let gl = &gfx.gl;
        let w = &sim.world;
        let mut started = false;
        for (id, _) in w.looks.iter().filter(|(_, l)| l.shadow) {
            let (Some(b), Some((_, View::Model { mesh, .. }))) = (w.bodies.get(id), self.views.get(id)) else { continue };
            if b.hidden || !cam.sees(sphere(b, &self.views[id].1)) {
                continue;
            }
            let o = b.pos;
            let tr = sim.trace_static(sammy_core::collision::Hull::Point, [o[0], o[1], o[2] + SHADOW_PROBE_UP], [o[0], o[1], o[2] - SHADOW_PROBE_DOWN]).trace;
            if !tr.hit() || tr.start_solid {
                continue;
            }
            if !started {
                started = true;
                unsafe {
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    gl.depth_mask(false);
                    gl.enable(glow::STENCIL_TEST);
                    gl.stencil_func(glow::EQUAL, 0, 0xff);
                    gl.stencil_op(glow::KEEP, glow::KEEP, glow::INCR);
                }
            }
            gfx.draw_shadow(mesh, &cam.view_proj, &transform(b, rotation(b.angles)), sun, tr.end[2] + SHADOW_LIFT, SHADOW_ALPHA);
        }
        if started {
            unsafe {
                gl.disable(glow::STENCIL_TEST);
                gl.disable(glow::BLEND);
                gl.depth_mask(true);
            }
        }
    }
}
