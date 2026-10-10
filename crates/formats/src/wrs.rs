//! WRS2 resource archive (Acknex A4 "publish" output).
//!
//! Layout: `"WRS2"` followed by entries until EOF. Each entry:
//! `name[32]` (XOR 0xA5, terminated by a raw 0x00), `u32be compressed_size`,
//! `u32be uncompressed_size`, then `compressed_size` bytes of LZSS data.

use crate::reader::{cstr, Reader};
use crate::{lzss, Error, Result};

const NAME_XOR: u8 = 0xA5;

pub struct Entry<'a> {
    pub name: String,
    pub uncompressed_size: usize,
    compressed: &'a [u8],
}

impl Entry<'_> {
    pub fn data(&self) -> Vec<u8> {
        lzss::decompress(self.compressed, self.uncompressed_size)
    }
}

pub fn parse(data: &[u8]) -> Result<Vec<Entry<'_>>> {
    let mut r = Reader::new(data);
    let magic = r.bytes(4)?;
    if magic != b"WRS2" {
        return Err(Error::BadMagic { expected: "WRS2", found: magic.to_vec() });
    }
    let mut entries = Vec::new();
    while r.remaining() > 0 {
        // The terminator/padding is a raw 0x00 (not XORed), so cut before decoding.
        let raw = r.bytes(32)?;
        let len = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let decoded: Vec<u8> = raw[..len].iter().map(|b| b ^ NAME_XOR).collect();
        let name = cstr(&decoded);
        let csize = r.u32_be()? as usize;
        let usize_ = r.u32_be()? as usize;
        let compressed = r.bytes(csize)?;
        entries.push(Entry { name, uncompressed_size: usize_, compressed });
    }
    Ok(entries)
}
