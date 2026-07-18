use glow::HasContext;

use crate::shader::link_program;

const VERTEX_SRC: &str = r#"
#version 330 core
out vec2 vUV;
void main() {
    vec2 positions[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    vec2 uvs[3] = vec2[3](vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
    gl_Position = vec4(positions[gl_VertexID], 0.0, 1.0);
    vUV = uvs[gl_VertexID];
}
"#;

/// Plain passthrough; the posterize/dither look is layered on top of this in
/// a later shader-profile revision without changing the composite pipeline.
pub const DEFAULT_FRAGMENT_SRC: &str = r#"
#version 330 core
in vec2 vUV;
out vec4 FragColor;
uniform sampler2D uSourceTex;
void main() {
    FragColor = texture(uSourceTex, vUV);
}
"#;

/// Draws a fullscreen triangle sampling the offscreen color texture into
/// whatever framebuffer/viewport is currently bound (the upscale is "free" —
/// GL_NEAREST sampling a small texture into a larger viewport).
pub struct CompositePass {
    program: glow::Program,
    empty_vao: glow::VertexArray,
    uniform_source_tex: Option<glow::UniformLocation>,
    uniform_color_levels: Option<glow::UniformLocation>,
    uniform_dither_strength: Option<glow::UniformLocation>,
}

/// Values applied by the composite pass's posterize/dither stage. Levels this
/// high with zero dither strength is visually a no-op passthrough.
pub struct PostParams {
    pub color_levels: f32,
    pub dither_strength: f32,
}

impl Default for PostParams {
    fn default() -> Self {
        Self {
            color_levels: 256.0,
            dither_strength: 0.0,
        }
    }
}

impl CompositePass {
    pub fn new(gl: &glow::Context, fragment_src: &str) -> anyhow::Result<Self> {
        let program = link_program(gl, VERTEX_SRC, fragment_src)?;
        let uniform_source_tex = unsafe { gl.get_uniform_location(program, "uSourceTex") };
        let uniform_color_levels = unsafe { gl.get_uniform_location(program, "uColorLevels") };
        let uniform_dither_strength =
            unsafe { gl.get_uniform_location(program, "uDitherStrength") };
        let empty_vao = unsafe { gl.create_vertex_array().map_err(anyhow::Error::msg)? };

        Ok(Self {
            program,
            empty_vao,
            uniform_source_tex,
            uniform_color_levels,
            uniform_dither_strength,
        })
    }

    pub fn draw(&self, gl: &glow::Context, source_texture: glow::Texture, params: &PostParams) {
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(source_texture));
            gl.uniform_1_i32(self.uniform_source_tex.as_ref(), 0);
            gl.uniform_1_f32(self.uniform_color_levels.as_ref(), params.color_levels);
            gl.uniform_1_f32(
                self.uniform_dither_strength.as_ref(),
                params.dither_strength,
            );
            gl.bind_vertex_array(Some(self.empty_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.enable(glow::DEPTH_TEST);
        }
    }

    /// # Safety
    /// The pass must not be used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_program(self.program);
        gl.delete_vertex_array(self.empty_vao);
    }
}
