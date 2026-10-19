//! Pick-a-pack: after waking, the run offers up to `MAX_OFFERS` sealed packs
//! (one dream theme each). The player opens one; the others are lost.
//! Pure data and rules; drawing lives in `pack_art.rs` and `reveal_ui.rs`.
//! See plans/PLAN-pick-a-pack.md.
//!
//! SKELETON: the types and signatures below are the contract the parallel
//! streams build against. Stream A fills in the `todo!()`s.
#![allow(dead_code)]

use crate::cards::{
    memory_chance, recall, Card, DreamRecord, MemoryBoost, Recalled,
};
use crate::dream::{DreamTheme, ALL_THEMES};

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
    pub fn for_theme(theme: DreamTheme, seed: u64) -> Emblem {
        let _ = (theme, seed);
        Emblem::Eye
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
///
/// Pure: nothing here touches `LotteryState`. Pity belongs to the opened
/// pack only (`lottery::apply_pack_pity_to_offer`).
pub fn offer_packs(log: &[DreamRecord], owned: &[Card], boost: MemoryBoost) -> Vec<PackOffer> {
    // Group by theme, remembering first appearance for a stable tie-break.
    let mut groups: Vec<(DreamTheme, Vec<&DreamRecord>)> = Vec::new();
    for r in log {
        match groups.iter_mut().find(|(t, _)| *t == r.theme) {
            Some((_, v)) => v.push(r),
            None => groups.push((r.theme, vec![r])),
        }
    }
    // Deepest first; equal depths keep first-appearance order (stable sort).
    groups.sort_by_key(|(_, v)| std::cmp::Reverse(v.iter().map(|r| r.depth).max().unwrap_or(0)));
    let mut picked: Vec<usize> = (0..groups.len().min(MAX_OFFERS)).collect();
    ensure_playable_pack(&groups, &mut picked, owned);

    let mut offers: Vec<PackOffer> = Vec::new();
    for &gi in &picked {
        let (theme, recs) = &groups[gi];
        let records = pack_records(recs);
        let cards = recall(&records, boost);
        let memory_chances = cards
            .iter()
            .map(|r| memory_chance(r.card.rarity, r.card.depth, boost))
            .collect();
        let base = pack_seed(*theme, &records);
        let (seed, emblem) = pick_emblem(*theme, base, &offers);
        offers.push(PackOffer {
            theme: *theme,
            emblem,
            seed,
            cards,
            memory_chances,
        });
    }
    offers
}

/// The deepest `MAX_PACK_CARDS` records (later ones win depth ties), in run order.
fn pack_records(recs: &[&DreamRecord]) -> Vec<DreamRecord> {
    let mut idx: Vec<usize> = (0..recs.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse((recs[i].depth, i)));
    idx.truncate(MAX_PACK_CARDS);
    idx.sort_unstable();
    idx.into_iter().map(|i| recs[i].clone()).collect()
}

fn ensure_playable_pack(
    _groups: &[(DreamTheme, Vec<&DreamRecord>)],
    _picked: &mut Vec<usize>,
    _owned: &[Card],
) {
}

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Deterministic from the theme and the records that fill the pack.
fn pack_seed(theme: DreamTheme, records: &[DreamRecord]) -> u64 {
    let t = ALL_THEMES.iter().position(|&a| a == theme).unwrap_or(0) as u64;
    records
        .iter()
        .fold(mix(t ^ 0x9AC4_5EED), |h, r| mix(h ^ r.seed))
}

fn pick_emblem(theme: DreamTheme, seed: u64, _taken: &[PackOffer]) -> (u64, Emblem) {
    (seed, Emblem::for_theme(theme, seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::tests::record;
    use crate::cards::{card_from, memory_chance};
    use crate::dream::DreamTheme::*;

    fn boost() -> MemoryBoost {
        MemoryBoost::default()
    }

    #[test]
    fn empty_log_offers_nothing() {
        assert!(offer_packs(&[], &[], boost()).is_empty());
    }

    #[test]
    fn offers_have_distinct_themes_deepest_first_and_cap_at_three() {
        let log = vec![
            record(Lobby, 1, 1, 0.1, false),
            record(Garden, 2, 5, 0.1, false),
            record(TheTunnel, 3, 3, 0.1, false),
            record(Lobby, 4, 2, 0.1, false),
            record(MirrorHall, 5, 4, 0.1, false),
            record(Awakening, 6, 6, 0.1, false),
        ];
        let owned = vec![card_from(&record(Elfworks, 99, 1, 0.1, false))];
        let offers = offer_packs(&log, &owned, boost());
        assert_eq!(offers.len(), MAX_OFFERS);
        let themes: Vec<_> = offers.iter().map(|o| o.theme).collect();
        assert_eq!(themes, vec![Awakening, Garden, MirrorHall]);
    }

    #[test]
    fn fewer_than_three_themes_gives_fewer_packs_never_zero() {
        let one = vec![
            record(Garden, 1, 1, 0.1, false),
            record(Garden, 2, 2, 0.1, false),
        ];
        assert_eq!(offer_packs(&one, &[], boost()).len(), 1);
        let two = vec![
            record(Garden, 1, 1, 0.1, false),
            record(Lobby, 2, 2, 0.1, false),
        ];
        assert_eq!(offer_packs(&two, &[], boost()).len(), 2);
    }

    #[test]
    fn depth_ties_keep_run_order() {
        let log = vec![
            record(Lobby, 1, 2, 0.1, false),
            record(Garden, 2, 2, 0.1, false),
            record(TheTunnel, 3, 2, 0.1, false),
            record(MirrorHall, 4, 2, 0.1, false),
        ];
        let themes: Vec<_> = offer_packs(&log, &[card_from(&log[0])], boost())
            .iter()
            .map(|o| o.theme)
            .collect();
        assert_eq!(themes, vec![Lobby, Garden, TheTunnel]);
    }

    #[test]
    fn a_pack_is_the_runs_dreams_of_that_theme_capped_at_five() {
        let mut log = Vec::new();
        for i in 0..7u64 {
            log.push(record(Garden, 100 + i, i as u32, 0.2, i % 2 == 0));
            log.push(record(Lobby, 200 + i, 0, 0.2, false));
        }
        let owned = vec![card_from(&log[1])];
        let offers = offer_packs(&log, &owned, boost());
        let garden = offers.iter().find(|o| o.theme == Garden).unwrap();
        // Deepest five (depth 2..=6), in run order.
        let want: Vec<_> = log
            .iter()
            .filter(|r| r.theme == Garden && r.depth >= 2)
            .cloned()
            .collect();
        assert_eq!(want.len(), MAX_PACK_CARDS);
        let got: Vec<_> = garden.cards.iter().map(|r| r.card.seed).collect();
        assert_eq!(got, want.iter().map(|r| r.seed).collect::<Vec<_>>());
        // Same card + roll as today's recall over that slice.
        assert_eq!(garden.cards, crate::cards::recall(&want, boost()));
        // Fewer than five: all of them, nothing from other themes.
        let small = vec![record(Lobby, 1, 1, 0.1, false), record(Lobby, 2, 2, 0.1, false)];
        let o = offer_packs(&small, &owned, boost());
        assert_eq!(o[0].cards.len(), 2);
        assert!(o[0].cards.iter().all(|r| r.card.theme == Lobby));
    }

    #[test]
    fn memory_chances_are_exactly_memory_chance() {
        let b = MemoryBoost {
            lucid_wake: true,
            deep_memory: true,
            extra: 0.07,
        };
        let log: Vec<_> = (0..12u64)
            .map(|i| record(crate::dream::ALL_THEMES[(i % 6) as usize], i, i as u32, 0.1 * (i % 9) as f32, i % 3 == 0))
            .collect();
        for o in offer_packs(&log, &[card_from(&log[0])], b) {
            assert_eq!(o.memory_chances.len(), o.cards.len());
            for (c, p) in o.cards.iter().zip(&o.memory_chances) {
                assert_eq!(*p, memory_chance(c.card.rarity, c.card.depth, b));
            }
        }
    }

    #[test]
    fn offers_are_deterministic_and_seeds_differ_between_packs() {
        let log = vec![
            record(Lobby, 1, 1, 0.1, false),
            record(Garden, 2, 2, 0.1, false),
            record(TheTunnel, 3, 3, 0.1, false),
        ];
        let owned = vec![card_from(&log[0])];
        let a = offer_packs(&log, &owned, boost());
        assert_eq!(a, offer_packs(&log, &owned, boost()));
        assert_ne!(a[0].seed, a[1].seed);
        assert_ne!(a[1].seed, a[2].seed);
    }
}
