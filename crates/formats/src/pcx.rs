//! ZSoft PCX reader (8-bit palettized with trailing 256-colour palette, or 24-bit planar).

use alloc::{vec::Vec, format};

use crate::image::RgbaImage;
use crate::reader::Reader;
use crate::{Error, Result};

pub struct Pcx {
    pub image: RgbaImage,
    /// Raw palette indices for 8-bit images (useful for colour-key transparency, index 0).
    pub indices: Option<Vec<u8>>,
}

pub fn parse(data: &[u8]) -> Result<Pcx> {
    let mut r = Reader::new(data);
    if r.u8()? != 0x0a {
        return Err(Error::BadMagic { expected: "PCX", found: data[..1].to_vec() });
    }
    let _version = r.u8()?;
    let encoding = r.u8()?;
    let bpp = r.u8()?;
    let (xmin, ymin, xmax, ymax) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?);
    r.seek(65);
    let planes = r.u8()? as usize;
    let bytes_per_line = r.u16()? as usize;
    let width = (xmax - xmin + 1) as u32;
    let height = (ymax - ymin + 1) as u32;
    if bpp != 8 || !(planes == 1 || planes == 3) {
        return Err(Error::Invalid(format!("unsupported PCX: {bpp} bpp, {planes} planes")));
    }

    // Decode RLE scanlines.
    let line_len = bytes_per_line * planes;
    let total = line_len * height as usize;
    let mut raw = Vec::with_capacity(total);
    r.seek(128);
    while raw.len() < total {
        let b = r.u8()?;
        if encoding == 1 && b >= 0xc0 {
            let v = r.u8()?;
            raw.extend(core::iter::repeat_n(v, (b & 0x3f) as usize));
        } else {
            raw.push(b);
        }
    }
    raw.truncate(total);

    if planes == 1 {
        if data.len() < 769 || data[data.len() - 769] != 0x0c {
            return Err(Error::Invalid("8-bit PCX missing palette".into()));
        }
        let mut palette = [[0u8; 3]; 256];
        for (i, c) in data[data.len() - 768..].chunks_exact(3).enumerate() {
            palette[i] = [c[0], c[1], c[2]];
        }
        let indices: Vec<u8> = raw
            .chunks_exact(bytes_per_line)
            .flat_map(|line| &line[..width as usize])
            .copied()
            .collect();
        let image = RgbaImage::from_indexed(width, height, &indices, &palette);
        Ok(Pcx { image, indices: Some(indices) })
    } else {
        let mut image = RgbaImage::new(width, height);
        for (y, line) in raw.chunks_exact(line_len).enumerate() {
            for x in 0..width as usize {
                let o = (y * width as usize + x) * 4;
                for c in 0..3 {
                    image.data[o + c] = line[c * bytes_per_line + x];
                }
                image.data[o + 3] = 255;
            }
        }
        Ok(Pcx { image, indices: None })
    }
}
