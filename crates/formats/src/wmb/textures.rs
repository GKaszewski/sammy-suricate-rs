//! The texture lump: names, sizes and mip 0 decoded to RGBA.

use alloc::{format, vec::Vec};

use super::*;
use crate::image::RgbaImage;
use crate::reader::Reader;
use crate::{Error, Result};

pub(super) fn parse_textures(b: &[u8], palette: Option<&[[u8; 3]; 256]>) -> Result<Vec<Texture>> {
    if b.is_empty() {
        return Ok(Vec::new());
    }
    let mut r = Reader::new(b);
    let count = r.u32()? as usize;
    if count > 4096 {
        return Err(Error::Invalid(format!("texture count {count}")));
    }
    let offsets: Vec<u32> = (0..count).map(|_| r.u32()).collect::<Result<_>>()?;
    let grey: [[u8; 3]; 256] = core::array::from_fn(|i| [i as u8; 3]);
    let palette = palette.unwrap_or(&grey);
    offsets
        .iter()
        .map(|&off| {
            let base = off as usize;
            let mut r = Reader::at(b, base);
            let name = r.cstr(16)?;
            let width = r.u32()?;
            let height = r.u32()?;
            let mips = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
            let px = width as usize * height as usize;
            if width == 0 || height == 0 || width > 4096 || height > 4096 {
                return Err(Error::Invalid(format!("texture {name:?} size {width}x{height}")));
            }
            // Pixel depth follows from the distance between mip 0 and mip 1.
            let bits = match mips[1].wrapping_sub(mips[0]) as usize {
                d if d == px * 2 => 16,
                d if d == px => 8,
                d => return Err(Error::Invalid(format!("texture {name:?}: mip stride {d} for {width}x{height}"))),
            };
            let mut r = Reader::at(b, base + mips[0] as usize);
            let (image, indices) = if bits == 16 {
                let raw = r.bytes(px * 2)?;
                let px = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]]));
                (RgbaImage::from_rgb565(width, height, px), None)
            } else {
                let idx = r.bytes(px)?;
                (RgbaImage::from_indexed(width, height, idx, palette), Some(idx.to_vec()))
            };
            Ok(Texture { name, width, height, bits, image, indices })
        })
        .collect()
}
