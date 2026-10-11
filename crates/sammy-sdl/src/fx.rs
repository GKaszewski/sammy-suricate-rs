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

/// The original screen's vertical arc basis: half of the 60° arc, at 4:3.
const ARC_HALF_DEG: f32 = 30.0;
const ASPECT_43: f32 = 4.0 / 3.0;
/// Rays at a grazing angle to the view axis are clamped to this cosine.
const MIN_RAY_COS: f32 = 0.1;
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

    /// A model's mesh posed at `frame` (fractional frames blend), reused per use.
    fn posed(&mut self, gfx: &Gfx, scene: &mut Scene, files: &GameFiles, file: &'static str, frame: f32, nth: usize) -> Option<(&Mesh, Texture, std::rc::Rc<crate::scene::ModelAsset>)> {
        let asset = scene.model(gfx, files, file)?;
        let last = asset.mdl.frames.len().saturating_sub(1);
        let f = frame.clamp(0.0, last as f32);
        let a = f.floor() as usize;
        let v = asset.vertices((a, (a + 1).min(last), f.fract()));
        let mesh = self.posed.entry((file, nth)).or_insert_with(|| Mesh::new(&gfx.gl, Layout::Model, &v, &asset.vb.indices, true));
        mesh.update(&gfx.gl, &v);
        let tex = asset.skin(1);
        Some((mesh, tex, asset))
    }

    /// The world's extras and the particles (after the scene's translucent pass).
    pub fn draw_world(&mut self, gfx: &Gfx, files: &GameFiles, scene: &mut Scene, sim: &Sim, cam: &Camera) {
        let gl = &gfx.gl;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
        }
        let mut uses: HashMap<&'static str, usize> = HashMap::new();
        for item in self.extras.items(sim) {
            match item {
                Item::Model { file, skin, frame, pos, angles, scale, blend } => {
                    let nth = *uses.entry(file).and_modify(|n| *n += 1).or_insert(0);
                    let Some((mesh, _, asset)) = self.posed(gfx, scene, files, file, frame, nth) else { continue };
                    set_blend(gl, blend);
                    let m = Mat4::from_scale_rotation_translation(Vec3::from(scale), rotation(angles), Vec3::from(pos));
                    let shade = Shade { unlit: true, cutoff: 0.0, ..Default::default() };
                    gfx.draw_model(mesh, &cam.view_proj, &m, asset.skin(skin), &shade, &Lights::default());
                }
                Item::Decal { file, pos, pan, size, alpha } => {
                    let Some(sprite) = scene.sprite(gfx, files, file) else { continue };
                    set_blend(gl, Blend::Alpha);
                    unsafe {
                        gl.enable(glow::POLYGON_OFFSET_FILL);
                        gl.polygon_offset(DECAL_OFFSET.0, DECAL_OFFSET.1);
                    }
                    let m = Mat4::from_scale_rotation_translation(Vec3::new(size, size, 1.0), Quat::from_rotation_z(pan.to_radians()), Vec3::from(pos));
                    let shade = Shade { color: [1.0, 1.0, 1.0, alpha], unlit: true, cutoff: 0.0, ..Default::default() };
                    gfx.draw_model(&self.decal, &cam.view_proj, &m, sprite.tex, &shade, &Lights::default());
                    unsafe { gl.disable(glow::POLYGON_OFFSET_FILL) };
                }
                Item::Sprite { file, frame, pos } => {
                    let Some(sprite) = scene.sprite(gfx, files, file) else { continue };
                    set_blend(gl, Blend::Alpha);
                    // Turned about the vertical to face the camera (the quad faces -X).
                    let d = Vec3::from(pos) - cam.pos;
                    let m = Mat4::from_rotation_translation(Quat::from_rotation_z(d.y.atan2(d.x)), Vec3::from(pos));
                    let shade = Shade { unlit: true, cutoff: 0.01, uv_offset: frame as f32 / sprite.frames.max(1) as f32, ..Default::default() };
                    gfx.draw_model(&sprite.mesh, &cam.view_proj, &m, sprite.tex, &shade, &Lights::default());
                }
                Item::Title { .. } => {}
            }
        }
        self.draw_particles(gfx, files, scene, cam);
        unsafe {
            gl.depth_mask(true);
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
    }

    fn draw_particles(&mut self, gfx: &Gfx, files: &GameFiles, scene: &mut Scene, cam: &Camera) {
        let gl = &gfx.gl;
        let (left, up) = (cam.axes.y_axis, cam.axes.z_axis);
        let right = -left;
        for (key, verts) in self.particles.quads(right.into(), up.into()) {
            let tex = match key.0 {
                Some(file) => match scene.sprite(gfx, files, file) {
                    Some(s) => s.tex,
                    None => continue,
                },
                None => self.dot,
            };
            let v: Vec<f32> = verts.iter().flat_map(|v| v.pos.into_iter().chain(v.uv).chain(v.color)).collect();
            let idx: Vec<u32> = (0..verts.len() as u32 / 4).flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i)).collect();
            let mesh = self.batches.entry(key).or_insert_with(|| Mesh::new(gl, Layout::Particle, &[], &[], true));
            mesh.set(gl, &v, &idx);
            set_blend(gl, if key.1 { Blend::Add } else { Blend::Alpha });
            gfx.draw_particles(mesh, &cam.view_proj, tex);
        }
    }

    /// Over the world: the lose title (lit by its own light) and the HUD models placed
    /// next to the 2D HUD at their mode-6 pixel.
    pub fn draw_over(&mut self, gfx: &Gfx, files: &GameFiles, scene: &mut Scene, sim: &Sim, cam: &Camera, hud: &HudScreen) {
        let HudScreen { window, ui_scale, shown } = *hud;
        let gl = &gfx.gl;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
        }
        for item in self.extras.items(sim) {
            if let Item::Title { file, pos, angles, scale, .. } = item
                && let Some((mesh, _, asset)) = self.posed(gfx, scene, files, file, 0.0, 0)
            {
                set_blend(gl, Blend::Alpha);
                let m = Mat4::from_scale_rotation_translation(Vec3::splat(scale), rotation(angles), Vec3::from(pos));
                let shade = Shade { unlit: true, ..Default::default() };
                gfx.draw_model(mesh, &cam.view_proj, &m, asset.skin(1), &shade, &Lights::default());
            }
        }
        let models = if shown { self.extras.hud_models(sim) } else { Vec::new() };
        let (w, h) = (sammy_present::hud::W, sammy_present::hud::H);
        // Same size relative to the 2D HUD as at 4:3 with the original 60° arc.
        let vfov_43 = 2.0 * (ARC_HALF_DEG.to_radians().tan() / ASPECT_43).atan();
        let size_k = (cam.fov / 2.0).tan() / (vfov_43 / 2.0).tan() * ui_scale / (window[1] / h);
        let half = (cam.fov / 2.0).tan();
        let cam_rot = Quat::from_mat3(&cam.axes);
        for m in models {
            let Some((mesh, _, asset)) = self.posed(gfx, scene, files, m.file, 0.0, 0) else { continue };
            // Mode-6 pixel → window pixel (negative x anchors to the right edge, y to the bottom).
            let x = if m.px[0] < 0.0 { window[0] + m.px[0] * ui_scale } else { window[0] / 2.0 + (m.px[0] - w / 2.0) * ui_scale };
            let y = window[1] - (h - m.px[1]) * ui_scale;
            let (nx, ny) = (x / window[0] * 2.0 - 1.0, 1.0 - y / window[1] * 2.0);
            let ray = (cam.axes.x_axis - cam.axes.y_axis * nx * half * cam.aspect + cam.axes.z_axis * ny * half).normalize();
            let pos = cam.pos + ray * (m.depth / ray.dot(cam.axes.x_axis).max(MIN_RAY_COS));
            let model = Mat4::from_scale_rotation_translation(Vec3::splat(m.scale * size_k), cam_rot * rotation(m.angles), pos);
            set_blend(gl, m.blend);
            let shade = Shade { unlit: true, cutoff: 0.01, ..Default::default() };
            gfx.draw_model(mesh, &cam.view_proj, &model, asset.skin(m.skin), &shade, &Lights::default());
        }
        unsafe {
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
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
