use glow::HasContext;

pub fn compile_shader(gl: &glow::Context, kind: u32, src: &str) -> anyhow::Result<glow::Shader> {
    unsafe {
        let shader = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
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
        let fs = compile_shader(gl, glow::FRAGMENT_SHADER, fragment_src)?;
        let program = gl.create_program().map_err(anyhow::Error::msg)?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            anyhow::bail!("program link error: {log}");
        }
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        Ok(program)
    }
}
