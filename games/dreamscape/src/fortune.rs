//! The daily fortune: one free lottery pull per local day, offered in the
//! Lucid Store's lottery tab. It costs no dust, and it never touches the pity
//! counters, sparks or pull count (a free pull cannot break or advance pity).
//!
//! A forgiving streak of consecutive fortune days (the same `DailyStreak` the
//! daily visit uses, rest tokens included) adds a tiny, capped nudge to the
//! Lucid+ chance. A missed day only pauses the streak; it never takes a pull
//! away and never shows a countdown. The nudge is shown exactly in the odds.
//!
//! Pure: the day number is injected, nothing here reads the clock.
use crate::cards::Booklet;
use crate::dream::DreamTheme;
use crate::lottery::{self, Odds};
use crate::retention::{press_pull, Pulled};
use crate::streaks::DailyStreak;
use serde::{Deserialize, Serialize};

/// Lucid+ chance added per earlier consecutive fortune day (1.0 = 100%).
pub const FORTUNE_STREAK_BONUS_PER_DAY: f32 = 0.005;
/// Most earlier days that count toward the nudge (so at most +3.0 points).
pub const FORTUNE_STREAK_CAP: u32 = 6;

/// Persisted fortune state (old saves lack it and load as default).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FortuneState {
    /// Days a fortune was claimed, as a forgiving streak.
    #[serde(default)]
    pub days: DailyStreak,
    /// Fortunes ever claimed.
    #[serde(default)]
    pub claimed: u32,
}

impl FortuneState {
    /// Is a fortune waiting on `today`? (A clock set back is "already claimed".)
    pub fn available(&self, today: u64) -> bool {
        self.days.last_day.map_or(true, |last| today > last)
    }

    /// Consecutive fortune days that would be behind the pull claimed on `today`.
    pub fn prior_days(&self, today: u64) -> u32 {
        if !self.available(today) {
            return self.days.streak.saturating_sub(1);
        }
        let mut preview = self.days.clone();
        preview.visit(today);
        preview.streak.saturating_sub(1)
    }

    /// The Lucid+ nudge (a probability) a claim on `today` gets.
    pub fn nudge(&self, today: u64) -> f32 {
        self.prior_days(today).min(FORTUNE_STREAK_CAP) as f32 * FORTUNE_STREAK_BONUS_PER_DAY
    }
}

/// The exact odds of a fortune claimed on `today` (pity state included).
pub fn fortune_odds(b: &Booklet, today: u64, featured: Option<DreamTheme>) -> Odds {
    lottery::odds_nudged(&b.lottery, featured, b.fortune.nudge(today))
}

/// This save's own seed stream for a day's fortune: the same day always
/// gives the same card, so quitting and reloading cannot re-roll it.
pub fn fortune_seed(save_seed: u64, today: u64) -> u64 {
    save_seed ^ today.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xF027_0E00_D41B_C0DE
}

/// Claim today's fortune. `None` (and nothing changes) if already claimed.
/// The pull uses the normal odds and the player's current pity position plus
/// the streak nudge, then the pity counters are put back: a free pull neither
/// spends nor breaks pity.
pub fn claim(b: &mut Booklet, today: u64, save_seed: u64) -> Option<Pulled> {
    if !b.fortune.available(today) {
        return None;
    }
    let nudge = b.fortune.nudge(today);
    let seed = fortune_seed(save_seed, today);
    let kept = b.lottery.clone();
    let pull = lottery::pull_nudged(
        &mut b.lottery,
        seed,
        Some(lottery::featured_theme(today)),
        nudge,
    );
    b.lottery = kept;
    b.fortune.days.visit(today);
    b.fortune.claimed = b.fortune.claimed.saturating_add(1);
    Some(press_pull(b, pull, seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum(o: &Odds) -> f32 {
        o.rarity.iter().sum()
    }

    #[test]
    fn a_fresh_save_has_a_fortune_waiting() {
        let b = Booklet::default();
        assert!(b.fortune.available(20_000));
        assert_eq!(b.fortune.nudge(20_000), 0.0, "no earlier days, no nudge");
    }

    #[test]
    fn one_claim_per_day_and_the_next_day_opens_again() {
        let mut b = Booklet::default();
        assert!(claim(&mut b, 100, 7).is_some());
        assert!(claim(&mut b, 100, 7).is_none(), "once per day");
        assert!(!b.fortune.available(100));
        assert!(!b.fortune.available(99), "a clock set back is not a new day");
        assert!(b.fortune.available(101));
        assert!(claim(&mut b, 101, 7).is_some());
        assert_eq!(b.fortune.claimed, 2);
    }

    #[test]
    fn the_streak_nudge_grows_then_caps() {
        let mut b = Booklet::default();
        let mut last = -1.0;
        for d in 0..(FORTUNE_STREAK_CAP + 4) as u64 {
            let n = b.fortune.nudge(100 + d);
            assert!(n >= last);
            assert!(n <= FORTUNE_STREAK_CAP as f32 * FORTUNE_STREAK_BONUS_PER_DAY + 1e-6);
            last = n;
            claim(&mut b, 100 + d, 1).unwrap();
        }
        let cap = FORTUNE_STREAK_CAP as f32 * FORTUNE_STREAK_BONUS_PER_DAY;
        assert!((last - cap).abs() < 1e-6, "{last} vs {cap}");
        assert!(cap <= 0.05, "the nudge stays tiny");
    }

    #[test]
    fn a_missed_day_pauses_the_streak_and_a_long_gap_restarts_gently() {
        let mut b = Booklet::default();
        for d in 0..4 {
            claim(&mut b, 100 + d, 1).unwrap();
        }
        let before = b.fortune.nudge(104);
        assert!(before > 0.0);
        // Skip two days: no rest banked yet, the streak restarts but nothing is lost
        // but the nudge; the claim itself is still free and available.
        assert!(b.fortune.available(107));
        assert_eq!(b.fortune.nudge(107), 0.0);
        claim(&mut b, 107, 1).unwrap();
        assert_eq!(b.fortune.days.best, 4, "best is kept");
        // A banked rest covers exactly one missed day.
        let mut r = Booklet::default();
        r.fortune.days = DailyStreak { last_day: Some(100), streak: 4, best: 4, rests: 1 };
        assert_eq!(r.fortune.prior_days(102), 4, "the rest keeps the streak alive");
    }

    #[test]
    fn the_nudge_is_in_the_shown_odds_and_odds_still_sum_to_one() {
        let mut b = Booklet::default();
        let plain = lottery::odds(&b.lottery, None);
        for d in 0..5 {
            claim(&mut b, 100 + d, 3).unwrap();
        }
        let shown = fortune_odds(&b, 105, None);
        assert!((sum(&shown) - 1.0).abs() < 1e-5);
        let gain = shown.lucid_or_better() - plain.lucid_or_better();
        assert!((gain - 5.0 * FORTUNE_STREAK_BONUS_PER_DAY).abs() < 1e-5, "{gain}");
    }

    #[test]
    fn a_free_pull_costs_nothing_and_leaves_pity_untouched() {
        let mut b = Booklet::default();
        b.stash.dust = 10;
        b.lottery.since_lucid = 11;
        b.lottery.since_prophetic = 40;
        b.lottery.sparks = 3;
        b.lottery.total_pulls = 9;
        let before = b.lottery.clone();
        let got = claim(&mut b, 500, 42).unwrap();
        assert_eq!(b.stash.dust, 10);
        assert_eq!(b.lottery, before, "pity, sparks and pulls are not moved");
        assert!(b.card(got.card.number).is_some(), "the card is in the booklet");
    }

    #[test]
    fn the_same_day_gives_the_same_card_so_reloading_cannot_reroll() {
        let mut a = Booklet::default();
        let mut c = Booklet::default();
        let x = claim(&mut a, 321, 99).unwrap();
        let y = claim(&mut c, 321, 99).unwrap();
        assert_eq!(x.card.name, y.card.name);
        assert_eq!(x.card.rarity, y.card.rarity);
    }

    #[test]
    fn old_saves_without_fortune_load_unchanged() {
        let s: FortuneState = ron::from_str("()").unwrap();
        assert_eq!(s, FortuneState::default());
        let again: FortuneState = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert_eq!(again, s);
    }
}
