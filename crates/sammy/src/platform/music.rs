//! Music on Bevy audio: [`sammy_platform::music`] decides which song plays and renders
//! it; here a song is a streaming audio source.

use std::sync::{Arc, Mutex};

use bevy::audio::{AddAudioSource, Decodable, Source, Volume};
use bevy::prelude::*;
use rustysynth::SoundFont;
use sammy_platform::music::{BLOCK, Playlist, Quality, Request, Song};

use crate::game::Game;
use crate::platform::storage::Storage;
use crate::platform::vfs::GameFiles;

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<MidiSong>().init_resource::<Music>().add_systems(Startup, load_soundfont).add_systems(Update, (update, apply_volume).chain());
    }
}

/// Rendered audio is stereo.
const CHANNELS: u16 = 2;

/// A song to play (the synthesiser is built when it starts).
#[derive(Asset, TypePath)]
pub struct MidiSong {
    song: Mutex<Option<Song>>,
}

pub struct MidiDecoder {
    song: Option<Song>,
    left: Vec<f32>,
    right: Vec<f32>,
    pos: usize,
}

impl Iterator for MidiDecoder {
    type Item = bevy::audio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= 2 * BLOCK {
            if !self.song.as_mut()?.render(&mut self.left, &mut self.right) {
                return None;
            }
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
        bevy::audio::SampleRate::new(Quality::default().rate).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Decodable for MidiSong {
    type Decoder = MidiDecoder;

    fn decoder(&self) -> MidiDecoder {
        MidiDecoder { song: self.song.lock().unwrap().take(), left: vec![0.0; BLOCK], right: vec![0.0; BLOCK], pos: 2 * BLOCK }
    }
}

/// Playback state.
#[derive(Resource)]
pub struct Music {
    font: Arc<Mutex<Option<Arc<SoundFont>>>>,
    playlist: Playlist,
    /// A song waiting for the SoundFont.
    pending: Option<Request>,
    entity: Option<Entity>,
}

impl Default for Music {
    fn default() -> Self {
        Self { font: default(), playlist: Playlist::from_clock(), pending: None, entity: None }
    }
}

/// Parses the SoundFont (tens of MB; on a thread where there are threads); songs wait
/// for it.
fn load_soundfont(music: Res<Music>, storage: Res<Storage>, files: Res<GameFiles>) {
    if std::env::var("SAMMY_SCREENSHOT").is_ok() {
        return;
    }
    let Some((name, data)) = sammy_platform::music::find_soundfont(&storage, &files) else {
        return warn!("no SoundFont found: music disabled (put a .sf2 next to the game or set SAMMY_SOUNDFONT)");
    };
    let slot = music.font.clone();
    let parse = move || match sammy_platform::music::parse_soundfont(&data) {
        Ok(sf) => {
            info!("SoundFont {name}");
            *slot.lock().unwrap() = Some(sf);
        }
        Err(e) => warn!("SoundFont {name}: {e}"),
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(parse);
    #[cfg(target_arch = "wasm32")]
    parse();
}

/// Song changes, started once the SoundFont is ready.
fn update(mut commands: Commands, files: Res<GameFiles>, game: Res<Game>, options: Res<crate::ui::options::OptionsState>, mut music: ResMut<Music>, mut songs: ResMut<Assets<MidiSong>>) {
    let playing = music.entity.is_some() || music.pending.is_some();
    if let Some(r) = music.playlist.update(&game, options.mode.is_some(), playing) {
        music.pending = Some(r);
    }
    let Some(request) = music.pending.clone() else { return };
    let font = music.font.lock().unwrap().clone();
    if matches!(request, Request::Play { .. }) && font.is_none() {
        return; // still loading
    }
    music.pending = None;
    if let Some(e) = music.entity.take() {
        commands.entity(e).try_despawn();
    }
    let (Request::Play { file, looping, volume }, Some(font)) = (request, font) else { return };
    if volume <= 0.0 {
        return;
    }
    match Song::new(&font, &files, file, looping, Quality::default()) {
        Ok(song) => {
            let handle = songs.add(MidiSong { song: Mutex::new(Some(song)) });
            let v = Volume::Linear(music.playlist.volume(&game));
            music.entity = Some(commands.spawn((MusicPlayer, AudioPlayer(handle), PlaybackSettings::DESPAWN.with_volume(v))).id());
        }
        Err(e) => warn!("music: {e}"),
    }
}

#[derive(Component)]
struct MusicPlayer;

/// `Midi_Vol = Slider_MusicVol`; forgets songs that ended (`Play_Song_Once`).
fn apply_volume(game: Res<Game>, mut music: ResMut<Music>, mut sinks: Query<&mut AudioSink, With<MusicPlayer>>, alive: Query<(), With<MusicPlayer>>) {
    // Commands from `update` are applied before this system (chained).
    if music.entity.is_some_and(|e| !alive.contains(e)) {
        music.entity = None;
    }
    let v = Volume::Linear(music.playlist.volume(&game));
    for mut sink in &mut sinks {
        sink.set_volume(v);
    }
}
