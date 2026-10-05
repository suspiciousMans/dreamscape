mod program;
mod variant;

pub use program::{compile_shader, link_program};
pub use variant::{ShaderVariantCache, AFFINE_UV_BIT};

use crate::material::MaterialVariant;

/// The vertex/fragment shader pair a material variant renders with,
/// relative to a game's `assets/` directory.
///
/// `Lit` and `Unlit` share the profile's standard mesh shaders (unlit is a
/// lighting-mode uniform there, not a separate program); only `LitNormals`
/// needs its own program, since it samples a normal map per fragment.
pub fn shader_files_for(variant: &MaterialVariant) -> (&'static str, &'static str) {
    match variant {
        MaterialVariant::LitNormals => ("shaders/mesh_normals.vert", "shaders/mesh_normals.frag"),
        MaterialVariant::Lit | MaterialVariant::Unlit => ("shaders/mesh.vert", "shaders/mesh.frag"),
    }
}
