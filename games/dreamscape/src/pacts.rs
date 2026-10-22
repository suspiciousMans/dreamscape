//! PACTS: opt-in difficulty for dust. From depth 3 a pick may also offer
//! a pact card in place of one upgrade: a run-long rule that only makes
//! the run harder, in exchange for a dust bonus paid at wake. Pure logic;
//! the game wires it into the choice screen and the existing multipliers.
#![allow(dead_code)]

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Chance that a pick (depth >= MIN_DEPTH) also offers a pact.
pub const PACT_CHANCE: f64 = 0.12;
pub const MIN_DEPTH: u32 = 3;
/// Total pact dust bonus never exceeds this (additive, on the multiplier).
pub const BONUS_CAP: f32 = 1.0;
/// Dust "owed" asks for now.
pub const OWED_DUST: u32 = 10;
pub const ENEMY_FASTER: f32 = 1.08;
pub const PLAYER_SLOWER: f32 = 0.95;
pub const ALERT_WIDER: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pact {
    NoRegrets,
    ThinAir,
    LoudSteps,
    ShortFuse,
    TheLongWay,
    HeavyDreams,
    Sleepless,
    Owed,
}

pub const ALL: [Pact; 8] = [
    Pact::NoRegrets,
    Pact::ThinAir,
    Pact::LoudSteps,
    Pact::ShortFuse,
    Pact::TheLongWay,
    Pact::HeavyDreams,
    Pact::Sleepless,
    Pact::Owed,
];

/// What a set of pacts changes. Neutral = `Effects::default()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    /// No reroll and no skip.
    pub no_reroll_skip: bool,
    /// Cards removed from every later pick.
    pub fewer_cards: usize,
    pub alert_mult: f32,
    /// Extra shards each later dream... run needs to wake.
    pub extra_shards: u32,
    pub enemy_speed_mult: f32,
    pub player_speed_mult: f32,
    /// Skipping a pick pays no dust.
    pub skip_pays_nothing: bool,
    /// Dust asked right when the pact is taken (or a card is lost).
    pub owed_dust: u32,
}

impl Default for Effects {
    fn default() -> Self {
        Effects {
            no_reroll_skip: false,
            fewer_cards: 0,
            alert_mult: 1.0,
            extra_shards: 0,
            enemy_speed_mult: 1.0,
            player_speed_mult: 1.0,
            skip_pays_nothing: false,
            owed_dust: 0,
        }
    }
}

impl Effects {
    fn merge(mut self, o: Effects) -> Effects {
        self.no_reroll_skip |= o.no_reroll_skip;
        self.fewer_cards += o.fewer_cards;
        self.alert_mult *= o.alert_mult;
        self.extra_shards += o.extra_shards;
        self.enemy_speed_mult *= o.enemy_speed_mult;
        self.player_speed_mult *= o.player_speed_mult;
        self.skip_pays_nothing |= o.skip_pays_nothing;
        self.owed_dust += o.owed_dust;
        self
    }
}

/// Combined effects of every pact taken.
pub fn effects(taken: &[Pact]) -> Effects {
    taken
        .iter()
        .fold(Effects::default(), |e, p| e.merge(p.effects()))
}

impl Pact {
    pub fn name(self) -> &'static str {
        match self {
            Pact::NoRegrets => "no regrets",
            Pact::ThinAir => "thin air",
            Pact::LoudSteps => "loud steps",
            Pact::ShortFuse => "short fuse",
            Pact::TheLongWay => "the long way",
            Pact::HeavyDreams => "heavy dreams",
            Pact::Sleepless => "sleepless",
            Pact::Owed => "owed",
        }
    }

    /// One dry line: the drawback.
    pub fn desc(self) -> &'static str {
        match self {
            Pact::NoRegrets => "no reroll, no skip. ever.",
            Pact::ThinAir => "one card fewer at every pick.",
            Pact::LoudSteps => "they hear you from farther.",
            Pact::ShortFuse => "one more shard to wake.",
            Pact::TheLongWay => "the watchers walk faster.",
            Pact::HeavyDreams => "your feet are heavier.",
            Pact::Sleepless => "skipping pays nothing.",
            Pact::Owed => "pay 10 dust now, or lose a card.",
        }
    }

    /// Dust multiplier added at wake (0.15..=0.35).
    pub fn bonus(self) -> f32 {
        match self {
            Pact::NoRegrets => 0.20,
            Pact::ThinAir => 0.30,
            Pact::LoudSteps => 0.20,
            Pact::ShortFuse => 0.25,
            Pact::TheLongWay => 0.30,
            Pact::HeavyDreams => 0.20,
            Pact::Sleepless => 0.15,
            Pact::Owed => 0.35,
        }
    }

    pub fn effects(self) -> Effects {
        let n = Effects::default();
        match self {
            Pact::NoRegrets => Effects { no_reroll_skip: true, ..n },
            Pact::ThinAir => Effects { fewer_cards: 1, ..n },
            Pact::LoudSteps => Effects { alert_mult: ALERT_WIDER, ..n },
            Pact::ShortFuse => Effects { extra_shards: 1, ..n },
            Pact::TheLongWay => Effects { enemy_speed_mult: ENEMY_FASTER, ..n },
            Pact::HeavyDreams => Effects { player_speed_mult: PLAYER_SLOWER, ..n },
            Pact::Sleepless => Effects { skip_pays_nothing: true, ..n },
            Pact::Owed => Effects { owed_dust: OWED_DUST, ..n },
        }
    }
}

/// Total dust bonus, added to the multiplier (capped).
pub fn dust_bonus(taken: &[Pact]) -> f32 {
    taken.iter().map(|p| p.bonus()).sum::<f32>().min(BONUS_CAP)
}

/// Does this pick offer a pact? Own seeded stream; never a duplicate.
pub fn roll_pact(seed: u64, depth: u32, taken: &[Pact]) -> Option<Pact> {
    if depth < MIN_DEPTH {
        return None;
    }
    let left: Vec<Pact> = ALL.iter().copied().filter(|p| !taken.contains(p)).collect();
    if left.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0x9AC7_5EED);
    if !rng.gen_bool(PACT_CHANCE) {
        return None;
    }
    Some(left[rng.gen_range(0..left.len())])
}

/// What "owed" costs: dust if you have it, else a card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OwedPayment {
    Dust(u32),
    LoseCard,
}

pub fn owed_payment(have_dust: u32) -> OwedPayment {
    if have_dust >= OWED_DUST {
        OwedPayment::Dust(OWED_DUST)
    } else {
        OwedPayment::LoseCard
    }
}

/// Wake-summary lines: one per pact, then the bonus.
pub fn summary_lines(taken: &[Pact]) -> Vec<String> {
    taken
        .iter()
        .map(|p| format!("{} (+{:.2})", p.name(), p.bonus()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_hit(depth: u32, taken: &[Pact]) -> Option<(u64, Pact)> {
        (0..5000u64).find_map(|s| roll_pact(s, depth, taken).map(|p| (s, p)))
    }

    #[test]
    fn table_is_sane() {
        assert_eq!(ALL.len(), 8);
        for p in ALL {
            assert!((0.15..=0.35).contains(&p.bonus()), "{p:?}");
            assert!(!p.name().is_empty() && !p.desc().is_empty());
            assert_ne!(p.effects(), Effects::default(), "{p:?} must bite");
        }
        let mut n: Vec<_> = ALL.iter().map(|p| p.name()).collect();
        n.sort();
        n.dedup();
        assert_eq!(n.len(), 8);
    }

    #[test]
    fn effects_only_make_it_harder() {
        for p in ALL {
            let e = p.effects();
            assert!(e.alert_mult >= 1.0 && e.enemy_speed_mult >= 1.0);
            assert!(e.player_speed_mult <= 1.0);
        }
    }

    #[test]
    fn gated_by_depth() {
        for s in 0..2000 {
            for d in 0..MIN_DEPTH {
                assert_eq!(roll_pact(s, d, &[]), None);
            }
        }
        assert!(first_hit(MIN_DEPTH, &[]).is_some());
    }

    #[test]
    fn deterministic_and_rare() {
        let hits = (0..4000).filter(|&s| roll_pact(s, 5, &[]).is_some()).count();
        assert!((300..700).contains(&hits), "{hits}");
        for s in 0..200 {
            assert_eq!(roll_pact(s, 5, &[]), roll_pact(s, 5, &[]));
        }
    }

    #[test]
    fn never_duplicates() {
        for s in 0..3000 {
            for taken in [vec![Pact::Owed], vec![Pact::Owed, Pact::ThinAir, Pact::NoRegrets]] {
                if let Some(p) = roll_pact(s, 6, &taken) {
                    assert!(!taken.contains(&p));
                }
            }
        }
        assert_eq!(roll_pact(1, 6, &ALL), None);
        let all_but = &ALL[1..];
        let got: Vec<_> = (0..3000).filter_map(|s| roll_pact(s, 6, all_but)).collect();
        assert!(!got.is_empty() && got.iter().all(|&p| p == ALL[0]));
    }

    #[test]
    fn bonus_stacks_and_caps() {
        assert_eq!(dust_bonus(&[]), 0.0);
        let two = dust_bonus(&[Pact::NoRegrets, Pact::ThinAir]);
        assert!((two - 0.5).abs() < 1e-6);
        assert!((dust_bonus(&ALL) - BONUS_CAP).abs() < 1e-6);
        assert!(ALL.iter().map(|p| p.bonus()).sum::<f32>() > BONUS_CAP);
    }

    #[test]
    fn effects_combine() {
        let e = effects(&[Pact::ThinAir, Pact::LoudSteps, Pact::TheLongWay, Pact::ShortFuse]);
        assert_eq!(e.fewer_cards, 1);
        assert_eq!(e.extra_shards, 1);
        assert!((e.alert_mult - 1.25).abs() < 1e-6);
        assert!((e.enemy_speed_mult - 1.08).abs() < 1e-6);
        assert_eq!(effects(&[]), Effects::default());
    }

    #[test]
    fn owed_pays_or_costs_a_card() {
        assert_eq!(owed_payment(10), OwedPayment::Dust(10));
        assert_eq!(owed_payment(9), OwedPayment::LoseCard);
    }

    #[test]
    fn summary_lists_pacts() {
        let l = summary_lines(&[Pact::Owed]);
        assert_eq!(l, vec!["owed (+0.35)".to_string()]);
    }
}
