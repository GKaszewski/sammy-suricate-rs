//! Drawing the world in passes: sky, level, entities, shadows, translucent, near.

use super::*;
use sammy_core::percent::Percent;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pass {
    Sky,
    Opaque,
    Translucent,
}

/// A conservative bounding sphere of an entity: its local bounds about its origin,
/// scaled (any rotation fits).
pub(super) fn sphere(b: &Body, view: &View) -> Sphere {
    let (lo, hi) = match view {
        View::Model { asset, .. } => asset.mdl.frames.first().map_or((Vec3::ZERO, Vec3::ZERO), |f| (Vec3::from(f.bbox_min), Vec3::from(f.bbox_max))),
        View::Prefab(p) => p.bounds,
        View::Sprite { asset, .. } => asset.bounds,
        View::Missing => (Vec3::ZERO, Vec3::ZERO),
    };
    let scale = b.scale.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    (Vec3::from(b.pos.get()), lo.length().max(hi.length()) * scale)
}

impl Scene {
    /// Draws the world from the game's camera.
    pub fn draw(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, parts: &str) {
        let gl = &gfx.gl;
        let w = &sim.world;
        let lights = SceneLights::new(sim, cam.pos, &self.extra_lights);
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
            let alpha = w.bodies.get(id).map_or(Percent::FULL, |b| b.alpha);
            if look.blend || look.flare || alpha < Percent::FULL {
                let d = w.bodies.get(id).map_or(0.0, |b| (Vec3::from(b.pos.get()) - cam.pos).length_squared());
                translucent.push((d, *id));
            } else {
                self.draw_entity(gfx, sim, cam, *id, Pass::Opaque, &mut f);
            }
        }
        if parts.contains("entities") {
            self.draw_shadows(gfx, sim, cam);
        }
        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        unsafe {
            gl.enable(glow::BLEND);
            gl.depth_mask(false);
        }
        for (_, id) in translucent {
            self.draw_entity(gfx, sim, cam, id, Pass::Translucent, &mut f);
        }
        unsafe {
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.stats.set(f.stats);
    }

    /// `Near` entities (help signs) over a fresh depth buffer, after the world's extras.
    pub fn draw_near(&self, gfx: &Gfx, sim: &Sim, cam: &Camera) {
        let gl = &gfx.gl;
        let mut f = FrameLights { lights: SceneLights::new(sim, cam.pos, &self.extra_lights), stats: self.stats.get() };
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.depth_mask(true);
            gl.clear(glow::DEPTH_BUFFER_BIT);
        }
        for (id, _) in sim.world.looks.iter().filter(|(_, l)| l.near) {
            self.draw_entity(gfx, sim, cam, *id, Pass::Translucent, &mut f);
        }
        unsafe {
            gl.disable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.stats.set(f.stats);
    }

    /// A4 engine model shadows (`my.shadow`): each caster's mesh flattened along the
    /// sun onto the floor below it, 50 % black, each floor pixel darkened once.
    pub(super) fn draw_shadows(&self, gfx: &Gfx, sim: &Sim, cam: &Camera) {
        let Some([pan, tilt]) = self.sun else { return };
        let (p, t) = (pan.to_radians(), tilt.to_radians());
        // Light travels from the sun toward the ground.
        let sun = -Vec3::new(t.cos() * p.cos(), t.cos() * p.sin(), t.sin());
        let gl = &gfx.gl;
        let w = &sim.world;
        let mut started = false;
        for (id, _) in w.looks.iter().filter(|(_, l)| l.shadow) {
            let (Some(b), Some((_, View::Model { mesh, .. }))) = (w.bodies.get(id), self.views.get(id)) else { continue };
            if b.hidden || !cam.sees(sphere(b, &self.views[id].1)) {
                continue;
            }
            let o = b.pos;
            let tr = sim.trace_static(sammy_core::collision::Hull::Point, [o[0], o[1], o[2] + SHADOW_PROBE_UP], [o[0], o[1], o[2] - SHADOW_PROBE_DOWN]).trace;
            if !tr.hit() || tr.start_solid {
                continue;
            }
            if !started {
                started = true;
                unsafe {
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    gl.depth_mask(false);
                    gl.enable(glow::STENCIL_TEST);
                    gl.stencil_func(glow::EQUAL, 0, 0xff);
                    gl.stencil_op(glow::KEEP, glow::KEEP, glow::INCR);
                }
            }
            gfx.draw_shadow(mesh, &cam.view_proj, &transform(b, rotation(b.angles)), sun, tr.end[2] + SHADOW_LIFT, SHADOW_ALPHA);
        }
        if started {
            unsafe {
                gl.disable(glow::STENCIL_TEST);
                gl.disable(glow::BLEND);
                gl.depth_mask(true);
            }
        }
    }

    pub(super) fn draw_entity(&self, gfx: &Gfx, sim: &Sim, cam: &Camera, id: Id, pass: Pass, f: &mut FrameLights) {
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
        let alpha = b.alpha.fraction().clamp(0.0, 1.0);
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
