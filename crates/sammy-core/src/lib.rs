//! Sammy Suricate's game rules, independent of any engine or platform.
//!
//! Everything here is plain Rust: no rendering, windows, audio, files, clocks or
//! threads. Time comes in as A4 ticks (16 per second, fractional per frame like the
//! original's `time`), positions are native A4 coordinates in quants (X forward,
//! Y left, Z up) and randomness comes from a seeded [`rng::Rng`]. A frontend (Bevy
//! today) feeds input and presents the state; see `docs/architecture.md`.

pub mod angle;
pub mod anim;
pub mod collision;
pub mod doors;
pub mod game;
pub mod gems;
pub mod keys;
pub mod levels;
pub mod physics;
pub mod pickups;
pub mod platforms;
pub mod rng;
pub mod save;
pub mod time;
