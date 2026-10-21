//! Procedural dream generation. Pure data — no GL calls.
mod build;
mod detail;
mod director;
mod grid;
mod layout;
mod meshes;
mod names;
mod texture;
mod theme;
mod variant;
mod vocab;

pub use grid::{Terrain, LEVEL_H};
pub use build::{
    blend_surfaces, generate_nightmare, generate_with, portal_surface, Atmosphere, Block,
    BlockKind, Dream, Goals, Pressure, TexSpec, SIGILS,
};
pub use director::{DreamDirector, RunLength, DEEPER_SHARDS, NIGHTMARE_EVERY, SHARD_CHANCE};
pub use meshes::{build as build_mesh, Shape, ALL_SHAPES};
pub use names::{dream_name, whisper};
pub use texture::{Pattern, TEX_SIZE};
pub use theme::{
    DreamTheme, EnemyKind, Feature, Mood, PropKind, ALL_ENEMY_KINDS, ALL_PROPS as ALL_PROP_KINDS,
    ALL_THEMES,
};
pub use variant::{
    echo_chase_target, echo_ghost, echo_reaches, mirror_input, mirror_stick,
    roll_with as roll_twists, Twists, Variant, ALL_VARIANTS, HASTY_DUST,
};
