//! Procedural dream generation. Pure data — no GL calls.
mod build;
mod director;
mod grid;
mod layout;
mod meshes;
mod names;
mod texture;
mod theme;
mod vocab;

pub use build::{generate, portal_surface, Atmosphere, BlockKind, Dream, TexSpec};
pub use director::{DreamDirector, LUCIDITY_TO_WAKE};
pub use meshes::{build as build_mesh, Shape, ALL_SHAPES};
pub use names::{dream_name, whisper};
pub use texture::TEX_SIZE;
pub use theme::{DreamTheme, PropKind};
