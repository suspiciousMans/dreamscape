//! Pick-a-pack: after waking, the run offers up to `MAX_OFFERS` sealed packs
//! (one dream theme each). The player opens one; the others are lost.
//! Pure data and rules; drawing lives in `pack_art.rs` and `reveal_ui.rs`.
//! See plans/PLAN-pick-a-pack.md.
//!
//! SKELETON: the types and signatures below are the contract the parallel
//! streams build against. Stream A fills in the `todo!()`s.
#![allow(dead_code)]

use crate::cards::{Card, DreamRecord, MemoryBoost, Recalled};
use crate::dream::DreamTheme;

pub const MAX_OFFERS: usize = 3;
pub const MAX_PACK_CARDS: usize = 5;

/// The centre glyph on a pack wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Emblem {
    Eye,
    Moth,
    Door,
    Shard,
    Crescent,
}

impl Emblem {
    pub const ALL: [Emblem; 5] = [
        Emblem::Eye,
        Emblem::Moth,
        Emblem::Door,
        Emblem::Shard,
        Emblem::Crescent,
    ];

    /// Deterministic: the same theme and seed always get the same emblem.
    pub fn for_theme(_theme: DreamTheme, _seed: u64) -> Emblem {
        todo!("stream A")
    }
}

/// One sealed pack on the pick screen.
#[derive(Clone, Debug, PartialEq)]
pub struct PackOffer {
    pub theme: DreamTheme,
    pub emblem: Emblem,
    /// Seeds the wrapper art (stable across frames and replays).
    pub seed: u64,
    /// The run's dreams of this theme (at most `MAX_PACK_CARDS`), already
    /// rolled remembered/faded. Never Fused/Resonant unless the source record
    /// had `blend`.
    pub cards: Vec<Recalled>,
    /// `memory_chance` for each card, same order as `cards`. Shown verbatim.
    pub memory_chances: Vec<f32>,
}

/// Up to `MAX_OFFERS` packs from a finished run, deepest theme first.
/// Fewer themes -> fewer packs, never zero for a non-empty log.
/// On an empty booklet a playable pack (Garden if present) is always offered.
pub fn offer_packs(
    _log: &[DreamRecord],
    _owned: &[Card],
    _boost: MemoryBoost,
) -> Vec<PackOffer> {
    todo!("stream A")
}
