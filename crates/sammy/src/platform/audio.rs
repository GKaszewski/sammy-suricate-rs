//! Sound effects declared in the original `sons.wdl` (`Sound Name, <file.wav>;`),
//! loaded from the game files and played by WDL name.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::audio::AudioSource;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::platform::vfs::GameFiles;

#[derive(Resource, Default)]
pub struct Sounds(HashMap<String, Handle<AudioSource>>);

pub fn load_sounds(files: Res<GameFiles>, mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
    let mut map = HashMap::new();
    let Ok(script) = files.read("sons.wdl") else { return };
    let text: String = script.iter().map(|&b| b as char).collect();
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        let Some(rest) = line.strip_prefix("Sound").or_else(|| line.strip_prefix("SOUND")).or_else(|| line.strip_prefix("sound")) else {
            continue;
        };
        let Some((name, file)) = rest.split_once(',') else { continue };
        let file = file.trim().trim_end_matches(';').trim().trim_start_matches('<').trim_end_matches('>');
        if let Ok(bytes) = files.read(file) {
            map.insert(name.trim().to_lowercase(), sources.add(AudioSource { bytes: Arc::from(bytes) }));
        }
    }
    info!("loaded {} sounds from sons.wdl", map.len());
    commands.insert_resource(Sounds(map));
}

#[derive(SystemParam)]
pub struct Sfx<'w, 's> {
    commands: Commands<'w, 's>,
    sounds: Option<Res<'w, Sounds>>,
}

impl Sfx<'_, '_> {
    /// `play_sound` / `play_entsound` by WDL sound name.
    pub fn play(&mut self, name: &str) {
        self.play_vol(name, 100.0);
    }

    /// `play_sound name, vol` with the WDL volume 0..100.
    pub fn play_vol(&mut self, name: &str, vol: f32) {
        if let Some(h) = self.sounds.as_ref().and_then(|s| s.0.get(&name.to_lowercase())) {
            let v = bevy::audio::Volume::Linear((vol / 100.0).clamp(0.0, 2.0));
            self.commands.spawn((AudioPlayer(h.clone()), PlaybackSettings::DESPAWN.with_volume(v)));
        }
    }
}
