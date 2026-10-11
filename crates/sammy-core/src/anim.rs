//! MDL animation clocks: WDL scenes (frame-name groups such as `walkB`, `TirA`) played
//! by percent per tick, cycling or once. Which vertex frames to blend is all a frontend
//! needs from [`Anim::sample`].

use std::sync::Arc;

use formats::mdl::Mdl;
use crate::time::Secs;

/// A model's scenes: name → frame indices (from the MDL frame names).
#[derive(Debug, Default)]
pub struct Scenes(Vec<(String, Vec<usize>)>);

impl Scenes {
    pub fn of(mdl: &Mdl) -> Self {
        Self(mdl.animations().into_iter().map(|a| (a.name.to_lowercase(), a.frames)).collect())
    }

    /// Frames of a scene (case-insensitive); empty if the model has no such scene.
    pub fn frames(&self, scene: &str) -> Vec<usize> {
        let scene = scene.to_lowercase();
        self.0.iter().find(|(n, _)| *n == scene).map(|(_, f)| f.clone()).unwrap_or_default()
    }
}

/// One model's animation state.
#[derive(Clone, Debug)]
pub struct Anim {
    scenes: Arc<Scenes>,
    pub frames: Vec<usize>,
    /// Frames per second (Acknex default: one frame per 1/16 s tick).
    pub fps: f32,
    /// Position in frames.
    pub time: f32,
    /// WDL "cycle" mode loops; "frame" mode plays once and holds the last frame.
    pub looping: bool,
    scene: String,
}

impl Anim {
    pub fn new(scenes: Arc<Scenes>, frames: Vec<usize>, fps: f32) -> Self {
        Self { scenes, frames, fps, time: 0.0, looping: true, scene: String::new() }
    }

    /// Starts a scene at `step` percent (WDL `PlayerAnimStep`).
    pub fn play(&mut self, scene: &str, looping: bool, step: f32) {
        self.frames = self.scenes.frames(scene);
        self.looping = looping;
        self.scene = scene.to_owned();
        self.time = step / 100.0 * self.span();
    }

    /// Sets the speed in percent of the scene per tick (WDL `PlayerAnimSpeed`).
    pub fn set_speed_pct(&mut self, pct: f32, ticks_per_sec: f32) {
        self.fps = pct / 100.0 * self.span() * ticks_per_sec;
    }

    /// Frames covered by 0..100%: all frames when cycling, up to the last when playing once.
    fn span(&self) -> f32 {
        let n = self.frames.len() as f32;
        if self.looping { n } else { (n - 1.0).max(1.0) }
    }

    pub fn scene(&self) -> &str {
        &self.scene
    }

    /// Jumps to `step` percent of the current scene (WDL `set_cycle`/`set_frame`).
    pub fn set_step(&mut self, step: f32) {
        self.time = step / 100.0 * self.span();
    }

    /// Model frame shown now (nearest key frame).
    pub fn current_frame(&self) -> Option<usize> {
        let n = self.frames.len();
        (n > 0).then(|| self.frames[(self.time.max(0.0) as usize) % n])
    }

    pub fn finished(&self) -> bool {
        !self.looping && self.time >= self.span()
    }

    /// Advances by `secs`; returns the two model frames to blend and the blend factor,
    /// or `None` for a still scene (fewer than two frames).
    pub fn advance(&mut self, secs: Secs) -> Option<(usize, usize, f32)> {
        let secs = secs.get();
        if self.frames.len() < 2 {
            return None;
        }
        self.time += secs * self.fps;
        if !self.looping {
            self.time = self.time.clamp(0.0, (self.frames.len() - 1) as f32);
        }
        self.sample()
    }

    /// The two model frames to blend now and the blend factor (`None` for a still
    /// scene).
    pub fn sample(&self) -> Option<(usize, usize, f32)> {
        let n = self.frames.len();
        if n < 2 {
            return None;
        }
        let (i, j, f) = if self.looping {
            let t = self.time.rem_euclid(n as f32);
            (t as usize % n, (t as usize + 1) % n, t.fract())
        } else {
            let t = self.time.clamp(0.0, (n - 1) as f32);
            (t as usize, (t as usize + 1).min(n - 1), t.fract())
        };
        Some((self.frames[i], self.frames[j], f))
    }
}
