//! The entity lump: positions, lights, objects and level info.

use alloc::{format, string::String, vec::Vec};

use crate::reader::Reader;
use crate::{Error, Result};

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

pub(super) fn parse_entities(b: &[u8]) -> Result<Vec<Entity>> {
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
