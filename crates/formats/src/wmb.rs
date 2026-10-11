//! WMB4 level / map-entity files (Acknex A4 BSP, a Quake BSP29 derivative).
//!
//! See `docs/formats/wmb4.md` for the byte-level layout. All data is kept in the
//! engine's native coordinate system (X forward, Y left, Z up, quants).
//!
//! [`parse`] decodes every lump into plain structs; [`Wmb::build_mesh`] turns the
//! faces into renderable per-texture triangle batches with a packed lightmap atlas.

use alloc::{vec::Vec, string::String, vec, format};

use crate::image::RgbaImage;
use crate::reader::Reader;
use crate::{Error, Result};

/// Lump indices in the 16-entry directory that follows the magic.
pub mod lump {
    pub const PALETTE: usize = 0;
    pub const PLANES: usize = 1;
    pub const TEXTURES: usize = 2;
    pub const VERTICES: usize = 3;
    pub const VIS: usize = 4;
    pub const NODES: usize = 5;
    pub const TEXINFO: usize = 6;
    pub const FACES: usize = 7;
    pub const LIGHTMAPS: usize = 8;
    pub const CLIPNODES: usize = 9;
    pub const LEAVES: usize = 10;
    pub const MARKSURFACES: usize = 11;
    pub const EDGES: usize = 12;
    pub const SURFEDGES: usize = 13;
    pub const MODELS: usize = 14;
    pub const ENTITIES: usize = 15;
    pub const COUNT: usize = 16;
}

/// Texinfo flag bits (meaning inferred from usage statistics; see docs).
pub mod surface_flags {
    /// Face has no lightmap ("flat" surface, lit by sun/ambient only).
    pub const FLAT: u32 = 1;
    /// Unknown; set on most flat surfaces of prefabs and decorative brushes.
    pub const UNKNOWN_2: u32 = 2;
    /// Not rendered (dummy/clip brushes, outer level hull, kill zones).
    pub const INVISIBLE: u32 = 8;
    /// Unknown; appears both on lit and flat surfaces.
    pub const UNKNOWN_16: u32 = 16;
    /// Unknown; seen once (`STONE005_128`, flags 131).
    pub const UNKNOWN_128: u32 = 128;
}

/// Entity flag bits. FLAG1..FLAG8 are confirmed by the game scripts; the higher
/// bits follow the later Acknex `atypes.h` order and match the observed data.
pub mod entity_flags {
    pub const FLAG1: u32 = 1 << 0;
    pub const FLAG2: u32 = 1 << 1;
    pub const FLAG3: u32 = 1 << 2;
    pub const FLAG4: u32 = 1 << 3;
    pub const FLAG5: u32 = 1 << 4;
    pub const FLAG6: u32 = 1 << 5;
    pub const FLAG7: u32 = 1 << 6;
    pub const FLAG8: u32 = 1 << 7;
    pub const INVISIBLE: u32 = 1 << 8;
    pub const PASSABLE: u32 = 1 << 9;
    pub const TRANSPARENT: u32 = 1 << 10;
    pub const SHADOW: u32 = 1 << 18;

    /// Names for the bits above, for diagnostics/export.
    pub const NAMES: &[(u32, &str)] = &[
        (FLAG1, "flag1"),
        (FLAG2, "flag2"),
        (FLAG3, "flag3"),
        (FLAG4, "flag4"),
        (FLAG5, "flag5"),
        (FLAG6, "flag6"),
        (FLAG7, "flag7"),
        (FLAG8, "flag8"),
        (INVISIBLE, "invisible"),
        (PASSABLE, "passable"),
        (TRANSPARENT, "transparent"),
        (SHADOW, "shadow"),
    ];
}

/// Leaf/clipnode contents values.
pub mod contents {
    pub const SOLID: i32 = -128;
    pub const EMPTY: i32 = -256;
}

/// Luxel spacing in texture space (same as Quake).
pub const LUXEL_SIZE: f32 = 16.0;

#[derive(Clone, Debug)]
pub struct Wmb {
    /// Lump 0: per-level 8-bit rendering tables (starts with an RGB palette,
    /// followed by colour lookup tables; 181520 bytes when present, else empty).
    /// Not used for texture decoding; kept raw.
    pub palette_lump: Vec<u8>,
    pub planes: Vec<Plane>,
    pub textures: Vec<Texture>,
    pub vertices: Vec<[f32; 3]>,
    /// Raw PVS data (Quake run-length encoding); see [`Wmb::decompress_vis`].
    pub vis: Vec<u8>,
    pub nodes: Vec<Node>,
    pub texinfos: Vec<TexInfo>,
    pub faces: Vec<Face>,
    /// Raw RGB565 lightmap data, indexed by [`Face::light_offset`] (bytes).
    pub lightmaps: Vec<u8>,
    pub clipnodes: Vec<ClipNode>,
    pub leaves: Vec<Leaf>,
    pub mark_surfaces: Vec<u32>,
    pub edges: Vec<[u32; 2]>,
    pub surfedges: Vec<i32>,
    pub models: Vec<Model>,
    pub entities: Vec<Entity>,
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: [f32; 3],
    pub dist: f32,
    /// Quake plane type: 0/1/2 axial X/Y/Z, 3..5 non-axial.
    pub kind: i32,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// 16 (RGB565) or 8 (palettized, Quake palette).
    pub bits: u8,
    /// Mip level 0 decoded to RGBA. 8-bit textures use the palette given to
    /// [`parse_with_palette`], or a greyscale ramp if none.
    pub image: RgbaImage,
    /// Raw mip-0 palette indices for 8-bit textures (for re-colouring).
    pub indices: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug)]
pub struct TexInfo {
    /// `s = dot(pos, s[0..3]) + s[3]` in texels.
    pub s: [f32; 4],
    pub t: [f32; 4],
    pub texture: i32,
    /// See [`surface_flags`].
    pub flags: u32,
    /// Surface ambient (signed percent, -100..100).
    pub ambient: i8,
    /// Surface albedo (percent, default 50).
    pub albedo: u8,
}

impl TexInfo {
    pub fn st(&self, p: [f32; 3]) -> [f32; 2] {
        let s = p[0] * self.s[0] + p[1] * self.s[1] + p[2] * self.s[2] + self.s[3];
        let t = p[0] * self.t[0] + p[1] * self.t[1] + p[2] * self.t[2] + self.t[3];
        [s, t]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// Unknown leading u16 (small values 0..~30; not the plane index as in Quake).
    pub unknown: u16,
    /// Non-zero: face normal is the negated plane normal.
    pub side: u16,
    pub first_edge: u32,
    pub num_edges: u16,
    pub texinfo: u16,
    /// Light styles; 0 = normal lightmap, 255 = none.
    pub styles: [u8; 4],
    /// Byte offset into [`Wmb::lightmaps`], or -1.
    pub light_offset: i32,
    /// Plane index (stored last, unlike Quake).
    pub plane: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub plane: i32,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    /// >= 0: node index; < 0: leaf index `-(c + 1)`.
    pub children: [i32; 2],
    pub first_face: u32,
    pub num_faces: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ClipNode {
    pub plane: i32,
    /// >= 0: clipnode index; < 0: contents (see [`contents`]).
    pub children: [i32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Leaf {
    /// See [`contents`]; -240 also occurs (empty with an extra bit 0x10).
    pub contents: i32,
    /// Offset into [`Wmb::vis`], or -1.
    pub vis_offset: i32,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    /// Unknown bytes 20..28 (often repeat maxs.z; Quake has ambient levels here).
    pub unknown: [u8; 8],
    pub first_mark_surface: u32,
    pub num_mark_surfaces: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub origin: [f32; 3],
    /// [0] BSP node root, [1..3] clipnode roots for hulls 1..3.
    pub head_nodes: [i32; 4],
    /// Collision hull boxes (hull 0 = point, 1 = "narrow" ±16, 2 = "fat").
    pub hull_mins: [[i32; 3]; 4],
    pub hull_maxs: [[i32; 3]; 4],
    pub vis_leafs: i32,
    pub first_face: u32,
    pub num_faces: u32,
}

/// Entity-lump records.
#[derive(Clone, Debug)]
pub enum Entity {
    /// Type 1: named position marker.
    Position { origin: [f32; 3], angles: [f32; 3], name: String },
    /// Type 2: static light (baked into lightmaps).
    Light { origin: [f32; 3], color: [f32; 3], range: f32, flags: u32 },
    /// Type 3 (A4 entity) and type 7 (A5-style entity).
    Object(Object),
    /// Type 5: level info (sun angles).
    Info { azimuth: f32, elevation: f32, raw: Vec<u8> },
    Unknown { kind: u32, raw: Vec<u8> },
}

#[derive(Clone, Debug, Default)]
pub struct Object {
    /// 3 or 7.
    pub kind: u32,
    pub origin: [f32; 3],
    /// pan, tilt, roll in degrees.
    pub angles: [f32; 3],
    pub scale: [f32; 3],
    pub name: String,
    /// Model/sprite/map file (`.MDL`, `.pcx`, `.wmb`).
    pub file: String,
    pub action: String,
    /// SKILL1..SKILL8 (type 3) or SKILL1..SKILL20 (type 7).
    pub skills: Vec<f32>,
    /// See [`entity_flags`].
    pub flags: u32,
    pub ambient: f32,
    /// Type 7 only.
    pub albedo: Option<f32>,
    pub path: Option<i32>,
    pub entity2: Option<i32>,
    pub material: Option<String>,
    pub string1: Option<String>,
    pub string2: Option<String>,
}

impl Object {
    /// WDL `SkillN`, 1-based (0 when the entity has fewer skills).
    pub fn skill(&self, n: usize) -> f32 {
        self.skills.get(n.wrapping_sub(1)).copied().unwrap_or(0.0)
    }

    /// WDL `FlagN`, 1-based (1..=8).
    pub fn flag(&self, n: u32) -> bool {
        (1..=8).contains(&n) && self.flags & (1 << (n - 1)) != 0
    }
}

impl Entity {
    pub fn kind(&self) -> u32 {
        match self {
            Entity::Position { .. } => 1,
            Entity::Light { .. } => 2,
            Entity::Object(o) => o.kind,
            Entity::Info { .. } => 5,
            Entity::Unknown { kind, .. } => *kind,
        }
    }
}

/// Parses a WMB4 file; 8-bit textures are decoded with a greyscale ramp.
pub fn parse(data: &[u8]) -> Result<Wmb> {
    parse_with_palette(data, None)
}

/// Like [`parse`], decoding 8-bit textures with `palette` (the standard Quake
/// palette, as shipped with WED as `palette.raw`). Only the editor's
/// `#default` texture is 8-bit in the shipped data (5 files).
pub fn parse_with_palette(data: &[u8], palette: Option<&[[u8; 3]; 256]>) -> Result<Wmb> {
    let mut r = Reader::new(data);
    let magic = r.bytes(4)?;
    if magic != b"WMB4" {
        return Err(Error::BadMagic { expected: "WMB4", found: magic.to_vec() });
    }
    let mut lumps = [(0usize, 0usize); lump::COUNT];
    for l in &mut lumps {
        *l = (r.u32()? as usize, r.u32()? as usize);
    }
    let lump_data = |i: usize| -> Result<&[u8]> {
        let (off, len) = lumps[i];
        if len == 0 {
            return Ok(&[]);
        }
        data.get(off..off.checked_add(len).ok_or(Error::Eof(off))?)
            .ok_or_else(|| Error::Invalid(format!("lump {i} out of range ({off:#x}+{len})")))
    };
    // Reads fixed-size records with `f`.
    fn records<T>(b: &[u8], size: usize, mut f: impl FnMut(&mut Reader) -> Result<T>) -> Result<Vec<T>> {
        let mut r = Reader::new(b);
        (0..b.len() / size)
            .map(|i| {
                r.seek(i * size);
                f(&mut r)
            })
            .collect()
    }
    let i16x3 = |r: &mut Reader| -> Result<[i16; 3]> { Ok([r.i16()?, r.i16()?, r.i16()?]) };

    let palette_lump = lump_data(lump::PALETTE)?.to_vec();
    let planes = records(lump_data(lump::PLANES)?, 20, |r| {
        Ok(Plane { normal: r.vec3()?, dist: r.f32()?, kind: r.i32()? })
    })?;
    let textures = parse_textures(lump_data(lump::TEXTURES)?, palette)?;
    let vertices = records(lump_data(lump::VERTICES)?, 12, |r| r.vec3())?;
    let vis = lump_data(lump::VIS)?.to_vec();
    let nodes = records(lump_data(lump::NODES)?, 40, |r| {
        let plane = r.i32()?;
        r.u32()?; // junk (in-memory pointer left by the compiler)
        let mins = i16x3(r)?;
        let maxs = i16x3(r)?;
        r.u32()?; // junk
        Ok(Node { plane, mins, maxs, children: [r.i32()?, r.i32()?], first_face: r.u32()?, num_faces: r.u32()? })
    })?;
    let texinfos = records(lump_data(lump::TEXINFO)?, 64, |r| {
        let s = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        let t = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        let texture = r.i32()?;
        let flags = r.u32()?;
        r.u16()?;
        Ok(TexInfo { s, t, texture, flags, ambient: r.i8()?, albedo: r.u8()? })
    })?;
    let faces = records(lump_data(lump::FACES)?, 24, |r| {
        Ok(Face {
            unknown: r.u16()?,
            side: r.u16()?,
            first_edge: r.u32()?,
            num_edges: r.u16()?,
            texinfo: r.u16()?,
            styles: r.array()?,
            light_offset: r.i32()?,
            plane: r.u32()?,
        })
    })?;
    let lightmaps = lump_data(lump::LIGHTMAPS)?.to_vec();
    let clipnodes = records(lump_data(lump::CLIPNODES)?, 16, |r| {
        let plane = r.i32()?;
        r.u32()?; // junk
        Ok(ClipNode { plane, children: [r.i32()?, r.i32()?] })
    })?;
    let leaves = records(lump_data(lump::LEAVES)?, 36, |r| {
        Ok(Leaf {
            contents: r.i32()?,
            vis_offset: r.i32()?,
            mins: i16x3(r)?,
            maxs: i16x3(r)?,
            unknown: r.array()?,
            first_mark_surface: r.u32()?,
            num_mark_surfaces: r.u32()?,
        })
    })?;
    let mark_surfaces = records(lump_data(lump::MARKSURFACES)?, 4, |r| r.u32())?;
    let edges = records(lump_data(lump::EDGES)?, 8, |r| Ok([r.u32()?, r.u32()?]))?;
    let surfedges = records(lump_data(lump::SURFEDGES)?, 4, |r| r.i32())?;
    let models = records(lump_data(lump::MODELS)?, 160, |r| {
        let mins = r.vec3()?;
        let maxs = r.vec3()?;
        let origin = r.vec3()?;
        let head_nodes = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
        let mut ivec = || -> Result<[i32; 3]> { Ok([r.i32()?, r.i32()?, r.i32()?]) };
        let hull_mins = [ivec()?, ivec()?, ivec()?, ivec()?];
        let hull_maxs = [ivec()?, ivec()?, ivec()?, ivec()?];
        Ok(Model {
            mins,
            maxs,
            origin,
            head_nodes,
            hull_mins,
            hull_maxs,
            vis_leafs: r.i32()?,
            first_face: r.u32()?,
            num_faces: r.u32()?,
        })
    })?;
    let entities = parse_entities(lump_data(lump::ENTITIES)?)?;

    let wmb = Wmb {
        palette_lump,
        planes,
        textures,
        vertices,
        vis,
        nodes,
        texinfos,
        faces,
        lightmaps,
        clipnodes,
        leaves,
        mark_surfaces,
        edges,
        surfedges,
        models,
        entities,
    };
    wmb.validate()?;
    Ok(wmb)
}

fn parse_textures(b: &[u8], palette: Option<&[[u8; 3]; 256]>) -> Result<Vec<Texture>> {
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
                let px = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
                (RgbaImage::from_rgb565(width, height, px), None)
            } else {
                let idx = r.bytes(px)?;
                (RgbaImage::from_indexed(width, height, idx, palette), Some(idx.to_vec()))
            };
            Ok(Texture { name, width, height, bits, image, indices })
        })
        .collect()
}

fn parse_entities(b: &[u8]) -> Result<Vec<Entity>> {
    if b.is_empty() {
        return Ok(Vec::new());
    }
    let mut r = Reader::new(b);
    let count = r.u32()? as usize;
    if count > b.len() / 4 {
        return Err(Error::Invalid(format!("entity count {count}")));
    }
    let offsets: Vec<usize> = (0..count).map(|_| r.u32().map(|o| o as usize)).collect::<Result<_>>()?;
    offsets
        .iter()
        .enumerate()
        .map(|(i, &off)| {
            // Records are stored in order; the next offset (or lump end) bounds this one.
            let end = offsets.get(i + 1).copied().unwrap_or(b.len()).max(off);
            let rec = b.get(off..end).ok_or(Error::Eof(off))?;
            parse_entity(rec)
        })
        .collect()
}

fn parse_entity(rec: &[u8]) -> Result<Entity> {
    let mut r = Reader::new(rec);
    let kind = r.u32()?;
    Ok(match kind {
        1 => {
            let origin = r.vec3()?;
            let angles = r.vec3()?;
            r.bytes(8)?;
            Entity::Position { origin, angles, name: r.cstr(20)? }
        }
        2 => Entity::Light { origin: r.vec3()?, color: r.vec3()?, range: r.f32()?, flags: r.u32()? },
        3 => {
            let mut o = Object {
                kind,
                origin: r.vec3()?,
                angles: r.vec3()?,
                scale: r.vec3()?,
                name: r.cstr(20)?,
                file: r.cstr(13)?,
                action: r.cstr(20)?,
                ..Default::default()
            };
            r.seek(96);
            o.skills = (0..8).map(|_| r.f32()).collect::<Result<_>>()?;
            o.flags = r.u32()?;
            o.ambient = r.f32()?;
            Entity::Object(o)
        }
        7 => {
            let mut o = Object {
                kind,
                origin: r.vec3()?,
                angles: r.vec3()?,
                scale: r.vec3()?,
                name: r.cstr(33)?,
                file: r.cstr(33)?,
                action: r.cstr(33)?,
                ..Default::default()
            };
            r.seek(140);
            o.skills = (0..20).map(|_| r.f32()).collect::<Result<_>>()?;
            o.flags = r.u32()?;
            o.ambient = r.f32()?;
            o.albedo = Some(r.f32()?);
            o.path = Some(r.i32()?);
            o.entity2 = Some(r.i32()?);
            o.material = Some(r.cstr(33)?);
            o.string1 = Some(r.cstr(33)?);
            o.string2 = Some(r.cstr(33)?);
            Entity::Object(o)
        }
        5 => {
            r.vec3()?; // origin, unused
            let azimuth = r.f32()?;
            let elevation = r.f32()?;
            Entity::Info { azimuth, elevation, raw: rec[r.pos()..].to_vec() }
        }
        _ => Entity::Unknown { kind, raw: rec.to_vec() },
    })
}

impl Wmb {
    fn validate(&self) -> Result<()> {
        let bad = |what: String| Err(Error::Invalid(what));
        // Edge 0 is an unused dummy (may hold garbage); only check referenced edges.
        for (i, &se) in self.surfedges.iter().enumerate() {
            let Some(e) = self.edges.get(se.unsigned_abs() as usize) else {
                return bad(format!("surfedge {i} references missing edge"));
            };
            if e.iter().any(|&v| v as usize >= self.vertices.len()) {
                return bad(format!("edge {se} references missing vertex"));
            }
        }
        for (i, f) in self.faces.iter().enumerate() {
            if f.first_edge as usize + f.num_edges as usize > self.surfedges.len()
                || f.texinfo as usize >= self.texinfos.len()
                || f.plane as usize >= self.planes.len()
            {
                return bad(format!("face {i} out of range"));
            }
        }
        Ok(())
    }

    /// Vertex positions of a face in winding order (as stored: clockwise when
    /// viewed from the front, like Quake).
    pub fn face_vertices(&self, face: usize) -> impl Iterator<Item = [f32; 3]> + '_ {
        let f = &self.faces[face];
        let se = &self.surfedges[f.first_edge as usize..][..f.num_edges as usize];
        se.iter().map(|&e| {
            let edge = self.edges[e.unsigned_abs() as usize];
            self.vertices[if e >= 0 { edge[0] } else { edge[1] } as usize]
        })
    }

    pub fn face_normal(&self, face: usize) -> [f32; 3] {
        let f = &self.faces[face];
        let n = self.planes[f.plane as usize].normal;
        if f.side != 0 { [-n[0], -n[1], -n[2]] } else { n }
    }

    /// Texture of a face (None if the texinfo references a missing texture).
    pub fn face_texture(&self, face: usize) -> Option<usize> {
        let ti = &self.texinfos[self.faces[face].texinfo as usize];
        usize::try_from(ti.texture).ok().filter(|&t| t < self.textures.len())
    }

    /// Whether the face is drawn (texinfo not flagged [`surface_flags::INVISIBLE`]).
    pub fn face_visible(&self, face: usize) -> bool {
        self.texinfos[self.faces[face].texinfo as usize].flags & surface_flags::INVISIBLE == 0
    }

    /// Lightmap rectangle of a face: luxel origin `(floor(min s/16), floor(min t/16))`
    /// and size in luxels. `None` if the face has no lightmap.
    ///
    /// Must be computed in f32 exactly like this to reproduce the compiler's
    /// rounding (verified against every face of every shipped level).
    pub fn face_lightmap_rect(&self, face: usize) -> Option<LightmapRect> {
        let f = &self.faces[face];
        if f.light_offset < 0 || f.styles[0] == 255 {
            return None;
        }
        let ti = &self.texinfos[f.texinfo as usize];
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for p in self.face_vertices(face) {
            let st = ti.st(p);
            for k in 0..2 {
                min[k] = min[k].min(st[k]);
                max[k] = max[k].max(st[k]);
            }
        }
        let lo = [libm::floorf(min[0] / LUXEL_SIZE), libm::floorf(min[1] / LUXEL_SIZE)];
        let hi = [libm::ceilf(max[0] / LUXEL_SIZE), libm::ceilf(max[1] / LUXEL_SIZE)];
        let w = (hi[0] - lo[0]) as u32 + 1;
        let h = (hi[1] - lo[1]) as u32 + 1;
        let offset = f.light_offset as usize;
        // Entries are padded to 4 bytes except possibly the last.
        if offset + (w * h * 2) as usize > self.lightmaps.len() || w > 1024 || h > 1024 {
            return None;
        }
        Some(LightmapRect { offset, origin: [lo[0] as i32, lo[1] as i32], width: w, height: h })
    }

    /// Decodes one face lightmap to RGBA.
    pub fn face_lightmap(&self, rect: &LightmapRect) -> RgbaImage {
        let n = (rect.width * rect.height) as usize;
        let raw = &self.lightmaps[rect.offset..rect.offset + n * 2];
        RgbaImage::from_rgb565(rect.width, rect.height, raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])))
    }

    /// Decompresses the PVS row of a leaf (Quake RLE: a 0 byte is followed by a
    /// count of zero bytes). Bit `i` set = leaf `i + 1` visible.
    pub fn decompress_vis(&self, leaf: usize) -> Option<Vec<u8>> {
        let leaf = self.leaves.get(leaf)?;
        let visleafs = self.models.first().map_or(0, |m| m.vis_leafs.max(0) as usize);
        let row = visleafs.div_ceil(8);
        let mut src = self.vis.get(usize::try_from(leaf.vis_offset).ok()?..)?.iter();
        let mut out = Vec::with_capacity(row);
        while out.len() < row {
            match *src.next()? {
                0 => {
                    let n = *src.next()? as usize;
                    out.extend(core::iter::repeat_n(0, n.min(row - out.len())));
                }
                b => out.push(b),
            }
        }
        Some(out)
    }

    /// Point-in-hull test: returns the contents at `p` for collision hull
    /// `hull` (0 = BSP nodes/leaves, 1..3 = clipnodes) of `model`.
    pub fn point_contents(&self, model: usize, hull: usize, p: [f32; 3]) -> i32 {
        let Some(m) = self.models.get(model) else { return contents::EMPTY };
        let mut n = m.head_nodes[hull.min(3)];
        let side = |plane: i32| -> usize {
            let pl = &self.planes[plane as usize];
            let d = pl.normal[0] * p[0] + pl.normal[1] * p[1] + pl.normal[2] * p[2] - pl.dist;
            usize::from(d < 0.0)
        };
        if hull == 0 {
            while n >= 0 {
                let node = &self.nodes[n as usize];
                n = node.children[side(node.plane)];
            }
            self.leaves.get((-n - 1) as usize).map_or(contents::SOLID, |l| l.contents)
        } else {
            while n >= 0 {
                let c = &self.clipnodes[n as usize];
                n = c.children[side(c.plane)];
            }
            n
        }
    }

    /// Builds render geometry for every BSP model (model 0 = world) plus a
    /// lightmap atlas shared by all lit batches.
    pub fn build_mesh(&self) -> LevelMesh {
        let rects: Vec<Option<LightmapRect>> = (0..self.faces.len())
            .map(|i| if self.face_visible(i) { self.face_lightmap_rect(i) } else { None })
            .collect();
        let atlas = Atlas::pack(&rects.iter().map(|r| r.map(|r| (r.width, r.height))).collect::<Vec<_>>());
        let lightmap = atlas.as_ref().map(|a| {
            let mut img = RgbaImage::new(a.width, a.height);
            for (rect, slot) in rects.iter().zip(&a.slots) {
                if let (Some(rect), Some(slot)) = (rect, slot) {
                    blit_padded(&mut img, &self.face_lightmap(rect), slot[0], slot[1]);
                }
            }
            img
        });

        let models = self
            .models
            .iter()
            .map(|m| {
                let mut batches: Vec<Batch> = Vec::new();
                let start = m.first_face as usize;
                let end = (start + m.num_faces as usize).min(self.faces.len());
                for (fi, rect) in rects.iter().enumerate().take(end).skip(start) {
                    if !self.face_visible(fi) || self.faces[fi].num_edges < 3 {
                        continue;
                    }
                    let tex = self.face_texture(fi);
                    let ti = &self.texinfos[self.faces[fi].texinfo as usize];
                    let lm = rect.zip(atlas.as_ref().and_then(|a| a.slots[fi]));
                    let lit = lm.is_some();
                    let b = match batches.iter_mut().position(|b| b.texture == tex && b.lit == lit) {
                        Some(i) => &mut batches[i],
                        None => {
                            batches.push(Batch { texture: tex, lit, ..Default::default() });
                            batches.last_mut().unwrap()
                        }
                    };
                    let normal = self.face_normal(fi);
                    let (tw, th) = tex.map_or((64.0, 64.0), |t| (self.textures[t].width as f32, self.textures[t].height as f32));
                    let base = b.positions.len() as u32;
                    for p in self.face_vertices(fi) {
                        let st = ti.st(p);
                        b.positions.push(p);
                        b.normals.push(normal);
                        b.uvs.push([st[0] / tw, st[1] / th]);
                        if let (Some((rect, slot)), Some(a)) = (lm, atlas.as_ref()) {
                            // Luxel i is centred at s = (origin + i) * 16.
                            let u = st[0] / LUXEL_SIZE - rect.origin[0] as f32 + slot[0] as f32 + 0.5;
                            let v = st[1] / LUXEL_SIZE - rect.origin[1] as f32 + slot[1] as f32 + 0.5;
                            b.lightmap_uvs.push([u / a.width as f32, v / a.height as f32]);
                        }
                    }
                    // Stored winding is clockwise seen from the front; emit CCW.
                    let n = self.faces[fi].num_edges as u32;
                    for k in 1..n - 1 {
                        b.indices.extend([base, base + k + 1, base + k]);
                    }
                    b.faces.push(fi as u32);
                }
                ModelMesh { batches }
            })
            .collect();
        LevelMesh { lightmap, models }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LightmapRect {
    /// Byte offset into [`Wmb::lightmaps`].
    pub offset: usize,
    /// Luxel-grid origin, i.e. `floor(min_st / 16)`.
    pub origin: [i32; 2],
    pub width: u32,
    pub height: u32,
}

pub struct LevelMesh {
    /// Packed lightmap atlas (RGBA8); `None` if the file has no lightmaps.
    pub lightmap: Option<RgbaImage>,
    /// One entry per BSP model; index 0 is the world.
    pub models: Vec<ModelMesh>,
}

pub struct ModelMesh {
    pub batches: Vec<Batch>,
}

/// Triangles sharing one texture and lighting mode.
#[derive(Default)]
pub struct Batch {
    /// Index into [`Wmb::textures`].
    pub texture: Option<usize>,
    /// True if every face has a lightmap (`lightmap_uvs` is filled).
    pub lit: bool,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Texture UVs normalised by texture size (repeating).
    pub uvs: Vec<[f32; 2]>,
    /// UVs into [`LevelMesh::lightmap`]; empty when `!lit`.
    pub lightmap_uvs: Vec<[f32; 2]>,
    /// Counter-clockwise triangles (front face, native coordinates).
    pub indices: Vec<u32>,
    /// Source face indices.
    pub faces: Vec<u32>,
}

/// Simple shelf packer; each rect gets a 1-luxel border to avoid bleeding.
struct Atlas {
    width: u32,
    height: u32,
    /// Top-left of the inner (unpadded) rect per input.
    slots: Vec<Option<[u32; 2]>>,
}

impl Atlas {
    fn pack(sizes: &[Option<(u32, u32)>]) -> Option<Atlas> {
        let area: u64 = sizes.iter().flatten().map(|&(w, h)| ((w + 2) * (h + 2)) as u64).sum();
        if area == 0 {
            return None;
        }
        let max_w = sizes.iter().flatten().map(|s| s.0 + 2).max().unwrap_or(1);
        let width = (libm::sqrt(area as f64 * 1.15) as u32).next_power_of_two().max(max_w.next_power_of_two()).max(64);
        let mut order: Vec<usize> = (0..sizes.len()).filter(|&i| sizes[i].is_some()).collect();
        order.sort_by_key(|&i| core::cmp::Reverse((sizes[i].unwrap().1, sizes[i].unwrap().0)));
        let mut slots = vec![None; sizes.len()];
        let (mut x, mut y, mut shelf_h) = (0u32, 0u32, 0u32);
        for i in order {
            let (w, h) = sizes[i].unwrap();
            let (pw, ph) = (w + 2, h + 2);
            if x + pw > width {
                x = 0;
                y += shelf_h;
                shelf_h = 0;
            }
            slots[i] = Some([x + 1, y + 1]);
            x += pw;
            shelf_h = shelf_h.max(ph);
        }
        let height = (y + shelf_h).next_multiple_of(4);
        Some(Atlas { width, height, slots })
    }
}

/// Copies `src` to (x, y) and replicates its edge pixels into the 1-pixel border.
fn blit_padded(dst: &mut RgbaImage, src: &RgbaImage, x: u32, y: u32) {
    let (sw, sh) = (src.width as i64, src.height as i64);
    for dy in -1..=sh {
        for dx in -1..=sw {
            let sx = dx.clamp(0, sw - 1) as usize;
            let sy = dy.clamp(0, sh - 1) as usize;
            let tx = (x as i64 + dx) as usize;
            let ty = (y as i64 + dy) as usize;
            let s = (sy * src.width as usize + sx) * 4;
            let d = (ty * dst.width as usize + tx) * 4;
            dst.data[d..d + 4].copy_from_slice(&src.data[s..s + 4]);
        }
    }
}

/// Convenience: entity flags as names (`flag1`, `passable`, ...) plus unknown bits.
pub fn entity_flag_names(flags: u32) -> (Vec<&'static str>, u32) {
    let mut names = Vec::new();
    let mut rest = flags;
    for &(bit, name) in entity_flags::NAMES {
        if flags & bit != 0 {
            names.push(name);
            rest &= !bit;
        }
    }
    (names, rest)
}
