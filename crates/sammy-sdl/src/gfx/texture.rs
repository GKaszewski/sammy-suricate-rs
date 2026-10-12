//! Textures: uploads, lightmaps and binding.

use super::*;

/// A GPU texture and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texture {
    pub tex: glow::Texture,
    pub width: u32,
    pub height: u32,
    /// Some texel is not fully opaque (colour-keyed): drawn with the `discard` variant.
    pub alpha: bool,
}

/// Uploads an RGBA image as a 2D texture.
pub(super) fn upload(gl: &glow::Context, img: &formats::image::RgbaImage, repeat: bool, linear: bool) -> Texture {
    unsafe {
        let tex = gl.create_texture().expect("texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        let wrap = if repeat { glow::REPEAT } else { glow::CLAMP_TO_EDGE } as i32;
        let filter = if linear { glow::LINEAR } else { glow::NEAREST } as i32;
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            img.width as i32,
            img.height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&img.data)),
        );
        Texture { tex, width: img.width, height: img.height, alpha: img.data.as_chunks::<4>().0.iter().any(|p| p[3] < 255) }
    }
}

impl Gfx {

    /// Uploads an image (`repeat` for level surfaces, clamped otherwise).
    pub fn texture(&self, img: &formats::image::RgbaImage, repeat: bool, linear: bool) -> Texture {
        upload(&self.gl, img, repeat, linear)
    }

    /// Uploads a level lightmap, gamma-encoded for the level shader.
    pub fn lightmap(&self, img: &formats::image::RgbaImage) -> Texture {
        let lut: Vec<u8> = (0..=255).map(|v| ((v as f32 / 255.0 * LIGHTMAP_SCALE).powf(1.0 / 2.2) / GAMMA_MAX * 255.0).round() as u8).collect();
        let mut img = img.clone();
        for px in img.data.as_chunks_mut::<4>().0.iter_mut() {
            for c in &mut px[..3] {
                *c = lut[*c as usize];
            }
        }
        upload(&self.gl, &img, false, true)
    }

    pub fn delete_texture(&self, t: Texture) {
        unsafe { self.gl.delete_texture(t.tex) };
    }

    pub(super) fn bind(&self, unit: u32, t: Texture) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(t.tex));
        }
    }
}
