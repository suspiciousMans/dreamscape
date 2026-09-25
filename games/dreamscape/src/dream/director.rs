//! The endless descent. Portals always lead deeper; the only way out is to
//! become lucid (collect `LUCIDITY_TO_WAKE` shards) and then choose the wake
//! door, which leads to the Awakening dream.

use super::theme::DreamTheme;
use rand::distributions::{Distribution, WeightedIndex};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub const LUCIDITY_TO_WAKE: u32 = 3;
/// Shards a LONG run needs before the wake door appears.
pub const LONG_LUCIDITY: u32 = 6;
/// GO DEEPER at the wake door: this many more shards to wake next time.
pub const DEEPER_SHARDS: u32 = 3;
/// Chance a (non-lucid) descent dream hides a lucidity shard.
pub const SHARD_CHANCE: f64 = 0.6;

/// Chosen on the title screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunLength {
    /// Three shards; difficulty climbs gently.
    #[default]
    Short,
    /// Six shards; every dream is harder than the last, exponentially.
    Long,
}

impl RunLength {
    pub fn label(self) -> &'static str {
        match self {
            RunLength::Short => "short",
            RunLength::Long => "long",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            RunLength::Short => RunLength::Long,
            RunLength::Long => RunLength::Short,
        }
    }
}

pub struct DreamDirector {
    rng: StdRng,
    run_seed: u64,
    pub depth: u32,
    pub theme: DreamTheme,
    /// Where the cyan portal leads — pre-rolled so the portal can preview it.
    pub next: DreamTheme,
    pub lucidity: u32,
    /// Whether the current dream has a shard slot (a shard, or the wake door once lucid).
    pub has_shard: bool,
    /// A shard was collected in the CURRENT dream (droppable if caught).
    pub shard_this_dream: bool,
    /// Shards needed to become lucid (grows each time you GO DEEPER).
    pub shards_to_wake: u32,
    /// Depth where the exponential climb began (`None` = gentle run).
    pub hard_from: Option<u32>,
    /// Times the dreamer refused to wake.
    pub overdrive: u32,
    /// Added to `SHARD_CHANCE` (run upgrades).
    pub shard_bonus: f64,
}

impl DreamDirector {
    pub fn new(run_seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(run_seed);
        let next = roll(DreamTheme::Lobby, &mut rng);
        Self {
            rng,
            run_seed,
            depth: 0,
            theme: DreamTheme::Lobby,
            next,
            lucidity: 0,
            has_shard: false,
            shard_this_dream: false,
            shards_to_wake: LUCIDITY_TO_WAKE,
            hard_from: None,
            overdrive: 0,
            shard_bonus: 0.0,
        }
    }

    pub fn with_length(run_seed: u64, length: RunLength) -> Self {
        let mut d = Self::new(run_seed);
        if length == RunLength::Long {
            d.shards_to_wake = LONG_LUCIDITY;
            d.hard_from = Some(0);
        }
        d
    }

    /// Refuse the wake door: the dream tightens its grip. More shards to
    /// wake, and from here on every dream is harder than the last.
    pub fn go_deeper(&mut self) {
        self.shards_to_wake = self.lucidity + DEEPER_SHARDS;
        self.overdrive += 1;
        self.hard_from.get_or_insert(self.depth);
    }

    /// Dreams since the climb began, and how many times you went deeper.
    pub fn hardness(&self) -> Option<(u32, u32)> {
        self.hard_from
            .map(|from| (self.depth.saturating_sub(from), self.overdrive))
    }

    /// Seed for the current dream's content (distinct per depth).
    pub fn dream_seed(&self) -> u64 {
        self.run_seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(self.depth as u64)
    }

    pub fn lucid(&self) -> bool {
        self.lucidity >= self.shards_to_wake
    }

    pub fn collect_shard(&mut self) {
        self.lucidity += 1;
        self.shard_this_dream = true;
    }

    /// Caught by the dream: drop the shard picked up in THIS dream, if any.
    /// Returns true if a shard was lost (the game respawns it).
    pub fn caught(&mut self) -> bool {
        if !self.shard_this_dream {
            return false;
        }
        self.shard_this_dream = false;
        self.lucidity = self.lucidity.saturating_sub(1);
        true
    }

    /// Take the cyan portal: one dream deeper. `None` once you are awake.
    pub fn descend(&mut self) -> Option<DreamTheme> {
        if self.theme == DreamTheme::Awakening {
            return None;
        }
        self.depth += 1;
        self.shard_this_dream = false;
        self.theme = self.next;
        self.next = roll(self.theme, &mut self.rng);
        // Always roll, so the shard sequence doesn't depend on lucidity.
        let roll: f64 = self.rng.gen();
        let lucky = roll < (SHARD_CHANCE + self.shard_bonus).min(0.95);
        self.has_shard = lucky || self.lucid();
        Some(self.theme)
    }

    /// Take the wake door.
    pub fn wake(&mut self) {
        self.depth += 1;
        self.shard_this_dream = false;
        self.theme = DreamTheme::Awakening;
        self.has_shard = false;
    }
}

fn roll(from: DreamTheme, rng: &mut StdRng) -> DreamTheme {
    let next = from.spec().next;
    if next.is_empty() {
        return DreamTheme::Awakening;
    }
    let dist = WeightedIndex::new(next.iter().map(|&(_, w)| w))
        .expect("non-final themes have weighted exits (theme tests)");
    next[dist.sample(rng)].0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn descent(seed: u64, n: usize) -> Vec<DreamTheme> {
        let mut d = DreamDirector::new(seed);
        let mut seq = vec![d.theme];
        for _ in 0..n {
            seq.push(d.descend().expect("descent never ends on its own"));
        }
        seq
    }

    #[test]
    fn descent_is_endless_and_never_wakes_by_itself() {
        for seed in 0..20 {
            let seq = descent(seed, 1000);
            assert_eq!(seq[0], DreamTheme::Lobby);
            assert!(
                !seq.contains(&DreamTheme::Awakening),
                "seed {seed} woke up on its own"
            );
            assert!(
                !seq[1..].contains(&DreamTheme::Lobby),
                "seed {seed} returned to the lobby"
            );
            assert!(
                seq.windows(2).all(|w| w[0] != w[1]),
                "seed {seed} repeated a dream back-to-back"
            );
        }
    }

    #[test]
    fn portal_preview_matches_the_real_destination() {
        let mut d = DreamDirector::new(5);
        for _ in 0..200 {
            let promised = d.next;
            assert_eq!(d.descend(), Some(promised));
        }
    }

    #[test]
    fn runs_are_reproducible_and_vary() {
        assert_eq!(descent(42, 50), descent(42, 50));
        let distinct: HashSet<Vec<DreamTheme>> = (0..50).map(|s| descent(s, 6)).collect();
        assert!(
            distinct.len() >= 20,
            "only {} distinct descents",
            distinct.len()
        );
    }

    #[test]
    fn shards_appear_about_as_often_as_promised() {
        let mut d = DreamDirector::new(9);
        assert!(!d.has_shard, "the lobby never has a shard");
        let n = 2000;
        let hits = (0..n)
            .filter(|_| {
                d.descend();
                d.has_shard
            })
            .count();
        let rate = hits as f64 / n as f64;
        assert!((rate - SHARD_CHANCE).abs() < 0.05, "shard rate {rate}");
    }

    #[test]
    fn lucidity_and_waking() {
        let mut d = DreamDirector::new(1);
        assert!(!d.caught(), "nothing to lose at the start");
        assert_eq!(d.lucidity, 0);
        for _ in 0..LUCIDITY_TO_WAKE {
            assert!(!d.lucid());
            d.collect_shard();
        }
        assert!(d.lucid());
        d.descend();
        assert!(d.has_shard, "lucid dreams always have a wake-door slot");
        let depth = d.depth;
        d.wake();
        assert_eq!(d.theme, DreamTheme::Awakening);
        assert_eq!(d.depth, depth + 1);
        assert_eq!(d.descend(), None, "waking ends the run");
    }

    #[test]
    fn long_runs_need_more_shards_and_start_hard() {
        let d = DreamDirector::with_length(3, RunLength::Long);
        assert_eq!(d.shards_to_wake, LONG_LUCIDITY);
        assert_eq!(d.hardness(), Some((0, 0)));
        let s = DreamDirector::with_length(3, RunLength::Short);
        assert_eq!(s.shards_to_wake, LUCIDITY_TO_WAKE);
        assert_eq!(s.hardness(), None);
        assert_eq!(RunLength::Short.toggled().toggled(), RunLength::Short);
    }

    #[test]
    fn going_deeper_raises_the_goal_and_starts_the_climb() {
        let mut d = DreamDirector::new(4);
        for _ in 0..4 {
            d.descend();
        }
        for _ in 0..LUCIDITY_TO_WAKE {
            d.collect_shard();
        }
        assert!(d.lucid());
        d.go_deeper();
        assert!(!d.lucid(), "the wake door closes");
        assert_eq!(d.shards_to_wake, LUCIDITY_TO_WAKE + DEEPER_SHARDS);
        assert_eq!(d.hardness(), Some((0, 1)));
        d.descend();
        d.descend();
        assert_eq!(d.hardness(), Some((2, 1)));
        d.go_deeper();
        assert_eq!(d.hardness(), Some((2, 2)), "the climb keeps its start");
    }

    #[test]
    fn shard_bonus_makes_shards_commoner() {
        let mut d = DreamDirector::new(9);
        d.shard_bonus = 0.3;
        let n = 2000;
        let hits = (0..n)
            .filter(|_| {
                d.descend();
                d.has_shard
            })
            .count();
        let rate = hits as f64 / n as f64;
        assert!((rate - 0.9).abs() < 0.05, "shard rate {rate}");
    }

    #[test]
    fn getting_caught_only_costs_this_dreams_shard() {
        let mut d = DreamDirector::new(1);
        d.descend();
        d.collect_shard();
        d.descend(); // that shard is now banked
        assert!(!d.caught(), "banked shards are safe");
        assert_eq!(d.lucidity, 1);
        d.collect_shard();
        assert_eq!(d.lucidity, 2);
        assert!(d.caught(), "the shard from this dream is dropped");
        assert_eq!(d.lucidity, 1);
        assert!(!d.caught(), "the same shard can't be lost twice");
        d.collect_shard(); // picked it back up
        d.wake();
        assert!(!d.caught(), "waking banks everything");
    }
}
