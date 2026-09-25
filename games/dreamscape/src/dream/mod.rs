//! Procedural dream generation. Pure data — no GL calls.
mod build;
mod director;
mod grid;
mod layout;
mod meshes;
mod names;
mod texture;
mod theme;
mod variant;
mod vocab;

pub use build::{
    generate_with, portal_surface, Atmosphere, Block, BlockKind, Dream, Pressure, TexSpec,
};
pub use director::{DreamDirector, RunLength, DEEPER_SHARDS};
pub use meshes::{build as build_mesh, Shape, ALL_SHAPES};
pub use names::{dream_name, whisper};
pub use texture::TEX_SIZE;
pub use theme::{DreamTheme, PropKind, ALL_THEMES};
pub use variant::{roll as roll_twists, Twists, Variant, HASTY_DUST};
