//! Relic pull engine (R2): pure, deterministic, no GL / clock / global RNG.
//! Every result is a function of `(RelicState, seed, featured)`.
//! Draw order is fixed: rarity -> theme -> foil (wear/etch are separate
//! pure helpers keyed by the relic's own seed).

use crate::dream::{DreamTheme, ALL_THEMES};
use crate::relic_types::{RelicRarity, RelicState};
use crate::worth::{mix, percent_roll};

pub const BASE_RELIC_ODDS: [f32; 4] = [0.50, 0.30, 0.17, 0.03];
pub const RELIC_FOIL_ODDS: f32 = 1.0 / 36.0;
/// Pull number (since the last Mythic) at which the soft-pity ramp starts.
pub const SOFT_PITY_START: u32 = 14;
/// Mythic odds added per pull from `SOFT_PITY_START` on.
pub const SOFT_PITY_STEP: f32 = 0.05;
/// Pull number (since the last Mythic) that is guaranteed Mythic.
pub const HARD_PITY_MYTHIC: u32 = 24;
pub const FEATURED_RELIC_THEME_WEIGHT: u32 = 2;
pub const RELIC_COST: [u32; 4] = [40, 60, 80, 120];

/// Number of etch glyphs `etch_glyph` can return (`0..ETCH_GLYPHS`).
pub const ETCH_GLYPHS: u32 = 10;
/// Foil is a 1-in-36 draw.
const FOIL_DENOM: u64 = 36;
/// A non-Mythic roll this close below the Mythic band counts as a near miss.
const NEAR_MISS_WINDOW: f32 = 0.03;

const RARITY_SALT: u64 = 0x5E11_C0DE_0000_0001;
const THEME_SALT: u64 = 0x5E11_C0DE_0000_0002;
const FOIL_SALT: u64 = 0x5E11_C0DE_0000_0003;
const WEAR_SALT: u64 = 0x5E11_C0DE_0000_0004;
const ETCH_SALT: u64 = 0x5E11_C0DE_0000_0005;

const RARITIES: [RelicRarity; 4] = [
    RelicRarity::Cracked,
    RelicRarity::Worn,
    RelicRarity::Pristine,
    RelicRarity::Mythic,
];
const MYTHIC: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct RelicPull {
    pub rarity: RelicRarity,
    pub theme: DreamTheme,
    pub foil: bool,
    /// Hard pity changed the rarity the roll would otherwise have given.
    pub pity: bool,
    /// Roll landed just under the Mythic band (cosmetic).
    pub near_miss: Option<RelicRarity>,
    pub cost: u32,
}

/// Exactly the probabilities `pull()` will use for the next pull.
#[derive(Clone, Debug, PartialEq)]
pub struct RelicOdds {
    /// Soft pity only.
    pub natural: [f32; 4],
    /// Soft + hard pity: what `pull()` rolls against.
    pub rarity: [f32; 4],
    /// Per-theme probability, in `pullable_themes()` order (sums to 1).
    pub themes: Vec<(DreamTheme, f32)>,
    pub foil: f32,
    /// Pulls left until Mythic is guaranteed (1 = next pull).
    pub pulls_to_hard_pity: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WearTier {
    Clean,
    Light,
    Heavy,
    Ruined,
}

pub fn pullable_themes() -> Vec<DreamTheme> {
    ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| t != DreamTheme::Awakening)
        .collect()
}

pub fn featured_relic_theme(day: u64) -> DreamTheme {
    let pool = pullable_themes();
    pool[(day % pool.len() as u64) as usize]
}

pub fn cost_for(rarity: RelicRarity) -> u32 {
    RELIC_COST[rarity_index(rarity)]
}

fn rarity_index(r: RelicRarity) -> usize {
    RARITIES.iter().position(|&x| x == r).unwrap_or(0)
}

fn normalise(mut t: [f32; 4]) -> [f32; 4] {
    let sum: f32 = t.iter().sum();
    if sum > 0.0 {
        t.iter_mut().for_each(|v| *v /= sum);
    }
    t
}

/// `(natural, final)` odds for the next pull. Both sum to 1.0.
pub fn rarity_tables(state: &RelicState) -> ([f32; 4], [f32; 4]) {
    let pull_no = state.since_mythic.saturating_add(1);
    let ramp = if pull_no >= SOFT_PITY_START {
        (pull_no - SOFT_PITY_START + 1) as f32 * SOFT_PITY_STEP
    } else {
        0.0
    };
    let mythic = (BASE_RELIC_ODDS[MYTHIC] + ramp).min(1.0);
    let rest_base: f32 = BASE_RELIC_ODDS[..MYTHIC].iter().sum();
    let scale = (1.0 - mythic) / rest_base;
    let mut natural = [0.0; 4];
    for i in 0..MYTHIC {
        natural[i] = BASE_RELIC_ODDS[i] * scale;
    }
    natural[MYTHIC] = mythic;
    let natural = normalise(natural);
    let fin = if pull_no >= HARD_PITY_MYTHIC {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        natural
    };
    (natural, fin)
}

/// Index of the bucket `u` (in `0..1`) falls in. Always a valid index.
pub fn pick_index(table: &[f32; 4], u: f32) -> usize {
    let mut acc = 0.0;
    for (i, p) in table.iter().enumerate() {
        acc += p;
        if u < acc && *p > 0.0 {
            return i;
        }
    }
    // float slack: last non-zero bucket
    table.iter().rposition(|&p| p > 0.0).unwrap_or(0)
}

fn theme_weights(featured: Option<DreamTheme>) -> Vec<(DreamTheme, u32)> {
    let featured = featured.filter(|&t| t != DreamTheme::Awakening);
    pullable_themes()
        .into_iter()
        .map(|t| {
            let w = if Some(t) == featured { FEATURED_RELIC_THEME_WEIGHT } else { 1 };
            (t, w)
        })
        .collect()
}

/// Weighted theme pick from a 64-bit roll.
pub fn roll_theme(rng: u64, featured: Option<DreamTheme>) -> DreamTheme {
    let w = theme_weights(featured);
    let total: u64 = w.iter().map(|&(_, n)| n as u64).sum();
    let mut r = rng % total;
    for (t, n) in &w {
        if r < *n as u64 {
            return *t;
        }
        r -= *n as u64;
    }
    w[0].0
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

pub fn odds(state: &RelicState, featured: Option<DreamTheme>) -> RelicOdds {
    let (natural, rarity) = rarity_tables(state);
    let w = theme_weights(featured);
    let total: f32 = w.iter().map(|&(_, n)| n as f32).sum();
    RelicOdds {
        natural,
        rarity,
        themes: w.into_iter().map(|(t, n)| (t, n as f32 / total)).collect(),
        foil: RELIC_FOIL_ODDS,
        pulls_to_hard_pity: HARD_PITY_MYTHIC.saturating_sub(state.since_mythic.saturating_add(1)) + 1,
    }
}

/// One pull. Advances the counters in `state`.
pub fn pull(state: &mut RelicState, seed: u64, featured: Option<DreamTheme>) -> RelicPull {
    let (natural, fin) = rarity_tables(state);
    let u = unit(mix(seed, RARITY_SALT));
    let idx = pick_index(&fin, u);
    let natural_idx = pick_index(&natural, u);
    let rarity = RARITIES[idx];
    let pity = idx != natural_idx;
    let near_miss = if idx != MYTHIC && u >= 1.0 - fin[MYTHIC] - NEAR_MISS_WINDOW {
        Some(RelicRarity::Mythic)
    } else {
        None
    };
    let theme = roll_theme(mix(seed, THEME_SALT), featured);
    let foil = mix(seed, FOIL_SALT) % FOIL_DENOM == 0;
    let cost = cost_for(rarity);

    state.total_pulls = state.total_pulls.saturating_add(1);
    state.ash_spent = state.ash_spent.saturating_add(cost);
    if rarity == RelicRarity::Mythic {
        state.since_mythic = 0;
    } else {
        state.since_mythic = state.since_mythic.saturating_add(1);
    }
    if matches!(rarity, RelicRarity::Mythic | RelicRarity::Pristine) {
        state.since_pristine = 0;
    } else {
        state.since_pristine = state.since_pristine.saturating_add(1);
    }
    RelicPull { rarity, theme, foil, pity, near_miss, cost }
}

/// Wear level for the art pass (R3). Pure in `seed`.
pub fn wear_tier(seed: u64) -> WearTier {
    match percent_roll(seed, WEAR_SALT) {
        0..=39 => WearTier::Clean,
        40..=69 => WearTier::Light,
        70..=89 => WearTier::Heavy,
        _ => WearTier::Ruined,
    }
}

/// Etch glyph index in `0..ETCH_GLYPHS`. Pure in `seed`.
pub fn etch_glyph(seed: u64) -> u8 {
    (percent_roll(seed, ETCH_SALT) % ETCH_GLYPHS) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(since_mythic: u32) -> RelicState {
        RelicState { since_mythic, ..Default::default() }
    }

    #[test]
    fn hard_pity_mythic_at_exactly_24() {
        for base in 0..100u64 {
            let mut st = RelicState::default();
            let mut since = 0;
            for i in 0..100u64 {
                let p = pull(&mut st, mix(base, i), None);
                since += 1;
                if p.rarity == RelicRarity::Mythic {
                    assert!(since <= HARD_PITY_MYTHIC);
                    since = 0;
                } else {
                    assert!(since < HARD_PITY_MYTHIC, "pull 24 must be Mythic");
                }
            }
        }
        // 23 misses then pull 24 is Mythic for any seed.
        for seed in 0..500u64 {
            let mut st = s(HARD_PITY_MYTHIC - 1);
            assert_eq!(pull(&mut st, seed, None).rarity, RelicRarity::Mythic);
            assert_eq!(st.since_mythic, 0);
        }
        // pull 23 is not guaranteed
        assert!(rarity_tables(&s(22)).1[MYTHIC] < 1.0);
    }

    #[test]
    fn soft_pity_ramps_from_14() {
        assert_eq!(rarity_tables(&s(12)).1[MYTHIC], BASE_RELIC_ODDS[MYTHIC]);
        let m14 = rarity_tables(&s(13)).1[MYTHIC];
        assert!((m14 - 0.08).abs() < 1e-5);
        assert!(rarity_tables(&s(14)).1[MYTHIC] > m14);
    }

    #[test]
    fn odds_sum_to_one_every_call() {
        for n in 0..40 {
            for f in [None, Some(DreamTheme::Garden)] {
                let st = s(n);
                let o = odds(&st, f);
                assert!((o.natural.iter().sum::<f32>() - 1.0).abs() < 1e-5);
                assert!((o.rarity.iter().sum::<f32>() - 1.0).abs() < 1e-5);
                assert!((o.themes.iter().map(|t| t.1).sum::<f32>() - 1.0).abs() < 1e-5);
                assert_eq!(o.rarity, rarity_tables(&st).1);
            }
        }
    }

    #[test]
    fn featured_theme_gets_weight_bump() {
        let f = DreamTheme::Garden;
        let o = odds(&RelicState::default(), Some(f));
        let pf = o.themes.iter().find(|t| t.0 == f).unwrap().1;
        let po = o.themes.iter().find(|t| t.0 != f).unwrap().1;
        assert!((pf / po - FEATURED_RELIC_THEME_WEIGHT as f32).abs() < 1e-4);
        let n = 20000u64;
        let hits = (0..n).filter(|&i| roll_theme(mix(7, i), Some(f)) == f).count() as f32 / n as f32;
        assert!((hits - pf).abs() < 0.015, "{hits} vs {pf}");
        assert!((0..n).all(|i| roll_theme(mix(9, i), None) != DreamTheme::Awakening));
    }

    #[test]
    fn foil_independent_of_rarity() {
        let n = 30000u64;
        let (mut foil, mut mythic, mut both) = (0u32, 0u32, 0u32);
        for i in 0..n {
            let p = pull(&mut RelicState::default(), mix(1, i), None);
            let m = p.rarity == RelicRarity::Mythic;
            foil += p.foil as u32;
            mythic += m as u32;
            both += (p.foil && m) as u32;
        }
        let (pf, pm) = (foil as f32 / n as f32, mythic as f32 / n as f32);
        assert!((pf - RELIC_FOIL_ODDS).abs() < 0.006, "{pf}");
        assert!((both as f32 / n as f32 - pf * pm).abs() < 0.0015);
        // forced Mythic via hard pity does not change foil
        let a = pull(&mut s(23), 5, None);
        let b = pull(&mut s(0), 5, None);
        assert_eq!(a.foil, b.foil);
        assert_eq!(a.theme, b.theme);
    }

    #[test]
    fn deterministic() {
        for seed in 0..200u64 {
            let (mut a, mut b) = (s(seed as u32 % 25), s(seed as u32 % 25));
            let f = Some(featured_relic_theme(seed));
            assert_eq!(pull(&mut a, seed, f), pull(&mut b, seed, f));
            assert_eq!(a, b);
        }
        assert_eq!(wear_tier(42), wear_tier(42));
        assert_eq!(etch_glyph(42), etch_glyph(42));
        assert!((0..1000u64).all(|i| (etch_glyph(i) as u32) < ETCH_GLYPHS));
    }

    #[test]
    fn counters_and_costs() {
        let mut st = RelicState::default();
        let p = pull(&mut st, 3, None);
        assert_eq!((st.total_pulls, st.ash_spent), (1, p.cost));
        assert_eq!(cost_for(RelicRarity::Mythic), 120);
        assert_eq!(featured_relic_theme(0), featured_relic_theme(19));
        assert_ne!(featured_relic_theme(0), DreamTheme::Awakening);
    }

    #[test]
    fn old_state_loads_from_empty_ron() {
        let st: RelicState = ron::from_str("()").unwrap();
        assert_eq!(st, RelicState::default());
    }
}
