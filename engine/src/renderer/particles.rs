use glow::HasContext;

use crate::mesh::GpuMesh;
use crate::shader::link_program;

const VERTEX_SRC: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aUV;

uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProj;

void main() {
    gl_Position = uProj * uView * uModel * vec4(aPos, 1.0);
}
"#;

const FRAGMENT_SRC: &str = r#"
#version 330 core
out vec4 FragColor;
uniform vec4 uColor;
void main() {
    FragColor = uColor;
}
"#;

/// Draws particles as flat-colored, camera-facing (billboarded) quads —
/// unlike `mesh.vert`/`mesh.frag`, no texture/lighting/fog, just a per-draw
/// solid color with alpha. The first thing in the engine to use GL
/// blending: enabled only for the span between `begin`/`end`, so the rest
/// of the (opaque) pipeline is untouched. Not instanced — particle counts
/// are meant to stay low and PS2-chunky, so one draw call per particle
/// keeps this simple; swap in instancing if a scene ever needs thousands.
pub struct ParticlePass {
    program: glow::Program,
    uniform_model: Option<glow::UniformLocation>,
    uniform_view: Option<glow::UniformLocation>,
    uniform_proj: Option<glow::UniformLocation>,
    uniform_color: Option<glow::UniformLocation>,
}

impl ParticlePass {
    pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
        let program = link_program(gl, VERTEX_SRC, FRAGMENT_SRC)?;
        Ok(Self {
            uniform_model: unsafe { gl.get_uniform_location(program, "uModel") },
            uniform_view: unsafe { gl.get_uniform_location(program, "uView") },
            uniform_proj: unsafe { gl.get_uniform_location(program, "uProj") },
            uniform_color: unsafe { gl.get_uniform_location(program, "uColor") },
            program,
        })
    }

    /// Enables blending and disables depth *writes* (particles still test
    /// against existing scene depth, so real geometry still occludes them —
    /// they just don't occlude each other or write into the depth buffer).
    pub fn begin(&self, gl: &glow::Context, view: &[f32; 16], proj: &[f32; 16]) {
        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.depth_mask(false);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.uniform_view.as_ref(), false, view);
            gl.uniform_matrix_4_f32_slice(self.uniform_proj.as_ref(), false, proj);
        }
    }

    pub fn draw_particle(
        &self,
        gl: &glow::Context,
        mesh: &GpuMesh,
        model: &[f32; 16],
        color: [f32; 4],
    ) {
        unsafe {
            gl.uniform_matrix_4_f32_slice(self.uniform_model.as_ref(), false, model);
            gl.uniform_4_f32(
                self.uniform_color.as_ref(),
                color[0],
                color[1],
                color[2],
                color[3],
            );
        }
        mesh.draw(gl);
    }

    pub fn end(&self, gl: &glow::Context) {
        unsafe {
            gl.depth_mask(true);
            gl.disable(glow::BLEND);
        }
    }

    /// # Safety
    /// The pass must not be used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_program(self.program);
    }
}
