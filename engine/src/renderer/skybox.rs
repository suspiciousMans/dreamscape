use glow::HasContext;

use crate::shader::link_program;

const VERTEX_SRC: &str = r#"
#version 330 core
out vec2 vClipPos;
void main() {
    vec2 positions[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    gl_Position = vec4(positions[gl_VertexID], 0.0, 1.0);
    vClipPos = positions[gl_VertexID];
}
"#;

const FRAGMENT_SRC: &str = r#"
#version 330 core
in vec2 vClipPos;
out vec4 FragColor;
uniform mat4 uInvViewProj;
uniform vec3 uHorizonColor;
uniform vec3 uZenithColor;
void main() {
    vec4 nearPoint = uInvViewProj * vec4(vClipPos, -1.0, 1.0);
    vec4 farPoint = uInvViewProj * vec4(vClipPos, 1.0, 1.0);
    nearPoint /= nearPoint.w;
    farPoint /= farPoint.w;
    vec3 dir = normalize(farPoint.xyz - nearPoint.xyz);
    float t = clamp(dir.y * 0.5 + 0.5, 0.0, 1.0);
    FragColor = vec4(mix(uHorizonColor, uZenithColor, t), 1.0);
}
"#;

/// A vertical-gradient sky, drawn as a fullscreen triangle **before** any
/// mesh — no cubemap/dome geometry needed. The fragment shader reconstructs
/// each pixel's world-space view ray from the inverse view-projection
/// matrix and mixes `horizon_color`/`zenith_color` by the ray's Y
/// component, so it looks correct from any camera angle and (since it's
/// drawn into the same low-res offscreen target as everything else) is
/// pixelated/dithered right along with the rest of the scene.
pub struct SkyboxPass {
    program: glow::Program,
    empty_vao: glow::VertexArray,
    uniform_inv_view_proj: Option<glow::UniformLocation>,
    uniform_horizon_color: Option<glow::UniformLocation>,
    uniform_zenith_color: Option<glow::UniformLocation>,
}

impl SkyboxPass {
    pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
        let program = link_program(gl, VERTEX_SRC, FRAGMENT_SRC)?;
        let uniform_inv_view_proj = unsafe { gl.get_uniform_location(program, "uInvViewProj") };
        let uniform_horizon_color = unsafe { gl.get_uniform_location(program, "uHorizonColor") };
        let uniform_zenith_color = unsafe { gl.get_uniform_location(program, "uZenithColor") };
        let empty_vao = unsafe { gl.create_vertex_array().map_err(anyhow::Error::msg)? };

        Ok(Self { program, empty_vao, uniform_inv_view_proj, uniform_horizon_color, uniform_zenith_color })
    }

    /// Draws the gradient into whatever framebuffer is currently bound.
    /// Call first, before any scene geometry: depth writing is disabled for
    /// the draw (so it never blocks anything drawn afterward) and restored
    /// before returning.
    pub fn draw(&self, gl: &glow::Context, inv_view_proj: [f32; 16], horizon_color: [f32; 3], zenith_color: [f32; 3]) {
        unsafe {
            gl.depth_mask(false);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.uniform_inv_view_proj.as_ref(), false, &inv_view_proj);
            gl.uniform_3_f32(self.uniform_horizon_color.as_ref(), horizon_color[0], horizon_color[1], horizon_color[2]);
            gl.uniform_3_f32(self.uniform_zenith_color.as_ref(), zenith_color[0], zenith_color[1], zenith_color[2]);
            gl.bind_vertex_array(Some(self.empty_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.depth_mask(true);
        }
    }

    /// # Safety
    /// The pass must not be used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_program(self.program);
        gl.delete_vertex_array(self.empty_vao);
    }
}
