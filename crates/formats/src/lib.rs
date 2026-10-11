//! Parsers for Conitec 3D GameStudio A4 (Acknex 4.22) data files used by Sammy Suricate.
//!
//! All parsers keep the engine's native coordinate system (Quake-style: X forward,
//! Y left, Z up, units = quants). Conversion to Y-up happens in consumers.

//!
//! `no_std` + `alloc`: usable on any target with an allocator.

#![no_std]

extern crate alloc;

pub mod image;
pub mod lzss;
pub mod mdl;
pub mod pcx;
pub mod reader;
pub mod sav;
pub mod trace;
pub mod wmb;
pub mod wrs;

use alloc::{string::String, vec::Vec};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad magic: expected {expected:?}, found {found:?}")]
    BadMagic { expected: &'static str, found: Vec<u8> },
    #[error("unexpected end of data at offset {0:#x}")]
    Eof(usize),
    #[error("invalid data: {0}")]
    Invalid(String),
}

pub type Result<T> = core::result::Result<T, Error>;
