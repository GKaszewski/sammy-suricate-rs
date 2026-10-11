//! Rendering a song.

use std::sync::Arc;

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

use crate::GameFiles;

/// How a song is synthesised: the output rate, the most voices at once, and the
/// reverb/chorus effects (cheaper without, for handhelds).
#[derive(Clone, Copy, Debug)]
pub struct Quality {
    pub rate: u32,
    pub polyphony: usize,
    pub effects: bool,
}

impl Default for Quality {
    fn default() -> Self {
        Self { rate: 44100, polyphony: 64, effects: true }
    }
}

/// A song being rendered: stereo blocks from a MIDI file.
pub struct Song {
    seq: MidiFileSequencer,
    looping: bool,
}

impl Song {
    pub fn new(font: &Arc<SoundFont>, files: &GameFiles, file: &str, looping: bool, quality: Quality) -> anyhow::Result<Self> {
        let data = files.read(file)?;
        let midi = MidiFile::new(&mut data.as_slice()).map_err(|e| anyhow::anyhow!("{file}: {e:?}"))?;
        let mut settings = SynthesizerSettings::new(quality.rate as i32);
        settings.maximum_polyphony = quality.polyphony;
        settings.enable_reverb_and_chorus = quality.effects;
        let synth = Synthesizer::new(font, &settings).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let mut seq = MidiFileSequencer::new(synth);
        seq.play(&Arc::new(midi), looping);
        Ok(Self { seq, looping })
    }

    /// Fills both channels (equal lengths); false once a non-looping song has ended.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> bool {
        if !self.looping && self.seq.end_of_sequence() {
            return false;
        }
        self.seq.render(left, right);
        true
    }
}
