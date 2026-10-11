//! MIDI music (`sons.wdl:66-74`, `PlayCurrentMusique` mm1.wdl:3935-3946), for any
//! frontend: which song plays when ([`Playlist`]), finding a General MIDI SoundFont (not
//! part of the game data) and rendering a song with `rustysynth` ([`Song`]). The
//! original played `sammy0N.mid` through the Windows MIDI synth.

use std::path::PathBuf;
use std::sync::Arc;

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use sammy_core::levels::EXTRO_SLOT;
use sammy_core::sim::Sim;

use crate::{GameFiles, Storage};

/// `MusicOrder[]` has one entry per level slot.
const LEVEL_SLOTS: usize = 50;
/// Song volume is a percentage; 100 plays at full level.
const FULL_VOLUME: f32 = 100.0;
/// Boot-time roll (sammypolish.wdl:284-345): levels from slot 2 get `rand(MUSIC_ROLL)`
/// (1..8), slots 1..12 alternate musique 1 / 2, and the bonus ranges `BONUS_ROLL_A`/`B`
/// get `rand(BONUS_ROLL)` (1..3).
const MUSIC_ROLL: u64 = 8;
const FIXED_ORDER_SLOTS: usize = 13;
const BONUS_ROLL: u64 = 3;
const BONUS_ROLL_A: std::ops::RangeInclusive<usize> = 17..=20;
const BONUS_ROLL_B: std::ops::RangeInclusive<usize> = 33..=36;
/// Xorshift64 shifts for the roll.
const XORSHIFT: (u32, u32, u32) = (13, 7, 17);
/// The options volume test plays musique4.
const TEST_SONG: u8 = 4;
/// Rendering block (samples per channel).
pub const BLOCK: usize = 1024;

/// `Music musique1..9`: musique8/9 reuse sammy01/02.
const FILES: [&str; 7] = ["sammy01.mid", "sammy02.mid", "sammy03.mid", "sammy04.mid", "sammy05.mid", "sammy06.mid", "sammy07.mid"];

/// A song change for the frontend's player.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Play this MIDI file, looping or once, at a song volume (0..100).
    Play { file: &'static str, looping: bool, volume: f32 },
    Stop,
}

/// Which song plays (`CurrentMusique`, `MusicOrder[]`).
#[derive(Clone, Debug)]
pub struct Playlist {
    /// `MusicOrder[level]` (1..8), rolled at boot.
    order: [u8; LEVEL_SLOTS],
    /// Song volume of the current song (0..100).
    song_volume: f32,
    /// The options audio tab started `musique4` as a volume test (options.wdl:224-230).
    test: bool,
    loads: u32,
    /// The last load's slot (None: menu), once a level was seen.
    last: Option<Option<u32>>,
    /// The music slider while the options are open.
    slider: Option<f32>,
}

impl Playlist {
    pub fn new(mut seed: u64) -> Self {
        seed |= 1;
        let mut rand = |n: u64| {
            seed ^= seed << XORSHIFT.0;
            seed ^= seed >> XORSHIFT.1;
            seed ^= seed << XORSHIFT.2;
            (seed % n) as u8 + 1
        };
        let mut order = [0u8; LEVEL_SLOTS];
        for v in order.iter_mut().skip(2) {
            *v = rand(MUSIC_ROLL);
        }
        for (n, v) in order.iter_mut().enumerate().take(FIXED_ORDER_SLOTS).skip(1) {
            *v = if n % 2 == 1 { 1 } else { 2 };
        }
        for n in BONUS_ROLL_A.chain(BONUS_ROLL_B) {
            order[n] = rand(BONUS_ROLL);
        }
        Self { order, song_volume: FULL_VOLUME, test: false, loads: 0, last: None, slider: None }
    }

    /// Seeded from the clock (the original rolled at boot).
    pub fn from_clock() -> Self {
        Self::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64))
    }

    /// `Play_Song musiqueN, vol` (musique8/9 = 1/2).
    fn play(&mut self, n: u8, looping: bool, volume: f32) -> Request {
        let n = match n {
            8 => 1,
            9 => 2,
            n => n,
        };
        self.song_volume = volume;
        Request::Play { file: FILES[(n.clamp(1, FILES.len() as u8) - 1) as usize], looping, volume }
    }

    /// `PlayCurrentMusique`: only musique1..7 are handled, so an 8 keeps the old song [Q].
    fn play_level(&mut self, slot: u32) -> Option<Request> {
        let n = self.order.get(slot as usize).copied().unwrap_or(1);
        (1..=FILES.len() as u8).contains(&n).then(|| self.play(n, true, FULL_VOLUME))
    }

    /// The frame's song change, if any:
    /// - at level loads: `PlayCurrentMusique` at game start (mm1.wdl:840), the intro's
    ///   looping `musique1` (Intro2.wdl:50), and `Play_Song_Once Musique1,0` on returning
    ///   to the menu, a volume-0 play that silences the music (interfac.wdl:382);
    /// - dragging the music slider with no song playing starts `musique4`; closing the
    ///   options then restarts `PlayCurrentMusique` (options.wdl:60-63, :218-231).
    ///
    /// `options_open`: the options overlay shows; `playing`: a song is audible.
    pub fn update(&mut self, sim: &Sim, options_open: bool, playing: bool) -> Option<Request> {
        let mut out = None;
        if sim.loads != self.loads {
            self.loads = sim.loads;
            out = match sim.flow.slot {
                Some(slot) => self.play_level(slot.min(EXTRO_SLOT)),
                None if self.last.is_none() => Some(self.play(1, true, FULL_VOLUME)),
                None => Some(Request::Stop),
            };
            self.last = Some(sim.flow.slot);
        }
        let music = sim.save.music;
        if options_open && self.slider.is_some_and(|v| v != music) && !playing && out.is_none() && !self.test {
            self.test = true;
            out = Some(self.play(TEST_SONG, true, FULL_VOLUME));
        }
        if !options_open && self.test {
            self.test = false;
            out = self.play_level(sim.flow.slot.unwrap_or(sim.save.last_level));
        }
        self.slider = options_open.then_some(music);
        out
    }

    /// `Midi_Vol = Slider_MusicVol` × the song's volume, 0..1.
    pub fn volume(&self, sim: &Sim) -> f32 {
        sim.save.music / FULL_VOLUME * self.song_volume / FULL_VOLUME
    }
}

/// `SAMMY_SOUNDFONT`, else the first `.sf2` next to the executable (releases bundle
/// one; on the Vita in the package), in the player's storage, among the game files, or in `/usr/share/soundfonts`.
/// Returns the file's bytes and a name for messages.
pub fn find_soundfont(storage: &Storage, files: &GameFiles) -> Option<(String, Vec<u8>)> {
    let from_path = |p: PathBuf| std::fs::read(&p).ok().map(|d| (p.display().to_string(), d));
    let first_in = |d: PathBuf| {
        let mut v: Vec<_> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sf2"))).collect();
        v.sort();
        v.into_iter().next()
    };
    if let Ok(p) = std::env::var("SAMMY_SOUNDFONT") {
        return from_path(p.into());
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    #[cfg(not(any(target_arch = "wasm32", target_os = "vita")))]
    dirs.extend(crate::locate::beside_executable());
    // The Vita package itself.
    #[cfg(target_os = "vita")]
    dirs.push("app0:".into());
    dirs.extend(storage.dir().map(PathBuf::from));
    if let Some(p) = dirs.into_iter().find_map(first_in) {
        return from_path(p);
    }
    if let Some(name) = files.find_extension("sf2") {
        return files.read(&name).ok().map(|d| (name, d));
    }
    #[cfg(target_os = "linux")]
    if let Some(p) = first_in("/usr/share/soundfonts".into()) {
        return from_path(p);
    }
    None
}

/// Parses a SoundFont.
pub fn parse_soundfont(data: &[u8]) -> anyhow::Result<Arc<SoundFont>> {
    SoundFont::new(&mut &data[..]).map(Arc::new).map_err(|e| anyhow::anyhow!("{e:?}"))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-time cost of each song at a few qualities (needs the game files and a
    /// SoundFont): `cargo test --release -p sammy-platform --features music -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn render_cost() {
        let storage = Storage::user();
        let Some(dir) = crate::Settings::load(&storage).game_dir else { return eprintln!("skip: no game dir") };
        let Ok(files) = GameFiles::open(std::path::Path::new(&dir)) else { return };
        let Some((name, data)) = find_soundfont(&storage, &files) else { return eprintln!("skip: no SoundFont") };
        let font = parse_soundfont(&data).unwrap();
        eprintln!("{name}");
        for q in [Quality::default(), Quality { rate: 22050, polyphony: 32, effects: false }, Quality { rate: 22050, polyphony: 24, effects: false }] {
            let secs = 20.0;
            let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            let mut worst: f64 = 0.0;
            let mut total = 0.0;
            for file in FILES {
                let mut song = Song::new(&font, &files, file, true, q).unwrap();
                let t = std::time::Instant::now();
                for _ in 0..(secs * q.rate as f32 / BLOCK as f32) as usize {
                    song.render(&mut l, &mut r);
                }
                let cost = t.elapsed().as_secs_f64() / secs as f64;
                worst = worst.max(cost);
                total += cost;
            }
            eprintln!("{q:?}: mean {:.2}% worst {:.2}% of real time", total / FILES.len() as f64 * 100.0, worst * 100.0);
        }
    }
}
