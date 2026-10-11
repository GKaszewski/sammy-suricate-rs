//! The intro's logo.

use super::*;

impl Scene {
    /// The intro's logo in front of everything (`None` drops it).
    pub fn draw_logo(&mut self, gfx: &Gfx, files: &GameFiles, cam: &Camera, logo: Option<&sammy_present::intro::Logo>) {
        use sammy_present::intro::LogoArt;
        let Some(l) = logo else {
            if let Some((_, mesh, tex)) = self.logo.take() {
                mesh.delete(&gfx.gl);
                if tex != gfx.white {
                    gfx.delete_texture(tex);
                }
            }
            return;
        };
        if self.logo.as_ref().is_none_or(|(id, ..)| *id != l.id) {
            self.draw_logo(gfx, files, cam, None);
            self.logo = match l.art {
                LogoArt::Image(file) => files.read(file).ok().and_then(|d| formats::tga::parse(&d)).map(|img| {
                    let (w, h) = (img.width as f32 / 2.0, img.height as f32 / 2.0);
                    (l.id, quad(gfx, w, h, 1.0), gfx.texture(&img, false, true))
                }),
                LogoArt::Model(file) => self.model(gfx, files, file).map(|a| {
                    // Its own copy of the first skin: the logo is deleted with it.
                    let tex = a.mdl.skins.first().map_or(gfx.white, |s| gfx.texture(&s.image, false, true));
                    (l.id, Mesh::new(&gfx.gl, Layout::Model, &a.vertices((0, 0, 0.0)), &a.vb.indices, false), tex)
                }),
            };
        }
        let Some((_, mesh, tex)) = &self.logo else { return };
        let gl = &gfx.gl;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        // Both kinds face the camera (the quad faces -X, the axes turn X to forward).
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(l.scale), Quat::from_mat3(&cam.axes), Vec3::from(l.pos));
        let shade = Shade { color: [1.0, 1.0, 1.0, l.alpha], unlit: true, cutoff: 0.01, ..Default::default() };
        gfx.draw_model(mesh, &cam.view_proj, &m, *tex, &shade, &Lights::default());
        unsafe { gl.disable(glow::BLEND) };
    }
}
