//! The draw calls: level, models, particles, shadows and 2D.

use super::*;

impl Gfx {

    /// A level batch: texture × lightmap (or flat light) plus dynamic lights.
    pub fn draw_world(&self, mesh: &Mesh, view_proj: &Mat4, tex: Texture, lightmap: Option<Texture>, lights: &Lights) {
        let gl = &self.gl;
        let p = if lights.count > 0 { &self.world_dynamic[lights.count - 1] } else { &self.world };
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &view_proj.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_1_i32(p.loc("u_lightmap"), 1);
            gl.uniform_4_f32_slice(p.loc("u_lights"), lights.data.as_flattened());
            gl.uniform_1_i32(p.loc("u_light_count"), lights.count as i32);
        }
        self.bind(0, tex);
        self.bind(1, lightmap.unwrap_or(self.flat_light));
        mesh.draw(gl);
    }

    /// A model, prefab part or sprite at `model` (world transform).
    pub fn draw_model(&self, mesh: &Mesh, view_proj: &Mat4, model: &Mat4, tex: Texture, shade: &Shade, lights: &Lights) {
        let gl = &self.gl;
        let p = if tex.alpha && shade.cutoff > 0.0 { &self.model_cutoff } else { &self.model };
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &(*view_proj * *model).to_cols_array());
            gl.uniform_matrix_4_f32_slice(p.loc("u_model"), false, &model.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_4_f32_slice(p.loc("u_color"), &shade.color);
            gl.uniform_2_f32(p.loc("u_uv_offset"), shade.uv_offset, 0.0);
            gl.uniform_1_f32(p.loc("u_cutoff"), shade.cutoff);
            gl.uniform_1_f32(p.loc("u_unlit"), if shade.unlit { 1.0 } else { 0.0 });
            gl.uniform_3_f32_slice(p.loc("u_sun"), &self.sun.to_array());
            gl.uniform_3_f32_slice(p.loc("u_ambient"), &self.ambient.to_array());
            gl.uniform_4_f32_slice(p.loc("u_lights"), lights.data.as_flattened());
            gl.uniform_1_i32(p.loc("u_light_count"), lights.count as i32);
        }
        self.bind(0, tex);
        mesh.draw(gl);
    }

    /// A particle batch (vertex-coloured quads); blending is set by the caller.
    pub fn draw_particles(&self, mesh: &Mesh, view_proj: &Mat4, tex: Texture) {
        let (gl, p) = (&self.gl, &self.particle);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &view_proj.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
        }
        self.bind(0, tex);
        mesh.draw(gl);
    }

    /// A model mesh's shadow flattened along `sun` (a unit direction pointing down)
    /// onto the plane `z = floor`; stencil and blending are set by the caller.
    pub fn draw_shadow(&self, mesh: &Mesh, view_proj: &Mat4, model: &Mat4, sun: Vec3, floor: f32, alpha: f32) {
        let (gl, p) = (&self.gl, &self.shadow);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_vp"), false, &view_proj.to_cols_array());
            gl.uniform_matrix_4_f32_slice(p.loc("u_model"), false, &model.to_cols_array());
            gl.uniform_4_f32(p.loc("u_sun"), sun.x, sun.y, sun.z, floor);
            gl.uniform_4_f32(p.loc("u_color"), 0.0, 0.0, 0.0, alpha);
        }
        mesh.draw(gl);
    }

    /// A 2D quad mesh in screen pixels (`proj` maps pixels to clip space).
    pub fn draw_ui(&self, mesh: &Mesh, proj: &Mat4, tex: Texture, color: [f32; 4]) {
        let (gl, p) = (&self.gl, &self.ui);
        unsafe {
            gl.use_program(Some(p.program));
            gl.uniform_matrix_4_f32_slice(p.loc("u_mvp"), false, &proj.to_cols_array());
            gl.uniform_1_i32(p.loc("u_tex"), 0);
            gl.uniform_4_f32_slice(p.loc("u_color"), &color);
        }
        self.bind(0, tex);
        mesh.draw(&self.gl);
    }
}
