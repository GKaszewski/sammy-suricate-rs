//! MIDI music (`sons.wdl:66-74`, `PlayCurrentMusique` mm1.wdl:3935-3946). The original
//! played `sammy0N.mid` through the Windows MIDI synth; here they are rendered with
//! `rustysynth` and a General MIDI SoundFont (not part of the game data):
//! `$SAMMY_SOUNDFONT`, else the first `.sf2` in the save directory or
//! `/usr/share/soundfonts`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::audio::{AddAudioSource, Decodable, Source, Volume};
use bevy::prelude::*;
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

use crate::flow::{CurrentLevel, EXTRO_SLOT};
use crate::platform::save::SaveGame;
use crate::platform::vfs::GameFiles;

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<MidiSong>()
            .init_resource::<Music>()
            .add_systems(Startup, load_soundfont)
            .add_systems(Update, (level_music, options_test, start_song, apply_volume).chain());
    }
}

const RATE: u32 = 44100;
const BLOCK: usize = 1024;
/// Rendered audio is stereo.
const CHANNELS: u16 = 2;
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

/// `Music musique1..9`: musique8/9 reuse sammy01/02.
const FILES: [&str; 7] = ["sammy01.mid", "sammy02.mid", "sammy03.mid", "sammy04.mid", "sammy05.mid", "sammy06.mid", "sammy07.mid"];

#[derive(Asset, TypePath)]
pub struct MidiSong {
    font: Arc<SoundFont>,
    midi: Arc<MidiFile>,
    looping: bool,
}

pub struct MidiDecoder {
    seq: MidiFileSequencer,
    left: Vec<f32>,
    right: Vec<f32>,
    pos: usize,
    looping: bool,
}

impl Iterator for MidiDecoder {
    type Item = bevy::audio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= 2 * BLOCK {
            if !self.looping && self.seq.end_of_sequence() {
                return None;
            }
            self.seq.render(&mut self.left, &mut self.right);
            self.pos = 0;
        }
        let i = self.pos / 2;
        let s = if self.pos.is_multiple_of(2) { self.left[i] } else { self.right[i] };
        self.pos += 1;
        Some(s)
    }
}

impl Source for MidiDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> bevy::audio::ChannelCount {
        bevy::audio::ChannelCount::new(CHANNELS).unwrap()
    }

    fn sample_rate(&self) -> bevy::audio::SampleRate {
        bevy::audio::SampleRate::new(RATE).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Decodable for MidiSong {
    type Decoder = MidiDecoder;

    fn decoder(&self) -> MidiDecoder {
        let synth = Synthesizer::new(&self.font, &SynthesizerSettings::new(RATE as i32)).expect("synthesizer");
        let mut seq = MidiFileSequencer::new(synth);
        seq.play(&self.midi, self.looping);
        MidiDecoder { seq, left: vec![0.0; BLOCK], right: vec![0.0; BLOCK], pos: 2 * BLOCK, looping: self.looping }
    }
}

/// Playback state (`CurrentMusique`, `MusicOrder[]`).
#[derive(Resource)]
pub struct Music {
    font: Arc<Mutex<Option<Arc<SoundFont>>>>,
    /// `MusicOrder[level]` (1..8), rolled at boot (sammypolish.wdl:284-345).
    order: [u8; LEVEL_SLOTS],
    /// Requested song: (musique 1..7, looping, song volume 0..100); musique 0 = stop.
    pending: Option<(u8, bool, f32)>,
    song_vol: f32,
    entity: Option<Entity>,
    /// The options audio tab started `musique4` as a volume test (options.wdl:224-230).
    test: bool,
    loads: u32,
}

impl Default for Music {
    fn default() -> Self {
        let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) | 1;
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
        Self { font: default(), order, pending: None, song_vol: FULL_VOLUME, entity: None, test: false, loads: 0 }
    }
}

impl Music {
    /// `Play_Song musiqueN, vol` (musique8/9 = 1/2).
    pub fn play(&mut self, n: u8, looping: bool, vol: f32) {
        let n = match n {
            8 => 1,
            9 => 2,
            n => n,
        };
        self.pending = Some((n, looping, vol));
    }

    pub fn stop(&mut self) {
        self.pending = Some((0, false, 0.0));
    }

    /// `PlayCurrentMusique`: only musique1..7 are handled, so an 8 keeps the old song [Q].
    pub fn play_level(&mut self, slot: u32) {
        let n = self.order.get(slot as usize).copied().unwrap_or(1);
        if (1..=FILES.len() as u8).contains(&n) {
            self.play(n, true, FULL_VOLUME);
        }
    }
}

fn soundfont_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SAMMY_SOUNDFONT") {
        return Some(p.into());
    }
    let dirs = [crate::platform::save::save_path().parent().map(PathBuf::from), Some(PathBuf::from("/usr/share/soundfonts"))];
    dirs.into_iter().flatten().find_map(|d| {
        let mut v: Vec<_> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sf2"))).collect();
        v.sort();
        v.into_iter().next()
    })
}

/// Parses the SoundFont on a thread (tens of MB); songs wait for it.
fn load_soundfont(music: Res<Music>) {
    if std::env::var("SAMMY_SCREENSHOT").is_ok() {
        return;
    }
    let Some(path) = soundfont_path() else {
        return warn!("no SoundFont found: music disabled (set SAMMY_SOUNDFONT)");
    };
    let slot = music.font.clone();
    std::thread::spawn(move || match std::fs::File::open(&path).map_err(|e| e.to_string()).and_then(|f| SoundFont::new(&mut std::io::BufReader::new(f)).map_err(|e| format!("{e:?}"))) {
        Ok(sf) => {
            info!("SoundFont {}", path.display());
            *slot.lock().unwrap() = Some(Arc::new(sf));
        }
        Err(e) => warn!("SoundFont {}: {e}", path.display()),
    });
}

/// Song changes at level loads: `PlayCurrentMusique` at game start (mm1.wdl:840), the
/// intro's looping `musique1` (Intro2.wdl:50), and `Play_Song_Once Musique1,0` on
/// returning to the menu, a volume-0 play that silences the music (interfac.wdl:382).
fn level_music(current: Res<CurrentLevel>, mut music: ResMut<Music>, mut last: Local<Option<Option<u32>>>) {
    if current.loads == music.loads {
        return;
    }
    music.loads = current.loads;
    match current.slot {
        Some(slot) => music.play_level(slot.min(EXTRO_SLOT)),
        None if last.is_none() => music.play(1, true, FULL_VOLUME),
        None => music.stop(),
    }
    *last = Some(current.slot);
}

/// Dragging the music slider with no song playing starts `musique4`; closing the
/// options then restarts `PlayCurrentMusique` (options.wdl:60-63, :218-231).
fn options_test(save: Res<SaveGame>, options: Res<crate::ui::options::OptionsState>, current: Res<CurrentLevel>, mut music: ResMut<Music>, mut old: Local<Option<f32>>) {
    let open = options.mode.is_some();
    if open && old.is_some_and(|v| v != save.music) && music.entity.is_none() && music.pending.is_none() && !music.test {
        music.test = true;
        music.play(TEST_SONG, true, FULL_VOLUME);
    }
    if !open && music.test {
        music.test = false;
        let slot = current.slot.unwrap_or(save.last_level);
        music.play_level(slot);
    }
    *old = open.then_some(save.music);
}

fn start_song(mut commands: Commands, files: Res<GameFiles>, save: Res<SaveGame>, mut music: ResMut<Music>, mut songs: ResMut<Assets<MidiSong>>) {
    let Some((n, looping, vol)) = music.pending else { return };
    if n != 0 && music.font.lock().unwrap().is_none() {
        return; // still loading
    }
    music.pending = None;
    if let Some(e) = music.entity.take() {
        commands.entity(e).try_despawn();
    }
    if n == 0 || vol <= 0.0 {
        return;
    }
    let font = music.font.lock().unwrap().clone().unwrap();
    let Some(midi) = files.read(FILES[n as usize - 1]).ok().and_then(|d| MidiFile::new(&mut d.as_slice()).ok()) else { return };
    music.song_vol = vol;
    let handle = songs.add(MidiSong { font, midi: Arc::new(midi), looping });
    let v = Volume::Linear(save.music / FULL_VOLUME * vol / FULL_VOLUME);
    music.entity = Some(commands.spawn((MusicPlayer, AudioPlayer(handle), PlaybackSettings::DESPAWN.with_volume(v))).id());
}

#[derive(Component)]
struct MusicPlayer;

/// `Midi_Vol = Slider_MusicVol`; forgets songs that ended (`Play_Song_Once`).
fn apply_volume(save: Res<SaveGame>, mut music: ResMut<Music>, mut sinks: Query<&mut AudioSink, With<MusicPlayer>>, alive: Query<(), With<MusicPlayer>>) {
    // Commands from `start_song` are applied before this system (chained).
    if music.entity.is_some_and(|e| !alive.contains(e)) {
        music.entity = None;
    }
    let v = Volume::Linear(save.music / FULL_VOLUME * music.song_vol / FULL_VOLUME);
    for mut sink in &mut sinks {
        sink.set_volume(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_sammy01() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted/sammy01.mid");
        let (Some(sf), Ok(mid)) = (soundfont_path(), std::fs::read(&root)) else { return eprintln!("skip {}", root.display()) };
        let font = Arc::new(SoundFont::new(&mut std::io::BufReader::new(std::fs::File::open(sf).unwrap())).unwrap());
        let song = MidiSong { font, midi: Arc::new(MidiFile::new(&mut mid.as_slice()).unwrap()), looping: false };
        let samples: Vec<f32> = song.decoder().take(RATE as usize * 2 * 5).collect();
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        eprintln!("rms {rms}");
        assert!(rms > 0.01 && rms < 0.7, "rms {rms}");
    }
}
