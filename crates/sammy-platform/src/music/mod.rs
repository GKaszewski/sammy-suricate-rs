//! MIDI music (`sons.wdl:66-74`, `PlayCurrentMusique` mm1.wdl:3935-3946), for any
//! frontend: which song plays when ([`Playlist`]), finding a General MIDI SoundFont (not
//! part of the game data) and rendering a song with `rustysynth` ([`Song`]). The
//! original played `sammy0N.mid` through the Windows MIDI synth.

mod playlist;
mod song;
mod soundfont;

pub use self::{playlist::*, song::*, soundfont::*};

/// Rendering block (samples per channel).
pub const BLOCK: usize = 1024;

/// `Music musique1..9`: musique8/9 reuse sammy01/02.
const FILES: [&str; 7] = ["sammy01.mid", "sammy02.mid", "sammy03.mid", "sammy04.mid", "sammy05.mid", "sammy06.mid", "sammy07.mid"];

#[cfg(test)]
mod tests;
