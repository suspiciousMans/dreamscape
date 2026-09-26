use glow::HasContext;

/// WebGL2 only speaks GLSL ES 3.00: swap the desktop `#version 330 core`
/// header for the ES one plus default precisions, and drop `noperspective`
/// (not in ES 3.00, so affine texture wobble falls back to perspective UVs).
#[cfg(target_os = "emscripten")]
fn to_gles(src: &str) -> String {
    let body = src.replacen("#version 330 core", "", 1).replace("noperspective ", "");
    format!(
        "#version 300 es\nprecision highp float;\nprecision highp int;\nprecision highp sampler2D;\n{body}"
    )
}

pub fn compile_shader(gl: &glow::Context, kind: u32, src: &str) -> anyhow::Result<glow::Shader> {
    #[cfg(target_os = "emscripten")]
    let src = &to_gles(src);
    unsafe {
        let shader = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            // Free the shader object before bailing — `ShaderVariantCache`
            // compiles game-supplied shader bodies at runtime, so a
            // malformed variant/post shader would otherwise orphan one
            // shader object per failed attempt.
            gl.delete_shader(shader);
            anyhow::bail!("shader compile error: {log}");
        }
        Ok(shader)
    }
}

pub fn link_program(
    gl: &glow::Context,
    vertex_src: &str,
    fragment_src: &str,
) -> anyhow::Result<glow::Program> {
    unsafe {
        let vs = compile_shader(gl, glow::VERTEX_SHADER, vertex_src)?;
        // If the fragment shader fails to compile, the already-compiled
        // vertex shader must be freed rather than leaked.
        let fs = match compile_shader(gl, glow::FRAGMENT_SHADER, fragment_src) {
            Ok(fs) => fs,
            Err(err) => {
                gl.delete_shader(vs);
                return Err(err);
            }
        };
        let program = match gl.create_program().map_err(anyhow::Error::msg) {
            Ok(program) => program,
            Err(err) => {
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                return Err(err);
            }
        };
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        // Flag the shaders for deletion on both paths — the linked program
        // keeps its own copy, and on failure they must not be orphaned.
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if !linked {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            anyhow::bail!("program link error: {log}");
        }
        Ok(program)
    }
}
