//! 2D drawing in HUD pixels: textured quads (image regions) over the 3D view, scaled by
//! the whole-number UI scale.

use glam::Mat4;
use glow::HasContext;

use crate::gfx::{Gfx, Layout, Mesh, Texture};

/// Draws image regions; one per call (the HUD and menus draw a few dozen).
pub struct Painter {
    quad: Mesh,
    proj: Mat4,
    /// HUD pixels per window pixel, and the screen in HUD pixels.
    pub scale: f32,
    pub size: [f32; 2],
}

impl Painter {
    pub fn new(gfx: &Gfx) -> Self {
        Self { quad: Mesh::new(&gfx.gl, Layout::Ui, &[0.0; 16], &[0, 1, 2, 0, 2, 3], true), proj: Mat4::IDENTITY, scale: 1.0, size: [0.0; 2] }
    }

    /// Sizes the 2D canvas for a `width × height` pixel screen at UI `scale`.
    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.scale = scale;
        self.size = [width / scale, height / scale];
        // HUD pixels → clip space (y down).
        self.proj = Mat4::orthographic_rh_gl(0.0, self.size[0], self.size[1], 0.0, -1.0, 1.0);
    }

    /// Starts a 2D pass.
    pub fn begin(&self, gfx: &Gfx) {
        unsafe {
            gfx.gl.disable(glow::DEPTH_TEST);
            gfx.gl.disable(glow::CULL_FACE);
            gfx.gl.enable(glow::BLEND);
            gfx.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
    }

    pub fn end(&self, gfx: &Gfx) {
        unsafe { gfx.gl.disable(glow::BLEND) };
    }

    /// `src` (image pixels, None = whole) drawn at `dst` (`[x, y, w, h]` HUD pixels).
    pub fn image(&self, gfx: &Gfx, tex: Texture, src: Option<[f32; 4]>, dst: [f32; 4], color: [f32; 4]) {
        let [sx, sy, sw, sh] = src.unwrap_or([0.0, 0.0, tex.width as f32, tex.height as f32]);
        let (tw, th) = (tex.width as f32, tex.height as f32);
        let (u0, v0, u1, v1) = (sx / tw, sy / th, (sx + sw) / tw, (sy + sh) / th);
        let [x, y, w, h] = dst;
        #[rustfmt::skip]
        let v = [x, y, u0, v0,  x + w, y, u1, v0,  x + w, y + h, u1, v1,  x, y + h, u0, v1];
        self.quad.update(&gfx.gl, &v);
        gfx.draw_ui(&self.quad, &self.proj, tex, color);
    }

    /// A plain rectangle.
    pub fn rect(&self, gfx: &Gfx, dst: [f32; 4], color: [f32; 4]) {
        self.image(gfx, gfx.white, None, dst, color);
    }
}
