//! Which song plays when.

use sammy_core::levels::{EXTRO_SLOT, LevelSlot};
use sammy_core::sim::Sim;

use super::FILES;

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
    last: Option<Option<LevelSlot>>,
    /// The music slider while the options are open.
    slider: Option<sammy_core::percent::Percent>,
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
    fn play_level(&mut self, slot: LevelSlot) -> Option<Request> {
        let n = self.order.get(slot.index()).copied().unwrap_or(1);
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
        sim.save.music.get() / FULL_VOLUME * self.song_volume / FULL_VOLUME
    }
}
