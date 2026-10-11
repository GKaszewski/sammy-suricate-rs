//! Particles ([`sammy_present::particles`]) and the extras ([`sammy_present::extras`]):
//! their state follows the game's events and clock; they are drawn in the world pass
//! (rings, shadows, popups, particles) and over it (the lose title, the HUD wasp and
//! Eye of Truth plate next to the 2D HUD).

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};
use glow::HasContext;
use sammy_core::player::State;
use sammy_core::sim::{Event, Sim};
use sammy_platform::GameFiles;
use sammy_present::extras::{Blend, Extras, Item, View};
use sammy_present::particles::{BatchKey, Particles};

use crate::gfx::{Gfx, Layout, Lights, Mesh, Shade, Texture};
use crate::scene::{Camera, Scene, rotation};

mod over;
mod world;

/// Ground decals sit over the floor by a depth offset (factor, units).
const DECAL_OFFSET: (f32, f32) = (-1.0, -2.0);

pub struct Fx {
    pub particles: Particles,
    pub extras: Extras,
    /// Sammy's state and teleporting last frame (for his dust and sparkles).
    sammy: Option<(State, bool)>,
    dot: Texture,
    /// One streamed mesh per particle batch.
    batches: HashMap<BatchKey, Mesh>,
    /// Posed model meshes of the extras, per (file, nth use this frame).
    posed: HashMap<(&'static str, usize), Mesh>,
    /// A unit square lying flat (decals).
    decal: Mesh,
}

impl Fx {
    pub fn new(gfx: &Gfx) -> Self {
        #[rustfmt::skip]
        let v = [
            -0.5, -0.5, 0.0,  0.0, 0.0, 1.0,  0.0, 1.0,
             0.5, -0.5, 0.0,  0.0, 0.0, 1.0,  1.0, 1.0,
             0.5,  0.5, 0.0,  0.0, 0.0, 1.0,  1.0, 0.0,
            -0.5,  0.5, 0.0,  0.0, 0.0, 1.0,  0.0, 0.0,
        ];
        Self {
            particles: Particles::default(),
            extras: Extras::default(),
            sammy: None,
            dot: gfx.texture(&sammy_present::particles::dot_image(), false, true),
            batches: HashMap::new(),
            posed: HashMap::new(),
            decal: Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false),
        }
    }

    /// Takes a game event (effects, call rings, popups; a new level clears).
    pub fn event(&mut self, sim: &Sim, e: &Event) {
        sammy_present::effects::play(&mut self.particles, e);
        self.extras.event(sim, e);
    }

    /// Advances `secs` of game time: the emitters, the extras, the particles.
    pub fn update(&mut self, sim: &Sim, cam: Option<&Camera>, secs: f32) {
        self.particles.advance(secs);
        if sim.flow.phase != sammy_core::flow::Phase::Intro {
            sammy_present::effects::emit_trails(&mut self.particles, sim);
            sammy_present::effects::emit_sammy(&mut self.particles, sim, &mut self.sammy);
        }
        let view = cam.map_or(View::default(), |c| View::looking(c.pos.into(), c.axes.x_axis.into()));
        self.extras.update(sim, view, secs, &mut self.particles);
        self.particles.update();
    }

    /// Lights of the extras (the lose title's), for the scene.
    pub fn lights(&self, sim: &Sim) -> Vec<(Vec3, f32, [f32; 3])> {
        self.extras
            .items(sim)
            .into_iter()
            .filter_map(|i| match i {
                Item::Title { pos, light: (color, range), .. } => Some((Vec3::from(pos), range, color.map(|c| c / 255.0))),
                _ => None,
            })
            .collect()
    }

}

/// Where the 2D HUD is: the screen in pixels, window pixels per HUD pixel, and whether
/// it shows.
#[derive(Clone, Copy, Debug)]
pub struct HudScreen {
    pub window: [f32; 2],
    pub ui_scale: f32,
    pub shown: bool,
}

fn set_blend(gl: &glow::Context, blend: Blend) {
    unsafe {
        match blend {
            Blend::Alpha => gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA),
            Blend::Add => gl.blend_func(glow::SRC_ALPHA, glow::ONE),
        }
    }
}
