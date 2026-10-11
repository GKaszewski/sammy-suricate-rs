//! Uncompressed true-colour TGA (24/32-bit), as used for the intro's A5 logo.

use crate::image::{RgbaImage, is_color_key};

/// Header length, image type 2 (uncompressed true colour) and the descriptor's
/// top-down bit.
const HEADER_LEN: usize = 18;
const TRUECOLOR: u8 = 2;
const TOP_DOWN_BIT: u8 = 0x20;

/// Decodes an uncompressed 24/32-bit TGA (bottom-up unless the descriptor says
/// otherwise); the A4 colour key (near-black) is transparent. `None` if it is another
/// kind of TGA or truncated.
pub fn parse(d: &[u8]) -> Option<RgbaImage> {
    if d.len() < HEADER_LEN || d[2] != TRUECOLOR {
        return None;
    }
    let (w, h) = (u16::from_le_bytes([d[12], d[13]]) as u32, u16::from_le_bytes([d[14], d[15]]) as u32);
    let bpp = d[16] as usize / 8;
    if bpp < 3 {
        return None;
    }
    let top_down = d[17] & TOP_DOWN_BIT != 0;
    let data = d.get(HEADER_LEN + d[0] as usize..)?;
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        let sy = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let s = ((sy * w + x) as usize) * bpp;
            let p = data.get(s..s + bpp)?;
            let rgb = [p[2], p[1], p[0]];
            let i = ((y * w + x) * 4) as usize;
            img.data[i..i + 3].copy_from_slice(&rgb);
            img.data[i + 3] = if is_color_key(&rgb) { 0 } else { 255 };
        }
    }
    Some(img)
}
