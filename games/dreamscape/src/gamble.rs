//! Risk: the moth's double-or-nothing coin and the wager portal.
//!
//! Pure. Run dust only (never the banked stash), never real money. Every
//! stake is opt-in, capped by named consts and confirmed in a second step.
//! An outcome is decided from its own seed stream at the moment of
//! commit, and the counter that seeds it is saved with the run, so quitting
//! and reloading cannot re-roll it.

use engine::glam::Vec3;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Dust you may put on the coin. Never more than half your run dust.
pub const STAKES: [u32; 3] = [5, 10, 20];
/// Lost gambles in one run after which the next is a guaranteed win.
pub const GAMBLE_LOSS_CAP: u32 = 2;
/// Most gambles in one visit to the moth.
pub const MAX_PER_VISIT: u32 = 3;
/// Chance (percent) the coin doubles your stake. Shown as rolled.
pub const WIN_PERCENT: u32 = 50;

/// Wager portals start at this depth.
pub const WAGER_MIN_DEPTH: u32 = 4;
/// Chance a normal dream grows a wager portal (about 1 in 6).
pub const WAGER_PORTAL_CHANCE: f64 = 1.0 / 6.0;
/// Enemy budget multiplier in the wagered dream.
pub const WAGER_ENEMY_MULT: f32 = 1.3;
/// Hazard strength steps added (as extra depth on the strength ramp).
pub const WAGER_HAZARD_STEPS: u32 = 1;
/// Dust earned in the wagered dream is multiplied by this.
pub const WAGER_DUST_MULT: u32 = 2;
/// Distance (flat) at which the wager portal is touched.
pub const WAGER_RADIUS: f32 = crate::gameplay::CELL * 0.6;

// Player-facing text (listed in NOTES-risk.md for approval).
pub const ROW_NAME: &str = "GAMBLE";
pub const ROW_DESC: &str = "put some run dust on a coin. win and the stake is doubled, lose and it is gone.";
pub const STAKE_TITLE: &str = "THE MOTH'S COIN";
pub const OWES_LINE: &str = "the moth owes you one";
pub const WAGER_TITLE: &str = "A SECOND DOOR";
pub const WAGER_SUBTITLE: &str = "harder dream, double reward";
pub const WAGER_YES: &str = "TAKE THE DEAL";
pub const WAGER_YES_DESC: &str = "the next dream is harder: more enemies and a stronger hazard. dust earned in it is doubled and you get one extra rare card pick.";
pub const WAGER_NO: &str = "NOT NOW";
pub const WAGER_NO_DESC: &str = "stay in this dream. the usual door is still there.";

/// The gambling state saved with a run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunState {
    /// Coins flipped this run: seeds the next flip, so it never repeats.
    pub flips: u32,
    /// Lost gambles since the last win.
    pub losses: u32,
    /// The portal just taken: the next dream is wagered.
    pub wager_next: bool,
    /// This dream is wagered.
    pub wager_active: bool,
    /// The wagered dream was left: one extra rare+ pick is due.
    pub pick_owed: bool,
}

/// What the coin says before it is flipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Odds {
    /// Percent chance to win (100 when the moth owes you one).
    pub win_percent: u32,
    pub owed: bool,
}

impl Odds {
    /// The exact line shown on the screen that asks for the stake.
    pub fn line(self) -> String {
        if self.owed {
            format!("{}: this coin is a win", OWES_LINE)
        } else {
            format!(
                "{}% double · {}% lose the stake",
                self.win_percent,
                100 - self.win_percent
            )
        }
    }
}

impl RunState {
    pub fn owed(&self) -> bool {
        self.losses >= GAMBLE_LOSS_CAP
    }

    pub fn odds(&self) -> Odds {
        if self.owed() {
            Odds { win_percent: 100, owed: true }
        } else {
            Odds { win_percent: WIN_PERCENT, owed: false }
        }
    }

    /// Stakes you may choose now: never more than half your run dust.
    pub fn stakes(dust: u32) -> Vec<u32> {
        STAKES.iter().copied().filter(|s| s * 2 <= dust).collect()
    }

    /// May the shop offer a gamble now? `done` = gambles this visit.
    #[cfg(test)]
    pub fn open(dust: u32, done: u32) -> bool {
        done < MAX_PER_VISIT && !Self::stakes(dust).is_empty()
    }

    /// Why the row is shut (for its tag), if it is.
    pub fn shut_reason(dust: u32, done: u32) -> Option<&'static str> {
        if done >= MAX_PER_VISIT {
            Some("enough for this visit")
        } else if Self::stakes(dust).is_empty() {
            Some("needs 10 dust")
        } else {
            None
        }
    }

    /// The outcome of the next flip. Pure: the same state and dream give
    /// the same answer; [`RunState::settle`] moves the counter on.
    pub fn roll(&self, dream_seed: u64) -> bool {
        if self.owed() {
            return true;
        }
        let mut rng = StdRng::seed_from_u64(dream_seed ^ 0x6A4B_1E00 ^ (self.flips as u64).wrapping_mul(0x9E37_79B9));
        rng.gen_range(0..100) < WIN_PERCENT
    }

    /// Record a flip's outcome (a win clears the lost count).
    pub fn settle(&mut self, won: bool) {
        self.flips += 1;
        if won {
            self.losses = 0;
        } else {
            self.losses += 1;
        }
    }
}

/// Run dust after staking `stake` and the coin saying `won` (a win returns
/// the stake twice over).
pub fn after_flip(dust: u32, stake: u32, won: bool) -> u32 {
    let rest = dust.saturating_sub(stake);
    if won {
        rest + stake * 2
    } else {
        rest
    }
}

// === wager portal ===
/// Where this dream's wager portal stands, if it has one. Rolled from the
/// dream seed alone (so co-op and resumed runs agree). Only in a normal
/// dream deep enough, whose next dream is not a nightmare or a sanctuary;
/// placed on one of the dream's `lairs` (floor off the route and clear of
/// spawn, portal and shard), so it can never sit on or block the way.
pub fn wager_portal(dream_seed: u64, depth: u32, normal: bool, next_special: bool, lairs: &[Vec3]) -> Option<Vec3> {
    if !normal || next_special || depth < WAGER_MIN_DEPTH || lairs.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(dream_seed ^ 0x57A6_E200);
    if rng.gen::<f64>() >= WAGER_PORTAL_CHANCE {
        return None;
    }
    Some(lairs[rng.gen_range(0..lairs.len())])
}

/// Standing in the wager portal.
pub fn at_wager(player: Vec3, portal: Vec3) -> bool {
    Vec3::new(player.x - portal.x, 0.0, player.z - portal.z).length() <= WAGER_RADIUS
}

impl RunState {
    /// The portal was taken: the next dream is wagered.
    pub fn take_wager(&mut self) {
        self.wager_next = true;
    }

    /// A dream begins: a taken wager becomes this dream's.
    pub fn begin_dream(&mut self) {
        self.wager_active = std::mem::take(&mut self.wager_next);
    }

    /// The wagered dream was left through its portal: the reward is due.
    pub fn finish_dream(&mut self) {
        if self.wager_active {
            self.wager_active = false;
            self.pick_owed = true;
        }
    }

    /// The run ended or you woke: nothing carries over.
    pub fn clear_wager(&mut self) {
        self.wager_next = false;
        self.wager_active = false;
        self.pick_owed = false;
    }
}

/// Enemy budget multiplier (the pressure hook's `enemies`).
pub fn enemy_mult(wagered: bool) -> f32 {
    if wagered {
        WAGER_ENEMY_MULT
    } else {
        1.0
    }
}

/// The depth the hazard strength ramp reads.
pub fn hazard_depth(depth: u32, wagered: bool) -> u32 {
    depth + if wagered { WAGER_HAZARD_STEPS } else { 0 }
}

/// Run dust at the end of a wagered dream: what it earned counts double.
/// `start` is the run dust when the dream began.
pub fn doubled(start: u32, now: u32) -> u32 {
    now + now.saturating_sub(start) * (WAGER_DUST_MULT - 1)
}
// === end wager portal ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stakes_are_capped_at_half_your_dust_and_need_five() {
        assert!(RunState::stakes(0).is_empty());
        assert!(RunState::stakes(9).is_empty(), "5 is more than half of 9");
        assert_eq!(RunState::stakes(10), vec![5]);
        assert_eq!(RunState::stakes(19), vec![5]);
        assert_eq!(RunState::stakes(20), vec![5, 10]);
        assert_eq!(RunState::stakes(40), vec![5, 10, 20]);
        for dust in 0..200 {
            for s in RunState::stakes(dust) {
                assert!(s * 2 <= dust && s >= 5);
            }
        }
    }

    #[test]
    fn at_most_three_gambles_a_visit() {
        assert!(RunState::open(100, 2));
        assert!(!RunState::open(100, MAX_PER_VISIT));
        assert!(!RunState::open(8, 0));
        assert_eq!(RunState::shut_reason(100, 3), Some("enough for this visit"));
        assert_eq!(RunState::shut_reason(8, 0), Some("needs 10 dust"));
        assert_eq!(RunState::shut_reason(100, 0), None);
    }

    #[test]
    fn the_coin_is_fifty_fifty_as_shown() {
        let mut wins = 0;
        let n = 4000u32;
        for i in 0..n {
            let s = RunState { flips: i, ..Default::default() };
            assert_eq!(s.odds(), Odds { win_percent: 50, owed: false });
            if s.roll(0xABCDEF) {
                wins += 1;
            }
        }
        let p = wins as f32 / n as f32;
        assert!((0.46..0.54).contains(&p), "wins {p}");
        assert_eq!(RunState::default().odds().line(), "50% double · 50% lose the stake");
    }

    #[test]
    fn two_lost_gambles_make_the_third_a_win_and_say_so() {
        let mut s = RunState::default();
        s.settle(false);
        assert!(!s.owed());
        s.settle(false);
        assert!(s.owed());
        assert_eq!(s.odds().line(), "the moth owes you one: this coin is a win");
        for seed in 0..500 {
            assert!(s.roll(seed), "guaranteed");
        }
        s.settle(true);
        assert!(!s.owed() && s.losses == 0, "the debt is paid");
    }

    #[test]
    fn a_flip_is_deterministic_and_reload_cannot_reroll_it() {
        let s = RunState { flips: 3, ..Default::default() };
        assert_eq!(s.roll(77), s.roll(77));
        // Save with the counter, load, ask again: same answer.
        let ron = ron::to_string(&s).unwrap();
        let back: RunState = ron::from_str(&ron).unwrap();
        assert_eq!(back.roll(77), s.roll(77));
        // After settling, the counter moved: the next flip is a new draw.
        let mut t = s.clone();
        let first = t.roll(77);
        t.settle(first);
        let seq: Vec<bool> = (0..32)
            .map(|i| RunState { flips: i, ..Default::default() }.roll(77))
            .collect();
        assert!(seq.iter().any(|&b| b) && seq.iter().any(|&b| !b), "{seq:?}");
    }

    #[test]
    fn payouts_double_or_lose_the_stake() {
        assert_eq!(after_flip(30, 10, true), 40);
        assert_eq!(after_flip(30, 10, false), 20);
        assert_eq!(after_flip(10, 5, true), 15);
        assert_eq!(after_flip(3, 5, false), 0, "never underflows");
    }

    #[test]
    fn old_saves_load_with_no_gambling_state() {
        let s: RunState = ron::from_str("()").unwrap();
        assert_eq!(s, RunState::default());
    }

    fn lairs() -> Vec<Vec3> {
        (0..6).map(|i| Vec3::new(i as f32 * 3.0, 0.5, 4.0)).collect()
    }

    #[test]
    fn the_wager_portal_is_rare_deep_and_never_in_a_special_dream() {
        let l = lairs();
        let count = |depth, normal, special| {
            (0..6000u64)
                .filter(|&s| wager_portal(s, depth, normal, special, &l).is_some())
                .count()
        };
        let p = count(6, true, false) as f64 / 6000.0;
        assert!((0.13..0.20).contains(&p), "about 1 in 6, got {p}");
        assert_eq!(count(WAGER_MIN_DEPTH - 1, true, false), 0);
        assert_eq!(count(8, false, false), 0, "lobby, sanctuary, nightmare, prologue, co-op");
        assert_eq!(count(8, true, true), 0, "next dream is a nightmare or sanctuary");
        assert!(wager_portal(1, 8, true, false, &[]).is_none());
    }

    #[test]
    fn the_wager_portal_is_decided_by_the_dream_seed_alone() {
        let l = lairs();
        for s in 0..300u64 {
            assert_eq!(wager_portal(s, 6, true, false, &l), wager_portal(s, 6, true, false, &l));
            if let Some(at) = wager_portal(s, 6, true, false, &l) {
                assert!(l.contains(&at));
            }
        }
    }

    #[test]
    fn a_taken_wager_marks_the_next_dream_pays_double_and_owes_one_pick() {
        let mut s = RunState::default();
        s.begin_dream();
        assert!(!s.wager_active, "nothing is wagered by default");
        s.take_wager();
        assert!(!s.wager_active && s.wager_next);
        s.begin_dream();
        assert!(s.wager_active && !s.wager_next);
        s.finish_dream();
        assert!(!s.wager_active && s.pick_owed);
        s.clear_wager();
        assert_eq!(s, RunState::default());
        // Leaving an ordinary dream owes nothing.
        s.finish_dream();
        assert!(!s.pick_owed);
    }

    #[test]
    fn the_wagered_dream_is_harder_by_the_stated_amounts() {
        assert_eq!(enemy_mult(false), 1.0);
        assert!((enemy_mult(true) - 1.3).abs() < 1e-6);
        assert_eq!(hazard_depth(5, false), 5);
        assert_eq!(hazard_depth(5, true), 5 + WAGER_HAZARD_STEPS);
        assert!(crate::theme_rules::strength(hazard_depth(5, true)) > crate::theme_rules::strength(5));
    }

    #[test]
    fn earnings_double_but_what_you_brought_does_not() {
        assert_eq!(doubled(30, 30), 30);
        assert_eq!(doubled(30, 50), 70);
        assert_eq!(doubled(30, 10), 10, "spending never goes negative");
    }

    #[test]
    fn every_wager_portal_stands_off_the_route_and_clear_of_spawn_portal_shard() {
        use crate::dream::{generate_with, Goals, Pressure, ALL_THEMES};
        let flat = |a: Vec3, b: Vec3| Vec3::new(a.x - b.x, 0.0, a.z - b.z).length();
        let mut placed = 0;
        for theme in ALL_THEMES {
            for seed in 0..60u64 {
                let d = generate_with(theme, seed, 6, None, true, Pressure::default(), Goals::default());
                let Some(at) = wager_portal(seed, 6, true, false, &d.lairs) else {
                    continue;
                };
                placed += 1;
                for w in d.route.iter().chain(d.lucid_route.iter()) {
                    assert!(flat(at, w.pos) > 0.5, "{theme:?}/{seed}: on the route");
                }
                for k in [d.spawn, d.portal].into_iter().chain(d.shard) {
                    assert!(flat(at, k) >= 3.0 * crate::gameplay::CELL - 1e-3, "{theme:?}/{seed}: near {k}");
                }
                for c in &d.crumbles {
                    assert!(flat(at, *c) > 0.5);
                }
            }
        }
        assert!(placed > 5, "some dreams grew one ({placed})");
    }
}
