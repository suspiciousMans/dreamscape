//! Dream tickets: a second, gentler currency. Tickets are earned by playing
//! (a clean dream, a new achievement, a weekly challenge) and SCRATCHED in
//! the third tab of the Lucid Store for a prize from a table that is shown
//! in full, with exact odds. Never real money; tickets cannot be bought.
//!
//! - **Pity**: `TICKET_PITY` scratches in a row of only the smallest prize,
//!   and the next one cannot be the smallest prize (the table shown says so).
//! - **No near misses**: a prize is decided whole, from one seeded roll.
//! - **Decided once**: the roll is seeded from the save and the saved scratch
//!   count, and the result is written with the stash, so reloading cannot
//!   re-roll it.
//!
//! Pure: no clock, no global RNG (the day number is injected).
use crate::cards::Booklet;
use crate::lottery;
use crate::retention::{press_pull, Pulled};
use crate::store::{Item, State, Stash, CATALOG};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Most tickets a clean-dream bonus can give in one run.
pub const TICKETS_PER_RUN_CAP: u32 = 5;
/// Tickets for a new achievement.
pub const TICKETS_PER_ACHIEVEMENT: u32 = 1;
/// Tickets for a weekly challenge.
pub const TICKETS_PER_WEEKLY: u32 = 2;
/// Most tickets that can be held at once.
pub const TICKET_BANK_CAP: u32 = 99;
/// Smallest-prize scratches in a row before the next is better.
pub const TICKET_PITY: u32 = 4;

/// Dust prizes, smallest first.
pub const DUST_PRIZES: [u32; 4] = [20, 40, 80, 150];

/// A kind of prize on the table (what the odds screen lists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dust(u32),
    /// A lottery pull (a free card, normal odds, normal pity).
    Pull,
    /// A cosmetic from the store you do not own.
    Cosmetic,
    /// A hint toward a chosen card: fills a spark pick.
    Hint,
}

impl Kind {
    pub fn label(self) -> String {
        match self {
            Kind::Dust(n) => format!("{n} DREAM DUST"),
            Kind::Pull => "A LOTTERY PULL".to_string(),
            Kind::Cosmetic => "A STORE COSMETIC".to_string(),
            Kind::Hint => "A DREAM OF YOUR CHOOSING".to_string(),
        }
    }
}

/// The table, with whole-number weights out of `TOTAL_WEIGHT`.
pub const TABLE: [(Kind, u32); 7] = [
    (Kind::Dust(20), 40),
    (Kind::Dust(40), 24),
    (Kind::Dust(80), 14),
    (Kind::Dust(150), 6),
    (Kind::Pull, 8),
    (Kind::Cosmetic, 6),
    (Kind::Hint, 2),
];
pub const TOTAL_WEIGHT: u32 = 100;

/// What a scratch revealed.
#[derive(Clone, Debug, PartialEq)]
pub enum Prize {
    Dust(u32),
    Pull,
    Cosmetic(Item),
    Hint,
}

/// Everything a scratch produced (the card, for a pull prize).
#[derive(Clone, Debug)]
pub struct Scratched {
    pub prize: Prize,
    pub pulled: Option<Pulled>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchError {
    NoTickets,
}

/// Cosmetics from the store that `stash` does not own (never perks).
pub fn unowned_cosmetics(stash: &Stash) -> Vec<Item> {
    CATALOG
        .iter()
        .copied()
        .filter(|i| !matches!(i, Item::Perk(_)))
        .filter(|&i| matches!(stash.state(i), State::Buy(p) if p > 0))
        .collect()
}

/// Weights actually used for the next scratch: pity removes the smallest
/// prize; with no cosmetic left to give, its share is paid as the second
/// smallest dust prize instead. Always sums to `TOTAL_WEIGHT`.
fn weights(dry: u32, cosmetic_left: bool) -> [(Kind, u32); 7] {
    let mut w = TABLE;
    if !cosmetic_left {
        w[1].1 += w[5].1;
        w[5].1 = 0;
    }
    if dry >= TICKET_PITY {
        w[1].1 += w[0].1;
        w[0].1 = 0;
    }
    w
}

/// Exact odds of each prize for the next scratch (they sum to 1; kinds with
/// no chance are left out).
pub fn odds(dry: u32, cosmetic_left: bool) -> Vec<(Kind, f32)> {
    weights(dry, cosmetic_left)
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|&(k, n)| (k, n as f32 / TOTAL_WEIGHT as f32))
        .collect()
}

/// Scratches left until the smallest prize is taken off the table
/// (0 = it already is).
pub fn scratches_to_pity(dry: u32) -> u32 {
    TICKET_PITY.saturating_sub(dry)
}

/// Roll the kind of prize. One draw from the seed.
pub fn roll_kind(seed: u64, dry: u32, cosmetic_left: bool) -> Kind {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut roll = rng.gen_range(0..TOTAL_WEIGHT);
    for (k, n) in weights(dry, cosmetic_left) {
        if roll < n {
            return k;
        }
        roll -= n;
    }
    Kind::Dust(DUST_PRIZES[1])
}

/// This save's own seed stream for ticket number `scratches` (the count
/// already saved, so a reload sees the same roll).
pub fn ticket_seed(save_seed: u64, scratches: u32) -> u64 {
    save_seed ^ (scratches as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x71C4_E7B1_77D5_0011
}

/// Add tickets to the stash (capped by `TICKET_BANK_CAP`); returns how many
/// were actually added.
pub fn give(stash: &mut Stash, n: u32) -> u32 {
    let room = TICKET_BANK_CAP.saturating_sub(stash.tickets);
    let got = n.min(room);
    stash.tickets += got;
    got
}

/// A clean dream (no nerve lost): +1 ticket, up to `TICKETS_PER_RUN_CAP`
/// this run. `earned_this_run` is the run's counter. Returns tickets given.
pub fn for_clean_dream(stash: &mut Stash, earned_this_run: &mut u32) -> u32 {
    if *earned_this_run >= TICKETS_PER_RUN_CAP {
        return 0;
    }
    *earned_this_run += 1;
    give(stash, 1)
}

/// Scratch one ticket. `today` picks the featured theme of a pull prize.
/// The whole outcome is applied to the booklet here; the caller saves it
/// (and rolls back from a clone if saving fails) before playing the reveal.
pub fn scratch(b: &mut Booklet, save_seed: u64, today: u64) -> Result<Scratched, ScratchError> {
    if b.stash.tickets == 0 {
        return Err(ScratchError::NoTickets);
    }
    let seed = ticket_seed(save_seed, b.stash.ticket_scratches);
    let unowned = unowned_cosmetics(&b.stash);
    let kind = roll_kind(seed, b.stash.ticket_dry, !unowned.is_empty());
    b.stash.tickets -= 1;
    b.stash.ticket_scratches = b.stash.ticket_scratches.saturating_add(1);
    b.stash.ticket_dry = if kind == Kind::Dust(DUST_PRIZES[0]) {
        b.stash.ticket_dry.saturating_add(1)
    } else {
        0
    };
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5CA7_C4ED);
    Ok(match kind {
        Kind::Dust(n) => {
            b.stash.earn(n);
            Scratched { prize: Prize::Dust(n), pulled: None }
        }
        Kind::Pull => {
            let pull = lottery::pull(&mut b.lottery, seed, Some(lottery::featured_theme(today)));
            let pulled = press_pull(b, pull, seed);
            Scratched { prize: Prize::Pull, pulled: Some(pulled) }
        }
        Kind::Cosmetic => {
            let item = unowned[rng.gen_range(0..unowned.len())];
            b.stash.owned.push(item);
            Scratched { prize: Prize::Cosmetic(item), pulled: None }
        }
        Kind::Hint => {
            b.lottery.sparks = b.lottery.sparks.max(lottery::SPARKS_FOR_PICK);
            Scratched { prize: Prize::Hint, pulled: None }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn booklet(tickets: u32) -> Booklet {
        let mut b = Booklet::default();
        b.stash.tickets = tickets;
        b
    }

    #[test]
    fn the_table_sums_to_one_in_every_state() {
        assert_eq!(TABLE.iter().map(|t| t.1).sum::<u32>(), TOTAL_WEIGHT);
        for dry in [0, TICKET_PITY - 1, TICKET_PITY, 40] {
            for cosmetic in [true, false] {
                let s: f32 = odds(dry, cosmetic).iter().map(|o| o.1).sum();
                assert!((s - 1.0).abs() < 1e-5, "{dry} {cosmetic}: {s}");
            }
        }
    }

    #[test]
    fn dust_is_most_of_the_table() {
        let dust: f32 = odds(0, true)
            .iter()
            .filter(|(k, _)| matches!(k, Kind::Dust(_)))
            .map(|o| o.1)
            .sum();
        assert!(dust > 0.5, "{dust}");
    }

    #[test]
    fn pity_removes_the_smallest_prize_after_the_dry_streak() {
        assert!(odds(TICKET_PITY - 1, true).iter().any(|o| o.0 == Kind::Dust(20)));
        assert!(!odds(TICKET_PITY, true).iter().any(|o| o.0 == Kind::Dust(20)));
        for seed in 0..300 {
            assert_ne!(roll_kind(seed, TICKET_PITY, true), Kind::Dust(20));
        }
        assert_eq!(scratches_to_pity(1), TICKET_PITY - 1);
        assert_eq!(scratches_to_pity(TICKET_PITY + 3), 0);
    }

    #[test]
    fn scratching_counts_dry_scratches_and_resets_on_better() {
        let mut b = booklet(200);
        let mut dry_seen = 0;
        for _ in 0..200 {
            let before = b.stash.ticket_dry;
            let got = scratch(&mut b, 5, 100).unwrap();
            if got.prize == Prize::Dust(20) {
                assert_eq!(b.stash.ticket_dry, before + 1);
                dry_seen += 1;
            } else {
                assert_eq!(b.stash.ticket_dry, 0);
            }
            assert!(b.stash.ticket_dry <= TICKET_PITY, "never longer than the pity");
        }
        assert!(dry_seen > 0);
        assert_eq!(b.stash.ticket_scratches, 200);
        assert_eq!(b.stash.tickets, 0);
    }

    #[test]
    fn no_tickets_no_scratch_and_nothing_changes() {
        let mut b = booklet(0);
        let before = b.clone();
        assert_eq!(scratch(&mut b, 1, 1).unwrap_err(), ScratchError::NoTickets);
        assert_eq!(b.stash, before.stash);
    }

    #[test]
    fn a_cosmetic_is_never_a_duplicate_and_falls_back_to_dust() {
        let mut b = booklet(600);
        let total = unowned_cosmetics(&b.stash).len();
        assert!(total > 0);
        let mut seen = std::collections::HashSet::new();
        let mut cosmetics = 0;
        for _ in 0..600 {
            let got = scratch(&mut b, 9, 3).unwrap();
            if let Prize::Cosmetic(i) = got.prize {
                assert!(seen.insert(i), "{i:?} given twice");
                cosmetics += 1;
            }
        }
        assert!(cosmetics > 0);
        // Once all are owned the table stops offering them, shown honestly.
        let all: Vec<Item> = unowned_cosmetics(&b.stash);
        b.stash.owned.extend(all);
        assert!(unowned_cosmetics(&b.stash).is_empty());
        assert!(!odds(0, false).iter().any(|o| o.0 == Kind::Cosmetic));
        for seed in 0..300 {
            assert_ne!(roll_kind(seed, 0, false), Kind::Cosmetic);
        }
    }

    #[test]
    fn the_outcome_is_decided_by_the_saved_count_so_a_reload_cannot_reroll() {
        let a = scratch(&mut booklet(3), 77, 5).unwrap();
        let b = scratch(&mut booklet(3), 77, 5).unwrap();
        assert_eq!(a.prize, b.prize);
        assert_ne!(ticket_seed(77, 0), ticket_seed(77, 1));
    }

    #[test]
    fn every_prize_kind_applies_its_effect() {
        let mut b = booklet(1);
        b.stash.earn(0);
        // dust
        let dust0 = b.stash.dust;
        let mut hit = [false; 4];
        let mut seed = 0u64;
        while !hit.iter().all(|&h| h) && seed < 5000 {
            seed += 1;
            let mut t = booklet(1);
            let got = scratch(&mut t, seed, 9).unwrap();
            match got.prize {
                Prize::Dust(n) => {
                    assert_eq!(t.stash.dust, n);
                    hit[0] = true;
                }
                Prize::Pull => {
                    let p = got.pulled.unwrap();
                    assert!(t.card(p.card.number).is_some());
                    assert_eq!(t.lottery.total_pulls, 1);
                    hit[1] = true;
                }
                Prize::Cosmetic(i) => {
                    assert!(t.stash.owned.contains(&i));
                    hit[2] = true;
                }
                Prize::Hint => {
                    assert!(lottery::spark_pick_ready(&t.lottery));
                    hit[3] = true;
                }
            }
        }
        assert_eq!(dust0, 0);
        assert!(hit.iter().all(|&h| h), "every kind showed up: {hit:?}");
    }

    #[test]
    fn earning_is_capped_per_run_and_in_the_bank() {
        let mut s = Stash::default();
        let mut earned = 0;
        for _ in 0..TICKETS_PER_RUN_CAP + 3 {
            for_clean_dream(&mut s, &mut earned);
        }
        assert_eq!(s.tickets, TICKETS_PER_RUN_CAP);
        let mut s = Stash::default();
        assert_eq!(give(&mut s, 500), TICKET_BANK_CAP);
        assert_eq!(give(&mut s, 1), 0);
    }

    #[test]
    fn old_saves_load_with_no_tickets() {
        let s: Stash = ron::from_str("(dust: 12)").unwrap();
        assert_eq!((s.tickets, s.ticket_scratches, s.ticket_dry), (0, 0, 0));
        assert_eq!(s.dust, 12);
        let mut t = Stash::default();
        t.tickets = 4;
        t.ticket_dry = 2;
        let again: Stash = ron::from_str(&ron::to_string(&t).unwrap()).unwrap();
        assert_eq!(again, t);
    }
}
