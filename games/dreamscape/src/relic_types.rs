//! Relic data types, shared by every relic stream (R1-R8).
//!
//! Data only: no logic, no GL, no clock. `relics.rs` (R2) owns the pull
//! maths, `relic_art.rs` (R3) the textures, `RelicEffect::to_quirk` (R4) the
//! buffs. This file exists so the streams can build in parallel against one
//! set of types; extend it, never fork it.

use serde::{Deserialize, Serialize};

use crate::dream::{DreamTheme, TexSpec};
use crate::worth::QuirkEffect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicRarity {
    Cracked,
    Worn,
    Pristine,
    Mythic,
}

/// Pull counters. Old saves load as all-zero.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RelicState {
    #[serde(default)]
    pub total_pulls: u32,
    #[serde(default)]
    pub since_mythic: u32,
    #[serde(default)]
    pub since_pristine: u32,
    #[serde(default)]
    pub ash_spent: u32,
}

/// The numbers a relic adds to a run: `QuirkEffect`'s fields, but
/// serialisable (`QuirkEffect` is not).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RelicEffect {
    #[serde(default)]
    pub dust_bonus_pct: u32,
    #[serde(default)]
    pub memory_bonus: f32,
    #[serde(default)]
    pub speed_pct: u32,
    #[serde(default)]
    pub grace_bonus: u32,
    #[serde(default)]
    pub calm_pct: u32,
    #[serde(default)]
    pub free_rerolls: u32,
    #[serde(default)]
    pub start_shards: u32,
}

impl RelicEffect {
    pub fn to_quirk(self) -> QuirkEffect {
        QuirkEffect {
            dust_bonus_pct: self.dust_bonus_pct,
            memory_bonus: self.memory_bonus,
            speed_pct: self.speed_pct,
            grace_bonus: self.grace_bonus,
            calm_pct: self.calm_pct,
            free_rerolls: self.free_rerolls,
            start_shards: self.start_shards,
        }
    }
}

/// A relic's weapon: an index into the weapon shapes (R5 maps it to
/// `dream::meshes::Shape`) and a scale in percent of the base size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weapon {
    pub shape: u8,
    pub scale_pct: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub id: u32,
    pub theme: DreamTheme,
    pub seed: u64,
    pub rarity: RelicRarity,
    pub foil: bool,
    pub name: String,
    pub flavour: String,
    #[serde(default)]
    pub effect: RelicEffect,
    pub art: TexSpec,
    #[serde(default)]
    pub weapon: Option<Weapon>,
}
