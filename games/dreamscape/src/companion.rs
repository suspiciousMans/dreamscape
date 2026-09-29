//! A collected dreamer can ride along in a run: they bring a boon (two
//! stacks of a passive) and their weight as a burden on one stat. Table
//! approved in `plans/2026-09-28-dreamscape-rest-of-project-plan.md`.

use crate::lore::Weight;
use crate::upgrades::Upgrade;

/// The passive a companion carrying `w` brings, twice.
pub fn boon(w: Weight) -> Upgrade {
    match w {
        Weight::Grief => Upgrade::DreamAnchor,
        Weight::Burnout => Upgrade::LongBreath,
        Weight::Anxiety => Upgrade::CalmMind,
        Weight::Loneliness => Upgrade::LuckyMemory,
        Weight::Guilt => Upgrade::ShardSense,
        Weight::Insomnia => Upgrade::HeavyAir,
        Weight::Heartbreak => Upgrade::SwiftFeet,
        Weight::Pressure => Upgrade::DustHoarder,
    }
}

/// Stacks of the boon.
pub const BOON_STACKS: u32 = 2;

/// The stat a burden weighs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Sight,
    Speed,
    Alert,
    Choices,
    Grace,
    EnemySpeed,
    Jump,
    ShardBonus,
}

pub fn burden(w: Weight) -> Stat {
    match w {
        Weight::Grief => Stat::Sight,
        Weight::Burnout => Stat::Speed,
        Weight::Anxiety => Stat::Alert,
        Weight::Loneliness => Stat::Choices,
        Weight::Guilt => Stat::Grace,
        Weight::Insomnia => Stat::EnemySpeed,
        Weight::Heartbreak => Stat::Jump,
        Weight::Pressure => Stat::ShardBonus,
    }
}

/// Multiplier (or, for choices and shard bonus, the amount taken away).
pub fn burden_factor(s: Stat) -> f32 {
    match s {
        Stat::Sight => 0.85,
        Stat::Speed => 0.88,
        Stat::Alert => 1.25,
        Stat::Choices => 1.0,
        Stat::Grace => 0.7,
        Stat::EnemySpeed => 1.12,
        Stat::Jump => 0.88,
        Stat::ShardBonus => 0.08,
    }
}

/// For the loadout screen: "+ CALM MIND x2 · enemies notice you sooner".
pub fn describe(w: Weight) -> String {
    let what = match burden(w) {
        Stat::Sight => "you see less",
        Stat::Speed => "you move slower",
        Stat::Alert => "enemies notice you sooner",
        Stat::Choices => "one fewer card at each pick",
        Stat::Grace => "less safety after a respawn",
        Stat::EnemySpeed => "enemies move faster",
        Stat::Jump => "you jump lower",
        Stat::ShardBonus => "shards turn up less",
    };
    format!("+ {} x{BOON_STACKS} · {what}", boon(w).info().name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::ALL_WEIGHTS;
    use crate::upgrades::RunUpgrades;

    #[test]
    fn every_weight_has_a_boon_and_a_different_burden() {
        let mut stats = Vec::new();
        for w in ALL_WEIGHTS {
            assert!(crate::upgrades::PASSIVES.contains(&boon(w)), "{w:?}");
            assert!(
                boon(w).info().max >= BOON_STACKS,
                "{w:?}: boon fits its cap"
            );
            assert!(!stats.contains(&burden(w)), "{w:?}");
            stats.push(burden(w));
            assert!(!describe(w).is_empty());
        }
    }

    #[test]
    fn each_burden_hits_the_stat_it_names() {
        let base = RunUpgrades::default();
        for w in ALL_WEIGHTS {
            let r = RunUpgrades {
                burden: Some(w),
                ..Default::default()
            };
            let changed = |s: Stat| match s {
                Stat::Sight => r.sight() < base.sight(),
                Stat::Speed => r.speed() < base.speed(),
                Stat::Alert => r.alert() > base.alert(),
                Stat::Choices => r.choice_count() < base.choice_count(),
                Stat::Grace => r.grace() < base.grace(),
                Stat::EnemySpeed => r.enemy_speed() > base.enemy_speed(),
                Stat::Jump => r.jump() < base.jump(),
                Stat::ShardBonus => r.shard_bonus() < base.shard_bonus(),
            };
            let all = [
                Stat::Sight,
                Stat::Speed,
                Stat::Alert,
                Stat::Choices,
                Stat::Grace,
                Stat::EnemySpeed,
                Stat::Jump,
                Stat::ShardBonus,
            ];
            for s in all {
                assert_eq!(changed(s), s == burden(w), "{w:?} on {s:?}");
            }
        }
    }

    #[test]
    fn a_companion_brings_two_stacks_of_its_boon() {
        let r = RunUpgrades::default().with_companion(Weight::Anxiety);
        assert_eq!(r.count(Upgrade::CalmMind), 2);
        assert_eq!(r.burden, Some(Weight::Anxiety));
    }
}
