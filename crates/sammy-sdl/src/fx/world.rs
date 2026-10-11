//! The world's extras and the particles.

use super::*;

impl Fx {

    /// A model's mesh posed at `frame` (fractional frames blend), reused per use.
    pub(super) fn posed(&mut self, gfx: &Gfx, scene: &mut Scene, files: &GameFiles, file: &'static str, frame: f32, nth: usize) -> Option<(&Mesh, Texture, std::rc::Rc<crate::scene::ModelAsset>)> {
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

    pub(super) fn draw_particles(&mut self, gfx: &Gfx, files: &GameFiles, scene: &mut Scene, cam: &Camera) {
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
}
