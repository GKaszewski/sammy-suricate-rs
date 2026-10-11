//! Sound effects and music: the game's WAVs (by WDL name, `sons.wdl`) resampled once to
//! the output rate, and the current song rendered as it plays, mixed in SDL's audio
//! callback.

use std::collections::HashMap;
use std::sync::Arc;

use sammy_platform::GameFiles;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

/// Output rate and buffer (samples per channel).
const RATE: i32 = 44100;
const BUFFER: u16 = 1024;
/// Most sounds playing at once (the oldest is dropped).
const MAX_VOICES: usize = 24;
/// Volumes are percentages (WDL 0..100).
const FULL: f32 = 100.0;

struct Voice {
    data: Arc<Vec<f32>>,
    at: usize,
    volume: f32,
}

pub struct Mixer {
    voices: Vec<Voice>,
    /// Effects volume (0..1).
    pub volume: f32,
    music: Option<sammy_platform::music::Song>,
    music_volume: f32,
    /// Song blocks rendered here (left, right).
    left: Vec<f32>,
    right: Vec<f32>,
    /// Seconds spent rendering the song, and of audio produced, since last asked.
    music_busy: f64,
    audio_secs: f64,
    rate: f64,
}

impl AudioCallback for Mixer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        self.audio_secs += frames as f64 / self.rate;
        if let Some(song) = &mut self.music {
            let t0 = now();
            self.left.resize(frames, 0.0);
            self.right.resize(frames, 0.0);
            if song.render(&mut self.left, &mut self.right) {
                for (i, frame) in out.chunks_exact_mut(2).enumerate() {
                    frame[0] = self.left[i] * self.music_volume;
                    frame[1] = self.right[i] * self.music_volume;
                }
            } else {
                self.music = None;
            }
            self.music_busy += now() - t0;
        }
        for v in &mut self.voices {
            for frame in out.chunks_exact_mut(2) {
                let Some(s) = v.data.get(v.at) else { break };
                let s = s * v.volume * self.volume;
                frame[0] += s;
                frame[1] += s;
                v.at += 1;
            }
        }
        self.voices.retain(|v| v.at < v.data.len());
        for s in out {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

pub struct Audio {
    device: Option<AudioDevice<Mixer>>,
    sounds: HashMap<String, Arc<Vec<f32>>>,
}

impl Audio {
    pub fn new(sdl: &sdl2::Sdl, files: &GameFiles) -> Self {
        let device = sdl.audio().and_then(|a| {
            a.open_playback(None, &AudioSpecDesired { freq: Some(RATE), channels: Some(2), samples: Some(BUFFER) }, |spec| Mixer {
                voices: Vec::new(),
                volume: 1.0,
                music: None,
                music_volume: 1.0,
                left: Vec::new(),
                right: Vec::new(),
                music_busy: 0.0,
                audio_secs: 0.0,
                rate: spec.freq as f64,
            })
        });
        let device = match device {
            Ok(d) => {
                d.resume();
                Some(d)
            }
            Err(e) => {
                log::warn!("no audio: {e}");
                None
            }
        };
        let rate = device.as_ref().map_or(RATE, |d| d.spec().freq);
        let mut sounds = HashMap::new();
        for (name, file) in sammy_platform::sounds::table(files) {
            if let Some(pcm) = files.read(&file).ok().and_then(|d| sammy_platform::sounds::decode_wav(&d)) {
                sounds.insert(name, Arc::new(resample(&pcm, rate as u32)));
            }
        }
        log::info!("loaded {} sounds from sons.wdl", sounds.len());
        Self { device, sounds }
    }

    /// `play_sound name, vol` (WDL volume 0..100).
    pub fn play(&mut self, name: &str, vol: f32) {
        let (Some(dev), Some(data)) = (&mut self.device, self.sounds.get(&name.to_lowercase())) else { return };
        let mut m = dev.lock();
        if m.voices.len() >= MAX_VOICES {
            m.voices.remove(0);
        }
        m.voices.push(Voice { data: data.clone(), at: 0, volume: (vol / FULL).clamp(0.0, 2.0) });
    }

    /// The output rate (Hz).
    pub fn rate(&self) -> u32 {
        self.device.as_ref().map_or(RATE, |d| d.spec().freq) as u32
    }

    /// Starts a song (or stops with `None`).
    pub fn set_music(&mut self, song: Option<sammy_platform::music::Song>) {
        if let Some(dev) = &mut self.device {
            dev.lock().music = song;
        }
    }

    /// A song is playing.
    pub fn music_playing(&mut self) -> bool {
        self.device.as_mut().is_some_and(|d| d.lock().music.is_some())
    }

    /// The music volume (0..1).
    pub fn set_music_volume(&mut self, v: f32) {
        if let Some(dev) = &mut self.device {
            dev.lock().music_volume = v.clamp(0.0, 1.0);
        }
    }

    /// The share of real time spent rendering music since the last call (0..1).
    pub fn music_cost(&mut self) -> f64 {
        let Some(dev) = &mut self.device else { return 0.0 };
        let mut m = dev.lock();
        let share = if m.audio_secs > 0.0 { m.music_busy / m.audio_secs } else { 0.0 };
        (m.music_busy, m.audio_secs) = (0.0, 0.0);
        share
    }

    /// The effects volume (the save's percentage).
    pub fn set_volume(&mut self, percent: f32) {
        if let Some(dev) = &mut self.device {
            dev.lock().volume = (percent / FULL).clamp(0.0, 1.0);
        }
    }
}

/// Seconds on SDL's high-resolution clock (usable on the audio thread).
fn now() -> f64 {
    unsafe { sdl2::sys::SDL_GetPerformanceCounter() as f64 / sdl2::sys::SDL_GetPerformanceFrequency() as f64 }
}

/// Mono samples (-1..1) at `rate`, linearly resampled.
fn resample(pcm: &sammy_platform::sounds::Pcm, rate: u32) -> Vec<f32> {
    let ch = pcm.channels as usize;
    let mono: Vec<f32> = pcm.samples.chunks_exact(ch).map(|f| f.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / ch as f32).collect();
    if mono.is_empty() || pcm.rate == rate {
        return mono;
    }
    let step = pcm.rate as f64 / rate as f64;
    let n = (mono.len() as f64 / step) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * step;
            let (a, f) = (t as usize, (t.fract()) as f32);
            let x = mono[a];
            let y = mono.get(a + 1).copied().unwrap_or(x);
            x + (y - x) * f
        })
        .collect()
}
