//! The game world on screen: the level's lightmapped geometry and every game entity with
//! a [`Look`], kept in step with the [`Sim`] (built when it appears or is morphed,
//! dropped when it is gone) and drawn in passes: sky, world, translucent, near.

use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat3, Mat4, Quat, Vec3, Vec4};
use glow::HasContext;
use sammy_core::camera::CameraPose;
use sammy_core::id::Id;
use sammy_core::sim::{Body, Look, Sim, SpriteFacing, Visual};
use sammy_platform::GameFiles;

use crate::gfx::{Gfx, Layout, Lights, MAX_LIGHTS, Mesh, Shade, Texture};

mod assets;
mod camera;
mod draw;
mod lights;
mod logo;
mod sync;

pub use self::{assets::*, camera::*};
use self::{draw::*, lights::*};

/// A bounding sphere: centre and radius (quants).
type Sphere = (Vec3, f32);

/// Model shadows: the floor probe from above the owner's origin down, the lift
/// against z-fighting, and 50 % black ([U] in the original).
const SHADOW_PROBE_UP: f32 = 8.0;
const SHADOW_PROBE_DOWN: f32 = 2000.0;
const SHADOW_LIFT: f32 = 0.5;
const SHADOW_ALPHA: f32 = 0.5;
/// Grey tint of a darkened (absent) level icon in the menu.
const DARK_TINT: f32 = 0.35;
/// Near and far planes (quants).
const NEAR: f32 = 4.0;
const FAR: f32 = 40000.0;
/// The original 4:3 screen: wider screens keep its vertical view and see more at the sides.
const ASPECT_43: f32 = 4.0 / 3.0;
/// Level textures named here are left out in the menu (the grid rooms' `black` frame).
const MENU_SKIP: &[&str] = &["black"];

/// A prefab's parts (one mesh per texture) and its local bounds.
struct Prefab {
    parts: Vec<(Mesh, Texture)>,
    bounds: Bounds,
}
type Parts = Rc<Prefab>;
/// Local axis-aligned bounds (min, max).
type Bounds = (Vec3, Vec3);

/// How an entity is drawn.
enum View {
    Model { asset: Rc<ModelAsset>, mesh: Mesh, tex: Texture, shown: Option<(usize, usize, f32)> },
    Prefab(Parts),
    Sprite { asset: Rc<SpriteAsset>, facing: SpriteFacing },
    Missing,
}

struct Level {
    /// Mesh, texture, lightmapped, bounds.
    batches: Vec<(Mesh, Texture, bool, Sphere)>,
    /// The menu's wide-screen grid walls (unlit).
    walls: Vec<(Mesh, Texture)>,
    lightmap: Option<Texture>,
    textures: Vec<Texture>,
}

#[derive(Default)]
pub struct Scene {
    level: Option<Level>,
    loaded: u32,
    models: HashMap<String, Option<Rc<ModelAsset>>>,
    prefabs: HashMap<String, Option<Parts>>,
    sprites: HashMap<String, Option<Rc<SpriteAsset>>>,
    views: HashMap<Id, (u32, View)>,
    /// What the last frame cost (for the profile log).
    pub stats: std::cell::Cell<Stats>,
    /// The intro logo being shown: its id, mesh and texture.
    logo: Option<(u8, Mesh, Texture)>,
    /// Lights not on game entities (the lose title's), and the model shadows' sun
    /// (pan, tilt), set before drawing.
    pub extra_lights: Vec<(Vec3, f32, [f32; 3])>,
    pub sun: Option<[f32; 2]>,
}

/// Per-frame counts for the profile log.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Entities drawn, and left out as off screen.
    pub drawn: u32,
    pub culled: u32,
    /// Animated meshes re-uploaded.
    pub animated: u32,
    /// Dynamic lights in the scene.
    pub lights: u32,
}
