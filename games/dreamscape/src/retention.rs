//! The glue between the booklet and the retention systems (`lottery`,
//! `worth`, `streaks`). Everything here is a pure function of a `Booklet`
//! (and plain numbers), so `main.rs` only has to call it and draw the answer.

use crate::cards::{Booklet, Card, Recalled};
use crate::dream::DreamTheme;
use crate::lottery::{self, Pull};
use crate::streaks::{self, ProgressInputs, StreakEvent};
use crate::worth::{self, QuirkEffect};

/// A card's bonuses summed over a set of cards (the run's loadout), with each
/// card's mastery level folded in.
pub fn card_effects(b: &Booklet, cards: &[Card]) -> QuirkEffect {
    cards.iter().fold(QuirkEffect::default(), |sum, c| {
        sum.plus(worth::card_effect(c, b.mastery.level_of(c.number)))
    })
}

/// What opening the app today earned.
#[derive(Clone, Debug, PartialEq)]
pub struct Visit {
    pub event: StreakEvent,
    pub streak: u32,
    pub dust: u32,
    pub milestone: Option<&'static str>,
}

/// Counts today's visit toward the streak and pays the dust for it.
/// `None` when today was already counted.
pub fn visit_today(b: &mut Booklet, today: u64) -> Option<Visit> {
    let event = b.streak.visit(today);
    if event == StreakEvent::Same {
        return None;
    }
    let streak = b.streak.streak;
    let dust = streaks::dust_reward(streak);
    b.stash.earn(dust);
    Some(Visit {
        event,
        streak,
        dust,
        milestone: streaks::milestone(streak),
    })
}

/// What a finished run did to the cards that went along.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunReport {
    /// (card number, new level) for every card that levelled up.
    pub level_ups: Vec<(u32, u8)>,
    pub xp_each: u32,
    /// Faded dreams that left an echo.
    pub echoes: u32,
}

/// Settles mastery after a run: every card in the loadout earns xp, every
/// faded dream leaves an echo of its theme, and banked echoes are poured
/// into the lowest-level card of their theme (so a fade is never dead).
pub fn settle_run(
    b: &mut Booklet,
    used: &[u32],
    pack: &[Recalled],
    depth: u32,
    lucid_wake: bool,
    shards: u32,
) -> RunReport {
    let mut report = RunReport::default();
    let before: Vec<(u32, u8)> = b
        .cards()
        .map(|c| (c.number, b.mastery.level_of(c.number)))
        .collect();

    report.xp_each = worth::run_xp(depth, lucid_wake, shards);
    let owned: Vec<u32> = used.iter().copied().filter(|&n| b.card(n).is_some()).collect();
    for n in owned {
        b.mastery.add_xp(n, report.xp_each);
    }
    for r in pack.iter().filter(|r| !r.remembered) {
        b.mastery.add_echo(r.card.theme, 1);
        report.echoes += 1;
    }
    feed_echoes(b);

    for (n, was) in before {
        let now = b.mastery.level_of(n);
        if now > was {
            report.level_ups.push((n, now));
        }
    }
    report
}

/// Pours every banked echo into the lowest-level owned card of its theme.
/// Echoes of a theme with no card yet stay banked.
pub fn feed_echoes(b: &mut Booklet) {
    let themes: Vec<DreamTheme> = b.mastery.echoes.iter().map(|&(t, _)| t).collect();
    for theme in themes {
        let n = b.mastery.echoes_of(theme);
        if n == 0 {
            continue;
        }
        let target = b
            .cards()
            .filter(|c| c.theme == theme)
            .min_by_key(|c| (b.mastery.level_of(c.number), c.number))
            .map(|c| c.number);
        if let Some(number) = target {
            b.mastery.spend_echoes(number, theme, n);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PullError {
    TooPoor { need: u32 },
}

/// A pull's outcome with the card it put in the booklet.
#[derive(Clone, Debug)]
pub struct Pulled {
    pub pull: Pull,
    pub card: Card,
    pub foil: bool,
    /// The booklet already held a card with this name (rarity was bumped).
    pub recurring: bool,
}

/// Spends `PULL_COST` dust on one pull and presses the card.
pub fn pull_card(b: &mut Booklet, seed: u64, today: u64) -> Result<Pulled, PullError> {
    if b.stash.dust < lottery::PULL_COST {
        return Err(PullError::TooPoor {
            need: lottery::PULL_COST - b.stash.dust,
        });
    }
    b.stash.dust -= lottery::PULL_COST;
    let pull = lottery::pull(
        &mut b.lottery,
        seed,
        Some(lottery::featured_theme(today)),
    );
    Ok(press_pull(b, pull, seed))
}

/// Spends sparks to pick a theme outright (always Vivid or better).
pub fn spark_pick(b: &mut Booklet, theme: DreamTheme, seed: u64) -> Option<Pulled> {
    let pull = lottery::redeem_spark_pick(&mut b.lottery, theme)?;
    Some(press_pull(b, pull, seed))
}

fn press_pull(b: &mut Booklet, pull: Pull, seed: u64) -> Pulled {
    let card = lottery::card_for_pull(&pull, seed);
    let card = b.add_pulled(seed, card);
    feed_echoes(b);
    Pulled {
        foil: worth::is_foil(&card),
        recurring: card.recurring,
        pull,
        card,
    }
}

/// The numbers the goal gradient is made of, read off the booklet.
pub fn progress_inputs(b: &Booklet, loadout: &[u32], best_depth: u32) -> ProgressInputs {
    let cheapest = crate::store::CATALOG
        .iter()
        .filter_map(|&i| match b.stash.state(i) {
            crate::store::State::Buy(p) | crate::store::State::Stocked { price: p, .. } => Some(p),
            _ => None,
        })
        .filter(|&p| p > 0)
        .min();
    let mastery_to_next = loadout
        .iter()
        .filter_map(|&n| worth::xp_to_next(b.mastery.xp_for(n)))
        .min();
    ProgressInputs {
        dust: b.stash.dust,
        cards: b.card_count() as u32,
        best_depth,
        ascension: b.codex.ascension_unlocked.min(u8::MAX as u32) as u8,
        dreams_until_perk: None,
        mastery_to_next,
        daily_streak: b.streak.streak,
        lottery_pulls_to_pity: lottery::HARD_PITY_LUCID.saturating_sub(b.lottery.since_lucid),
        sparks: b.lottery.sparks,
        spark_goal: lottery::SPARKS_FOR_PICK,
        store_cheapest_unowned: cheapest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, DreamRecord};
    use crate::dream::{dream_name, portal_surface, whisper};

    fn record(theme: DreamTheme, seed: u64) -> DreamRecord {
        DreamRecord {
            theme,
            seed,
            depth: 3,
            name: dream_name(theme, seed),
            whisper: whisper(theme, seed),
            strangeness: 0.4,
            enemies: 1,
            shard_taken: true,
            art: portal_surface(theme, seed),
            blend: None,
        }
    }

    fn booklet_with(themes: &[DreamTheme]) -> Booklet {
        let mut b = Booklet::default();
        let recs: Vec<_> = themes
            .iter()
            .enumerate()
            .map(|(i, &t)| record(t, 100 + i as u64))
            .collect();
        b.add_run(1, &recs);
        b
    }

    #[test]
    fn the_first_visit_pays_and_the_second_is_silent() {
        let mut b = Booklet::default();
        let v = visit_today(&mut b, 100).expect("first visit counts");
        assert_eq!(v.streak, 1);
        assert_eq!(b.stash.dust, v.dust);
        assert!(visit_today(&mut b, 100).is_none(), "same day pays nothing");
        let v2 = visit_today(&mut b, 101).unwrap();
        assert_eq!(v2.streak, 2);
    }

    #[test]
    fn pulling_costs_dust_and_adds_a_card() {
        let mut b = Booklet::default();
        assert_eq!(
            pull_card(&mut b, 1, 5).unwrap_err(),
            PullError::TooPoor { need: lottery::PULL_COST }
        );
        b.stash.earn(lottery::PULL_COST + 7);
        let p = pull_card(&mut b, 42, 5).unwrap();
        assert_eq!(b.stash.dust, 7);
        assert_eq!(b.card_count(), 1);
        assert_eq!(b.lottery.total_pulls, 1);
        assert_eq!(b.card(p.card.number).unwrap().seed, p.card.seed);
        assert_eq!(p.foil, p.pull.foil, "the announced foil is the real foil");
    }

    #[test]
    fn a_hundred_pulls_hit_pity_and_never_corrupt_the_booklet() {
        let mut b = Booklet::default();
        b.stash.earn(lottery::PULL_COST * 100);
        let mut best = 0;
        for i in 0..100u64 {
            let p = pull_card(&mut b, i * 7919 + 3, 9).unwrap();
            best = best.max(p.pull.rarity as u8);
        }
        assert_eq!(b.card_count(), 100);
        assert!(best >= crate::cards::Rarity::Lucid as u8, "pity guarantees a Lucid in 30");
        let numbers: std::collections::HashSet<_> = b.cards().map(|c| c.number).collect();
        assert_eq!(numbers.len(), 100, "card numbers stay unique");
    }

    #[test]
    fn spark_pick_needs_sparks() {
        let mut b = Booklet::default();
        assert!(spark_pick(&mut b, DreamTheme::Garden, 1).is_none());
        b.lottery.sparks = lottery::SPARKS_FOR_PICK;
        let p = spark_pick(&mut b, DreamTheme::Garden, 1).unwrap();
        assert_eq!(p.card.theme, DreamTheme::Garden);
        assert!(p.card.rarity >= crate::cards::Rarity::Vivid);
        assert_eq!(b.lottery.sparks, 0);
    }

    #[test]
    fn a_run_levels_the_cards_that_went_along() {
        let mut b = booklet_with(&[DreamTheme::Garden, DreamTheme::Lobby]);
        let used = b.cards().next().unwrap().number;
        let other = b.cards().nth(1).unwrap().number;
        let r = settle_run(&mut b, &[used], &[], 10, true, 5);
        assert!(r.xp_each > 0);
        assert_eq!(b.mastery.xp_for(used), r.xp_each);
        assert_eq!(b.mastery.xp_for(other), 0, "left behind earns nothing");
    }

    #[test]
    fn faded_dreams_leave_echoes_that_feed_a_card_of_the_same_theme() {
        let mut b = booklet_with(&[DreamTheme::Garden]);
        let garden = b.cards().next().unwrap().number;
        let faded = Recalled {
            card: card_from(&record(DreamTheme::Garden, 999)),
            remembered: false,
        };
        let r = settle_run(&mut b, &[], &[faded.clone(), faded], 4, false, 0);
        assert_eq!(r.echoes, 2);
        assert_eq!(b.mastery.echoes_of(DreamTheme::Garden), 0, "spent");
        assert_eq!(b.mastery.xp_for(garden), worth::echo_xp(2));
    }

    #[test]
    fn echoes_wait_for_a_card_of_their_theme() {
        let mut b = Booklet::default();
        let faded = Recalled {
            card: card_from(&record(DreamTheme::Garden, 5)),
            remembered: false,
        };
        settle_run(&mut b, &[], &[faded], 2, false, 0);
        assert_eq!(b.mastery.echoes_of(DreamTheme::Garden), 1, "banked");
    }

    #[test]
    fn level_ups_are_reported() {
        let mut b = booklet_with(&[DreamTheme::Garden]);
        let n = b.cards().next().unwrap().number;
        b.mastery.add_xp(n, worth::LEVEL_THRESHOLDS[1] - 1);
        let r = settle_run(&mut b, &[n], &[], 1, false, 0);
        assert_eq!(r.level_ups, vec![(n, 1)]);
    }

    #[test]
    fn card_effects_sum_over_the_loadout_and_include_mastery() {
        let mut b = booklet_with(&[DreamTheme::Garden, DreamTheme::Lobby]);
        let cards: Vec<Card> = b.cards().cloned().collect();
        let base = card_effects(&b, &cards);
        let n = cards[0].number;
        b.mastery.add_xp(n, worth::LEVEL_THRESHOLDS[5]);
        let leveled = card_effects(&b, &cards);
        assert!(
            leveled.dust_bonus_pct + leveled.speed_pct > base.dust_bonus_pct + base.speed_pct,
            "mastery adds to the bonus"
        );
    }

    #[test]
    fn goal_inputs_read_the_booklet() {
        let mut b = booklet_with(&[DreamTheme::Garden]);
        b.stash.earn(33);
        b.lottery.since_lucid = 10;
        b.lottery.sparks = 4;
        let i = progress_inputs(&b, &[], 7);
        assert_eq!((i.dust, i.cards, i.best_depth), (33, 1, 7));
        assert_eq!(i.lottery_pulls_to_pity, lottery::HARD_PITY_LUCID - 10);
        assert_eq!(i.sparks, 4);
        assert!(i.store_cheapest_unowned.is_some());
        assert!(!streaks::teaser(&i).is_empty());
    }
}
