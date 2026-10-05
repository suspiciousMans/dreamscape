//! Kind retention helpers: a forgiving daily streak, a clean-dream streak,
//! a goal gradient ("you're close!") and a weekly challenge.
//!
//! Everything here is pure and number-only. Principle: missing a day is never
//! punished. A streak that breaks restarts gently, `best` is always kept, and
//! rest tokens quietly cover a single missed day.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Daily streak
// ---------------------------------------------------------------------------

/// Streak lengths that earn a named milestone and a dust bonus.
const MILESTONES: [u32; 4] = [3, 7, 14, 30];
/// A rest token is earned every this many days of streak.
const REST_EVERY: u32 = 7;
/// Most rest tokens that can be banked.
pub const MAX_RESTS: u32 = 2;

/// What happened when the player showed up today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreakEvent {
    /// Already counted today (or the clock went backwards).
    Same,
    /// First visit ever.
    Started,
    /// Came back the very next day.
    Continued { streak: u32 },
    /// Missed exactly one day; a rest token covered it and the streak lives.
    Rested { streak: u32 },
    /// Gap too long: gently restarts at 1 (`best` is kept).
    Reset { previous: u32 },
}

/// Consecutive days played, with generous rest tokens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DailyStreak {
    /// The last day number (days since epoch) that was counted.
    #[serde(default)]
    pub last_day: Option<u64>,
    /// Current streak in days.
    #[serde(default)]
    pub streak: u32,
    /// Longest streak ever; never lowered.
    #[serde(default)]
    pub best: u32,
    /// Banked rest tokens (0..=MAX_RESTS).
    #[serde(default)]
    pub rests: u32,
}

impl DailyStreak {
    /// Register a visit on day `today`. Never panics; a day earlier than
    /// `last_day` (clock skew) is treated as already counted.
    pub fn visit(&mut self, today: u64) -> StreakEvent {
        let Some(last) = self.last_day else {
            self.last_day = Some(today);
            self.streak = 1;
            self.best = self.best.max(1);
            return StreakEvent::Started;
        };
        if today <= last {
            return StreakEvent::Same;
        }
        let gap = today - last;
        self.last_day = Some(today);
        if gap == 1 {
            self.advance();
            StreakEvent::Continued { streak: self.streak }
        } else if gap == 2 && self.rests > 0 {
            self.rests -= 1;
            self.advance();
            StreakEvent::Rested { streak: self.streak }
        } else {
            let previous = self.streak;
            self.streak = 1;
            self.best = self.best.max(1);
            StreakEvent::Reset { previous }
        }
    }

    /// Add a day to the streak, update best, and grant a rest token on weeks.
    fn advance(&mut self) {
        self.streak = self.streak.saturating_add(1);
        self.best = self.best.max(self.streak);
        if self.streak % REST_EVERY == 0 {
            self.rests = (self.rests + 1).min(MAX_RESTS);
        }
    }
}

/// Dream Dust for reaching day `streak`: a small daily gift (growing to a
/// cap) plus a bonus on milestone days. 0 for a streak of 0.
pub fn dust_reward(streak: u32) -> u32 {
    if streak == 0 {
        return 0;
    }
    let base = 5 + streak.min(10);
    let bonus = match streak {
        3 => 10,
        7 => 25,
        14 => 50,
        30 => 100,
        _ => 0,
    };
    base + bonus
}

/// Name of the milestone reached at exactly `streak` days, if any.
pub fn milestone(streak: u32) -> Option<&'static str> {
    match streak {
        3 => Some("THREE NIGHTS RUNNING"),
        7 => Some("A WEEK OF DREAMS"),
        14 => Some("FORTNIGHT DREAMER"),
        30 => Some("A MONTH IN THE MIST"),
        _ => None,
    }
}

/// The next milestone length strictly above `streak`. Past 30, every further
/// multiple of 30.
pub fn next_milestone(streak: u32) -> u32 {
    MILESTONES
        .iter()
        .copied()
        .find(|&m| m > streak)
        .unwrap_or_else(|| (streak / 30).saturating_add(1).saturating_mul(30))
}

// ---------------------------------------------------------------------------
// Clean streak
// ---------------------------------------------------------------------------

/// Outcome of finishing one dream, for toasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanEvent {
    /// Finished unseen; `label` is set at toast-worthy lengths.
    Clean { current: u32, label: Option<&'static str> },
    /// Was caught; the streak (of length `lost`) ends.
    Caught { lost: u32 },
}

/// Consecutive dreams in one run finished without being caught.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CleanStreak {
    #[serde(default)]
    pub current: u32,
    #[serde(default)]
    pub best: u32,
}

impl CleanStreak {
    /// A new run begins: the in-run streak starts over (`best` is kept).
    pub fn on_run_start(&mut self) {
        self.current = 0;
    }

    /// A dream ended; `caught` says whether the player was caught.
    pub fn on_dream_end(&mut self, caught: bool) -> CleanEvent {
        if caught {
            let lost = self.current;
            self.current = 0;
            CleanEvent::Caught { lost }
        } else {
            self.current = self.current.saturating_add(1);
            self.best = self.best.max(self.current);
            CleanEvent::Clean { current: self.current, label: label(self.current) }
        }
    }
}

/// Tiny dust multiplier: +1% per clean dream, capped at +10%.
pub fn multiplier(current: u32) -> f32 {
    1.0 + 0.01 * current.min(10) as f32
}

/// Toast label at notable clean-streak lengths.
pub fn label(current: u32) -> Option<&'static str> {
    match current {
        3 => Some("UNSEEN"),
        5 => Some("GHOSTLIKE"),
        8 => Some("NIGHTWALKER"),
        10 => Some("PHANTOM"),
        15 => Some("NEVER WAS HERE"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Goal gradient
// ---------------------------------------------------------------------------

/// Plain numbers describing how close the player is to various things.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgressInputs {
    pub dust: u32,
    pub cards: u32,
    pub best_depth: u32,
    pub ascension: u8,
    /// Dreams left until the next perk, if one is pending.
    pub dreams_until_perk: Option<u32>,
    /// Mastery points to the next level, if any.
    pub mastery_to_next: Option<u32>,
    pub daily_streak: u32,
    /// Lottery pulls left until the pity guarantee.
    pub lottery_pulls_to_pity: u32,
    pub sparks: u32,
    /// Sparks needed for the spark reward (0 = none).
    pub spark_goal: u32,
    /// Price of the cheapest store item not yet owned.
    pub store_cheapest_unowned: Option<u32>,
}

/// A goal with progress. `have` is 0 when only "remaining" is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    pub label: String,
    pub have: u32,
    pub need: u32,
}

impl Goal {
    /// Progress in 0.0..=1.0 (1.0 when `need` is 0).
    pub fn fraction(&self) -> f32 {
        if self.need == 0 {
            return 1.0;
        }
        (self.have as f32 / self.need as f32).clamp(0.0, 1.0)
    }

    /// How much is left to do.
    pub fn remaining(&self) -> u32 {
        self.need.saturating_sub(self.have)
    }
}

/// What kind of goal won, used to word the teaser (in priority order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Store,
    Perk,
    Mastery,
    Sparks,
    Pity,
    Streak,
}

fn pick(i: &ProgressInputs) -> (Goal, Kind) {
    let mut c: Vec<(Goal, Kind)> = Vec::new();
    if let Some(price) = i.store_cheapest_unowned {
        if i.dust < price {
            c.push((Goal { label: "A NEW STORE FIND".into(), have: i.dust, need: price }, Kind::Store));
        }
    }
    if let Some(n) = i.dreams_until_perk.filter(|&n| n > 0) {
        c.push((Goal { label: "YOUR NEXT PERK".into(), have: 0, need: n }, Kind::Perk));
    }
    if let Some(n) = i.mastery_to_next.filter(|&n| n > 0) {
        c.push((Goal { label: "THE NEXT MASTERY".into(), have: 0, need: n }, Kind::Mastery));
    }
    if i.spark_goal > i.sparks {
        c.push((Goal { label: "A SPARK REWARD".into(), have: i.sparks, need: i.spark_goal }, Kind::Sparks));
    }
    if i.lottery_pulls_to_pity > 0 {
        c.push((Goal { label: "A GUARANTEED PULL".into(), have: 0, need: i.lottery_pulls_to_pity }, Kind::Pity));
    }
    let m = next_milestone(i.daily_streak);
    c.push((Goal { label: "THE NEXT STREAK MILESTONE".into(), have: i.daily_streak, need: m }, Kind::Streak));
    // Strict `<`: the first minimum wins, so earlier (higher-priority) entries win ties.
    let mut best = 0;
    for (idx, (g, _)) in c.iter().enumerate() {
        if g.remaining() < c[best].0.remaining() {
            best = idx;
        }
    }
    c.swap_remove(best)
}

/// The nearest achievable goal (smallest remaining amount; ties go to the
/// higher-priority goal: store, perk, mastery, sparks, pity, streak).
/// Always returns something (the streak milestone is the fallback).
pub fn next_goal(i: &ProgressInputs) -> Goal {
    pick(i).0
}

/// One-line nudge such as "ONE MORE DREAM: 4 DUST FROM A NEW STORE FIND".
pub fn teaser(i: &ProgressInputs) -> String {
    let (g, kind) = pick(i);
    let r = g.remaining();
    match kind {
        Kind::Store => format!("ONE MORE DREAM: {r} DUST FROM {}", g.label),
        Kind::Perk if r == 1 => "ONE MORE DREAM: YOUR NEXT PERK".to_string(),
        Kind::Perk => format!("{r} DREAMS FROM YOUR NEXT PERK"),
        Kind::Mastery => format!("{r} MASTERY FROM THE NEXT LEVEL"),
        Kind::Sparks => format!("{r} SPARKS FROM {}", g.label),
        Kind::Pity if r == 1 => "ONE MORE PULL: GUARANTEED".to_string(),
        Kind::Pity => format!("{r} PULLS FROM A GUARANTEED PULL"),
        Kind::Streak if r == 1 => format!("ONE MORE DAY: {}", g.label),
        Kind::Streak => format!("{r} DAYS FROM {}", g.label),
    }
}

// ---------------------------------------------------------------------------
// Weekly challenge
// ---------------------------------------------------------------------------

/// The week number (7-day blocks of day numbers) a day falls in.
pub fn week_of(day: u64) -> u64 {
    day / 7
}

/// A rule twist applied to the week's seeded dream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Twist {
    NoAbilities,
    DoubleEnemies,
    Fog,
    Speedrun,
    GlassDreamer,
}

impl Twist {
    const ALL: [Twist; 5] =
        [Twist::NoAbilities, Twist::DoubleEnemies, Twist::Fog, Twist::Speedrun, Twist::GlassDreamer];

    /// Short display name.
    pub fn name(self) -> &'static str {
        match self {
            Twist::NoAbilities => "NO ABILITIES",
            Twist::DoubleEnemies => "DOUBLE TROUBLE",
            Twist::Fog => "THICK FOG",
            Twist::Speedrun => "SPEEDRUN",
            Twist::GlassDreamer => "GLASS DREAMER",
        }
    }
}

/// This week's challenge, the same for everyone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeeklyChallenge {
    pub week: u64,
    pub seed: u64,
    pub twist: Twist,
    /// Depth to reach to claim the reward.
    pub goal_depth: u32,
    pub reward_dust: u32,
}

/// SplitMix64 finaliser, distinct from `progress::daily_seed`.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deterministic challenge for `week`.
pub fn weekly(week: u64) -> WeeklyChallenge {
    let seed = mix(week ^ 0x005E_ED0F_77EE_CAFE);
    let r = mix(seed);
    let twist = Twist::ALL[(r % Twist::ALL.len() as u64) as usize];
    let goal_depth = 4 + ((r >> 8) % 5) as u32; // 4..=8
    let reward_dust = 40 + goal_depth * 10;
    WeeklyChallenge { week, seed, twist, goal_depth, reward_dust }
}

/// The player's best result for one week.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeeklyProgress {
    #[serde(default)]
    pub week: u64,
    #[serde(default)]
    pub best: u32,
    #[serde(default)]
    pub claimed: bool,
}

impl WeeklyProgress {
    /// Record a depth reached in `week`. A new week starts fresh.
    pub fn record(&mut self, week: u64, depth: u32) {
        if week != self.week {
            *self = WeeklyProgress { week, best: 0, claimed: false };
        }
        self.best = self.best.max(depth);
    }

    /// True if this week's goal is met and the reward is not yet claimed.
    pub fn claimable(&self, w: &WeeklyChallenge) -> bool {
        self.week == w.week && !self.claimed && self.best >= w.goal_depth
    }

    /// Claim the reward if claimable; returns the dust awarded.
    pub fn claim(&mut self, w: &WeeklyChallenge) -> Option<u32> {
        if self.claimable(w) {
            self.claimed = true;
            Some(w.reward_dust)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn streak_at(s: u32, last: u64, rests: u32) -> DailyStreak {
        DailyStreak { last_day: Some(last), streak: s, best: s, rests }
    }

    #[test]
    fn streaks_first_visit_starts() {
        let mut d = DailyStreak::default();
        assert_eq!(d.visit(100), StreakEvent::Started);
        assert_eq!((d.streak, d.best, d.last_day), (1, 1, Some(100)));
    }

    #[test]
    fn streaks_same_day_is_same() {
        let mut d = DailyStreak::default();
        d.visit(100);
        assert_eq!(d.visit(100), StreakEvent::Same);
        assert_eq!(d.streak, 1);
    }

    #[test]
    fn streaks_consecutive_continues() {
        let mut d = DailyStreak::default();
        d.visit(100);
        assert_eq!(d.visit(101), StreakEvent::Continued { streak: 2 });
        assert_eq!(d.best, 2);
    }

    #[test]
    fn streaks_one_day_gap_without_rest_resets_keeping_best() {
        let mut d = streak_at(5, 100, 0);
        assert_eq!(d.visit(102), StreakEvent::Reset { previous: 5 });
        assert_eq!((d.streak, d.best), (1, 5));
    }

    #[test]
    fn streaks_one_day_gap_with_rest_survives() {
        let mut d = streak_at(5, 100, 1);
        assert_eq!(d.visit(102), StreakEvent::Rested { streak: 6 });
        assert_eq!((d.streak, d.rests, d.best), (6, 0, 6));
    }

    #[test]
    fn streaks_long_gap_resets_even_with_rests() {
        let mut d = streak_at(9, 100, 2);
        assert_eq!(d.visit(110), StreakEvent::Reset { previous: 9 });
        assert_eq!((d.streak, d.rests, d.best), (1, 2, 9));
    }

    #[test]
    fn streaks_rest_granted_every_seven_and_capped() {
        let mut d = DailyStreak::default();
        for day in 0..7 {
            d.visit(day);
        }
        assert_eq!((d.streak, d.rests), (7, 1));
        for day in 7..21 {
            d.visit(day);
        }
        assert_eq!((d.streak, d.rests), (21, MAX_RESTS));
        for day in 21..35 {
            d.visit(day);
        }
        assert_eq!(d.rests, MAX_RESTS);
    }

    #[test]
    fn streaks_clock_backwards_is_same_and_harmless() {
        let mut d = streak_at(4, 100, 1);
        let before = d.clone();
        assert_eq!(d.visit(50), StreakEvent::Same);
        assert_eq!(d.visit(0), StreakEvent::Same);
        assert_eq!(d, before);
        let mut e = DailyStreak::default();
        e.visit(u64::MAX);
        assert_eq!(e.visit(0), StreakEvent::Same);
    }

    #[test]
    fn streaks_reset_then_rebuild_best() {
        let mut d = streak_at(2, 10, 0);
        d.visit(20);
        for day in 21..30 {
            d.visit(day);
        }
        assert_eq!(d.best, d.streak);
        assert!(d.best > 2);
    }

    #[test]
    fn streaks_dust_reward_monotonic_with_milestones() {
        assert_eq!(dust_reward(0), 0);
        for s in 1..=60 {
            assert!(dust_reward(s) > 0);
            assert!(dust_reward(s + 1) >= dust_reward(s) || milestone(s).is_some());
        }
        for m in MILESTONES {
            assert!(milestone(m).is_some());
            assert!(dust_reward(m) > dust_reward(m - 1));
            assert!(dust_reward(m) > dust_reward(m + 1));
        }
        assert!(dust_reward(30) > dust_reward(14));
        assert!(dust_reward(14) > dust_reward(7));
        assert!(dust_reward(7) > dust_reward(3));
        assert_eq!(milestone(4), None);
    }

    #[test]
    fn streaks_next_milestone_walks_up() {
        assert_eq!(next_milestone(0), 3);
        assert_eq!(next_milestone(3), 7);
        assert_eq!(next_milestone(6), 7);
        assert_eq!(next_milestone(7), 14);
        assert_eq!(next_milestone(29), 30);
        assert_eq!(next_milestone(30), 60);
        assert_eq!(next_milestone(61), 90);
        let _ = next_milestone(u32::MAX); // must not panic
    }

    #[test]
    fn streaks_clean_streak_behaviour() {
        let mut c = CleanStreak::default();
        c.on_run_start();
        assert_eq!(c.on_dream_end(false), CleanEvent::Clean { current: 1, label: None });
        c.on_dream_end(false);
        assert_eq!(c.on_dream_end(false), CleanEvent::Clean { current: 3, label: Some("UNSEEN") });
        assert_eq!(c.on_dream_end(true), CleanEvent::Caught { lost: 3 });
        assert_eq!((c.current, c.best), (0, 3));
        c.on_dream_end(false);
        c.on_run_start();
        assert_eq!((c.current, c.best), (0, 3));
    }

    #[test]
    fn streaks_clean_multiplier_capped_and_labels() {
        assert_eq!(multiplier(0), 1.0);
        assert!(multiplier(5) > multiplier(1));
        assert_eq!(multiplier(10), multiplier(1000));
        assert!(multiplier(u32::MAX) <= 1.1001);
        assert_eq!(label(5), Some("GHOSTLIKE"));
        assert_eq!(label(4), None);
    }

    #[test]
    fn streaks_next_goal_picks_nearest() {
        let i = ProgressInputs {
            dust: 96,
            dreams_until_perk: Some(30),
            mastery_to_next: Some(90),
            store_cheapest_unowned: Some(100),
            daily_streak: 7, // next milestone 7 days away; store is 4 dust away
            ..Default::default()
        };
        let g = next_goal(&i);
        assert_eq!((g.have, g.need), (96, 100));
        assert!(teaser(&i).starts_with("ONE MORE DREAM: 4 DUST"));
        let j = ProgressInputs { dreams_until_perk: Some(1), store_cheapest_unowned: Some(500), ..i.clone() };
        assert_eq!(teaser(&j), "ONE MORE DREAM: YOUR NEXT PERK");
    }

    #[test]
    fn streaks_next_goal_tie_breaks_by_priority() {
        let i = ProgressInputs {
            dreams_until_perk: Some(2),
            mastery_to_next: Some(2),
            ..Default::default()
        };
        assert_eq!(next_goal(&i).label, "YOUR NEXT PERK");
    }

    #[test]
    fn streaks_next_goal_all_none_still_works() {
        let i = ProgressInputs::default();
        let g = next_goal(&i);
        assert_eq!((g.have, g.need), (0, 3));
        assert!(!teaser(&i).is_empty());
        let k = ProgressInputs { daily_streak: 6, ..Default::default() };
        assert_eq!(teaser(&k), "ONE MORE DAY: THE NEXT STREAK MILESTONE");
    }

    #[test]
    fn streaks_goal_ignores_affordable_and_fraction_in_range() {
        let i = ProgressInputs { dust: 999, store_cheapest_unowned: Some(10), ..Default::default() };
        assert_ne!(next_goal(&i).label, "A NEW STORE FIND");
        let cases = [
            Goal { label: String::new(), have: 0, need: 0 },
            Goal { label: String::new(), have: 5, need: 10 },
            Goal { label: String::new(), have: 50, need: 10 },
            Goal { label: String::new(), have: 0, need: 10 },
        ];
        for g in cases {
            let f = g.fraction();
            assert!((0.0..=1.0).contains(&f), "{f}");
        }
        let extreme = ProgressInputs {
            dust: u32::MAX,
            sparks: u32::MAX,
            spark_goal: u32::MAX,
            daily_streak: u32::MAX,
            ..Default::default()
        };
        let f = next_goal(&extreme).fraction();
        assert!((0.0..=1.0).contains(&f));
    }

    #[test]
    fn streaks_weekly_deterministic_and_differs() {
        assert_eq!(weekly(100), weekly(100));
        assert_ne!(weekly(100).seed, weekly(101).seed);
        let seeds: std::collections::HashSet<u64> = (0..200).map(|w| weekly(w).seed).collect();
        assert_eq!(seeds.len(), 200);
        let twists: std::collections::HashSet<&str> = (0..200).map(|w| weekly(w).twist.name()).collect();
        assert!(twists.len() > 1);
        for w in 0..200 {
            let c = weekly(w);
            assert!((4..=8).contains(&c.goal_depth));
            assert!(c.reward_dust > 0);
        }
    }

    #[test]
    fn streaks_week_of_groups_days() {
        assert_eq!(week_of(0), 0);
        assert_eq!(week_of(6), 0);
        assert_eq!(week_of(7), 1);
        assert_eq!(week_of(20_000), 20_000 / 7);
    }

    #[test]
    fn streaks_weekly_progress_claim_logic() {
        let w = weekly(50);
        let mut p = WeeklyProgress::default();
        p.record(50, w.goal_depth - 1);
        assert!(!p.claimable(&w));
        p.record(50, w.goal_depth);
        p.record(50, 1); // lower depth never lowers best
        assert_eq!(p.best, w.goal_depth);
        assert!(p.claimable(&w));
        assert_eq!(p.claim(&w), Some(w.reward_dust));
        assert!(!p.claimable(&w));
        assert_eq!(p.claim(&w), None);
        // new week resets
        p.record(51, 2);
        assert_eq!((p.week, p.best, p.claimed), (51, 2, false));
        // stale progress cannot claim another week's reward
        let mut q = WeeklyProgress { week: 50, best: 99, claimed: false };
        assert!(!q.claimable(&weekly(51)));
        assert_eq!(q.claim(&weekly(51)), None);
    }

    #[test]
    fn streaks_serde_round_trips_and_defaults() {
        let d = streak_at(8, 123, 2);
        let t = ron::to_string(&d).unwrap();
        assert_eq!(ron::from_str::<DailyStreak>(&t).unwrap(), d);
        let c = CleanStreak { current: 2, best: 7 };
        assert_eq!(ron::from_str::<CleanStreak>(&ron::to_string(&c).unwrap()).unwrap(), c);
        let p = WeeklyProgress { week: 9, best: 4, claimed: true };
        assert_eq!(ron::from_str::<WeeklyProgress>(&ron::to_string(&p).unwrap()).unwrap(), p);
        assert_eq!(ron::from_str::<DailyStreak>("()").unwrap(), DailyStreak::default());
        assert_eq!(ron::from_str::<CleanStreak>("()").unwrap(), CleanStreak::default());
        assert_eq!(ron::from_str::<WeeklyProgress>("()").unwrap(), WeeklyProgress::default());
        assert_eq!(ron::from_str::<DailyStreak>("(streak: 3)").unwrap().streak, 3);
    }
}
