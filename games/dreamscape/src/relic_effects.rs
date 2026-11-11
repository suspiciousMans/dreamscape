//! Relic buff numbers (R4): a pure `effect_for(rarity, theme, seed)`.
//!
//! Cracked: one small lever. Worn: one medium. Pristine: two. Mythic: three
//! plus a nerve or shard bonus. The theme biases which levers come up.

use crate::dream::DreamTheme;
use crate::relic_types::{RelicEffect, RelicRarity};
use crate::worth::mix;

pub const MAX_DUST_PCT: u32 = 15;
pub const MAX_SPEED_PCT: u32 = 8;
pub const MAX_CALM_PCT: u32 = 10;
#[cfg(test)]
pub const MAX_MEMORY: f32 = 0.06;
pub const MAX_REROLLS: u32 = 2;
pub const MAX_SHARDS: u32 = 2;
pub const MAX_GRACE: u32 = 2;
pub const MAX_NERVE: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lever {
    Dust,
    Speed,
    Calm,
    Memory,
    Rerolls,
    Grace,
}

const LEVERS: [Lever; 6] = [
    Lever::Dust,
    Lever::Speed,
    Lever::Calm,
    Lever::Memory,
    Lever::Rerolls,
    Lever::Grace,
];

/// The levers a theme leans toward.
fn favoured(theme: DreamTheme) -> [Lever; 2] {
    use DreamTheme::*;
    match theme {
        Garden | MyceliumGrove | Elfworks => [Lever::Memory, Lever::Calm],
        MeltingClockworks | SkyStairs => [Lever::Speed, Lever::Grace],
        DrownedLibrary | MirrorHall | SynesthesiaHall => [Lever::Memory, Lever::Rerolls],
        NightmareFactory | CursedForest => [Lever::Dust, Lever::Grace],
        VoidPlatforms | TheTunnel | JellyfishSky => [Lever::Speed, Lever::Calm],
        _ => [Lever::Dust, Lever::Rerolls],
    }
}

/// Share of the cap a rarity pays, in percent.
fn strength_pct(r: RelicRarity) -> u32 {
    match r {
        RelicRarity::Cracked => 40,
        RelicRarity::Worn => 70,
        RelicRarity::Pristine => 85,
        RelicRarity::Mythic => 100,
    }
}

/// Number of main levers a rarity carries.
fn lever_count(r: RelicRarity) -> usize {
    match r {
        RelicRarity::Cracked | RelicRarity::Worn => 1,
        RelicRarity::Pristine => 2,
        RelicRarity::Mythic => 3,
    }
}

/// `cap * pct / 100`, rounded, at least 1.
fn scaled(cap: u32, pct: u32) -> u32 {
    ((cap * pct + 50) / 100).clamp(1, cap)
}

/// Picks `n` distinct levers, favoured ones weighing 3 against 1.
fn pick(theme: DreamTheme, seed: u64, n: usize) -> Vec<Lever> {
    let fav = favoured(theme);
    let mut pool: Vec<Lever> = LEVERS.to_vec();
    let mut out = Vec::new();
    for i in 0..n {
        let w = |l: &Lever| if fav.contains(l) { 3 } else { 1 };
        let total: u64 = pool.iter().map(w).sum::<u32>() as u64;
        let mut roll = mix(seed, 0x4E11_0000 + i as u64) % total;
        let mut at = 0;
        for (j, l) in pool.iter().enumerate() {
            let wt = w(l) as u64;
            if roll < wt {
                at = j;
                break;
            }
            roll -= wt;
        }
        out.push(pool.remove(at));
    }
    out
}

pub fn effect_for(rarity: RelicRarity, theme: DreamTheme, seed: u64) -> RelicEffect {
    let pct = strength_pct(rarity);
    let mut e = RelicEffect::default();
    for l in pick(theme, seed, lever_count(rarity)) {
        match l {
            Lever::Dust => e.dust_bonus_pct = scaled(MAX_DUST_PCT, pct),
            Lever::Speed => e.speed_pct = scaled(MAX_SPEED_PCT, pct),
            Lever::Calm => e.calm_pct = scaled(MAX_CALM_PCT, pct),
            Lever::Memory => e.memory_bonus = scaled(60, pct) as f32 / 1000.0,
            Lever::Rerolls => e.free_rerolls = scaled(MAX_REROLLS, pct),
            Lever::Grace => e.grace_bonus = scaled(MAX_GRACE, pct),
        }
    }
    if rarity == RelicRarity::Mythic {
        if mix(seed, 0x4E11_FFFF) % 2 == 0 {
            e.max_nerve = MAX_NERVE;
        } else {
            e.start_shards = scaled(MAX_SHARDS, pct);
        }
    }
    e
}

/// How many levers are non-zero.
#[cfg(test)]
fn lever_total(e: &RelicEffect) -> usize {
    [
        e.dust_bonus_pct > 0,
        e.memory_bonus > 0.0,
        e.speed_pct > 0,
        e.grace_bonus > 0,
        e.calm_pct > 0,
        e.free_rerolls > 0,
        e.start_shards > 0,
        e.max_nerve > 0,
    ]
    .iter()
    .filter(|b| **b)
    .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    const RARITIES: [RelicRarity; 4] = [
        RelicRarity::Cracked,
        RelicRarity::Worn,
        RelicRarity::Pristine,
        RelicRarity::Mythic,
    ];

    #[test]
    fn deterministic() {
        for r in RARITIES {
            assert_eq!(
                effect_for(r, DreamTheme::Garden, 7),
                effect_for(r, DreamTheme::Garden, 7)
            );
        }
    }

    #[test]
    fn caps_hold_over_10k_seeds() {
        for s in 0..10_000u64 {
            let t = ALL_THEMES[(s % ALL_THEMES.len() as u64) as usize];
            for r in RARITIES {
                let e = effect_for(r, t, s.wrapping_mul(0x9E37_79B9));
                assert!(e.dust_bonus_pct <= MAX_DUST_PCT);
                assert!(e.speed_pct <= MAX_SPEED_PCT);
                assert!(e.calm_pct <= MAX_CALM_PCT);
                assert!(e.memory_bonus <= MAX_MEMORY + 1e-6);
                assert!(e.free_rerolls <= MAX_REROLLS);
                assert!(e.start_shards <= MAX_SHARDS);
                assert!(e.grace_bonus <= MAX_GRACE);
                assert!(e.max_nerve <= MAX_NERVE);
            }
        }
    }

    #[test]
    fn rarity_sets_the_lever_count() {
        for s in 0..500u64 {
            for t in ALL_THEMES {
                assert_eq!(lever_total(&effect_for(RelicRarity::Cracked, t, s)), 1);
                assert_eq!(lever_total(&effect_for(RelicRarity::Worn, t, s)), 1);
                assert_eq!(lever_total(&effect_for(RelicRarity::Pristine, t, s)), 2);
                let m = effect_for(RelicRarity::Mythic, t, s);
                assert_eq!(lever_total(&m), 4);
                assert!(m.max_nerve > 0 || m.start_shards > 0);
            }
        }
    }

    #[test]
    fn theme_biases_levers() {
        let hits = |t: DreamTheme| {
            (0..1000u64)
                .filter(|s| effect_for(RelicRarity::Cracked, t, *s).memory_bonus > 0.0)
                .count()
        };
        assert!(hits(DreamTheme::Garden) > hits(DreamTheme::MeltingClockworks) * 2);
    }
}
