//! A linked shader program and its uniforms.

use super::*;

/// A linked program and its uniforms by name.
pub struct Program {
    pub program: glow::Program,
    pub(super) uniforms: Vec<(&'static str, Option<glow::UniformLocation>)>,
}

impl Program {
    pub(super) fn new(gl: &glow::Context, vs: &str, fs: &str, defines: &str, attribs: &[&str], uniforms: &[&'static str]) -> anyhow::Result<Self> {
        let header = format!("#define GAMMA_MAX {GAMMA_MAX:.6}\n{defines}");
        unsafe {
            let program = gl.create_program().map_err(anyhow::Error::msg)?;
            for (kind, src) in [(glow::VERTEX_SHADER, vs.to_owned()), (glow::FRAGMENT_SHADER, header.clone() + fs)] {
                let s = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
                gl.shader_source(s, &src);
                gl.compile_shader(s);
                if !gl.get_shader_compile_status(s) {
                    anyhow::bail!("shader: {}", gl.get_shader_info_log(s));
                }
                gl.attach_shader(program, s);
            }
            for (i, a) in attribs.iter().enumerate() {
                gl.bind_attrib_location(program, i as u32, a);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                anyhow::bail!("program: {}", gl.get_program_info_log(program));
            }
            let uniforms = uniforms.iter().map(|u| (*u, gl.get_uniform_location(program, u))).collect();
            Ok(Self { program, uniforms })
        }
    }

    pub fn loc(&self, name: &str) -> Option<&glow::UniformLocation> {
        self.uniforms.iter().find(|(n, _)| *n == name).and_then(|(_, l)| l.as_ref())
    }
}
