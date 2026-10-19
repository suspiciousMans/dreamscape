//! Pick-a-pack: after waking, the run offers up to `MAX_OFFERS` sealed packs
//! (one dream theme each). The player opens one; the others are lost.
//! Pure data and rules; drawing lives in `pack_art.rs` and `reveal_ui.rs`.
//! See plans/PLAN-pick-a-pack.md.
//!
//! SKELETON: the types and signatures below are the contract the parallel
//! streams build against. Stream A fills in the `todo!()`s.
#![allow(dead_code)]

use crate::cards::{
    memory_chance, recall, shape_first_pack, Card, DreamRecord, MemoryBoost, Recalled,
};
use crate::dream::{DreamTheme, ALL_THEMES};

/// Packs on the table for a short run (and the least ever offered).
pub const MAX_OFFERS: usize = 3;
/// One more pack to open per this many dreams played.
pub const DREAMS_PER_PICK: usize = 3;
/// The most packs a run can open.
pub const MAX_PICKS: usize = 4;
/// Packs on the table beyond the ones you may open, so there's still a choice.
pub const EXTRA_OFFERS: usize = 2;

/// How many packs a run of `dreams` dreams may open: 1 for 1-3 dreams, 2 for
/// 4-6, 3 for 7-9, 4 for 10 or more.
pub fn picks_for(dreams: usize) -> usize {
    (1 + dreams.saturating_sub(1) / DREAMS_PER_PICK).clamp(1, MAX_PICKS)
}

/// How many packs are offered for a run of `dreams` dreams (fewer if the run
/// visited fewer themes).
pub fn offer_count(dreams: usize) -> usize {
    (picks_for(dreams) + EXTRA_OFFERS).max(MAX_OFFERS)
}
pub const MAX_PACK_CARDS: usize = 5;

/// Odds (percent weights) that an opened pack gives 1, 2 or 3 cards. One card
/// is the most likely; three is rare.
pub const TAKE_WEIGHTS: [u32; 3] = [60, 30, 10];

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
    ///
    /// The Awakening and the Lobby always wear the Eye (the thing that
    /// watches you wake / the first room). Every other theme draws from all
    /// five by hash, so each emblem is reachable and wrappers vary by seed.
    pub fn for_theme(theme: DreamTheme, seed: u64) -> Emblem {
        if matches!(theme, DreamTheme::Awakening | DreamTheme::Lobby) {
            return Emblem::Eye;
        }
        let t = ALL_THEMES.iter().position(|&a| a == theme).unwrap_or(0) as u64;
        Emblem::ALL[(mix(seed ^ mix(t ^ 0xE3B1_E3B1)) % Emblem::ALL.len() as u64) as usize]
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
    let mut picked: Vec<usize> = (0..groups.len().min(offer_count(log.len()))).collect();
    ensure_playable_pack(&groups, &mut picked, owned);

    let mut offers: Vec<PackOffer> = Vec::new();
    for &gi in &picked {
        let (theme, recs) = &groups[gi];
        let records = pack_records(recs);
        let mut cards = recall(&records, boost);
        // A new player's first cards: today's shaping, applied per pack so
        // the guarantee holds whichever pack they open.
        shape_first_pack(&mut cards, owned);
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

/// First-run safety across the offer. With nothing owned the offer must hold
/// a pack the player can build a loadout from: the Garden's if the run had
/// one (even when it was too shallow to rank), else the first playable theme.
/// It replaces the shallowest pick; `picked` stays in rank order.
fn ensure_playable_pack(
    groups: &[(DreamTheme, Vec<&DreamRecord>)],
    picked: &mut Vec<usize>,
    owned: &[Card],
) {
    let playable = |gi: usize| groups[gi].0 != DreamTheme::Awakening;
    if !owned.is_empty() || picked.is_empty() {
        return;
    }
    let want = groups
        .iter()
        .position(|(t, _)| *t == DreamTheme::Garden)
        .or_else(|| (0..groups.len()).find(|&gi| playable(gi)));
    let Some(want) = want else { return };
    let has_garden = groups.iter().any(|(t, _)| *t == DreamTheme::Garden);
    let satisfied = if has_garden {
        picked.contains(&want)
    } else {
        picked.iter().any(|&gi| playable(gi))
    };
    if !satisfied {
        *picked.last_mut().expect("non-empty") = want;
        picked.sort_unstable();
    }
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

/// The wrapper seed and emblem for a pack. Packs on one table should look
/// different, so if an earlier pack already wears this emblem the seed is
/// re-mixed (deterministically) until a free one comes up. Themes with a
/// fixed emblem keep it.
fn pick_emblem(theme: DreamTheme, seed: u64, taken: &[PackOffer]) -> (u64, Emblem) {
    let mut s = seed;
    for k in 1..=32u64 {
        let e = Emblem::for_theme(theme, s);
        if !taken.iter().any(|o| o.emblem == e) {
            return (s, e);
        }
        s = mix(seed ^ k);
    }
    (seed, Emblem::for_theme(theme, seed))
}

/// Chance of getting 1, 2, 3 cards out of a pack with `candidates` dreams in
/// it (index 0 = one card). A pack can't give more cards than it holds, so the
/// weights are cut to what fits and renormalised. Sums to 1.
pub fn take_odds(candidates: usize) -> Vec<f32> {
    let n = candidates.clamp(1, TAKE_WEIGHTS.len());
    let total: u32 = TAKE_WEIGHTS[..n].iter().sum();
    TAKE_WEIGHTS[..n]
        .iter()
        .map(|&w| w as f32 / total as f32)
        .collect()
}

/// Open a pack: roll how many cards it gives (`take_odds`), then which of the
/// run's dreams of this theme they are. Deterministic per offer, so the reveal
/// replays exactly. Cards keep run order. Nothing here can add a rarity: the
/// cards are the offer's own, so Fused/Resonant only appear if the dream was
/// already blended.
pub fn open(offer: &PackOffer) -> Vec<Recalled> {
    use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
    if offer.cards.is_empty() {
        return Vec::new();
    }
    let mut rng = StdRng::seed_from_u64(offer.seed ^ 0x7A4B_E5C1_0D1E_55ED);
    let odds = take_odds(offer.cards.len());
    let roll: f32 = rng.gen();
    let mut acc = 0.0;
    let mut take = odds.len();
    for (i, p) in odds.iter().enumerate() {
        acc += p;
        if roll < acc {
            take = i + 1;
            break;
        }
    }
    let mut picks: Vec<usize> = (0..offer.cards.len()).collect();
    picks.shuffle(&mut rng);
    picks.truncate(take);
    picks.sort_unstable();
    picks.into_iter().map(|i| offer.cards[i].clone()).collect()
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
    fn offers_have_distinct_themes_deepest_first_and_are_capped() {
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
        assert_eq!(offers.len(), offer_count(log.len()));
        let themes: Vec<_> = offers.iter().map(|o| o.theme).collect();
        assert_eq!(themes, vec![Awakening, Garden, MirrorHall, TheTunnel]);
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
        assert_eq!(themes, vec![Lobby, Garden, TheTunnel, MirrorHall]);
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
        // Same card + roll as today's recall over that slice (then the same
        // first-pack shaping `open_pack` applies today).
        let mut today = crate::cards::recall(&want, boost());
        crate::cards::shape_first_pack(&mut today, &owned);
        assert_eq!(garden.cards, today);
        // Rarity and the raw roll are untouched by shaping.
        let raw = crate::cards::recall(&want, boost());
        for (a, b) in garden.cards.iter().zip(&raw) {
            assert_eq!(a.card, b.card);
        }
        // Fewer than five: all of them, nothing from other themes.
        let small = vec![
            record(Lobby, 1, 1, 0.1, false),
            record(Lobby, 2, 2, 0.1, false),
        ];
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
            .map(|i| {
                record(
                    crate::dream::ALL_THEMES[(i % 6) as usize],
                    i,
                    i as u32,
                    0.1 * (i % 9) as f32,
                    i % 3 == 0,
                )
            })
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

    /// A run whose Garden is too shallow to rank in the top three.
    fn run_with_shallow_garden() -> Vec<DreamRecord> {
        vec![
            record(Garden, 1, 0, 0.1, false),
            record(Lobby, 2, 3, 0.1, false),
            record(TheTunnel, 3, 4, 0.1, false),
            record(MirrorHall, 4, 5, 0.1, false),
            record(Awakening, 5, 6, 0.1, false),
        ]
    }

    fn playable_remembered(o: &PackOffer) -> bool {
        o.cards
            .iter()
            .any(|r| r.remembered && r.card.theme != Awakening)
    }

    #[test]
    fn empty_booklet_always_offers_the_garden_pack_when_the_run_has_one() {
        let run = run_with_shallow_garden();
        let offers = offer_packs(&run, &[], boost());
        assert_eq!(offers.len(), offer_count(run.len()));
        assert!(offers.iter().any(|o| o.theme == Garden));
        // Still deepest first after the swap.
        let depths: Vec<_> = offers
            .iter()
            .map(|o| o.cards.iter().map(|r| r.card.depth).max().unwrap())
            .collect();
        assert!(depths.windows(2).all(|w| w[0] >= w[1]), "{depths:?}");
        // A non-empty booklet gets no such favour.
        let owned = vec![card_from(&record(Elfworks, 99, 1, 0.1, false))];
        let offers = offer_packs(&run_with_shallow_garden(), &owned, boost());
        assert!(!offers.iter().any(|o| o.theme == Garden));
    }

    #[test]
    fn empty_booklet_offers_a_playable_remembered_card_without_a_garden() {
        let log = vec![
            record(Awakening, 1, 5, 0.1, false),
            record(Lobby, 2, 1, 0.1, false),
        ];
        let offers = offer_packs(&log, &[], boost());
        assert!(offers.iter().any(playable_remembered));
        // The Awakening pack stays remembered as today.
        let awake = offers.iter().find(|o| o.theme == Awakening).unwrap();
        assert!(awake.cards.iter().all(|r| r.remembered));
    }

    #[test]
    fn first_run_guarantee_holds_over_random_runs() {
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(31);
        for n in 0..2_000u64 {
            let len = rng.gen_range(1..=14usize);
            let log: Vec<_> = (0..len as u64)
                .map(|i| {
                    let t =
                        crate::dream::ALL_THEMES[rng.gen_range(0..crate::dream::ALL_THEMES.len())];
                    record(t, n * 100 + i, rng.gen_range(0..9), rng.gen(), rng.gen())
                })
                .collect();
            let offers = offer_packs(&log, &[], boost());
            assert!(!offers.is_empty());
            let has_playable = log.iter().any(|r| r.theme != Awakening);
            if has_playable {
                assert!(offers.iter().any(playable_remembered), "run {n}: {log:?}");
                // Opening ANY playable offered pack leaves something to build with.
                for o in offers.iter().filter(|o| o.theme != Awakening) {
                    assert!(playable_remembered(o), "run {n}: {:?}", o.theme);
                }
            }
            if log.iter().any(|r| r.theme == Garden) {
                assert!(offers.iter().any(|o| o.theme == Garden), "run {n}");
            }
            let mut themes: Vec<_> = offers.iter().map(|o| o.theme).collect();
            themes.dedup();
            assert_eq!(themes.len(), offers.len());
        }
    }

    #[test]
    fn emblems_are_deterministic_and_all_reachable() {
        use std::collections::HashSet;
        for t in crate::dream::ALL_THEMES {
            for seed in 0..40u64 {
                assert_eq!(Emblem::for_theme(t, seed), Emblem::for_theme(t, seed));
            }
        }
        for seed in 0..200u64 {
            assert_eq!(Emblem::for_theme(Awakening, seed), Emblem::Eye);
            assert_eq!(Emblem::for_theme(Lobby, seed), Emblem::Eye);
        }
        let seen: HashSet<Emblem> = crate::dream::ALL_THEMES
            .iter()
            .flat_map(|&t| (0..40u64).map(move |s| Emblem::for_theme(t, s)))
            .collect();
        for e in Emblem::ALL {
            assert!(seen.contains(&e), "{e:?} is never chosen");
        }
    }

    #[test]
    fn an_offers_emblem_is_for_theme_of_its_own_seed_and_packs_differ() {
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(5);
        for n in 0..500u64 {
            let log: Vec<_> = (0..rng.gen_range(1..10u64))
                .map(|i| {
                    let t =
                        crate::dream::ALL_THEMES[rng.gen_range(0..crate::dream::ALL_THEMES.len())];
                    record(t, n * 100 + i, rng.gen_range(0..9), 0.3, false)
                })
                .collect();
            let offers = offer_packs(&log, &[card_from(&log[0])], boost());
            assert_eq!(offers, offer_packs(&log, &[card_from(&log[0])], boost()));
            for o in &offers {
                assert_eq!(o.emblem, Emblem::for_theme(o.theme, o.seed));
            }
            // Different wrappers on the table whenever the themes allow it.
            let free: Vec<_> = offers
                .iter()
                .filter(|o| !matches!(o.theme, Awakening | Lobby))
                .map(|o| o.emblem)
                .collect();
            let mut uniq = free.clone();
            uniq.sort_by_key(|e| *e as u8);
            uniq.dedup();
            assert_eq!(uniq.len(), free.len(), "run {n}: {free:?}");
        }
    }

    #[test]
    fn random_runs_never_yield_fused_or_resonant_unless_blended() {
        use crate::cards::Rarity;
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(77);
        let (mut cards, mut blended) = (0usize, 0usize);
        for n in 0..10_000u64 {
            let log: Vec<_> = (0..rng.gen_range(1..=12u64))
                .map(|i| {
                    let t =
                        crate::dream::ALL_THEMES[rng.gen_range(0..crate::dream::ALL_THEMES.len())];
                    let mut r = record(t, n * 100 + i, rng.gen_range(0..12), rng.gen(), rng.gen());
                    r.enemies = rng.gen_range(0..6);
                    if rng.gen_ratio(1, 12) {
                        r.blend = Some(
                            crate::dream::ALL_THEMES
                                [rng.gen_range(0..crate::dream::ALL_THEMES.len())],
                        );
                    }
                    r
                })
                .collect();
            let owned: Vec<Card> = if n % 2 == 0 {
                vec![]
            } else {
                vec![card_from(&log[0])]
            };
            for o in offer_packs(&log, &owned, boost()) {
                for r in &o.cards {
                    let src = log.iter().find(|x| x.seed == r.card.seed).unwrap();
                    cards += 1;
                    if src.blend.is_some() {
                        blended += 1;
                        assert_eq!(r.card.rarity, Rarity::Fused);
                    } else {
                        assert!(
                            !matches!(r.card.rarity, Rarity::Fused | Rarity::Resonant),
                            "run {n}: {:?}",
                            r.card
                        );
                        assert_eq!(r.card.fused, None);
                    }
                }
            }
        }
        assert!(
            cards > 10_000 && blended > 0,
            "{cards} cards, {blended} blended"
        );
    }

    #[test]
    fn take_odds_sum_to_one_and_one_card_is_most_likely() {
        for n in 1..=6 {
            let o = take_odds(n);
            assert_eq!(o.len(), n.min(3));
            assert!((o.iter().sum::<f32>() - 1.0).abs() < 1e-5);
            assert!(
                o.windows(2).all(|w| w[0] > w[1]),
                "fewer cards is likelier: {o:?}"
            );
        }
        assert_eq!(take_odds(1), vec![1.0]);
        assert_eq!(take_odds(3), vec![0.6, 0.3, 0.1]);
    }

    #[test]
    fn opening_gives_a_few_cards_not_all_and_is_deterministic() {
        let mut counts = [0usize; 4];
        for seed in 0..4000u64 {
            let cards: Vec<Recalled> = (0..5)
                .map(|i| Recalled {
                    card: crate::cards::card_from(&crate::cards::tests::record(
                        DreamTheme::Garden,
                        seed * 10 + i,
                        1 + i as u32,
                        0.2,
                        false,
                    )),
                    remembered: true,
                })
                .collect();
            let offer = PackOffer {
                theme: DreamTheme::Garden,
                emblem: Emblem::Eye,
                seed,
                memory_chances: vec![0.5; 5],
                cards,
            };
            let got = open(&offer);
            assert_eq!(got, open(&offer), "same offer, same cards");
            assert!((1..=3).contains(&got.len()), "never all five");
            assert!(got.iter().all(|r| offer.cards.contains(r)));
            // Run order is kept.
            let pos: Vec<usize> = got
                .iter()
                .map(|r| offer.cards.iter().position(|c| c == r).unwrap())
                .collect();
            assert!(pos.windows(2).all(|w| w[0] < w[1]));
            counts[got.len()] += 1;
        }
        // About 60 / 30 / 10 percent of 4000.
        assert!((2200..2600).contains(&counts[1]), "{counts:?}");
        assert!((1000..1400).contains(&counts[2]), "{counts:?}");
        assert!((250..550).contains(&counts[3]), "{counts:?}");
    }

    #[test]
    fn a_small_pack_never_gives_more_than_it_holds() {
        let rec = |s| Recalled {
            card: crate::cards::card_from(&crate::cards::tests::record(
                DreamTheme::Garden,
                s,
                1,
                0.2,
                false,
            )),
            remembered: true,
        };
        for seed in 0..200 {
            let one = PackOffer {
                theme: DreamTheme::Garden,
                emblem: Emblem::Eye,
                seed,
                memory_chances: vec![0.5],
                cards: vec![rec(seed)],
            };
            assert_eq!(open(&one).len(), 1);
            let two = PackOffer {
                cards: vec![rec(seed), rec(seed + 1)],
                memory_chances: vec![0.5; 2],
                ..one.clone()
            };
            assert!((1..=2).contains(&open(&two).len()));
        }
        let none = PackOffer {
            theme: DreamTheme::Garden,
            emblem: Emblem::Eye,
            seed: 1,
            memory_chances: vec![],
            cards: vec![],
        };
        assert!(open(&none).is_empty());
    }

    #[test]
    fn longer_runs_open_and_offer_more_packs() {
        let picks: Vec<usize> = [0, 1, 3, 4, 6, 7, 9, 10, 99]
            .iter()
            .map(|&d| picks_for(d))
            .collect();
        assert_eq!(picks, vec![1, 1, 1, 2, 2, 3, 3, 4, 4]);
        let offered: Vec<usize> = [1, 3, 4, 7, 10, 99]
            .iter()
            .map(|&d| offer_count(d))
            .collect();
        assert_eq!(offered, vec![3, 3, 4, 5, 6, 6]);
        // Always more packs on the table than you can open, so there's a choice.
        for d in 1..40 {
            assert!(offer_count(d) > picks_for(d));
        }
    }

    #[test]
    fn a_long_run_across_many_themes_offers_more_packs() {
        let themes = [
            Garden,
            CursedForest,
            MirrorHall,
            TheTunnel,
            MeltingClockworks,
            WatchingWallpaper,
            Elfworks,
            VoidPlatforms,
        ];
        let log: Vec<DreamRecord> = (0..10)
            .map(|i| {
                record(
                    themes[i % themes.len()],
                    50 + i as u64,
                    1 + i as u32,
                    0.2,
                    false,
                )
            })
            .collect();
        let offers = offer_packs(&log, &[], boost());
        assert_eq!(offers.len(), offer_count(10));
        assert_eq!(offers.len(), 6);
        let short = offer_packs(&log[..4], &[], boost());
        assert_eq!(short.len(), offer_count(4));
        // Fewer themes than the table allows: fewer packs, never padded.
        let one_theme: Vec<DreamRecord> = (0..10)
            .map(|i| record(Garden, 80 + i, 1 + i as u32, 0.2, false))
            .collect();
        assert_eq!(offer_packs(&one_theme, &[], boost()).len(), 1);
    }
}
