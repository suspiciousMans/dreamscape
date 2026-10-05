//! What a dream card is *worth*: quirks, foil, dust value, mastery and echoes.
//!
//! Pure data and arithmetic only. Everything about a card (quirk, foil) is
//! derived deterministically from `Card::seed` and `Card::rarity`, so nothing
//! about a card needs saving. Only `Mastery` is meant to be persisted
//! (every field is `#[serde(default)]`, so old saves keep parsing).
//!
//! Quirks are expressed as data (`QuirkEffect`) that maps onto levers the run
//! already has: dust payout, memory chance, respawn grace, stride speed,
//! enemy calm, upgrade rerolls and starting shards.
#![allow(dead_code)]

use crate::cards::{Attribute, Card, Rarity};
use crate::dream::DreamTheme;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Seed hashing
// ---------------------------------------------------------------------------

const QUIRK_CHANCE_SALT: u64 = 0x517C_C1B7_2722_0A95;
const QUIRK_PICK_SALT: u64 = 0x2545_F491_4F6C_DD1D;
const FOIL_SALT: u64 = 0x9E37_79B9_7F4A_7C15;

/// One round of SplitMix64: a well-mixed u64 from `seed ^ salt`.
fn mix(seed: u64, salt: u64) -> u64 {
    let mut z = (seed ^ salt).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Uniform value in `0..100` from a hash.
fn percent_roll(seed: u64, salt: u64) -> u32 {
    (mix(seed, salt) % 100) as u32
}

// ---------------------------------------------------------------------------
// Quirks
// ---------------------------------------------------------------------------

/// A small run-time trait some cards carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Quirk {
    /// +10% dust from the run.
    Magpie,
    /// +12% dust, nothing else.
    Windfall,
    /// Dreams are remembered a little more often.
    Keepsake,
    /// Slightly better memory and a slightly faster stride.
    Drifter,
    /// One extra beat of respawn grace.
    SoftLanding,
    /// Slightly faster stride.
    Fleet,
    /// Enemies notice you a little less.
    Hushed,
    /// A free reroll on your first dream upgrade offer.
    SecondDraw,
    /// Start the run with one lucidity shard.
    Glimmer,
    /// Calmer enemies and a little grace.
    Lullaby,
}

/// Every quirk, in a fixed order.
pub const ALL_QUIRKS: [Quirk; 10] = [
    Quirk::Magpie,
    Quirk::Windfall,
    Quirk::Keepsake,
    Quirk::Drifter,
    Quirk::SoftLanding,
    Quirk::Fleet,
    Quirk::Hushed,
    Quirk::SecondDraw,
    Quirk::Glimmer,
    Quirk::Lullaby,
];

/// The numbers a quirk (or a mastery level) adds to a run. All zero = no effect.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuirkEffect {
    /// Extra percent on the run's dust payout.
    pub dust_bonus_pct: u32,
    /// Added to the memory chance (0.05 = +5 points).
    pub memory_bonus: f32,
    /// Extra percent on movement speed.
    pub speed_pct: u32,
    /// Extra respawn grace, in "beats" (same unit as the SLOW HEART grace step).
    pub grace_bonus: u32,
    /// Percent reduction of enemy aggression/speed.
    pub calm_pct: u32,
    /// Free upgrade rerolls granted at the start of the run.
    pub free_rerolls: u32,
    /// Lucidity shards held at the start of the run.
    pub start_shards: u32,
}

impl QuirkEffect {
    /// Field-wise sum of two effects.
    pub fn plus(self, other: QuirkEffect) -> QuirkEffect {
        QuirkEffect {
            dust_bonus_pct: self.dust_bonus_pct + other.dust_bonus_pct,
            memory_bonus: self.memory_bonus + other.memory_bonus,
            speed_pct: self.speed_pct + other.speed_pct,
            grace_bonus: self.grace_bonus + other.grace_bonus,
            calm_pct: self.calm_pct + other.calm_pct,
            free_rerolls: self.free_rerolls + other.free_rerolls,
            start_shards: self.start_shards + other.start_shards,
        }
    }

    /// True when the effect changes nothing.
    pub fn is_zero(&self) -> bool {
        *self == QuirkEffect::default()
    }
}

/// The run-time effect of a quirk.
pub fn effect(q: Quirk) -> QuirkEffect {
    let none = QuirkEffect::default();
    match q {
        Quirk::Magpie => QuirkEffect { dust_bonus_pct: 10, ..none },
        Quirk::Windfall => QuirkEffect { dust_bonus_pct: 12, ..none },
        Quirk::Keepsake => QuirkEffect { memory_bonus: 0.05, ..none },
        Quirk::Drifter => QuirkEffect { memory_bonus: 0.02, speed_pct: 3, ..none },
        Quirk::SoftLanding => QuirkEffect { grace_bonus: 1, ..none },
        Quirk::Fleet => QuirkEffect { speed_pct: 5, ..none },
        Quirk::Hushed => QuirkEffect { calm_pct: 10, ..none },
        Quirk::SecondDraw => QuirkEffect { free_rerolls: 1, ..none },
        Quirk::Glimmer => QuirkEffect { start_shards: 1, ..none },
        Quirk::Lullaby => QuirkEffect { calm_pct: 6, grace_bonus: 1, ..none },
    }
}

/// Short ALL-CAPS HUD label (at most 16 characters).
pub fn label(q: Quirk) -> &'static str {
    match q {
        Quirk::Magpie => "MAGPIE",
        Quirk::Windfall => "WINDFALL",
        Quirk::Keepsake => "KEEPSAKE",
        Quirk::Drifter => "DRIFTER",
        Quirk::SoftLanding => "SOFT LANDING",
        Quirk::Fleet => "FLEET",
        Quirk::Hushed => "HUSHED",
        Quirk::SecondDraw => "SECOND DRAW",
        Quirk::Glimmer => "GLIMMER",
        Quirk::Lullaby => "LULLABY",
    }
}

/// One short evocative line saying what the quirk does.
pub fn describe(q: Quirk) -> &'static str {
    match q {
        Quirk::Magpie => "SHINY THINGS FOLLOW YOU HOME. +10% DUST",
        Quirk::Windfall => "THE POCKETS ARE DEEPER. +12% DUST",
        Quirk::Keepsake => "IT WANTS TO BE REMEMBERED. +5% MEMORY",
        Quirk::Drifter => "LIGHT ON YOUR FEET, LIGHT ON YOUR MIND",
        Quirk::SoftLanding => "THE FLOOR CATCHES YOU. +20% GRACE",
        Quirk::Fleet => "THE HALLWAY LEANS BACKWARD. +5% SPEED",
        Quirk::Hushed => "THE WATCHERS LOSE THE THREAD. CALMER FOES",
        Quirk::SecondDraw => "ASK THE DREAM AGAIN. 1 FREE REROLL",
        Quirk::Glimmer => "A SPARK IN YOUR HAND. START WITH A SHARD",
        Quirk::Lullaby => "A SLOW SONG. CALMER FOES, +20% GRACE",
    }
}

/// Chance, in percent, that a card of this rarity has a quirk.
pub fn quirk_chance_pct(r: Rarity) -> u32 {
    match r {
        Rarity::Faint => 10,
        Rarity::Hazy => 25,
        Rarity::Vivid => 45,
        Rarity::Lucid => 70,
        Rarity::Prophetic | Rarity::Fused | Rarity::Resonant => 100,
    }
}

/// The two quirks an attribute favours (flavour; chosen ~60% of the time).
fn favoured(a: Attribute) -> [Quirk; 2] {
    use Attribute::*;
    use Quirk::*;
    match a {
        Velvet => [Hushed, Lullaby],
        Static => [Drifter, SecondDraw],
        Void => [Fleet, Glimmer],
        Bloom => [Keepsake, Magpie],
        Rust => [Windfall, SoftLanding],
        Dawn => [Glimmer, Keepsake],
        Hex => [SoftLanding, Hushed],
        Tide => [Lullaby, Drifter],
        Aether => [Fleet, Drifter],
        Glass => [SecondDraw, Hushed],
        Spore => [Keepsake, Windfall],
        Abyss => [Fleet, Magpie],
        Halo => [SecondDraw, Glimmer],
        Jest => [Magpie, Windfall],
        Trace => [SoftLanding, SecondDraw],
        Chord => [Fleet, Lullaby],
        Hour => [SoftLanding, Drifter],
        Lumen => [Glimmer, Lullaby],
        Gaze => [Hushed, Keepsake],
        Blank => [Drifter, Glimmer],
    }
}

/// The quirk a card carries, if any. Deterministic from the card's seed,
/// rarity and attribute. For a fixed seed, a higher rarity never loses a quirk
/// a lower rarity had (the same roll is compared with a growing chance).
pub fn quirk_of(card: &Card) -> Option<Quirk> {
    if percent_roll(card.seed, QUIRK_CHANCE_SALT) >= quirk_chance_pct(card.rarity) {
        return None;
    }
    let pick = mix(card.seed, QUIRK_PICK_SALT);
    if pick % 10 < 6 {
        let pair = favoured(card.attribute);
        Some(pair[((pick / 10) % 2) as usize])
    } else {
        Some(ALL_QUIRKS[((pick / 10) % ALL_QUIRKS.len() as u64) as usize])
    }
}

// ---------------------------------------------------------------------------
// Foil and worth
// ---------------------------------------------------------------------------

/// One card in this many is foil.
pub const FOIL_ONE_IN: u64 = 48;

/// Foil is a chase variant: 1 in 48, independent of rarity.
pub fn is_foil(card: &Card) -> bool {
    seed_is_foil(card.seed)
}

/// Foil depends on the seed alone, so a seed can be chosen *for* its foilness.
pub fn seed_is_foil(seed: u64) -> bool {
    mix(seed, FOIL_SALT) % FOIL_ONE_IN == 0
}

/// Dust value of a rarity before depth, foil and quirk.
pub fn base_worth(r: Rarity) -> u32 {
    match r {
        Rarity::Faint => 4,
        Rarity::Hazy => 8,
        Rarity::Vivid => 16,
        Rarity::Lucid => 32,
        Rarity::Prophetic => 64,
        Rarity::Fused => 96,
        Rarity::Resonant => 140,
    }
}

/// Depth factor in percent: 100% at depth 0, +4% per depth, capped at depth 20.
pub fn depth_factor_pct(depth: u32) -> u32 {
    100 + 4 * depth.min(20)
}

/// Dust value of a card: rarity base x depth factor x foil (x3) x quirk (+25%).
/// Used for display and for converting duplicates.
pub fn worth(card: &Card) -> u32 {
    let mut w = base_worth(card.rarity) as u64 * depth_factor_pct(card.depth) as u64;
    if is_foil(card) {
        w *= 3;
    }
    if quirk_of(card).is_some() {
        w = w * 125 / 100;
    }
    // `w` is in hundredths of dust; round to nearest.
    ((w + 50) / 100) as u32
}

/// A label for a worth value.
pub fn worth_tier(w: u32) -> &'static str {
    match w {
        0..=7 => "TRINKET",
        8..=19 => "KEEPSAKE",
        20..=49 => "TREASURE",
        50..=119 => "RELIC",
        120..=299 => "HEIRLOOM",
        _ => "LEGEND",
    }
}

// ---------------------------------------------------------------------------
// Mastery
// ---------------------------------------------------------------------------

/// Highest mastery level.
pub const MAX_LEVEL: u8 = 5;

/// Total xp needed to reach each level (index = level). Strictly increasing.
pub const LEVEL_THRESHOLDS: [u32; 6] = [0, 30, 80, 160, 280, 450];

/// Xp per echo spent.
pub const XP_PER_ECHO: u32 = 6;

/// Mastery level (0..=5) for a total xp amount.
pub fn level(xp: u32) -> u8 {
    let mut lv = 0;
    for (i, &t) in LEVEL_THRESHOLDS.iter().enumerate() {
        if xp >= t {
            lv = i as u8;
        }
    }
    lv
}

/// Xp still needed for the next level, or `None` at max level.
pub fn xp_to_next(xp: u32) -> Option<u32> {
    let lv = level(xp);
    if lv >= MAX_LEVEL {
        None
    } else {
        Some(LEVEL_THRESHOLDS[lv as usize + 1] - xp)
    }
}

/// Xp a card used in a run earns: depth counts (up to 20), shards, and a lucid wake.
pub fn run_xp(depth: u32, lucid_wake: bool, shards: u32) -> u32 {
    5 + 3 * depth.min(20) + 4 * shards + if lucid_wake { 10 } else { 0 }
}

/// Total bonus for being at `level` (stacks with the card's quirk).
/// Levels above 5 count as 5.
pub fn level_bonus(level: u8) -> QuirkEffect {
    let lv = level.min(MAX_LEVEL) as u32;
    QuirkEffect {
        dust_bonus_pct: 2 * lv,
        memory_bonus: 0.01 * lv as f32,
        speed_pct: lv / 2,
        grace_bonus: if lv >= 3 { 1 } else { 0 },
        ..QuirkEffect::default()
    }
}

/// Cosmetic title for a mastered card (level 5 only).
pub fn mastery_title(level: u8) -> Option<&'static str> {
    if level >= MAX_LEVEL {
        Some("DREAM MASTER")
    } else {
        None
    }
}

/// Full run bonus of a card at a given mastery level: quirk plus level perks.
pub fn card_effect(card: &Card, level: u8) -> QuirkEffect {
    let base = level_bonus(level);
    match quirk_of(card) {
        Some(q) => base.plus(effect(q)),
        None => base,
    }
}

/// Per-card xp and per-theme echoes. Persist this; all fields default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mastery {
    /// (card number, xp)
    #[serde(default)]
    pub xp: Vec<(u32, u32)>,
    /// (theme, spare echoes from duplicate dreams)
    #[serde(default)]
    pub echoes: Vec<(DreamTheme, u32)>,
}

/// Xp that `n` echoes are worth.
pub fn echo_xp(echoes: u32) -> u32 {
    echoes.saturating_mul(XP_PER_ECHO)
}

impl Mastery {
    /// Xp earned by a card (0 if never used).
    pub fn xp_for(&self, number: u32) -> u32 {
        self.xp.iter().find(|(n, _)| *n == number).map_or(0, |(_, x)| *x)
    }

    /// Add xp to a card (saturating).
    pub fn add_xp(&mut self, number: u32, amount: u32) {
        match self.xp.iter_mut().find(|(n, _)| *n == number) {
            Some((_, x)) => *x = x.saturating_add(amount),
            None => self.xp.push((number, amount)),
        }
    }

    /// Mastery level of a card.
    pub fn level_of(&self, number: u32) -> u8 {
        level(self.xp_for(number))
    }

    /// Bank `n` echoes for a theme (a duplicate dream was found).
    pub fn add_echo(&mut self, theme: DreamTheme, n: u32) {
        match self.echoes.iter_mut().find(|(t, _)| *t == theme) {
            Some((_, e)) => *e = e.saturating_add(n),
            None => self.echoes.push((theme, n)),
        }
    }

    /// Echoes banked for a theme.
    pub fn echoes_of(&self, theme: DreamTheme) -> u32 {
        self.echoes.iter().find(|(t, _)| *t == theme).map_or(0, |(_, e)| *e)
    }

    /// Spend up to `n` echoes of `theme` on card `number` (a card of that
    /// theme; the caller checks). Returns the xp gained (0 if no echoes).
    pub fn spend_echoes(&mut self, number: u32, theme: DreamTheme, n: u32) -> u32 {
        let spend = n.min(self.echoes_of(theme));
        if spend == 0 {
            return 0;
        }
        if let Some((_, e)) = self.echoes.iter_mut().find(|(t, _)| *t == theme) {
            *e -= spend;
        }
        self.echoes.retain(|(_, e)| *e > 0);
        let gained = echo_xp(spend);
        self.add_xp(number, gained);
        gained
    }
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// A line of origin for a card, e.g. `FOUND AT DEPTH 7 // DREAM #12`.
/// Cards not yet numbered (number 0) omit the dream number.
pub fn provenance(card: &Card) -> String {
    if card.number == 0 {
        format!("FOUND AT DEPTH {}", card.depth)
    } else {
        format!("FOUND AT DEPTH {} // DREAM #{}", card.depth, card.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(rarity: Rarity, seed: u64, depth: u32) -> Card {
        let theme = DreamTheme::Garden;
        Card {
            number: 12,
            theme,
            seed,
            depth,
            name: "TEST".into(),
            description: "test".into(),
            rarity,
            attribute: Attribute::of(theme),
            dread: 0,
            drift: 0,
            recurring: false,
            art: crate::dream::portal_surface(theme, seed),
            fused: None,
        }
    }

    const RARITIES: [Rarity; 7] = [
        Rarity::Faint,
        Rarity::Hazy,
        Rarity::Vivid,
        Rarity::Lucid,
        Rarity::Prophetic,
        Rarity::Fused,
        Rarity::Resonant,
    ];

    #[test]
    fn quirk_and_foil_are_deterministic() {
        for seed in 0..500u64 {
            let c = card(Rarity::Vivid, seed, 3);
            assert_eq!(quirk_of(&c), quirk_of(&c.clone()));
            assert_eq!(is_foil(&c), is_foil(&c.clone()));
            assert_eq!(worth(&c), worth(&c.clone()));
        }
    }

    #[test]
    fn quirk_chance_by_rarity_matches_table() {
        let n = 20_000u64;
        for r in RARITIES {
            let hits = (0..n).filter(|&s| quirk_of(&card(r, s, 1)).is_some()).count() as f64;
            let rate = hits / n as f64;
            let want = quirk_chance_pct(r) as f64 / 100.0;
            assert!((rate - want).abs() < 0.02, "{r:?}: {rate} vs {want}");
        }
        assert_eq!(quirk_chance_pct(Rarity::Faint), 10);
        for r in [Rarity::Prophetic, Rarity::Fused, Rarity::Resonant] {
            assert!((0..2000u64).all(|s| quirk_of(&card(r, s, 0)).is_some()));
        }
    }

    #[test]
    fn quirk_pool_is_used_and_leans_on_attribute() {
        let mut seen = std::collections::HashSet::new();
        for s in 0..5000u64 {
            if let Some(q) = quirk_of(&card(Rarity::Prophetic, s, 1)) {
                seen.insert(q);
            }
        }
        assert_eq!(seen.len(), ALL_QUIRKS.len());
        // Garden's attribute is Bloom: Keepsake / Magpie should dominate.
        let fav = favoured(Attribute::Bloom);
        let hits = (0..5000u64)
            .filter(|&s| fav.contains(&quirk_of(&card(Rarity::Prophetic, s, 1)).unwrap()))
            .count();
        assert!(hits > 3000, "favoured quirks should dominate: {hits}");
    }

    #[test]
    fn foil_rate_is_about_one_in_48_and_independent_of_rarity() {
        let n = 20_000u64;
        let want = n as f64 / 48.0;
        for r in [Rarity::Faint, Rarity::Prophetic] {
            let foils = (0..n).filter(|&s| is_foil(&card(r, s, 1))).count() as f64;
            assert!((foils - want).abs() < want * 0.2, "{r:?}: {foils} vs {want}");
        }
        for s in 0..2000u64 {
            assert_eq!(is_foil(&card(Rarity::Faint, s, 1)), is_foil(&card(Rarity::Resonant, s, 9)));
        }
    }

    #[test]
    fn worth_is_monotonic_in_rarity() {
        for seed in 0..2000u64 {
            let mut prev = 0;
            for r in RARITIES {
                let w = worth(&card(r, seed, 5));
                assert!(w >= prev, "seed {seed} {r:?}: {w} < {prev}");
                prev = w;
            }
        }
        for pair in RARITIES.windows(2) {
            assert!(base_worth(pair[0]) < base_worth(pair[1]));
        }
        // With the quirk roll held equal (Prophetic and up always have one),
        // worth rises strictly.
        let (p, f, r) = (
            worth(&card(Rarity::Prophetic, 1, 5)),
            worth(&card(Rarity::Fused, 1, 5)),
            worth(&card(Rarity::Resonant, 1, 5)),
        );
        assert!(p < f && f < r);
    }

    #[test]
    fn worth_grows_with_depth() {
        let shallow = worth(&card(Rarity::Vivid, 5, 1));
        let deep = worth(&card(Rarity::Vivid, 5, 15));
        assert!(deep > shallow);
        assert_eq!(depth_factor_pct(20), depth_factor_pct(99));
    }

    #[test]
    fn foil_is_strictly_worth_more() {
        for r in RARITIES {
            let seed = (0..10_000u64).find(|&s| is_foil(&card(r, s, 6))).unwrap();
            let c = card(r, seed, 6);
            // The same card without the foil factor (quirk factor kept).
            let mut plain = base_worth(r) as u64 * depth_factor_pct(6) as u64;
            if quirk_of(&c).is_some() {
                plain = plain * 125 / 100;
            }
            let plain = ((plain + 50) / 100) as u32;
            assert!(worth(&c) > plain, "{r:?}: {} <= {plain}", worth(&c));
        }
    }

    #[test]
    fn worth_tiers_are_ordered() {
        assert_eq!(worth_tier(0), "TRINKET");
        assert_eq!(worth_tier(10), "KEEPSAKE");
        assert_eq!(worth_tier(30), "TREASURE");
        assert_eq!(worth_tier(80), "RELIC");
        assert_eq!(worth_tier(200), "HEIRLOOM");
        assert_eq!(worth_tier(5000), "LEGEND");
    }

    #[test]
    fn level_thresholds_strictly_increase() {
        assert_eq!(LEVEL_THRESHOLDS[0], 0);
        for w in LEVEL_THRESHOLDS.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn level_is_monotonic_and_capped() {
        let mut prev = 0;
        for xp in 0..1000u32 {
            let l = level(xp);
            assert!(l >= prev && l <= MAX_LEVEL);
            prev = l;
        }
        assert_eq!(level(0), 0);
        assert_eq!(level(29), 0);
        assert_eq!(level(30), 1);
        assert_eq!(level(449), 4);
        assert_eq!(level(450), 5);
        assert_eq!(level(u32::MAX), 5);
    }

    #[test]
    fn xp_to_next_counts_down_and_ends() {
        assert_eq!(xp_to_next(0), Some(30));
        assert_eq!(xp_to_next(29), Some(1));
        assert_eq!(xp_to_next(30), Some(50));
        assert_eq!(xp_to_next(449), Some(1));
        assert_eq!(xp_to_next(450), None);
    }

    #[test]
    fn run_xp_rewards_depth_shards_and_lucid() {
        assert_eq!(run_xp(0, false, 0), 5);
        assert_eq!(run_xp(4, false, 0), 17);
        assert_eq!(run_xp(4, true, 0), 27);
        assert_eq!(run_xp(4, false, 2), 25);
        assert_eq!(run_xp(100, false, 0), run_xp(20, false, 0));
    }

    #[test]
    fn level_bonus_stacks_and_title_is_level_five_only() {
        assert!(level_bonus(0).is_zero());
        for l in 1..=MAX_LEVEL {
            assert!(level_bonus(l).dust_bonus_pct > level_bonus(l - 1).dust_bonus_pct);
        }
        assert_eq!(level_bonus(9), level_bonus(5));
        for l in 0..5 {
            assert_eq!(mastery_title(l), None);
        }
        assert_eq!(mastery_title(5), Some("DREAM MASTER"));
    }

    #[test]
    fn card_effect_adds_quirk_to_level() {
        let c = card(Rarity::Prophetic, 1, 2);
        let q = quirk_of(&c).unwrap();
        assert_eq!(card_effect(&c, 0), effect(q));
        assert_eq!(
            card_effect(&c, 2).dust_bonus_pct,
            level_bonus(2).dust_bonus_pct + effect(q).dust_bonus_pct
        );
        let plain = (0..).map(|s| card(Rarity::Faint, s, 2)).find(|c| quirk_of(c).is_none()).unwrap();
        assert_eq!(card_effect(&plain, 3), level_bonus(3));
    }

    #[test]
    fn xp_accumulates_per_card() {
        let mut m = Mastery::default();
        assert_eq!(m.xp_for(7), 0);
        m.add_xp(7, 10);
        m.add_xp(7, 25);
        m.add_xp(8, 1);
        assert_eq!(m.xp_for(7), 35);
        assert_eq!(m.xp_for(8), 1);
        assert_eq!(m.level_of(7), 1);
        m.add_xp(7, u32::MAX);
        assert_eq!(m.xp_for(7), u32::MAX);
    }

    #[test]
    fn echoes_convert_to_xp() {
        let mut m = Mastery::default();
        m.add_echo(DreamTheme::Garden, 3);
        m.add_echo(DreamTheme::Garden, 2);
        m.add_echo(DreamTheme::Lobby, 1);
        assert_eq!(m.echoes_of(DreamTheme::Garden), 5);
        assert_eq!(echo_xp(5), 30);
        assert_eq!(m.spend_echoes(12, DreamTheme::Garden, 2), 12);
        assert_eq!(m.echoes_of(DreamTheme::Garden), 3);
        assert_eq!(m.xp_for(12), 12);
        // Asking for more than banked spends only what exists.
        assert_eq!(m.spend_echoes(12, DreamTheme::Garden, 99), 18);
        assert_eq!(m.echoes_of(DreamTheme::Garden), 0);
        assert_eq!(m.xp_for(12), 30);
        assert_eq!(m.level_of(12), 1);
        // Nothing banked: nothing gained; other themes untouched.
        assert_eq!(m.spend_echoes(12, DreamTheme::Garden, 1), 0);
        assert_eq!(m.echoes_of(DreamTheme::Lobby), 1);
    }

    #[test]
    fn mastery_round_trips_and_parses_empty() {
        let mut m = Mastery::default();
        m.add_xp(3, 44);
        m.add_echo(DreamTheme::Lobby, 2);
        let text = ron::to_string(&m).unwrap();
        assert_eq!(ron::from_str::<Mastery>(&text).unwrap(), m);
        assert_eq!(ron::from_str::<Mastery>("()").unwrap(), Mastery::default());
        let only_xp: Mastery = ron::from_str("(xp: [(1, 5)])").unwrap();
        assert_eq!(only_xp.xp_for(1), 5);
        assert!(only_xp.echoes.is_empty());
    }

    #[test]
    fn quirk_serde_round_trips() {
        for q in ALL_QUIRKS {
            let text = ron::to_string(&q).unwrap();
            assert_eq!(ron::from_str::<Quirk>(&text).unwrap(), q);
        }
    }

    #[test]
    fn every_quirk_has_text_and_a_real_effect() {
        let mut labels = std::collections::HashSet::new();
        for q in ALL_QUIRKS {
            let l = label(q);
            assert!(!l.is_empty() && l.chars().count() <= 16, "{l}");
            assert_eq!(l, l.to_uppercase());
            assert!(labels.insert(l), "duplicate label {l}");
            assert!(!describe(q).is_empty());
            assert!(!effect(q).is_zero(), "{q:?}");
        }
    }

    #[test]
    fn provenance_reads_well() {
        let c = card(Rarity::Hazy, 1, 7);
        assert_eq!(provenance(&c), "FOUND AT DEPTH 7 // DREAM #12");
        let mut fresh = c.clone();
        fresh.number = 0;
        assert_eq!(provenance(&fresh), "FOUND AT DEPTH 7");
    }
}
