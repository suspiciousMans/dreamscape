use std::collections::HashMap;

use super::link_program;

/// Selects `noperspective` UV interpolation (affine-style texture mapping)
/// instead of perspective-correct — a compile-time GLSL qualifier, so this
/// swaps the active compiled program rather than a uniform.
pub const AFFINE_UV_BIT: u32 = 1 << 0;
/// The vertex shader reads each instance's model matrix from vertex
/// attributes (`aModel`) instead of the `uModel` uniform.
pub const INSTANCED_BIT: u32 = 1 << 1;

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

    /// Every flag combination a game can ask for (one entry per bit set of
    /// the defined flags). Compile them all up front with [`Self::prewarm`]:
    /// a variant compiled on first use costs a visible hitch (about 55 ms on
    /// Intel Mesa with a cold shader cache) in the middle of play.
    pub const ALL_FLAGS: [u32; 4] = [0, AFFINE_UV_BIT, INSTANCED_BIT, AFFINE_UV_BIT | INSTANCED_BIT];

    /// Compiles `flags` now (a no-op for ones already compiled). A failure
    /// is logged and left for the first real use to report.
    pub fn prewarm(&mut self, gl: &glow::Context, flags: &[u32]) {
        for &f in flags {
            if let Err(e) = self.get_or_compile(gl, f) {
                log::warn!("shader variant {f:#x} did not prewarm: {e}");
            }
        }
    }

    fn defines_for(flags: u32) -> String {
        let mut defines = String::new();
        if flags & AFFINE_UV_BIT != 0 {
            defines.push_str("#define AFFINE_UV\n");
        }
        if flags & INSTANCED_BIT != 0 {
            defines.push_str("#define INSTANCED\n");
        }
        defines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_flags_covers_every_combination_of_the_defined_bits() {
        // One defined bit today: 2 variants. Adding a flag bit must extend ALL_FLAGS.
        let defined = AFFINE_UV_BIT | INSTANCED_BIT;
        let combos: Vec<u32> = (0..=defined).filter(|f| f & !defined == 0).collect();
        let mut all = ShaderVariantCache::ALL_FLAGS.to_vec();
        all.sort_unstable();
        assert_eq!(all, combos);
    }

    #[test]
    fn the_affine_variant_defines_its_macro_and_the_plain_one_does_not() {
        assert_eq!(ShaderVariantCache::defines_for(0), "");
        assert!(ShaderVariantCache::defines_for(AFFINE_UV_BIT).contains("AFFINE_UV"));
        let both = ShaderVariantCache::defines_for(AFFINE_UV_BIT | INSTANCED_BIT);
        assert!(both.contains("AFFINE_UV") && both.contains("INSTANCED"));
        assert!(!ShaderVariantCache::defines_for(AFFINE_UV_BIT).contains("INSTANCED"));
    }
}
