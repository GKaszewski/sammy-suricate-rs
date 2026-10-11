//! Music: [`sammy_platform::music`] picks the song; it is rendered in the audio
//! callback ([`crate::audio`]). The SoundFont (tens of MB) is parsed on a thread and
//! songs wait for it.

use std::sync::{Arc, Mutex};

use rustysynth::SoundFont;
use sammy_core::sim::Sim;
use sammy_platform::music::{Playlist, Quality, Request, Song};
use sammy_platform::{GameFiles, Storage};

use crate::audio::Audio;

/// Handhelds render fewer voices and no reverb/chorus.
const HANDHELD_POLYPHONY: usize = 32;

pub struct Music {
    font: Arc<Mutex<Option<Arc<SoundFont>>>>,
    playlist: Playlist,
    pending: Option<Request>,
}

impl Music {
    pub fn new(storage: &Storage, files: &GameFiles) -> Self {
        let font = Arc::new(Mutex::new(None));
        if std::env::var_os("SAMMY_SCREENSHOT").is_none() {
            match sammy_platform::music::find_soundfont(storage, files) {
                Some((name, data)) => {
                    let slot = font.clone();
                    std::thread::spawn(move || match sammy_platform::music::parse_soundfont(&data) {
                        Ok(sf) => {
                            log::info!("SoundFont {name}");
                            *slot.lock().unwrap() = Some(sf);
                        }
                        Err(e) => log::warn!("SoundFont {name}: {e}"),
                    });
                }
                None => log::warn!("no SoundFont found: music disabled (put a .sf2 next to the game or set SAMMY_SOUNDFONT)"),
            }
        }
        Self { font, playlist: Playlist::from_clock(), pending: None }
    }

    /// The frame's song changes and volume.
    pub fn update(&mut self, sim: &Sim, options_open: bool, audio: &mut Audio, files: &GameFiles) {
        let playing = audio.music_playing() || self.pending.is_some();
        if let Some(r) = self.playlist.update(sim, options_open, playing) {
            self.pending = Some(r);
        }
        audio.set_music_volume(self.playlist.volume(sim));
        let Some(request) = self.pending.clone() else { return };
        let font = self.font.lock().unwrap().clone();
        if matches!(request, Request::Play { .. }) && font.is_none() {
            return; // still loading
        }
        self.pending = None;
        let song = match (request, font) {
            (Request::Play { file, looping, volume }, Some(font)) if volume > 0.0 => {
                let handheld = cfg!(target_os = "vita");
                let quality = Quality { rate: audio.rate(), polyphony: if handheld { HANDHELD_POLYPHONY } else { Quality::default().polyphony }, effects: !handheld };
                Song::new(&font, files, file, looping, quality).map_err(|e| log::warn!("music: {e}")).ok()
            }
            _ => None,
        };
        audio.set_music(song);
    }
}
