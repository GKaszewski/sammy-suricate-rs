//! Vertex layouts and meshes.

use super::*;

/// Vertex layout of a mesh: float counts per attribute, in attribute order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout {
    /// pos 3, uv 2, uv2 2 (level).
    World,
    /// pos 3, normal 3, uv 2 (models, prefabs, sprites).
    Model,
    /// pos 2, uv 2 (HUD).
    Ui,
    /// pos 3, uv 2, colour 4 (particles).
    Particle,
}

impl Layout {
    pub(super) fn sizes(self) -> &'static [i32] {
        match self {
            Layout::World => &[3, 2, 2],
            Layout::Model => &[3, 3, 2],
            Layout::Ui => &[2, 2],
            Layout::Particle => &[3, 2, 4],
        }
    }

    pub fn floats(self) -> usize {
        self.sizes().iter().sum::<i32>() as usize
    }
}

/// An indexed triangle mesh (interleaved floats, u32 indices).
pub struct Mesh {
    pub(super) vbo: glow::Buffer,
    pub(super) ibo: glow::Buffer,
    pub(super) count: i32,
    pub(super) layout: Layout,
}

pub(super) fn bytes<T>(v: &[T]) -> &[u8] {
    // Plain-old-data slices (f32 / u32) as bytes for upload.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

impl Mesh {
    pub fn new(gl: &glow::Context, layout: Layout, vertices: &[f32], indices: &[u32], dynamic: bool) -> Self {
        unsafe {
            let vbo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(vertices), if dynamic { glow::DYNAMIC_DRAW } else { glow::STATIC_DRAW });
            let ibo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes(indices), glow::STATIC_DRAW);
            Self { vbo, ibo, count: indices.len() as i32, layout }
        }
    }

    /// Replaces the vertices (same count).
    pub fn update(&self, gl: &glow::Context, vertices: &[f32]) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes(vertices));
        }
    }

    /// Replaces vertices and indices (any count; for meshes rebuilt every frame).
    pub fn set(&mut self, gl: &glow::Context, vertices: &[f32], indices: &[u32]) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(vertices), glow::STREAM_DRAW);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes(indices), glow::STREAM_DRAW);
        }
        self.count = indices.len() as i32;
    }

    pub fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            let stride = self.layout.floats() as i32 * 4;
            let mut offset = 0;
            for (i, &n) in self.layout.sizes().iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32);
                gl.vertex_attrib_pointer_f32(i as u32, n, glow::FLOAT, false, stride, offset);
                offset += n * 4;
            }
            gl.draw_elements(glow::TRIANGLES, self.count, glow::UNSIGNED_INT, 0);
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
        }
    }
}
