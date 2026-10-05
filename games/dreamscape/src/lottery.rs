//! The Dream Lottery: a pure, seeded "pull" engine. Players spend in-game
//! Dream Dust (never real money) to pull a dream card.
//!
//! Principles (see PLAN-dopamine.md): odds are shown exactly as they are,
//! and pity bounds every dry spell:
//! - **Soft pity**: after `SOFT_PITY_LUCID` pulls without a Lucid-or-better,
//!   the Lucid+ chance rises by 6 percentage points per pull.
//! - **Hard pity**: a Lucid+ by pull `HARD_PITY_LUCID`, a Prophetic by pull
//!   `HARD_PITY_PROPHETIC`, always.
//! - **Sparks**: every pull earns a spark; `SPARKS_FOR_PICK` sparks buy a
//!   chosen-theme card (Vivid).
//! - **Pack pity**: `PACK_PITY` run packs in a row without a remembered
//!   Lucid+ card, and the next pack upgrades one card to Lucid.
//!
//! Nothing here touches the clock or global RNG: every result is a function
//! of `(state, seed)`.
//!
//! Rarity pool for pulls is Faint..=Prophetic only; Fused and Resonant are
//! made at the moth and never pulled.
#![allow(dead_code)]

use crate::cards::{card_from, Card, DreamRecord, Rarity, Recalled};
use crate::dream::{dream_name, portal_surface, whisper, DreamTheme, ALL_THEMES};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Dream Dust per pull.
pub const PULL_COST: u32 = 60;
/// Sparks earned by every pull.
pub const SPARKS_PER_PULL: u32 = 1;
/// Sparks needed to choose a theme card.
pub const SPARKS_FOR_PICK: u32 = 30;
/// Pulls without a Lucid+ before the chance starts ramping.
pub const SOFT_PITY_LUCID: u32 = 18;
/// Lucid+ chance gained per pull past `SOFT_PITY_LUCID` (percentage points / 100).
pub const SOFT_PITY_STEP: f32 = 0.06;
/// The pull number (counting dry pulls) at which a Lucid+ is guaranteed.
pub const HARD_PITY_LUCID: u32 = 30;
/// The pull number at which a Prophetic is guaranteed.
pub const HARD_PITY_PROPHETIC: u32 = 60;
/// Chance of a foil, independent of rarity.
pub const FOIL_ODDS: f32 = 1.0 / 48.0;
/// Run packs in a row without a remembered Lucid+ before the next is upgraded.
pub const PACK_PITY: u32 = 4;
/// Weight of the featured theme against 1 for every other theme.
pub const FEATURED_WEIGHT: u32 = 3;
/// A roll this close (in probability) under the next rarity is a near miss.
pub const NEAR_MISS_WINDOW: f32 = 0.03;

/// Base odds for Faint, Hazy, Vivid, Lucid, Prophetic. Sum to 1.
pub const BASE_ODDS: [f32; 5] = [0.40, 0.30, 0.20, 0.08, 0.02];

/// The rarities a pull can produce, lowest to highest (index = position in
/// `Odds::rarity`).
pub const PULL_RARITIES: [Rarity; 5] = [
    Rarity::Faint,
    Rarity::Hazy,
    Rarity::Vivid,
    Rarity::Lucid,
    Rarity::Prophetic,
];

const LUCID_INDEX: usize = 3;
const PROPHETIC_INDEX: usize = 4;

/// Persisted lottery counters. Every field defaults so old saves still parse.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LotteryState {
    #[serde(default)]
    pub total_pulls: u32,
    /// Pulls since a Lucid-or-better.
    #[serde(default)]
    pub since_lucid: u32,
    /// Pulls since a Prophetic.
    #[serde(default)]
    pub since_prophetic: u32,
    #[serde(default)]
    pub sparks: u32,
    /// Packs since a remembered Lucid+ card.
    #[serde(default)]
    pub pack_dry: u32,
}

/// The exact probabilities of the next pull.
#[derive(Clone, Debug, PartialEq)]
pub struct Odds {
    /// Probability of each of `PULL_RARITIES` (same order). Sums to 1.
    pub rarity: [f32; 5],
    /// Chance that a pull's theme is the featured one (0 if none).
    pub featured_share: f32,
    /// Chance of each specific non-featured theme.
    pub other_share: f32,
    /// Chance of a foil.
    pub foil: f32,
    /// Pulls left until a Lucid+ is guaranteed (1 = the next pull).
    pub pulls_to_lucid_pity: u32,
    /// Pulls left until a Prophetic is guaranteed (1 = the next pull).
    pub pulls_to_prophetic_pity: u32,
}

impl Odds {
    /// Probability of Lucid or better.
    pub fn lucid_or_better(&self) -> f32 {
        self.rarity[LUCID_INDEX] + self.rarity[PROPHETIC_INDEX]
    }
}

/// What one pull produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Pull {
    pub theme: DreamTheme,
    pub rarity: Rarity,
    pub foil: bool,
    /// Hard pity raised the rarity above what the roll would have given.
    pub pity: bool,
    /// The next-better rarity was within `NEAR_MISS_WINDOW` of the roll.
    /// Cosmetic only: computed from the same roll that decided the rarity.
    pub near_miss: Option<Rarity>,
    pub sparks_gained: u32,
}

/// Themes a pull can land on: every theme except the waking Awakening.
pub fn pullable_themes() -> Vec<DreamTheme> {
    ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| t != DreamTheme::Awakening)
        .collect()
}

/// Featured theme for a day: stable per day, cycling through the pullable themes.
pub fn featured_theme(day: u64) -> DreamTheme {
    let pool = pullable_themes();
    pool[(day % pool.len() as u64) as usize]
}

/// The featured theme only counts if it can actually be pulled.
fn effective_featured(featured: Option<DreamTheme>) -> Option<DreamTheme> {
    featured.filter(|&t| t != DreamTheme::Awakening)
}

/// Rarity tables for the next pull: `(natural, final)`.
/// `natural` has soft pity only; `final` also applies hard pity. A roll
/// whose rarity differs between the two was changed by hard pity.
fn rarity_tables(state: &LotteryState) -> ([f32; 5], [f32; 5]) {
    let lucid_pull = state.since_lucid.saturating_add(1);
    let prophetic_pull = state.since_prophetic.saturating_add(1);

    let ramp = lucid_pull.saturating_sub(SOFT_PITY_LUCID) as f32 * SOFT_PITY_STEP;
    let base_lucid_plus = BASE_ODDS[LUCID_INDEX] + BASE_ODDS[PROPHETIC_INDEX];
    let lucid_plus = (base_lucid_plus + ramp).min(1.0);

    // Lucid+ mass is split Prophetic (fixed share) / Lucid (the rest); the
    // remaining mass keeps the base proportions of Faint/Hazy/Vivid.
    let prophetic = BASE_ODDS[PROPHETIC_INDEX].min(lucid_plus);
    let low_base: f32 = BASE_ODDS[..LUCID_INDEX].iter().sum();
    let low_mass = 1.0 - lucid_plus;
    let mut natural = [0.0; 5];
    for i in 0..LUCID_INDEX {
        natural[i] = low_mass * BASE_ODDS[i] / low_base;
    }
    natural[LUCID_INDEX] = lucid_plus - prophetic;
    natural[PROPHETIC_INDEX] = prophetic;

    let mut fin = natural;
    if lucid_pull >= HARD_PITY_LUCID {
        for slot in fin.iter_mut().take(LUCID_INDEX) {
            *slot = 0.0;
        }
        fin[LUCID_INDEX] = 1.0 - fin[PROPHETIC_INDEX];
    }
    if prophetic_pull >= HARD_PITY_PROPHETIC {
        fin = [0.0, 0.0, 0.0, 0.0, 1.0];
    }
    (natural, fin)
}

/// Index of the rarity a roll `u` in `[0,1)` lands on.
fn pick_index(table: &[f32; 5], u: f32) -> usize {
    let mut cumulative = 0.0;
    for (i, p) in table.iter().enumerate() {
        cumulative += p;
        if u < cumulative {
            return i;
        }
    }
    // Float rounding left a sliver at the top: use the best possible rarity.
    table.iter().rposition(|&p| p > 0.0).unwrap_or(0)
}

/// The exact current probabilities, pity included, for honest display.
pub fn odds(state: &LotteryState, featured: Option<DreamTheme>) -> Odds {
    let (_, fin) = rarity_tables(state);
    let n = pullable_themes().len() as f32;
    let (featured_share, other_share) = match effective_featured(featured) {
        Some(_) => {
            let total = (n - 1.0) + FEATURED_WEIGHT as f32;
            (FEATURED_WEIGHT as f32 / total, 1.0 / total)
        }
        None => (0.0, 1.0 / n),
    };
    Odds {
        rarity: fin,
        featured_share,
        other_share,
        foil: FOIL_ODDS,
        pulls_to_lucid_pity: HARD_PITY_LUCID.saturating_sub(state.since_lucid).max(1),
        pulls_to_prophetic_pity: HARD_PITY_PROPHETIC
            .saturating_sub(state.since_prophetic)
            .max(1),
    }
}

/// Rarity/probability pairs for the UI, lowest rarity first.
pub fn display_odds(o: &Odds) -> Vec<(Rarity, f32)> {
    PULL_RARITIES.iter().copied().zip(o.rarity).collect()
}

/// Weighted theme choice: the featured theme has `FEATURED_WEIGHT`, others 1.
fn roll_theme(rng: &mut StdRng, featured: Option<DreamTheme>) -> DreamTheme {
    let pool = pullable_themes();
    let featured = effective_featured(featured);
    let weight = |t: DreamTheme| if Some(t) == featured { FEATURED_WEIGHT } else { 1 };
    let total: u32 = pool.iter().map(|&t| weight(t)).sum();
    let mut roll = rng.gen_range(0..total);
    for &t in &pool {
        let w = weight(t);
        if roll < w {
            return t;
        }
        roll -= w;
    }
    pool[pool.len() - 1]
}

/// One pull. Deterministic for `(state, seed, featured)`. The draw order
/// from the seeded RNG is fixed: rarity roll, theme, foil.
pub fn pull(state: &mut LotteryState, seed: u64, featured: Option<DreamTheme>) -> Pull {
    let mut rng = StdRng::seed_from_u64(seed);
    let (natural, fin) = rarity_tables(state);

    let u: f32 = rng.gen();
    let index = pick_index(&fin, u);
    let natural_index = pick_index(&natural, u);
    let pity = index > natural_index;

    // Near miss: the roll sat just under the start of the next rarity band.
    let band_end: f32 = fin[..=index].iter().sum();
    let near_miss = if index + 1 < PULL_RARITIES.len()
        && fin[index + 1] > 0.0
        && band_end - u <= NEAR_MISS_WINDOW
    {
        Some(PULL_RARITIES[index + 1])
    } else {
        None
    };

    let theme = roll_theme(&mut rng, featured);
    let foil = rng.gen::<f32>() < FOIL_ODDS;
    let rarity = PULL_RARITIES[index];

    state.total_pulls = state.total_pulls.saturating_add(1);
    state.sparks = state.sparks.saturating_add(SPARKS_PER_PULL);
    state.since_lucid = if index >= LUCID_INDEX {
        0
    } else {
        state.since_lucid.saturating_add(1)
    };
    state.since_prophetic = if index == PROPHETIC_INDEX {
        0
    } else {
        state.since_prophetic.saturating_add(1)
    };

    Pull {
        theme,
        rarity,
        foil,
        pity,
        near_miss,
        sparks_gained: SPARKS_PER_PULL,
    }
}

/// Enough sparks to choose a card?
pub fn spark_pick_ready(state: &LotteryState) -> bool {
    state.sparks >= SPARKS_FOR_PICK
}

/// Spend `SPARKS_FOR_PICK` sparks to choose a theme: a Vivid card of it.
/// Returns `None` (and spends nothing) if there are too few sparks or the
/// theme can't be pulled (Awakening). This is not a random pull: it does
/// not move `total_pulls` or the pity counters.
pub fn redeem_spark_pick(state: &mut LotteryState, theme: DreamTheme) -> Option<Pull> {
    if !spark_pick_ready(state) || theme == DreamTheme::Awakening {
        return None;
    }
    state.sparks -= SPARKS_FOR_PICK;
    Some(Pull {
        theme,
        rarity: Rarity::Vivid,
        foil: false,
        pity: false,
        near_miss: None,
        sparks_gained: 0,
    })
}

/// Pack pity, applied to a run's pack after `cards::recall` (and after
/// `shape_first_pack`). A remembered Lucid+ card resets the dry count. Once
/// `PACK_PITY` packs in a row had none, the next pack that has none upgrades
/// its best non-Awakening card (remembered preferred on ties, so cards that
/// shaping faded stay faded) to at least Lucid and marks it remembered.
/// The Awakening card is never touched, so its "always remembered" holds.
pub fn apply_pack_pity(state: &mut LotteryState, pack: &mut [Recalled]) {
    if pack.is_empty() {
        return;
    }
    if pack
        .iter()
        .any(|r| r.remembered && r.card.rarity >= Rarity::Lucid)
    {
        state.pack_dry = 0;
        return;
    }
    if state.pack_dry >= PACK_PITY {
        let mut best: Option<usize> = None;
        for (i, r) in pack.iter().enumerate() {
            if r.card.theme == DreamTheme::Awakening {
                continue;
            }
            let better = match best {
                None => true,
                Some(b) => {
                    (r.card.rarity, r.remembered) > (pack[b].card.rarity, pack[b].remembered)
                }
            };
            if better {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            pack[i].card.rarity = pack[i].card.rarity.max(Rarity::Lucid);
            pack[i].remembered = true;
            state.pack_dry = 0;
            return;
        }
    }
    state.pack_dry = state.pack_dry.saturating_add(1);
}

/// Strangeness used for a pulled card's stats: rarer cards are stranger.
fn pull_strangeness(r: Rarity) -> f32 {
    match r {
        Rarity::Faint => 0.1,
        Rarity::Hazy => 0.3,
        Rarity::Vivid => 0.5,
        Rarity::Lucid => 0.7,
        _ => 0.9,
    }
}

/// Build the card for a pull by making a `DreamRecord` (name, whisper and
/// art from the `dream` generators, keyed by a seed derived from `seed`) and
/// running it through `cards::card_from`, then forcing the rarity to the
/// pulled one (the record's own roll is ignored).
///
/// Foil is a pure function of the card's seed (`worth::is_foil`), so the seed
/// is nudged, deterministically, until the card's foil matches `Pull::foil`:
/// what the lottery announced is exactly what lands in the booklet.
/// `number` is 0 until the booklet assigns it.
pub fn card_for_pull(p: &Pull, seed: u64) -> Card {
    const STEP: u64 = 0x9E37_79B9_7F4A_7C15;
    let seed = (0..4096u64)
        .map(|k| seed.wrapping_add(k.wrapping_mul(STEP)))
        .find(|&s| crate::worth::seed_is_foil(s) == p.foil)
        .unwrap_or(seed);
    let record = DreamRecord {
        theme: p.theme,
        seed,
        depth: 1,
        name: dream_name(p.theme, seed),
        whisper: whisper(p.theme, seed),
        strangeness: pull_strangeness(p.rarity),
        enemies: 0,
        shard_taken: false,
        art: portal_surface(p.theme, seed),
        blend: None,
    };
    let mut card = card_from(&record);
    card.rarity = p.rarity;
    card
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn state_with(since_lucid: u32, since_prophetic: u32) -> LotteryState {
        LotteryState {
            since_lucid,
            since_prophetic,
            ..Default::default()
        }
    }

    fn recalled(theme: DreamTheme, rarity: Rarity, remembered: bool) -> Recalled {
        let p = Pull {
            theme,
            rarity,
            foil: false,
            pity: false,
            near_miss: None,
            sparks_gained: 0,
        };
        Recalled {
            card: card_for_pull(&p, 7),
            remembered,
        }
    }

    #[test]
    fn base_odds_sum_to_one() {
        let sum: f32 = BASE_ODDS.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pull_is_deterministic() {
        for seed in 0..200u64 {
            let mut a = state_with(5, 9);
            let mut b = state_with(5, 9);
            let f = Some(DreamTheme::Garden);
            assert_eq!(pull(&mut a, seed, f), pull(&mut b, seed, f));
            assert_eq!(a, b);
        }
    }

    #[test]
    fn odds_sum_to_one_in_every_pity_state() {
        for sl in 0..=70 {
            for sp in [0, 10, 40, 59, 60, 100] {
                let o = odds(&state_with(sl, sp), Some(DreamTheme::Lobby));
                let sum: f32 = o.rarity.iter().sum();
                assert!((sum - 1.0).abs() < 1e-4, "sl={sl} sp={sp} sum={sum}");
                assert!(o.rarity.iter().all(|&p| (0.0..=1.0 + 1e-6).contains(&p)));
                let shown: f32 = display_odds(&o).iter().map(|(_, p)| p).sum();
                assert!((shown - 1.0).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn fresh_odds_are_the_base_odds() {
        let o = odds(&LotteryState::default(), None);
        for i in 0..5 {
            assert!((o.rarity[i] - BASE_ODDS[i]).abs() < 1e-6, "tier {i}");
        }
        assert_eq!(o.featured_share, 0.0);
    }

    #[test]
    fn empirical_distribution_matches_odds() {
        // Fresh state every time, so every pull has the same exact odds.
        let n = 20_000u64;
        let expected = odds(&LotteryState::default(), None);
        let mut counts = [0u32; 5];
        let mut foils = 0u32;
        for seed in 0..n {
            let p = pull(&mut LotteryState::default(), seed, None);
            let i = PULL_RARITIES.iter().position(|&r| r == p.rarity).unwrap();
            counts[i] += 1;
            foils += p.foil as u32;
        }
        for i in 0..5 {
            let got = counts[i] as f32 / n as f32;
            assert!(
                (got - expected.rarity[i]).abs() < 0.015,
                "tier {i}: got {got}, want {}",
                expected.rarity[i]
            );
        }
        let foil_rate = foils as f32 / n as f32;
        assert!((foil_rate - FOIL_ODDS).abs() < 0.007, "foil {foil_rate}");
    }

    #[test]
    fn empirical_distribution_matches_odds_under_soft_pity() {
        let n = 20_000u64;
        let start = state_with(24, 0);
        let expected = odds(&start, None);
        let mut lucid_plus = 0u32;
        for seed in 0..n {
            let p = pull(&mut start.clone(), seed, None);
            lucid_plus += (p.rarity >= Rarity::Lucid) as u32;
        }
        let got = lucid_plus as f32 / n as f32;
        assert!((got - expected.lucid_or_better()).abs() < 0.015, "{got}");
    }

    #[test]
    fn only_pullable_rarities_and_themes_are_produced() {
        let mut s = LotteryState::default();
        for seed in 0..3000u64 {
            let p = pull(&mut s, seed, None);
            assert!(PULL_RARITIES.contains(&p.rarity));
            assert_ne!(p.theme, DreamTheme::Awakening);
        }
    }

    #[test]
    fn hard_pity_lucid_always_triggers_by_pull_30() {
        for seed in 0..3000u64 {
            let mut s = LotteryState::default();
            let mut pulls = 0u64;
            loop {
                pulls += 1;
                let p = pull(&mut s, seed.wrapping_mul(1000).wrapping_add(pulls), None);
                if p.rarity >= Rarity::Lucid {
                    break;
                }
                assert!(pulls < HARD_PITY_LUCID as u64, "seed {seed} dry for {pulls}");
            }
            assert!(pulls <= HARD_PITY_LUCID as u64);
        }
    }

    #[test]
    fn hard_pity_marks_the_forced_pull() {
        let s = state_with(HARD_PITY_LUCID - 1, 0);
        // Over many seeds, at least one roll would naturally have been low.
        let mut forced = 0;
        for seed in 0..500u64 {
            let p = pull(&mut s.clone(), seed, None);
            assert!(p.rarity >= Rarity::Lucid);
            forced += p.pity as u32;
        }
        assert!(forced > 0);
        // No pity flag on a pull with plenty of room left.
        for seed in 0..500u64 {
            assert!(!pull(&mut LotteryState::default(), seed, None).pity);
        }
    }

    #[test]
    fn hard_pity_prophetic_always_triggers_by_pull_60() {
        for seed in 0..1500u64 {
            let mut s = LotteryState::default();
            let mut pulls = 0u64;
            loop {
                pulls += 1;
                let p = pull(&mut s, seed.wrapping_mul(1000).wrapping_add(pulls), None);
                if p.rarity == Rarity::Prophetic {
                    break;
                }
                assert!(pulls < HARD_PITY_PROPHETIC as u64, "seed {seed}");
            }
            assert!(pulls <= HARD_PITY_PROPHETIC as u64);
        }
    }

    #[test]
    fn soft_pity_raises_lucid_chance_monotonically() {
        let mut last = 0.0;
        for since in 0..HARD_PITY_LUCID {
            let p = odds(&state_with(since, 0), None).lucid_or_better();
            assert!(p >= last - 1e-6, "dropped at since={since}");
            last = p;
        }
        // Flat before soft pity, +6 points per pull after it.
        let flat = odds(&state_with(0, 0), None).lucid_or_better();
        let at_soft = odds(&state_with(SOFT_PITY_LUCID - 1, 0), None).lucid_or_better();
        assert!((flat - at_soft).abs() < 1e-6);
        let after = odds(&state_with(SOFT_PITY_LUCID, 0), None).lucid_or_better();
        assert!((after - at_soft - SOFT_PITY_STEP).abs() < 1e-5);
        let last_pull = odds(&state_with(HARD_PITY_LUCID - 1, 0), None).lucid_or_better();
        assert!((last_pull - 1.0).abs() < 1e-6);
    }

    #[test]
    fn counters_update_and_reset() {
        let mut s = LotteryState::default();
        let mut expect_since_lucid = 0;
        let mut expect_since_prophetic = 0;
        for seed in 0..400u64 {
            let p = pull(&mut s, seed, None);
            assert_eq!(p.sparks_gained, SPARKS_PER_PULL);
            expect_since_lucid = if p.rarity >= Rarity::Lucid { 0 } else { expect_since_lucid + 1 };
            expect_since_prophetic =
                if p.rarity == Rarity::Prophetic { 0 } else { expect_since_prophetic + 1 };
            assert_eq!(s.since_lucid, expect_since_lucid);
            assert_eq!(s.since_prophetic, expect_since_prophetic);
            assert_eq!(s.total_pulls, seed as u32 + 1);
            assert_eq!(s.sparks, seed as u32 + 1);
            assert!(s.since_lucid < HARD_PITY_LUCID);
            assert!(s.since_prophetic < HARD_PITY_PROPHETIC);
        }
    }

    #[test]
    fn prophetic_resets_both_pity_counters() {
        let mut s = state_with(10, HARD_PITY_PROPHETIC - 1);
        let p = pull(&mut s, 1, None);
        assert_eq!(p.rarity, Rarity::Prophetic);
        assert_eq!((s.since_lucid, s.since_prophetic), (0, 0));
    }

    #[test]
    fn spark_pick_costs_sparks_and_gives_vivid_plus() {
        let mut s = LotteryState {
            sparks: SPARKS_FOR_PICK - 1,
            ..Default::default()
        };
        assert!(!spark_pick_ready(&s));
        assert_eq!(redeem_spark_pick(&mut s, DreamTheme::Garden), None);
        assert_eq!(s.sparks, SPARKS_FOR_PICK - 1);
        s.sparks += 1;
        assert!(spark_pick_ready(&s));
        // Awakening can't be chosen, and the refusal is free.
        assert_eq!(redeem_spark_pick(&mut s, DreamTheme::Awakening), None);
        assert_eq!(s.sparks, SPARKS_FOR_PICK);
        let p = redeem_spark_pick(&mut s, DreamTheme::MirrorHall).unwrap();
        assert_eq!(p.theme, DreamTheme::MirrorHall);
        assert!(p.rarity >= Rarity::Vivid);
        assert_eq!(s.sparks, 0);
        assert_eq!(s.total_pulls, 0);
    }

    #[test]
    fn sparks_reach_a_pick_after_thirty_pulls() {
        let mut s = LotteryState::default();
        for seed in 0..SPARKS_FOR_PICK as u64 {
            assert!(!spark_pick_ready(&s));
            pull(&mut s, seed, None);
        }
        assert!(spark_pick_ready(&s));
    }

    #[test]
    fn state_round_trips_through_ron() {
        let s = LotteryState {
            total_pulls: 12,
            since_lucid: 7,
            since_prophetic: 11,
            sparks: 12,
            pack_dry: 3,
        };
        let text = ron::to_string(&s).unwrap();
        assert_eq!(ron::from_str::<LotteryState>(&text).unwrap(), s);
    }

    #[test]
    fn old_saves_missing_fields_still_parse() {
        assert_eq!(ron::from_str::<LotteryState>("()").unwrap(), LotteryState::default());
        let partial: LotteryState = ron::from_str("(total_pulls: 5, sparks: 5)").unwrap();
        assert_eq!(partial.total_pulls, 5);
        assert_eq!(partial.since_lucid, 0);
        assert_eq!(partial.pack_dry, 0);
    }

    #[test]
    fn pack_pity_fires_exactly_after_pack_pity_dry_packs() {
        let mut s = LotteryState::default();
        let dry_pack = || {
            vec![
                recalled(DreamTheme::Awakening, Rarity::Vivid, true),
                recalled(DreamTheme::Garden, Rarity::Hazy, true),
                recalled(DreamTheme::Lobby, Rarity::Vivid, false),
            ]
        };
        for n in 1..=PACK_PITY {
            let mut pack = dry_pack();
            let before = pack.clone();
            apply_pack_pity(&mut s, &mut pack);
            assert_eq!(pack, before, "pack {n} must be untouched");
            assert_eq!(s.pack_dry, n);
        }
        let mut pack = dry_pack();
        apply_pack_pity(&mut s, &mut pack);
        // Best card is the faded Vivid Lobby: Lucid and remembered now.
        assert_eq!(pack[2].card.rarity, Rarity::Lucid);
        assert!(pack[2].remembered);
        assert_eq!(pack[1].card.rarity, Rarity::Hazy);
        assert_eq!(s.pack_dry, 0);
    }

    #[test]
    fn pack_pity_keeps_awakening_remembered_and_untouched() {
        let mut s = LotteryState {
            pack_dry: PACK_PITY,
            ..Default::default()
        };
        let mut pack = vec![
            recalled(DreamTheme::Awakening, Rarity::Vivid, true),
            recalled(DreamTheme::Garden, Rarity::Faint, false),
        ];
        apply_pack_pity(&mut s, &mut pack);
        assert_eq!(pack[0].card.rarity, Rarity::Vivid);
        assert!(pack[0].remembered);
        assert_eq!(pack[1].card.rarity, Rarity::Lucid);
        assert!(pack[1].remembered);
    }

    #[test]
    fn remembered_lucid_resets_pack_dry_and_blocks_pity() {
        let mut s = LotteryState {
            pack_dry: PACK_PITY,
            ..Default::default()
        };
        let mut pack = vec![
            recalled(DreamTheme::Garden, Rarity::Lucid, true),
            recalled(DreamTheme::Lobby, Rarity::Faint, false),
        ];
        let before = pack.clone();
        apply_pack_pity(&mut s, &mut pack);
        assert_eq!(pack, before);
        assert_eq!(s.pack_dry, 0);
    }

    #[test]
    fn faded_lucid_does_not_count_as_remembered_lucid() {
        let mut s = LotteryState {
            pack_dry: 1,
            ..Default::default()
        };
        let mut pack = vec![recalled(DreamTheme::Garden, Rarity::Prophetic, false)];
        apply_pack_pity(&mut s, &mut pack);
        assert_eq!(s.pack_dry, 2);
        assert!(!pack[0].remembered);
    }

    #[test]
    fn pack_pity_without_candidates_keeps_waiting() {
        let mut s = LotteryState {
            pack_dry: PACK_PITY,
            ..Default::default()
        };
        let mut pack = vec![recalled(DreamTheme::Awakening, Rarity::Vivid, true)];
        apply_pack_pity(&mut s, &mut pack);
        assert_eq!(pack[0].card.rarity, Rarity::Vivid);
        assert_eq!(s.pack_dry, PACK_PITY + 1);
    }

    #[test]
    fn featured_theme_is_stable_per_day_and_cycles_all_pullable() {
        let pool = pullable_themes();
        assert_eq!(pool.len(), ALL_THEMES.len() - 1);
        for day in [0u64, 1, 19, 20_000, u64::MAX] {
            assert_eq!(featured_theme(day), featured_theme(day));
            assert_ne!(featured_theme(day), DreamTheme::Awakening);
        }
        let seen: std::collections::HashSet<_> =
            (0..pool.len() as u64).map(featured_theme).collect();
        assert_eq!(seen.len(), pool.len());
        assert_eq!(featured_theme(3), featured_theme(3 + pool.len() as u64));
    }

    #[test]
    fn featured_theme_is_about_three_times_as_likely() {
        let n = 20_000u64;
        let featured = DreamTheme::Garden;
        let mut counts: HashMap<DreamTheme, u32> = HashMap::new();
        for seed in 0..n {
            let p = pull(&mut LotteryState::default(), seed, Some(featured));
            *counts.entry(p.theme).or_default() += 1;
        }
        let feat = counts[&featured] as f32;
        let others: Vec<f32> = pullable_themes()
            .into_iter()
            .filter(|&t| t != featured)
            .map(|t| counts.get(&t).copied().unwrap_or(0) as f32)
            .collect();
        let avg_other = others.iter().sum::<f32>() / others.len() as f32;
        let ratio = feat / avg_other;
        assert!((2.6..3.4).contains(&ratio), "ratio {ratio}");

        let o = odds(&LotteryState::default(), Some(featured));
        assert!((o.featured_share / o.other_share - 3.0).abs() < 1e-4);
        let total = o.featured_share + o.other_share * (pullable_themes().len() - 1) as f32;
        assert!((total - 1.0).abs() < 1e-4);
        assert!((feat / n as f32 - o.featured_share).abs() < 0.015);
    }

    #[test]
    fn near_miss_never_changes_the_outcome() {
        // The rarity is decided before near_miss is looked at, from the same
        // roll: so the pull is identical with near_miss stripped, state
        // updates are identical, and a near miss is always a reachable,
        // strictly better rarity.
        let mut near = 0;
        for seed in 0..5000u64 {
            let mut s = LotteryState::default();
            let o = odds(&s, None);
            let p = pull(&mut s, seed, None);
            let mut s2 = LotteryState::default();
            let q = Pull {
                near_miss: None,
                ..pull(&mut s2, seed, None)
            };
            assert_eq!((p.rarity, p.theme, p.foil), (q.rarity, q.theme, q.foil));
            assert_eq!(s, s2);
            if let Some(better) = p.near_miss {
                near += 1;
                assert!(better > p.rarity);
                let i = PULL_RARITIES.iter().position(|&r| r == better).unwrap();
                assert!(o.rarity[i] > 0.0);
            }
        }
        assert!(near > 0, "near misses should occur sometimes");
    }

    #[test]
    fn near_miss_rate_matches_the_window() {
        // Landing in the top NEAR_MISS_WINDOW of each of the 4 non-top bands
        // (every band here is wider than the window).
        let n = 20_000u64;
        let mut near = 0u32;
        for seed in 0..n {
            near += pull(&mut LotteryState::default(), seed, None).near_miss.is_some() as u32;
        }
        let rate = near as f32 / n as f32;
        assert!((rate - 4.0 * NEAR_MISS_WINDOW).abs() < 0.02, "rate {rate}");
    }

    #[test]
    fn card_for_pull_matches_the_pull() {
        for (i, &rarity) in PULL_RARITIES.iter().enumerate() {
            let p = Pull {
                theme: DreamTheme::JellyfishSky,
                rarity,
                foil: false,
                pity: false,
                near_miss: None,
                sparks_gained: 1,
            };
            let a = card_for_pull(&p, 40 + i as u64);
            assert_eq!(a, card_for_pull(&p, 40 + i as u64));
            assert_eq!(a.rarity, rarity);
            assert_eq!(a.theme, DreamTheme::JellyfishSky);
            assert_eq!(a.fused, None);
            assert!(!a.name.is_empty());
        }
    }

    #[test]
    fn the_card_is_foil_exactly_when_the_pull_was() {
        let mut state = LotteryState::default();
        let mut foils = 0;
        for seed in 0..600u64 {
            let p = pull(&mut state, seed.wrapping_mul(0x1234_5678_9ABC_DEF1), None);
            let card = card_for_pull(&p, seed);
            assert_eq!(crate::worth::is_foil(&card), p.foil, "seed {seed}");
            assert_eq!(card.rarity, p.rarity);
            foils += p.foil as u32;
        }
        assert!(foils > 0, "the sweep should include some foils");
    }
}
