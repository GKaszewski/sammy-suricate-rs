//! Entity ids as the core sees them: opaque handles chosen by the frontend (Bevy uses
//! `Entity::to_bits`).

/// An entity handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Id(pub u64);
