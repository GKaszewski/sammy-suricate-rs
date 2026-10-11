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

/// A bounding sphere: centre and radius (quants).
type Sphere = (Vec3, f32);

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
    bounds: Bounds,
}

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

/// The camera of a frame.
pub struct Camera {
    pub view_proj: Mat4,
    pub pos: Vec3,
    /// World axes of the view: forward, left, up.
    pub axes: Mat3,
    /// Frustum planes (normal inward, `n·p + d >= 0` inside).
    planes: [Vec4; 6],
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
        let view_proj = proj * view;
        // Gribb-Hartmann: each plane is the w row plus or minus another row.
        let rows = [0, 1, 2, 3].map(|i| view_proj.row(i));
        let planes = [rows[3] + rows[0], rows[3] - rows[0], rows[3] + rows[1], rows[3] - rows[1], rows[3] + rows[2], rows[3] - rows[2]].map(|p| p / p.truncate().length());
        Self { view_proj, pos, axes: Mat3::from_cols(forward, left, up), planes }
    }

    /// Whether a sphere is at least partly in view.
    pub fn sees(&self, (c, r): Sphere) -> bool {
        self.planes.iter().all(|p| p.truncate().dot(c) + p.w >= -r)
    }
}

/// A frame's lights and counts, carried through the draw passes.
struct FrameLights {
    lights: SceneLights,
    stats: Stats,
}

/// The scene's dynamic lights (position, range, colour 0..1), nearest the camera first.
struct SceneLights(Vec<(Vec3, f32, [f32; 3])>);

impl SceneLights {
    fn new(sim: &Sim, cam: Vec3) -> Self {
        let w = &sim.world;
        let mut list: Vec<(Vec3, f32, [f32; 3])> = w
            .lights
            .iter()
            .filter(|(_, l)| l.range > 0.0)
            .filter_map(|(id, l)| w.bodies.get(id).map(|b| (Vec3::from(b.pos), l.range, l.color.map(|c| c / 255.0))))
            .collect();
        list.sort_by(|a, b| (a.0 - cam).length_squared().total_cmp(&(b.0 - cam).length_squared()));
        Self(list)
    }

    /// The lights reaching a sphere (at most [`MAX_LIGHTS`]); the shaders skip the rest.
    fn reaching(&self, (c, r): Sphere) -> Lights {
        let mut out = Lights::default();
        for (p, range, color) in self.0.iter().filter(|(p, range, _)| p.distance(c) < range + r).take(MAX_LIGHTS) {
            out.data[out.count * 2] = [p.x, p.y, p.z, *range];
            out.data[out.count * 2 + 1] = [color[0], color[1], color[2], 0.0];
            out.count += 1;
        }
        out
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
                let batches = level.models.iter().flat_map(|m| &m.batches);
                let points = batches.clone().flat_map(|b| b.positions.iter().map(|p| Vec3::from(*p)));
                let bounds = points.fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)));
                let parts = batches
                    .map(|b| {
                        let v: Vec<f32> = (0..b.positions.len()).flat_map(|i| b.positions[i].into_iter().chain(b.normals[i]).chain(b.uvs[i])).collect();
                        (Mesh::new(&gfx.gl, Layout::Model, &v, &b.indices, false), b.texture.map_or(gfx.white, |t| textures[t]))
                    })
                    .collect();
                Some(Rc::new(Prefab { parts, bounds }))
            })
            .clone()
    }

    fn sprite(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<SpriteAsset>> {
        let key = file.to_lowercase();
        self.sprites.entry(key.clone()).or_insert_with(|| load_sprite(gfx, files, &key)).clone()
    }

    /// Rebuilds the level geometry after a load, and the entity views. Animated models
    /// in view of `cam` (all without one) get this frame's pose.
    pub fn sync(&mut self, gfx: &Gfx, files: &GameFiles, sim: &Sim, cam: Option<&Camera>) {
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
        // Animated models on screen: blend this frame's vertex frames.
        let mut animated = 0;
        for (id, (_, v)) in self.views.iter_mut() {
            let visible = w.bodies.get(id).is_some_and(|b| !b.hidden && cam.is_none_or(|c| c.sees(sphere(b, v))));
            if let View::Model { asset, mesh, shown, .. } = v
                && let Some(blend) = w.animators.get(id).and_then(|a| a.anim.sample())
                && *shown != Some(blend)
                && visible
            {
                *shown = Some(blend);
                mesh.update(&gfx.gl, &asset.vertices(blend));
                animated += 1;
            }
        }
        self.stats.set(Stats { animated, ..Default::default() });
    }

    /// Where an entity is on screen: its bounds projected to window pixels
    /// (`[x0, y0, x1, y1]`, y down) and its distance, if it is in front of the camera.
    pub fn screen_rect(&self, sim: &Sim, id: Id, cam: &Camera, size: [f32; 2]) -> Option<([f32; 4], f32)> {
        let b = sim.world.bodies.get(&id)?;
        let (_, view) = self.views.get(&id)?;
        let (bounds, rot) = match view {
            View::Model { asset, .. } => {
                let f = asset.mdl.frames.first()?;
                ((Vec3::from(f.bbox_min), Vec3::from(f.bbox_max)), rotation(b.angles))
            }
            View::Prefab(p) => (p.bounds, rotation(b.angles)),
            View::Sprite { asset, facing } => (asset.bounds, sprite_rotation(b, *facing, cam)),
            View::Missing => return None,
        };
        let m = cam.view_proj * transform(b, rot);
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for i in 0..8 {
            let c = Vec3::new(if i & 1 == 0 { bounds.0.x } else { bounds.1.x }, if i & 2 == 0 { bounds.0.y } else { bounds.1.y }, if i & 4 == 0 { bounds.0.z } else { bounds.1.z });
            let p = m * c.extend(1.0);
            if p.w <= 0.0 {
                return None;
            }
            let (x, y) = ((p.x / p.w * 0.5 + 0.5) * size[0], (0.5 - p.y / p.w * 0.5) * size[1]);
            lo = [lo[0].min(x), lo[1].min(y)];
            hi = [hi[0].max(x), hi[1].max(y)];
        }
        Some(([lo[0], lo[1], hi[0], hi[1]], (Vec3::from(b.pos) - cam.pos).length()))
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
            for (m, _) in &old.walls {
                m.delete(&gfx.gl);
            }
            for t in old.textures.into_iter().chain(old.lightmap) {
                gfx.delete_texture(t);
            }
        }
        let Some(wmb) = sim.level.clone() else { return };
        let mesh = wmb.build_mesh();
        let lightmap = mesh.lightmap.as_ref().map(|lm| gfx.lightmap(lm));
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
            let tex = b.texture.map_or(gfx.white, |t| textures[t]);
            for (indices, bounds) in chunks(&b.positions, &b.indices) {
                // Each chunk gets its own vertices, numbered from 0.
                let mut map = HashMap::new();
                let mut v: Vec<f32> = Vec::new();
                let local: Vec<u32> = indices
                    .iter()
                    .map(|&i| {
                        *map.entry(i).or_insert_with(|| {
                            let i = i as usize;
                            v.extend(b.positions[i].into_iter().chain(b.uvs[i]).chain(if lit { b.lightmap_uvs[i] } else { [0.0, 0.0] }));
                            (v.len() / Layout::World.floats() - 1) as u32
                        })
                    })
                    .collect();
                batches.push((Mesh::new(&gfx.gl, Layout::World, &v, &local, false), tex, lit, bounds));
            }
        }
        let walls = if menu {
            sammy_present::menu::grid_walls(&wmb, sim.world.menu_markers.values())
                .into_iter()
                .map(|q| {
                    let v: Vec<f32> = (0..4).flat_map(|i| q.corners[i].into_iter().chain([0.0, -1.0, 0.0]).chain(q.uvs[i])).collect();
                    (Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false), textures[q.texture])
                })
                .collect()
        } else {
            Vec::new()
        };
        self.level = Some(Level { batches, walls, lightmap, textures });
    }

    /// Draws the world from the game's camera.
    pub fn draw(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, parts: &str) {
        let gl = &gfx.gl;
        let w = &sim.world;
        let lights = SceneLights::new(sim, cam.pos);
        let mut stats = self.stats.get();
        stats.lights = lights.0.len() as u32;
        let mut f = FrameLights { lights, stats };
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
                self.draw_entity(gfx, sim, cam, *id, Pass::Sky, &mut f);
            }
            unsafe {
                gl.clear(glow::DEPTH_BUFFER_BIT);
                gl.enable(glow::CULL_FACE);
            }
        }
        if let Some(level) = self.level.as_ref().filter(|_| parts.contains("level")) {
            for (mesh, tex, lit, bounds) in &level.batches {
                if cam.sees(*bounds) {
                    gfx.draw_world(mesh, &cam.view_proj, *tex, level.lightmap.filter(|_| *lit), &f.lights.reaching(*bounds));
                }
            }
            let l = sammy_present::menu::WALL_LIGHT;
            let wall = Shade { color: [l, l, l, 1.0], unlit: true, ..Default::default() };
            unsafe { gl.disable(glow::CULL_FACE) };
            for (mesh, tex) in &level.walls {
                gfx.draw_model(mesh, &cam.view_proj, &Mat4::IDENTITY, *tex, &wall, &Lights::default());
            }
            unsafe { gl.enable(glow::CULL_FACE) };
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
                self.draw_entity(gfx, sim, cam, *id, Pass::Opaque, &mut f);
            }
        }
        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        unsafe {
            gl.enable(glow::BLEND);
            gl.depth_mask(false);
        }
        for (_, id) in translucent {
            self.draw_entity(gfx, sim, cam, id, Pass::Translucent, &mut f);
        }
        // `Near` entities (help signs) over a fresh depth buffer.
        unsafe {
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
        }
        for (id, _) in w.looks.iter().filter(|(_, l)| l.near) {
            self.draw_entity(gfx, sim, cam, *id, Pass::Translucent, &mut f);
        }
        unsafe {
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.stats.set(f.stats);
    }

    fn draw_entity(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, id: Id, pass: Pass, f: &mut FrameLights) {
        let (lights, stats) = (&f.lights, &mut f.stats);
        let w = &sim.world;
        let (Some(b), Some(look), Some((_, view))) = (w.bodies.get(&id), w.looks.get(&id), self.views.get(&id)) else { return };
        if b.hidden {
            return;
        }
        let bounds = sphere(b, view);
        if pass != Pass::Sky && !cam.sees(bounds) {
            stats.culled += 1;
            return;
        }
        stats.drawn += 1;
        let lights = &if look.sky || matches!(view, View::Sprite { .. }) { Lights::default() } else { lights.reaching(bounds) };
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
            View::Prefab(prefab) => {
                let m = transform(b, rotation(b.angles));
                for (mesh, tex) in &prefab.parts {
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
                let rot = sprite_rotation(b, *facing, cam);
                gfx.draw_model(&asset.mesh, &cam.view_proj, &transform(b, rot), asset.tex, &shade, lights);
            }
            View::Missing => {}
        }
    }
}

impl Scene {
    /// The intro's logo in front of everything (`None` drops it).
    pub fn draw_logo(&mut self, gfx: &Gfx, files: &GameFiles, cam: &Camera, logo: Option<&sammy_present::intro::Logo>) {
        use sammy_present::intro::LogoArt;
        let Some(l) = logo else {
            if let Some((_, mesh, tex)) = self.logo.take() {
                mesh.delete(&gfx.gl);
                if tex != gfx.white {
                    gfx.delete_texture(tex);
                }
            }
            return;
        };
        if self.logo.as_ref().is_none_or(|(id, ..)| *id != l.id) {
            self.draw_logo(gfx, files, cam, None);
            self.logo = match l.art {
                LogoArt::Image(file) => files.read(file).ok().and_then(|d| formats::tga::parse(&d)).map(|img| {
                    let (w, h) = (img.width as f32 / 2.0, img.height as f32 / 2.0);
                    (l.id, quad(gfx, w, h, 1.0), gfx.texture(&img, false, true))
                }),
                LogoArt::Model(file) => self.model(gfx, files, file).map(|a| {
                    // Its own copy of the first skin: the logo is deleted with it.
                    let tex = a.mdl.skins.first().map_or(gfx.white, |s| gfx.texture(&s.image, false, true));
                    (l.id, Mesh::new(&gfx.gl, Layout::Model, &a.vertices((0, 0, 0.0)), &a.vb.indices, false), tex)
                }),
            };
        }
        let Some((_, mesh, tex)) = &self.logo else { return };
        let gl = &gfx.gl;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        // Both kinds face the camera (the quad faces -X, the axes turn X to forward).
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(l.scale), Quat::from_mat3(&cam.axes), Vec3::from(l.pos));
        let shade = Shade { color: [1.0, 1.0, 1.0, l.alpha], unlit: true, cutoff: 0.01, ..Default::default() };
        gfx.draw_model(mesh, &cam.view_proj, &m, *tex, &shade, &Lights::default());
        unsafe { gl.disable(glow::BLEND) };
    }
}

/// An upright `2w × 2h` quad facing -X, showing `u1` of its texture's width.
fn quad(gfx: &Gfx, w: f32, h: f32, u1: f32) -> Mesh {
    #[rustfmt::skip]
    let v = [
        0.0, w, -h,  -1.0, 0.0, 0.0,  0.0, 1.0,
        0.0, -w, -h, -1.0, 0.0, 0.0,  u1, 1.0,
        0.0, -w, h,  -1.0, 0.0, 0.0,  u1, 0.0,
        0.0, w, h,   -1.0, 0.0, 0.0,  0.0, 0.0,
    ];
    Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false)
}

/// How a sprite is turned this frame.
fn sprite_rotation(b: &Body, facing: SpriteFacing, cam: &Camera) -> Quat {
    match facing {
        // Turn about the vertical axis to face the camera (A4 sprites).
        SpriteFacing::Billboard => {
            let d = Vec3::from(b.pos) - cam.pos;
            Quat::from_rotation_z(d.y.atan2(d.x))
        }
        SpriteFacing::Screen => Quat::from_mat3(&cam.axes),
        // The quad faces -X; an oriented sprite faces along its angles.
        SpriteFacing::Oriented => rotation(b.angles) * Quat::from_rotation_z(std::f32::consts::PI),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Sky,
    Opaque,
    Translucent,
}

/// Level chunk edge (quants): batches are split so a dynamic light only re-shades the
/// chunks it reaches and off-screen chunks are skipped.
const CHUNK: f32 = 1024.0;

/// Splits a batch's triangles by the chunk of their centre: each chunk's indices and
/// bounding sphere.
fn chunks(positions: &[[f32; 3]], indices: &[u32]) -> Vec<(Vec<u32>, Sphere)> {
    let mut cells: std::collections::BTreeMap<[i32; 3], Vec<u32>> = Default::default();
    for tri in indices.chunks_exact(3) {
        let c = tri.iter().map(|&i| Vec3::from(positions[i as usize])).sum::<Vec3>() / 3.0;
        cells.entry((c / CHUNK).floor().as_ivec3().to_array()).or_default().extend_from_slice(tri);
    }
    cells
        .into_values()
        .map(|idx| {
            let (lo, hi) = idx.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), &i| (lo.min(Vec3::from(positions[i as usize])), hi.max(Vec3::from(positions[i as usize]))));
            (idx, ((lo + hi) / 2.0, (hi - lo).length() / 2.0))
        })
        .collect()
}

/// A conservative bounding sphere of an entity: its local bounds about its origin,
/// scaled (any rotation fits).
fn sphere(b: &Body, view: &View) -> Sphere {
    let (lo, hi) = match view {
        View::Model { asset, .. } => asset.mdl.frames.first().map_or((Vec3::ZERO, Vec3::ZERO), |f| (Vec3::from(f.bbox_min), Vec3::from(f.bbox_max))),
        View::Prefab(p) => p.bounds,
        View::Sprite { asset, .. } => asset.bounds,
        View::Missing => (Vec3::ZERO, Vec3::ZERO),
    };
    let scale = b.scale.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    (Vec3::from(b.pos), lo.length().max(hi.length()) * scale)
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
    let mesh = quad(gfx, w, h, 1.0 / frames as f32);
    Some(Rc::new(SpriteAsset { mesh, tex: gfx.texture(&img, false, true), frames, bounds: (Vec3::new(-1.0, -w, -h), Vec3::new(1.0, w, h)) }))
}
