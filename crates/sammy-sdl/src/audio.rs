//! Sound effects: the game's WAVs (by WDL name, `sons.wdl`) resampled once to the output
//! rate and mixed in SDL's audio callback.

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
}

impl AudioCallback for Mixer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        out.fill(0.0);
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
            a.open_playback(None, &AudioSpecDesired { freq: Some(RATE), channels: Some(2), samples: Some(BUFFER) }, |_| Mixer { voices: Vec::new(), volume: 1.0 })
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

    /// The effects volume (the save's percentage).
    pub fn set_volume(&mut self, percent: f32) {
        if let Some(dev) = &mut self.device {
            dev.lock().volume = (percent / FULL).clamp(0.0, 1.0);
        }
    }
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
