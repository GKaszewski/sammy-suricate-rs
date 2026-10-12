//! Decoding an `MDL3` / `MDL5` file.

use alloc::{format, vec::Vec};

use super::*;
use crate::image::RgbaImage;
use crate::reader::Reader;
use crate::{Error, Result};

const HEADER_SIZE: usize = 0x54;

/// Errors out early if `count` items of `size` bytes cannot fit in what is left,
/// so corrupt counts never trigger huge allocations.
fn check_fits(r: &Reader, count: usize, size: usize) -> Result<()> {
    match count.checked_mul(size) {
        Some(n) if n <= r.remaining() => Ok(()),
        _ => Err(Error::Eof(r.pos())),
    }
}

fn count(r: &mut Reader, what: &str) -> Result<usize> {
    let v = r.i32()?;
    usize::try_from(v).map_err(|_| Error::Invalid(format!("negative {what}: {v}")))
}

pub fn parse(data: &[u8]) -> Result<Mdl> {
    let mut r = Reader::new(data);
    let magic = r.array::<4>()?;
    let version = match &magic {
        b"MDL3" => Version::Mdl3,
        b"MDL5" => Version::Mdl5,
        _ => return Err(Error::BadMagic { expected: "MDL3/MDL5", found: magic.to_vec() }),
    };
    let _unused = r.u32()?; // always 0
    let scale = r.vec3()?;
    let translate = r.vec3()?;
    let bounding_radius = r.f32()?;
    let eye_position = r.vec3()?;
    let num_skins = count(&mut r, "skin count")?;
    let skin_w = count(&mut r, "skin width")?;
    let skin_h = count(&mut r, "skin height")?;
    let num_verts = count(&mut r, "vertex count")?;
    let num_tris = count(&mut r, "triangle count")?;
    let num_frames = count(&mut r, "frame count")?;
    let num_skin_verts = count(&mut r, "skin vertex count")?;
    let flags = r.u32()?;
    let _unused2 = r.u32()?; // Quake's `size`; always 0
    debug_assert_eq!(r.pos(), HEADER_SIZE);

    // Skins.
    check_fits(&r, num_skins, 4)?;
    let mut skins = Vec::with_capacity(num_skins);
    for _ in 0..num_skins {
        let ty = r.u32()?;
        let (w, h) = match version {
            Version::Mdl3 => (skin_w, skin_h),
            Version::Mdl5 => (count(&mut r, "skin width")?, count(&mut r, "skin height")?),
        };
        let px = w.checked_mul(h).ok_or(Error::Eof(r.pos()))?;
        let (w32, h32) = (w as u32, h as u32);
        let skin = match ty {
            0 => {
                let idx = r.bytes(px)?.to_vec();
                let mut gray = [[0u8; 3]; 256];
                for (i, g) in gray.iter_mut().enumerate() {
                    *g = [i as u8; 3];
                }
                Skin { format: SkinFormat::Indexed8, image: RgbaImage::from_indexed(w32, h32, &idx, &gray), indices: Some(idx) }
            }
            2 | 3 => {
                check_fits(&r, px, 2)?;
                let raw = r.bytes(px * 2)?;
                let words = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]]));
                if ty == 2 {
                    Skin { format: SkinFormat::Rgb565, image: RgbaImage::from_rgb565(w32, h32, words), indices: None }
                } else {
                    let data = words
                        .flat_map(|p| {
                            let n = |s: u16| ((p >> s) & 0xf) as u8 * 17;
                            [n(8), n(4), n(0), n(12)]
                        })
                        .collect();
                    Skin { format: SkinFormat::Argb4444, image: RgbaImage { width: w32, height: h32, data }, indices: None }
                }
            }
            _ => return Err(Error::Invalid(format!("unsupported skin type {ty} at {:#x}", r.pos() - 4))),
        };
        skins.push(skin);
    }
    let skin_size = match (version, skins.first()) {
        (Version::Mdl5, Some(s)) => [s.image.width, s.image.height],
        _ => [skin_w as u32, skin_h as u32],
    };

    // Skin vertices (pixel coordinates).
    check_fits(&r, num_skin_verts, 4)?;
    let mut skin_verts = Vec::with_capacity(num_skin_verts);
    for _ in 0..num_skin_verts {
        skin_verts.push([r.i16()?, r.i16()?]);
    }
    let (sw, sh) = (skin_size[0].max(1) as f32, skin_size[1].max(1) as f32);
    let uvs = skin_verts.iter().map(|&[u, v]| [u as f32 / sw, v as f32 / sh]).collect();

    // Triangles.
    check_fits(&r, num_tris, 12)?;
    let mut triangles = Vec::with_capacity(num_tris);
    for _ in 0..num_tris {
        let xyz = [r.u16()?, r.u16()?, r.u16()?];
        let uv = [r.u16()?, r.u16()?, r.u16()?];
        if xyz.iter().any(|&i| i as usize >= num_verts) || uv.iter().any(|&i| i as usize >= num_skin_verts) {
            return Err(Error::Invalid(format!("triangle index out of range at {:#x}", r.pos() - 12)));
        }
        triangles.push(Triangle { xyz, uv });
    }

    // Frames.
    check_fits(&r, num_frames, 4 + 8 + 16 + num_verts.saturating_mul(4))?;
    let decode = |p: [u16; 3]| -> [f32; 3] { core::array::from_fn(|k| scale[k] * p[k] as f32 + translate[k]) };
    let mut frames = Vec::with_capacity(num_frames);
    for _ in 0..num_frames {
        let ty = r.u32()?;
        let wide = match ty {
            0 => false,
            2 => true,
            _ => return Err(Error::Invalid(format!("unsupported frame type {ty} at {:#x}", r.pos() - 4))),
        };
        let vert = |r: &mut Reader| -> Result<([u16; 3], u8)> {
            if wide {
                let p = [r.u16()?, r.u16()?, r.u16()?];
                let n = r.u8()?;
                let _pad = r.u8()?;
                Ok((p, n))
            } else {
                let b = r.array::<4>()?;
                Ok(([b[0] as u16, b[1] as u16, b[2] as u16], b[3]))
            }
        };
        let bbox_min = decode(vert(&mut r)?.0);
        let bbox_max = decode(vert(&mut r)?.0);
        let name = r.cstr(16)?;
        check_fits(&r, num_verts, if wide { 8 } else { 4 })?;
        let mut positions = Vec::with_capacity(num_verts);
        let mut normals = Vec::with_capacity(num_verts);
        let mut normal_indices = Vec::with_capacity(num_verts);
        for _ in 0..num_verts {
            let (p, n) = vert(&mut r)?;
            positions.push(decode(p));
            let [x, y, z] = *NORMALS.get(n as usize).unwrap_or(&[0.0, 0.0, -1.0]);
            normals.push([-x, -y, -z]);
            normal_indices.push(n);
        }
        frames.push(Frame { name, bbox_min, bbox_max, positions, normals, normal_indices });
    }
    if r.remaining() != 0 {
        return Err(Error::Invalid(format!("{} trailing bytes after frames", r.remaining())));
    }

    Ok(Mdl {
        version,
        scale,
        translate,
        bounding_radius,
        eye_position,
        flags,
        skin_size,
        skins,
        skin_verts,
        uvs,
        triangles,
        frames,
    })
}
