//! Conitec A4/A5 vertex-animated models (`MDL3` / `MDL5`), a Quake 1 MDL derivative.
//!
//! See `docs/formats/mdl3.md` for the byte layout. Everything is returned in native
//! Acknex coordinates (X forward, Y left, Z up, quants).
//!
//! Winding: as in Quake, stored triangles are *clockwise* when seen from their
//! visible side (in this right-handed frame), and the stored normal indices point
//! the same way as `(b-a)x(c-a)`, i.e. *into* the surface. [`Frame::normals`] is
//! therefore negated (outward) and [`Mdl::vertex_buffer`] emits counter-clockwise
//! triangles, so both follow the usual (glTF/Bevy) convention.

use alloc::{vec::Vec, string::String, string::ToString, format};

use alloc::collections::BTreeMap;

use crate::image::RgbaImage;
use crate::reader::Reader;
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// A4 (`MDL3`): skin size in header, 8-bit frame vertices.
    Mdl3,
    /// A5 (`MDL5`): per-skin size, frames may use 16-bit vertices.
    Mdl5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinFormat {
    /// Type 0: 8-bit palette indices. No palette is stored in the file; `Skin::image`
    /// is a grayscale stand-in and the raw indices are kept in `Skin::indices`.
    Indexed8,
    /// Type 2: 16-bit RGB565.
    Rgb565,
    /// Type 3: 16-bit ARGB4444 (only seen in MDL5 files).
    Argb4444,
}

#[derive(Clone, Debug)]
pub struct Skin {
    pub format: SkinFormat,
    pub image: RgbaImage,
    /// Raw palette indices for `SkinFormat::Indexed8`.
    pub indices: Option<Vec<u8>>,
}

/// One triangle exactly as stored (clockwise front face, see module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle {
    /// Indices into each frame's `positions`/`normals`.
    pub xyz: [u16; 3],
    /// Indices into `Mdl::skin_verts` / `Mdl::uvs`.
    pub uv: [u16; 3],
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub name: String,
    /// Decoded bounding box (scale * packed + translate).
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub positions: Vec<[f32; 3]>,
    /// Outward unit normals: `-NORMALS[normal_indices[i]]`.
    pub normals: Vec<[f32; 3]>,
    pub normal_indices: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Mdl {
    pub version: Version,
    pub scale: [f32; 3],
    pub translate: [f32; 3],
    pub bounding_radius: f32,
    pub eye_position: [f32; 3],
    pub flags: u32,
    /// Skin size used to normalise UVs (header value for MDL3, first skin for MDL5).
    pub skin_size: [u32; 2],
    pub skins: Vec<Skin>,
    /// Texture coordinates in skin pixels, as stored.
    pub skin_verts: Vec<[i16; 2]>,
    /// `skin_verts` divided by `skin_size` (0..1, origin top-left, as glTF).
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<Triangle>,
    pub frames: Vec<Frame>,
}

/// Frames sharing a name prefix (trailing digits/whitespace stripped), e.g.
/// `walk01`..`walk32` -> `walk`. This is what `set_cycle`/`set_frame` address.
#[derive(Clone, Debug)]
pub struct Animation {
    pub name: String,
    /// Frame indices in file order.
    pub frames: Vec<usize>,
}

/// Render-ready vertex layout: one vertex per unique (xyz index, uv index) pair.
#[derive(Clone, Debug)]
pub struct VertexBuffer {
    /// For each output vertex, the index into a frame's `positions`/`normals`.
    pub xyz_index: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    /// Triangle list into the output vertices (CCW front faces).
    pub indices: Vec<u32>,
}

impl VertexBuffer {
    pub fn positions(&self, frame: &Frame) -> Vec<[f32; 3]> {
        self.xyz_index.iter().map(|&i| frame.positions[i as usize]).collect()
    }
    pub fn normals(&self, frame: &Frame) -> Vec<[f32; 3]> {
        self.xyz_index.iter().map(|&i| frame.normals[i as usize]).collect()
    }
}

/// Strips trailing digits and whitespace: `"walk01"` -> `"walk"`, `"Frame 1"` -> `"Frame"`.
pub fn frame_group(name: &str) -> &str {
    name.trim_end_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
}

impl Mdl {
    /// Groups frames by `frame_group(name)` in order of first appearance. Grouping is
    /// case-sensitive on the stored name; Acknex itself matches names case-insensitively.
    pub fn animations(&self) -> Vec<Animation> {
        let mut out: Vec<Animation> = Vec::new();
        let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
        for (i, f) in self.frames.iter().enumerate() {
            let key = frame_group(&f.name);
            let slot = *by_name.entry(key).or_insert_with(|| {
                out.push(Animation { name: key.to_string(), frames: Vec::new() });
                out.len() - 1
            });
            out[slot].frames.push(i);
        }
        out
    }

    pub fn vertex_buffer(&self) -> VertexBuffer {
        let mut map: BTreeMap<(u16, u16), u32> = BTreeMap::new();
        let mut vb = VertexBuffer { xyz_index: Vec::new(), uvs: Vec::new(), indices: Vec::new() };
        for t in &self.triangles {
            // Reverse the stored (clockwise) order to get CCW front faces.
            for k in [0, 2, 1] {
                let key = (t.xyz[k], t.uv[k]);
                let idx = *map.entry(key).or_insert_with(|| {
                    vb.xyz_index.push(key.0 as u32);
                    vb.uvs.push(self.uvs[key.1 as usize]);
                    vb.xyz_index.len() as u32 - 1
                });
                vb.indices.push(idx);
            }
        }
        vb
    }
}

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
                let words = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
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

/// Quake's 162 precomputed vertex normals (`anorms.h`), indexed by the 4th byte of
/// each frame vertex. Verified against face normals of the Sammy models; note the
/// stored indices point inward (see module docs), so `Frame::normals` negates them.
pub const NORMALS: [[f32; 3]; 162] = [
    [-0.525731, 0.000000, 0.850651], [-0.442863, 0.238856, 0.864188], [-0.295242, 0.000000, 0.955423],
    [-0.309017, 0.500000, 0.809017], [-0.162460, 0.262866, 0.951056], [0.000000, 0.000000, 1.000000],
    [0.000000, 0.850651, 0.525731], [-0.147621, 0.716567, 0.681718], [0.147621, 0.716567, 0.681718],
    [0.000000, 0.525731, 0.850651], [0.309017, 0.500000, 0.809017], [0.525731, 0.000000, 0.850651],
    [0.295242, 0.000000, 0.955423], [0.442863, 0.238856, 0.864188], [0.162460, 0.262866, 0.951056],
    [-0.681718, 0.147621, 0.716567], [-0.809017, 0.309017, 0.500000], [-0.587785, 0.425325, 0.688191],
    [-0.850651, 0.525731, 0.000000], [-0.864188, 0.442863, 0.238856], [-0.716567, 0.681718, 0.147621],
    [-0.688191, 0.587785, 0.425325], [-0.500000, 0.809017, 0.309017], [-0.238856, 0.864188, 0.442863],
    [-0.425325, 0.688191, 0.587785], [-0.716567, 0.681718, -0.147621], [-0.500000, 0.809017, -0.309017],
    [-0.525731, 0.850651, 0.000000], [0.000000, 0.850651, -0.525731], [-0.238856, 0.864188, -0.442863],
    [0.000000, 0.955423, -0.295242], [-0.262866, 0.951056, -0.162460], [0.000000, 1.000000, 0.000000],
    [0.000000, 0.955423, 0.295242], [-0.262866, 0.951056, 0.162460], [0.238856, 0.864188, 0.442863],
    [0.262866, 0.951056, 0.162460], [0.500000, 0.809017, 0.309017], [0.238856, 0.864188, -0.442863],
    [0.262866, 0.951056, -0.162460], [0.500000, 0.809017, -0.309017], [0.850651, 0.525731, 0.000000],
    [0.716567, 0.681718, 0.147621], [0.716567, 0.681718, -0.147621], [0.525731, 0.850651, 0.000000],
    [0.425325, 0.688191, 0.587785], [0.864188, 0.442863, 0.238856], [0.688191, 0.587785, 0.425325],
    [0.809017, 0.309017, 0.500000], [0.681718, 0.147621, 0.716567], [0.587785, 0.425325, 0.688191],
    [0.955423, 0.295242, 0.000000], [1.000000, 0.000000, 0.000000], [0.951056, 0.162460, 0.262866],
    [0.850651, -0.525731, 0.000000], [0.955423, -0.295242, 0.000000], [0.864188, -0.442863, 0.238856],
    [0.951056, -0.162460, 0.262866], [0.809017, -0.309017, 0.500000], [0.681718, -0.147621, 0.716567],
    [0.850651, 0.000000, 0.525731], [0.864188, 0.442863, -0.238856], [0.809017, 0.309017, -0.500000],
    [0.951056, 0.162460, -0.262866], [0.525731, 0.000000, -0.850651], [0.681718, 0.147621, -0.716567],
    [0.681718, -0.147621, -0.716567], [0.850651, 0.000000, -0.525731], [0.809017, -0.309017, -0.500000],
    [0.864188, -0.442863, -0.238856], [0.951056, -0.162460, -0.262866], [0.147621, 0.716567, -0.681718],
    [0.309017, 0.500000, -0.809017], [0.425325, 0.688191, -0.587785], [0.442863, 0.238856, -0.864188],
    [0.587785, 0.425325, -0.688191], [0.688191, 0.587785, -0.425325], [-0.147621, 0.716567, -0.681718],
    [-0.309017, 0.500000, -0.809017], [0.000000, 0.525731, -0.850651], [-0.525731, 0.000000, -0.850651],
    [-0.442863, 0.238856, -0.864188], [-0.295242, 0.000000, -0.955423], [-0.162460, 0.262866, -0.951056],
    [0.000000, 0.000000, -1.000000], [0.295242, 0.000000, -0.955423], [0.162460, 0.262866, -0.951056],
    [-0.442863, -0.238856, -0.864188], [-0.309017, -0.500000, -0.809017], [-0.162460, -0.262866, -0.951056],
    [0.000000, -0.850651, -0.525731], [-0.147621, -0.716567, -0.681718], [0.147621, -0.716567, -0.681718],
    [0.000000, -0.525731, -0.850651], [0.309017, -0.500000, -0.809017], [0.442863, -0.238856, -0.864188],
    [0.162460, -0.262866, -0.951056], [0.238856, -0.864188, -0.442863], [0.500000, -0.809017, -0.309017],
    [0.425325, -0.688191, -0.587785], [0.716567, -0.681718, -0.147621], [0.688191, -0.587785, -0.425325],
    [0.587785, -0.425325, -0.688191], [0.000000, -0.955423, -0.295242], [0.000000, -1.000000, 0.000000],
    [0.262866, -0.951056, -0.162460], [0.000000, -0.850651, 0.525731], [0.000000, -0.955423, 0.295242],
    [0.238856, -0.864188, 0.442863], [0.262866, -0.951056, 0.162460], [0.500000, -0.809017, 0.309017],
    [0.716567, -0.681718, 0.147621], [0.525731, -0.850651, 0.000000], [-0.238856, -0.864188, -0.442863],
    [-0.500000, -0.809017, -0.309017], [-0.262866, -0.951056, -0.162460], [-0.850651, -0.525731, 0.000000],
    [-0.716567, -0.681718, -0.147621], [-0.716567, -0.681718, 0.147621], [-0.525731, -0.850651, 0.000000],
    [-0.500000, -0.809017, 0.309017], [-0.238856, -0.864188, 0.442863], [-0.262866, -0.951056, 0.162460],
    [-0.864188, -0.442863, 0.238856], [-0.809017, -0.309017, 0.500000], [-0.688191, -0.587785, 0.425325],
    [-0.681718, -0.147621, 0.716567], [-0.442863, -0.238856, 0.864188], [-0.587785, -0.425325, 0.688191],
    [-0.309017, -0.500000, 0.809017], [-0.147621, -0.716567, 0.681718], [-0.425325, -0.688191, 0.587785],
    [-0.162460, -0.262866, 0.951056], [0.442863, -0.238856, 0.864188], [0.162460, -0.262866, 0.951056],
    [0.309017, -0.500000, 0.809017], [0.147621, -0.716567, 0.681718], [0.000000, -0.525731, 0.850651],
    [0.425325, -0.688191, 0.587785], [0.587785, -0.425325, 0.688191], [0.688191, -0.587785, 0.425325],
    [-0.955423, 0.295242, 0.000000], [-0.951056, 0.162460, 0.262866], [-1.000000, 0.000000, 0.000000],
    [-0.850651, 0.000000, 0.525731], [-0.955423, -0.295242, 0.000000], [-0.951056, -0.162460, 0.262866],
    [-0.864188, 0.442863, -0.238856], [-0.951056, 0.162460, -0.262866], [-0.809017, 0.309017, -0.500000],
    [-0.864188, -0.442863, -0.238856], [-0.951056, -0.162460, -0.262866], [-0.809017, -0.309017, -0.500000],
    [-0.681718, 0.147621, -0.716567], [-0.681718, -0.147621, -0.716567], [-0.850651, 0.000000, -0.525731],
    [-0.688191, 0.587785, -0.425325], [-0.587785, 0.425325, -0.688191], [-0.425325, 0.688191, -0.587785],
    [-0.425325, -0.688191, -0.587785], [-0.587785, -0.425325, -0.688191], [-0.688191, -0.587785, -0.425325],
];
