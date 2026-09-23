//! Procedural dream generation. Pure data — no GL calls.
mod build;
mod director;
mod grid;
mod layout;
mod theme;

pub use build::{generate, Atmosphere, BlockKind, Dream};
pub use director::{DreamDirector, DREAMS_PER_RUN};
pub use theme::{DreamTheme, PropKind};
