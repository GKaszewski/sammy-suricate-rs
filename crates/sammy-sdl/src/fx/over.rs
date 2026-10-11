//! Over the world: the lose title and the HUD models.

use super::*;

/// The original screen's vertical arc basis: half of the 60° arc, at 4:3.
pub(super) const ARC_HALF_DEG: f32 = 30.0;
pub(super) const ASPECT_43: f32 = 4.0 / 3.0;
/// Rays at a grazing angle to the view axis are clamped to this cosine.
pub(super) const MIN_RAY_COS: f32 = 0.1;

impl Fx {
    /// Over the world: the lose title (lit by its own light) and the HUD models placed
    /// next to the 2D HUD at their mode-6 pixel.
    pub fn draw_over(&mut self, gfx: &Gfx, files: &GameFiles, scene: &mut Scene, sim: &Sim, cam: &Camera, hud: &HudScreen) {
        let HudScreen { window, ui_scale, shown } = *hud;
        let gl = &gfx.gl;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
        }
        for item in self.extras.items(sim) {
            if let Item::Title { file, pos, angles, scale, .. } = item
                && let Some((mesh, _, asset)) = self.posed(gfx, scene, files, file, 0.0, 0)
            {
                set_blend(gl, Blend::Alpha);
                let m = Mat4::from_scale_rotation_translation(Vec3::splat(scale), rotation(angles), Vec3::from(pos));
                let shade = Shade { unlit: true, ..Default::default() };
                gfx.draw_model(mesh, &cam.view_proj, &m, asset.skin(1), &shade, &Lights::default());
            }
        }
        let models = if shown { self.extras.hud_models(sim) } else { Vec::new() };
        let (w, h) = (sammy_present::hud::W, sammy_present::hud::H);
        // Same size relative to the 2D HUD as at 4:3 with the original 60° arc.
        let vfov_43 = 2.0 * (ARC_HALF_DEG.to_radians().tan() / ASPECT_43).atan();
        let size_k = (cam.fov / 2.0).tan() / (vfov_43 / 2.0).tan() * ui_scale / (window[1] / h);
        let half = (cam.fov / 2.0).tan();
        let cam_rot = Quat::from_mat3(&cam.axes);
        for m in models {
            let Some((mesh, _, asset)) = self.posed(gfx, scene, files, m.file, 0.0, 0) else { continue };
            // Mode-6 pixel → window pixel (negative x anchors to the right edge, y to the bottom).
            let x = if m.px[0] < 0.0 { window[0] + m.px[0] * ui_scale } else { window[0] / 2.0 + (m.px[0] - w / 2.0) * ui_scale };
            let y = window[1] - (h - m.px[1]) * ui_scale;
            let (nx, ny) = (x / window[0] * 2.0 - 1.0, 1.0 - y / window[1] * 2.0);
            let ray = (cam.axes.x_axis - cam.axes.y_axis * nx * half * cam.aspect + cam.axes.z_axis * ny * half).normalize();
            let pos = cam.pos + ray * (m.depth / ray.dot(cam.axes.x_axis).max(MIN_RAY_COS));
            let model = Mat4::from_scale_rotation_translation(Vec3::splat(m.scale * size_k), cam_rot * rotation(m.angles), pos);
            set_blend(gl, m.blend);
            let shade = Shade { unlit: true, cutoff: 0.01, ..Default::default() };
            gfx.draw_model(mesh, &cam.view_proj, &model, asset.skin(m.skin), &shade, &Lights::default());
        }
        unsafe {
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
    }
}
