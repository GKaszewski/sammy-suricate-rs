//! What an entity looks like: its body, model or sprite, animation and light.

use super::*;

/// Where an entity is and how it shows (native coordinates, degrees).
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub pos: [f32; 3],
    /// Pan, tilt, roll.
    pub angles: [f32; 3],
    /// Native axes (X forward, Y left, Z up).
    pub scale: [f32; 3],
    pub hidden: bool,
    /// Opacity in percent (100 = opaque).
    pub alpha: f32,
}

impl Body {
    pub fn new(name: impl Into<String>, pos: [f32; 3]) -> Self {
        Self { name: name.into(), pos, angles: [0.0; 3], scale: [1.0; 3], hidden: false, alpha: OPAQUE }
    }

    /// Uniform scale.
    pub fn scaled(mut self, s: f32) -> Self {
        self.scale = [s; 3];
        self
    }

    /// Scales all axes by `k`.
    pub fn shrink(&mut self, k: f32) {
        self.scale = self.scale.map(|s| s * k);
    }
}

/// Opacity of an opaque body (percent).
pub const OPAQUE: f32 = 100.0;

/// How a sprite is turned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SpriteFacing {
    /// Turns around the vertical axis toward the camera (A4 sprites).
    #[default]
    Billboard,
    /// Parallel to the screen (`facing`).
    Screen,
    /// Fixed in the world by its angles.
    Oriented,
}

/// What an entity is drawn with.
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// MDL model; `frame` fixes a vertex frame, otherwise the entity's [`Animator`] drives it.
    Model { file: String, skin: u32, overlay: bool, frame: Option<usize> },
    /// WMB prefab (map entity).
    Prefab { file: String },
    /// PCX sprite (`name+N.pcx` = N-frame strip, played by the entity's [`SpriteAnim`]).
    Sprite { file: String, facing: SpriteFacing },
}

/// An entity's visual and its render options. `rev` changes when the visual is swapped
/// (`morph`).
#[derive(Clone, Debug)]
pub struct Look {
    pub visual: Visual,
    pub rev: u32,
    /// Unlit, behind the world (sky domes).
    pub sky: bool,
    /// Drawn last, over the world (help signs).
    pub near: bool,
    /// Casts the engine's model shadow.
    pub shadow: bool,
    /// Own blended material (fading signs, translucent sprites).
    pub blend: bool,
    /// Drawn additively (`flare`).
    pub flare: bool,
}

impl Look {
    pub fn new(visual: Visual) -> Self {
        Self { visual, rev: 0, sky: false, near: false, shadow: false, blend: false, flare: false }
    }

    /// Swaps the visual (WDL `morph`).
    pub fn morph(&mut self, visual: Visual) {
        self.visual = visual;
        self.rev += 1;
    }
}

/// A model's animation clock.
#[derive(Clone, Debug)]
pub struct Animator {
    pub model: Arc<Model>,
    pub anim: Anim,
}

impl Animator {
    pub fn new(model: Arc<Model>, frames: Vec<usize>, fps: f32) -> Self {
        let anim = Anim::new(model.scenes.clone(), frames, fps);
        Self { model, anim }
    }
}

/// A sprite strip's clock (frames per second; negative plays backwards).
#[derive(Clone, Copy, Debug)]
pub struct SpriteAnim {
    pub frames: u32,
    pub frame: f32,
    pub fps: f32,
}

/// A dynamic light on an entity: colour 0..255, range in quants (0 = off); a fading
/// light shrinks `rate` per tick and goes out at `until`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub color: [f32; 3],
    pub range: f32,
    pub fade: Option<(f32, f32)>,
}

impl Light {
    /// A flash: start range, shrink per tick, cut-off range.
    pub fn flash(color: [f32; 3], range: f32, rate: f32, until: f32) -> Self {
        Self { color, range, fade: Some((rate, until)) }
    }
}
