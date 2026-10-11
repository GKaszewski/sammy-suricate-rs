//! Decoding a WMB4 file into a [`Wmb`].

use alloc::{format, string::String, vec::Vec};

use super::entity::parse_entities;
use super::textures::parse_textures;
use super::*;
use crate::reader::Reader;
use crate::{Error, Result};

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
}
