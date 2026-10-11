//! Game files the simulation reads (levels, prefabs, models, sprite sizes), parsed once
//! and shared. The frontend supplies the bytes ([`Files`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use formats::mdl::Mdl;
use formats::wmb::Wmb;

use crate::anim::Scenes;

/// Read access to the game's files (by name, case-insensitive on the frontend's side).
pub trait Files {
    fn read(&self, name: &str) -> Option<Vec<u8>>;
}

/// A parsed MDL with its scene table.
#[derive(Debug)]
pub struct Model {
    pub mdl: Mdl,
    pub scenes: Arc<Scenes>,
}

/// Parsed files by lower-case name; failures are remembered too.
#[derive(Default, Debug, Clone)]
pub struct Assets {
    models: BTreeMap<String, Option<Arc<Model>>>,
    prefabs: BTreeMap<String, Option<Arc<Wmb>>>,
    /// Sprite sizes in pixels (one frame wide).
    sprites: BTreeMap<String, Option<[u32; 2]>>,
}

impl Assets {
    pub fn load_model(&mut self, files: &dyn Files, name: &str) -> Option<Arc<Model>> {
        let name = name.to_lowercase();
        self.models
            .entry(name.clone())
            .or_insert_with(|| {
                let mdl = formats::mdl::parse(&files.read(&name)?).ok()?;
                let scenes = Arc::new(Scenes::of(&mdl));
                Some(Arc::new(Model { mdl, scenes }))
            })
            .clone()
    }

    pub fn load_prefab(&mut self, files: &dyn Files, name: &str) -> Option<Arc<Wmb>> {
        let name = name.to_lowercase();
        self.prefabs.entry(name.clone()).or_insert_with(|| formats::wmb::parse(&files.read(&name)?).ok().map(Arc::new)).clone()
    }

    /// One frame's size of a sprite (`name+N.pcx` strips are N frames wide).
    pub fn load_sprite(&mut self, files: &dyn Files, name: &str) -> Option<[u32; 2]> {
        let name = name.to_lowercase();
        *self.sprites.entry(name.clone()).or_insert_with(|| {
            let pcx = formats::pcx::parse(&files.read(&name)?).ok()?;
            Some([pcx.image.width / sprite_frames(&name), pcx.image.height])
        })
    }

    /// A model loaded before (None if never loaded or missing).
    pub fn model(&self, name: &str) -> Option<Arc<Model>> {
        self.models.get(&name.to_lowercase()).cloned().flatten()
    }

    pub fn prefab(&self, name: &str) -> Option<Arc<Wmb>> {
        self.prefabs.get(&name.to_lowercase()).cloned().flatten()
    }
}

/// Frames in a sprite strip: `name+N.pcx` has N.
pub fn sprite_frames(file: &str) -> u32 {
    file.rsplit_once('+').and_then(|(_, n)| n.to_lowercase().trim_end_matches(".pcx").parse().ok()).unwrap_or(1).max(1)
}
