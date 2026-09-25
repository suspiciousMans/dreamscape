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
    generate_nightmare, generate_with, portal_surface, Atmosphere, Block, BlockKind, Dream,
    Pressure, TexSpec, SIGILS,
};
pub use director::{DreamDirector, RunLength, DEEPER_SHARDS, NIGHTMARE_EVERY};
pub use meshes::{build as build_mesh, Shape, ALL_SHAPES};
pub use names::{dream_name, whisper};
pub use texture::{Pattern, TEX_SIZE};
pub use theme::{
    DreamTheme, EnemyKind, Mood, PropKind, ALL_ENEMY_KINDS, ALL_PROPS as ALL_PROP_KINDS, ALL_THEMES,
};
pub use variant::{roll_with as roll_twists, Twists, Variant, HASTY_DUST};
