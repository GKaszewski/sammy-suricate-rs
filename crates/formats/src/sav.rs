//! Acknex A4 `Save_Info` files (`levelN.SAV`, `Ver1.SAV`, `BestSco0.SAV`;
//! docs/hud-saves.md §4.2): a big-endian length header, LZSS (as in WRS archives),
//! then little-endian `Var_Info` records and named `Type Global` skills. Values are
//! 22.10 fixed point.

use std::collections::HashMap;

use crate::{Error, Result};

/// Decoded save: `Var_Info` arrays by engine var id and skills by name (x, y, z).
#[derive(Debug, Default)]
pub struct Save {
    pub vars: HashMap<u32, Vec<f32>>,
    pub skills: HashMap<String, [f32; 3]>,
}

/// `Var_Info` ids (docs/hud-saves.md §4.4).
pub mod ids {
    pub const JOYPAD: u32 = 0x134;
    pub const EXTRO_VUE: u32 = 0x130;
    pub const SLIDER_MUSIC: u32 = 0x12B;
    pub const SLIDER_SOUND: u32 = 0x12A;
    pub const LEVELS_EXIST: u32 = 0x124;
    pub const BONUS_STATE: u32 = 0x121;
    pub const LAST_JUMP: u32 = 0x120;
    pub const LAST_THINK: u32 = 0x11F;
    pub const LAST_TUT: u32 = 0x11E;
    pub const BEST_SCORES: u32 = 0x119;
    pub const RESTART_Z: u32 = 0x118;
    pub const RESTART_Y: u32 = 0x117;
    pub const RESTART_X: u32 = 0x116;
}

fn u32_at(d: &[u8], o: usize, le: bool) -> Result<u32> {
    let b: [u8; 4] = d.get(o..o + 4).ok_or(Error::Eof(o))?.try_into().unwrap();
    Ok(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

pub fn parse(file: &[u8]) -> Result<Save> {
    let out_len = u32_at(file, 4, false)? as usize;
    let d = crate::lzss::decompress(file.get(8..).ok_or(Error::Eof(8))?, out_len);
    if u32_at(&d, 0, true)? != 0x70C {
        return Err(Error::Invalid("not a Save_Info payload".into()));
    }
    let fixed = |v: u32| v as i32 as f32 / 1024.0;
    let mut save = Save::default();
    let mut o = 12;
    let records = u32_at(&d, o, true)?;
    o += 4;
    for _ in 0..records {
        let id = u32_at(&d, o, true)?;
        let count = u32_at(&d, o + 4, true)? as usize;
        o += 8;
        let vals = (0..count).map(|i| u32_at(&d, o + i * 4, true).map(fixed)).collect::<Result<Vec<_>>>()?;
        o += count * 4;
        save.vars.insert(id, vals);
    }
    let skills = u32_at(&d, o, true)?;
    o += 4;
    for _ in 0..skills {
        let end = d[o..].iter().position(|&b| b == 0).ok_or(Error::Eof(o))? + o;
        let name = String::from_utf8_lossy(&d[o..end]).into_owned();
        o = end + 1;
        let v = [fixed(u32_at(&d, o, true)?), fixed(u32_at(&d, o + 4, true)?), fixed(u32_at(&d, o + 8, true)?)];
        o += 12;
        save.skills.insert(name, v);
    }
    Ok(save)
}

impl Save {
    /// Scalar `Var_Info` (first element).
    pub fn var(&self, id: u32) -> Option<f32> {
        self.vars.get(&id).and_then(|v| v.first().copied())
    }

    pub fn array(&self, id: u32) -> &[f32] {
        self.vars.get(&id).map_or(&[], |v| v.as_slice())
    }

    pub fn skill(&self, name: &str) -> Option<f32> {
        self.skills.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v[0])
    }
}
