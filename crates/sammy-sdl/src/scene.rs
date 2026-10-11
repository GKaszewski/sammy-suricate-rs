//! The game world on screen: the level's lightmapped geometry and every game entity with
//! a [`Look`], kept in step with the [`Sim`] (built when it appears or is morphed,
//! dropped when it is gone) and drawn in passes: sky, world, translucent, near.

use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat3, Mat4, Quat, Vec3};
use glow::HasContext;
use sammy_core::camera::CameraPose;
use sammy_core::id::Id;
use sammy_core::sim::{Body, Look, Sim, SpriteFacing, Visual};
use sammy_platform::GameFiles;

use crate::gfx::{Gfx, Layout, Lights, MAX_LIGHTS, Mesh, Shade, Texture};

/// Opacity of an opaque body (percent).
const OPAQUE: f32 = 100.0;
/// Grey tint of a darkened (absent) level icon in the menu.
const DARK_TINT: f32 = 0.35;
/// Near and far planes (quants).
const NEAR: f32 = 4.0;
const FAR: f32 = 40000.0;
/// The original 4:3 screen: wider screens keep its vertical view and see more at the sides.
const ASPECT_43: f32 = 4.0 / 3.0;
/// Level textures named here are left out in the menu (the grid rooms' `black` frame).
const MENU_SKIP: &[&str] = &["black"];

/// A parsed MDL ready to draw: its vertex frames and skins.
struct ModelAsset {
    mdl: formats::mdl::Mdl,
    vb: formats::mdl::VertexBuffer,
    skins: Vec<Texture>,
    /// Skins with the `overlay` colour key, built on demand.
    overlay: std::cell::RefCell<HashMap<u32, Texture>>,
}

impl ModelAsset {
    /// Interleaved pos, normal, uv of frames `a`→`b` blended by `f`.
    fn vertices(&self, (a, b, f): (usize, usize, f32)) -> Vec<f32> {
        let last = self.mdl.frames.len() - 1;
        let (fa, fb) = (&self.mdl.frames[a.min(last)], &self.mdl.frames[b.min(last)]);
        let mut out = Vec::with_capacity(self.vb.xyz_index.len() * 8);
        for (i, &xi) in self.vb.xyz_index.iter().enumerate() {
            let xi = xi as usize;
            let lerp = |p: [f32; 3], q: [f32; 3]| [0, 1, 2].map(|k| p[k] + (q[k] - p[k]) * f);
            out.extend(lerp(fa.positions[xi], fb.positions[xi]));
            out.extend(lerp(fa.normals[xi], fb.normals[xi]));
            out.extend(self.vb.uvs[i]);
        }
        out
    }

    fn skin(&self, n: u32) -> Texture {
        self.skins[(n.max(1) as usize - 1).min(self.skins.len() - 1)]
    }
}

/// A sprite: its quad (one frame of the strip), texture and frame count.
struct SpriteAsset {
    mesh: Mesh,
    tex: Texture,
    frames: u32,
}

/// A prefab's parts: one mesh per texture.
type Parts = Rc<Vec<(Mesh, Texture)>>;

/// How an entity is drawn.
enum View {
    Model { asset: Rc<ModelAsset>, mesh: Mesh, tex: Texture, shown: Option<(usize, usize, f32)> },
    Prefab(Parts),
    Sprite { asset: Rc<SpriteAsset>, facing: SpriteFacing },
    Missing,
}

struct Level {
    /// Mesh, texture, lightmapped.
    batches: Vec<(Mesh, Texture, bool)>,
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
}

/// The camera of a frame.
pub struct Camera {
    pub view_proj: Mat4,
    pub pos: Vec3,
    /// World axes of the view: forward, left, up.
    pub axes: Mat3,
}

impl Camera {
    pub fn from_pose(pose: &CameraPose, arc: f32, aspect: f32) -> Self {
        let (pos, target) = match *pose {
            CameraPose::LookAt { pos, target } => (Vec3::from(pos), Vec3::from(target)),
            CameraPose::Angles { pos, angles } => {
                let (p, t) = (angles[0].to_radians(), angles[1].to_radians());
                let pos = Vec3::from(pos);
                (pos, pos + Vec3::new(t.cos() * p.cos(), t.cos() * p.sin(), t.sin()))
            }
        };
        let forward = (target - pos).normalize_or(Vec3::X);
        let left = Vec3::Z.cross(forward).normalize_or(Vec3::Y);
        let up = forward.cross(left);
        // `arc` is horizontal on the original 4:3 screen (Hor+ beyond it).
        let fov = 2.0 * ((arc.to_radians() / 2.0).tan() / aspect.min(ASPECT_43)).atan();
        let view = Mat4::look_at_rh(pos, target, Vec3::Z);
        let proj = Mat4::perspective_rh_gl(fov, aspect, NEAR, FAR);
        Self { view_proj: proj * view, pos, axes: Mat3::from_cols(forward, left, up) }
    }
}

/// A4 euler angles (pan about Z, tilt nose-up, roll) as a rotation in native axes.
pub fn rotation([pan, tilt, roll]: [f32; 3]) -> Quat {
    Quat::from_rotation_z(pan.to_radians()) * Quat::from_rotation_y(-tilt.to_radians()) * Quat::from_rotation_x(roll.to_radians())
}

fn transform(b: &Body, rot: Quat) -> Mat4 {
    Mat4::from_scale_rotation_translation(Vec3::from(b.scale), rot, Vec3::from(b.pos))
}

impl Scene {
    fn model(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<ModelAsset>> {
        let key = file.to_lowercase();
        self.models
            .entry(key.clone())
            .or_insert_with(|| {
                let mdl = formats::mdl::parse(&files.read(&key).ok()?).ok()?;
                let vb = mdl.vertex_buffer();
                let mut skins: Vec<Texture> = mdl.skins.iter().map(|s| gfx.texture(&s.image, false, true)).collect();
                if skins.is_empty() {
                    skins.push(gfx.white);
                }
                Some(Rc::new(ModelAsset { mdl, vb, skins, overlay: Default::default() }))
            })
            .clone()
    }

    fn prefab(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Parts> {
        let key = file.to_lowercase();
        self.prefabs
            .entry(key.clone())
            .or_insert_with(|| {
                let wmb = formats::wmb::parse(&files.read(&key).ok()?).ok()?;
                let level = wmb.build_mesh();
                let textures: Vec<Texture> = wmb.textures.iter().map(|t| gfx.texture(&t.image, true, true)).collect();
                let parts = level
                    .models
                    .iter()
                    .flat_map(|m| &m.batches)
                    .map(|b| {
                        let v: Vec<f32> = (0..b.positions.len()).flat_map(|i| b.positions[i].into_iter().chain(b.normals[i]).chain(b.uvs[i])).collect();
                        (Mesh::new(&gfx.gl, Layout::Model, &v, &b.indices, false), b.texture.map_or(gfx.white, |t| textures[t]))
                    })
                    .collect();
                Some(Rc::new(parts))
            })
            .clone()
    }

    fn sprite(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<SpriteAsset>> {
        let key = file.to_lowercase();
        self.sprites.entry(key.clone()).or_insert_with(|| load_sprite(gfx, files, &key)).clone()
    }

    /// Rebuilds the level geometry after a load, and the entity views.
    pub fn sync(&mut self, gfx: &Gfx, files: &GameFiles, sim: &Sim) {
        if self.loaded != sim.loads {
            self.loaded = sim.loads;
            self.build_level(gfx, sim);
        }
        let w = &sim.world;
        self.views.retain(|id, (_, v)| {
            let keep = w.looks.contains_key(id);
            if !keep && let View::Model { mesh, .. } = v {
                mesh.delete(&gfx.gl);
            }
            keep
        });
        for (id, look) in &w.looks {
            if self.views.get(id).is_some_and(|(rev, _)| *rev == look.rev) {
                continue;
            }
            let view = self.view(gfx, files, sim, *id, look);
            if let Some((_, View::Model { mesh, .. })) = self.views.insert(*id, (look.rev, view)) {
                mesh.delete(&gfx.gl);
            }
        }
        // Animated models: blend this frame's vertex frames.
        for (id, (_, v)) in self.views.iter_mut() {
            if let View::Model { asset, mesh, shown, .. } = v
                && let Some(blend) = w.animators.get(id).and_then(|a| a.anim.sample())
                && *shown != Some(blend)
            {
                *shown = Some(blend);
                mesh.update(&gfx.gl, &asset.vertices(blend));
            }
        }
    }

    fn view(&mut self, gfx: &Gfx, files: &GameFiles, sim: &Sim, id: Id, look: &Look) -> View {
        match &look.visual {
            Visual::Model { file, skin, overlay, frame } => {
                let Some(asset) = self.model(gfx, files, file) else { return View::Missing };
                let first = frame.or_else(|| sim.world.animators.get(&id).and_then(|a| a.anim.sample()).map(|(a, _, _)| a)).unwrap_or(0);
                let mesh = Mesh::new(&gfx.gl, Layout::Model, &asset.vertices((first, first, 0.0)), &asset.vb.indices, frame.is_none());
                let tex = if *overlay { overlay_skin(gfx, &asset, *skin) } else { asset.skin(*skin) };
                View::Model { asset, mesh, tex, shown: frame.map(|f| (f, f, 0.0)) }
            }
            Visual::Prefab { file } => self.prefab(gfx, files, file).map_or(View::Missing, View::Prefab),
            Visual::Sprite { file, facing } => self.sprite(gfx, files, file).map_or(View::Missing, |asset| View::Sprite { asset, facing: *facing }),
        }
    }

    fn build_level(&mut self, gfx: &Gfx, sim: &Sim) {
        if let Some(old) = self.level.take() {
            for (m, ..) in &old.batches {
                m.delete(&gfx.gl);
            }
            for t in old.textures.into_iter().chain(old.lightmap) {
                gfx.delete_texture(t);
            }
        }
        let Some(wmb) = sim.level.clone() else { return };
        let mesh = wmb.build_mesh();
        let lightmap = mesh.lightmap.as_ref().map(|lm| gfx.texture(lm, false, true));
        let textures: Vec<Texture> = wmb.textures.iter().map(|t| gfx.texture(&t.image, true, true)).collect();
        let menu = sim.flow.file.eq_ignore_ascii_case(sammy_core::levels::MENU_FILE);
        let mut batches = Vec::new();
        for b in mesh.models.iter().flat_map(|m| &m.batches) {
            let name = b.texture.map_or("", |t| wmb.textures[t].name.as_str());
            if menu && MENU_SKIP.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                continue;
            }
            // Batches without lightmap UVs draw with flat light.
            let lit = b.lit && lightmap.is_some();
            let v: Vec<f32> = (0..b.positions.len()).flat_map(|i| b.positions[i].into_iter().chain(b.uvs[i]).chain(if lit { b.lightmap_uvs[i] } else { [0.0, 0.0] })).collect();
            batches.push((Mesh::new(&gfx.gl, Layout::World, &v, &b.indices, false), b.texture.map_or(gfx.white, |t| textures[t]), lit));
        }
        self.level = Some(Level { batches, lightmap, textures });
    }

    /// Draws the world from the game's camera.
    pub fn draw(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, parts: &str) {
        let gl = &gfx.gl;
        let w = &sim.world;
        let lights = nearest_lights(sim, cam.pos);
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.disable(glow::BLEND);
            gl.enable(glow::CULL_FACE);
            gl.front_face(glow::CCW);
        }
        // Sky domes first, then a fresh depth buffer for the world.
        let sky: Vec<Id> = w.looks.iter().filter(|(_, l)| l.sky && parts.contains("sky")).map(|(id, _)| *id).collect();
        if !sky.is_empty() {
            unsafe { gl.disable(glow::CULL_FACE) };
            for id in &sky {
                self.draw_entity(gfx, sim, cam, *id, &lights, Pass::Sky);
            }
            unsafe {
                gl.clear(glow::DEPTH_BUFFER_BIT);
                gl.enable(glow::CULL_FACE);
            }
        }
        if let Some(level) = self.level.as_ref().filter(|_| parts.contains("level")) {
            for (mesh, tex, lit) in &level.batches {
                gfx.draw_world(mesh, &cam.view_proj, *tex, level.lightmap.filter(|_| *lit), &lights);
            }
        }
        // Models face both ways like Bevy's skinned meshes (some are open shells).
        unsafe { gl.disable(glow::CULL_FACE) };
        let mut translucent = Vec::new();
        for (id, look) in w.looks.iter().filter(|_| parts.contains("entities")) {
            if look.sky || look.near {
                continue;
            }
            let alpha = w.bodies.get(id).map_or(OPAQUE, |b| b.alpha);
            if look.blend || look.flare || alpha < OPAQUE {
                let d = w.bodies.get(id).map_or(0.0, |b| (Vec3::from(b.pos) - cam.pos).length_squared());
                translucent.push((d, *id));
            } else {
                self.draw_entity(gfx, sim, cam, *id, &lights, Pass::Opaque);
            }
        }
        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        unsafe {
            gl.enable(glow::BLEND);
            gl.depth_mask(false);
        }
        for (_, id) in translucent {
            self.draw_entity(gfx, sim, cam, id, &lights, Pass::Translucent);
        }
        // `Near` entities (help signs) over a fresh depth buffer.
        unsafe {
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
        }
        for (id, _) in w.looks.iter().filter(|(_, l)| l.near) {
            self.draw_entity(gfx, sim, cam, *id, &lights, Pass::Translucent);
        }
        unsafe {
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
    }

    fn draw_entity(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, id: Id, lights: &Lights, pass: Pass) {
        let w = &sim.world;
        let (Some(b), Some(look), Some((_, view))) = (w.bodies.get(&id), w.looks.get(&id), self.views.get(&id)) else { return };
        if b.hidden {
            return;
        }
        let gl = &gfx.gl;
        let alpha = (b.alpha / OPAQUE).clamp(0.0, 1.0);
        let tint = if w.menu_items.get(&id).is_some_and(|i| i.dark) { DARK_TINT } else { 1.0 };
        let mut shade = Shade { color: [tint, tint, tint, alpha], unlit: look.sky, ..Default::default() };
        if pass == Pass::Translucent {
            shade.cutoff = 0.01;
            unsafe {
                if look.flare {
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE);
                } else {
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                }
            }
        }
        match view {
            View::Model { mesh, tex, .. } => gfx.draw_model(mesh, &cam.view_proj, &transform(b, rotation(b.angles)), *tex, &shade, lights),
            View::Prefab(parts) => {
                let m = transform(b, rotation(b.angles));
                for (mesh, tex) in parts.iter() {
                    gfx.draw_model(mesh, &cam.view_proj, &m, *tex, &shade, lights);
                }
            }
            View::Sprite { asset, facing } => {
                shade.unlit = true;
                if let Some(a) = w.sprite_anims.get(&id)
                    && asset.frames > 1
                {
                    shade.uv_offset = a.frame.floor() / asset.frames as f32;
                }
                let rot = match facing {
                    // Turn about the vertical axis to face the camera (A4 sprites).
                    SpriteFacing::Billboard => {
                        let d = Vec3::from(b.pos) - cam.pos;
                        Quat::from_rotation_z(d.y.atan2(d.x))
                    }
                    SpriteFacing::Screen => Quat::from_mat3(&cam.axes),
                    // The quad faces -X; an oriented sprite faces along its angles.
                    SpriteFacing::Oriented => rotation(b.angles) * Quat::from_rotation_z(std::f32::consts::PI),
                };
                gfx.draw_model(&asset.mesh, &cam.view_proj, &transform(b, rot), asset.tex, &shade, lights);
            }
            View::Missing => {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Sky,
    Opaque,
    Translucent,
}

/// The lights nearest the camera (position + range, colour 0..1).
fn nearest_lights(sim: &Sim, cam: Vec3) -> Lights {
    let w = &sim.world;
    let mut list: Vec<(f32, [f32; 3], &sammy_core::sim::Light)> =
        w.lights.iter().filter(|(_, l)| l.range > 0.0).filter_map(|(id, l)| w.bodies.get(id).map(|b| ((Vec3::from(b.pos) - cam).length_squared(), b.pos, l))).collect();
    list.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = [[0.0; 4]; MAX_LIGHTS * 2];
    for (i, (_, p, l)) in list.into_iter().take(MAX_LIGHTS).enumerate() {
        out[i * 2] = [p[0], p[1], p[2], l.range];
        out[i * 2 + 1] = [l.color[0] / 255.0, l.color[1] / 255.0, l.color[2] / 255.0, 0.0];
    }
    out
}

/// A skin with A4 `overlay`: palette index 0 (or black) is transparent.
fn overlay_skin(gfx: &Gfx, asset: &ModelAsset, n: u32) -> Texture {
    if let Some(t) = asset.overlay.borrow().get(&n) {
        return *t;
    }
    let i = (n.max(1) as usize - 1).min(asset.mdl.skins.len().saturating_sub(1));
    let Some(skin) = asset.mdl.skins.get(i) else { return asset.skin(n) };
    let mut img = skin.image.clone();
    match &skin.indices {
        Some(idx) => img.data.chunks_exact_mut(4).zip(idx).filter(|(_, i)| **i == 0).for_each(|(px, _)| px[3] = 0),
        None => img.data.chunks_exact_mut(4).filter(|px| formats::image::is_color_key(px)).for_each(|px| px[3] = 0),
    }
    let t = gfx.texture(&img, false, true);
    asset.overlay.borrow_mut().insert(n, t);
    t
}

/// Sprites are upright quads sized in quants = pixels, facing -X; `name+N.pcx` strips
/// show one frame at a time.
fn load_sprite(gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<SpriteAsset>> {
    let pcx = formats::pcx::parse(&files.read(file).ok()?).ok()?;
    let frames = sammy_core::sim::assets::sprite_frames(file);
    let mut img = pcx.image.clone();
    // Colour key: palette index 0 (8-bit) or near-black (24-bit).
    match &pcx.indices {
        Some(idx) => img.data.chunks_exact_mut(4).zip(idx).filter(|(_, i)| **i == 0).for_each(|(px, _)| px[3] = 0),
        None => img.data.chunks_exact_mut(4).filter(|px| formats::image::is_color_key(px)).for_each(|px| px[3] = 0),
    }
    let (w, h) = (img.width as f32 / frames as f32 / 2.0, img.height as f32 / 2.0);
    let u1 = 1.0 / frames as f32;
    #[rustfmt::skip]
    let v = [
        0.0, w, -h,  -1.0, 0.0, 0.0,  0.0, 1.0,
        0.0, -w, -h, -1.0, 0.0, 0.0,  u1, 1.0,
        0.0, -w, h,  -1.0, 0.0, 0.0,  u1, 0.0,
        0.0, w, h,   -1.0, 0.0, 0.0,  0.0, 0.0,
    ];
    let mesh = Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false);
    Some(Rc::new(SpriteAsset { mesh, tex: gfx.texture(&img, false, true), frames }))
}
