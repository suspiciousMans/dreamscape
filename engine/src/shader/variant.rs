use std::collections::HashMap;

use super::link_program;

/// Selects `noperspective` UV interpolation (affine-style texture mapping)
/// instead of perspective-correct — a compile-time GLSL qualifier, so this
/// swaps the active compiled program rather than a uniform.
pub const AFFINE_UV_BIT: u32 = 1 << 0;

/// Compiles and caches shader program variants keyed by a `#define` bitmask,
/// so toggling a compile-time flag (like affine UV mapping) swaps a cached
/// program instead of recompiling every frame.
///
/// `vertex_body`/`fragment_body` must NOT include a `#version` line — this
/// cache prepends `#version 330 core` followed by the defines for the
/// requested flags.
pub struct ShaderVariantCache {
    vertex_body: String,
    fragment_body: String,
    variants: HashMap<u32, glow::Program>,
}

impl ShaderVariantCache {
    pub fn new(vertex_body: impl Into<String>, fragment_body: impl Into<String>) -> Self {
        Self {
            vertex_body: vertex_body.into(),
            fragment_body: fragment_body.into(),
            variants: HashMap::new(),
        }
    }

    pub fn get_or_compile(
        &mut self,
        gl: &glow::Context,
        flags: u32,
    ) -> anyhow::Result<glow::Program> {
        if let Some(&program) = self.variants.get(&flags) {
            return Ok(program);
        }

        let defines = Self::defines_for(flags);
        let vertex_src = format!("#version 330 core\n{defines}{}", self.vertex_body);
        let fragment_src = format!("#version 330 core\n{defines}{}", self.fragment_body);
        let program = link_program(gl, &vertex_src, &fragment_src)?;
        self.variants.insert(flags, program);
        Ok(program)
    }

    fn defines_for(flags: u32) -> String {
        let mut defines = String::new();
        if flags & AFFINE_UV_BIT != 0 {
            defines.push_str("#define AFFINE_UV\n");
        }
        defines
    }
}
