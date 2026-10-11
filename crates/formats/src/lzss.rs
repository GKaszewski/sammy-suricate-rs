//! Okumura-style LZSS as used by Acknex WRS archives.
//!
//! 4096-byte ring buffer pre-filled with spaces, write cursor starting at N-F,
//! flag bytes read LSB-first (1 = literal), back-references are 12-bit position
//! plus 4-bit length (+3).

use alloc::{vec::Vec};

const N: usize = 4096;
const F: usize = 18;
const THRESHOLD: usize = 2;

pub fn decompress(src: &[u8], out_len: usize) -> Vec<u8> {
    let mut window = [b' '; N];
    let mut r = N - F;
    let mut out = Vec::with_capacity(out_len);
    let mut flags: u32 = 0;
    let mut i = 0;
    while out.len() < out_len {
        flags >>= 1;
        if flags & 0x100 == 0 {
            let Some(&b) = src.get(i) else { break };
            flags = b as u32 | 0xff00;
            i += 1;
        }
        if flags & 1 != 0 {
            let Some(&c) = src.get(i) else { break };
            i += 1;
            out.push(c);
            window[r] = c;
            r = (r + 1) % N;
        } else {
            let (Some(&a), Some(&b)) = (src.get(i), src.get(i + 1)) else { break };
            i += 2;
            let pos = a as usize | ((b as usize & 0xf0) << 4);
            let len = (b as usize & 0x0f) + THRESHOLD + 1;
            for k in 0..len {
                let c = window[(pos + k) % N];
                out.push(c);
                window[r] = c;
                r = (r + 1) % N;
            }
        }
    }
    out.truncate(out_len);
    out
}
