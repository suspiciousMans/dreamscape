//! Procedural dream generation. Pure data — no GL calls.
mod build;
mod director;
mod grid;
mod layout;
mod texture;
mod theme;

pub use build::{generate, portal_surface, Atmosphere, BlockKind, Dream, TexSpec};
pub use director::{DreamDirector, LUCIDITY_TO_WAKE};
pub use texture::TEX_SIZE;
pub use theme::{DreamTheme, PropKind};
