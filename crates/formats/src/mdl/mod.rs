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

mod model;
mod normals;
mod parse;

pub use model::*;
pub use normals::NORMALS;
pub use parse::parse;
