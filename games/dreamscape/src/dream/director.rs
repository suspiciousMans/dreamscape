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
/// Every this-many dreams, the next one is a nightmare arena.
pub const NIGHTMARE_EVERY: u32 = 5;

/// Chance a (non-lucid) descent dream hides a lucidity shard.
pub const SHARD_CHANCE: f64 = 0.6;
/// Hard runs: chance each dream that the next is the White Dissolve.
pub const WHITE_CHANCE: f64 = 0.08;

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
    /// The current dream is a nightmare arena (no shard, a hunter, sigils).
    pub nightmare: bool,
    /// Every this-many dreams is a nightmare (ascension shortens it).
    pub nightmare_every: u32,
    /// Themes to use next, front first, instead of rolling (the prologue).
    pub forced: Vec<DreamTheme>,
    /// The loadout's dreams, each due once at `CARD_DEPTHS`.
    pub cards: Vec<CardDream>,
    /// The current dream is one of the loadout's.
    pub card_dream: bool,
    /// The current dream's second theme, when it's a fused one.
    pub blend: Option<DreamTheme>,
    /// Index into `cards` of the dream the portal leads to, if it's one.
    next_card: Option<usize>,
}

/// A loadout card's dream, planned into the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDream {
    pub theme: DreamTheme,
    pub blend: Option<DreamTheme>,
    /// The depth it's due at; it slides later past a nightmare.
    pub at: u32,
    pub done: bool,
}

/// Card dreams come at depths 3, 6, 9, 12, 15.
pub fn card_depth(i: usize) -> u32 {
    3 + 3 * i as u32
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
            nightmare: false,
            nightmare_every: NIGHTMARE_EVERY,
            forced: Vec::new(),
            cards: Vec::new(),
            card_dream: false,
            blend: None,
            next_card: None,
        }
    }

    /// Plans the loadout's dreams into the run (the waking dream can't be
    /// planned: it would end the run).
    pub fn plan_cards(&mut self, dreams: &[(DreamTheme, Option<DreamTheme>)]) {
        self.cards = dreams
            .iter()
            .filter(|(t, _)| *t != DreamTheme::Awakening)
            .enumerate()
            .map(|(i, &(theme, blend))| CardDream {
                theme,
                blend,
                at: card_depth(i),
                done: false,
            })
            .collect();
        self.pick_next_card();
    }

    pub fn is_nightmare_depth(&self, depth: u32) -> bool {
        depth.is_multiple_of(self.nightmare_every.max(2))
    }

    /// Points the portal at a due card dream, unless the next dream is a
    /// nightmare or this one was a card dream. (A card dream may repeat the
    /// dream before it; sliding for that too could push it two late.)
    fn pick_next_card(&mut self) {
        self.next_card = None;
        if !self.forced.is_empty() || self.card_dream {
            return;
        }
        let upcoming = self.depth + 1;
        if self.is_nightmare_depth(upcoming) {
            return;
        }
        if let Some(i) = self.cards.iter().position(|c| !c.done && c.at <= upcoming) {
            self.next_card = Some(i);
            self.next = self.cards[i].theme;
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
        self.theme = if self.forced.is_empty() {
            self.next
        } else {
            self.forced.remove(0)
        };
        self.next = match self.forced.first() {
            Some(&t) => t,
            None => roll(self.theme, &mut self.rng),
        };
        // The white is only reachable by refusing to wake (GO DEEPER).
        if self.hard_from.is_some()
            && self.theme != DreamTheme::WhiteDissolve
            && self.rng.gen_bool(WHITE_CHANCE)
        {
            self.next = DreamTheme::WhiteDissolve;
        }
        self.card_dream = false;
        self.blend = None;
        if let Some(i) = self.next_card.take() {
            let c = &mut self.cards[i];
            c.done = true;
            self.card_dream = true;
            self.blend = c.blend;
        }
        self.pick_next_card();
        // Always roll, so the shard sequence doesn't depend on lucidity.
        let roll: f64 = self.rng.gen();
        let lucky = roll < (SHARD_CHANCE + self.shard_bonus).clamp(0.05, 0.95);
        self.nightmare = self.is_nightmare_depth(self.depth);
        // A nightmare has no shard slot: the wake door waits for the next dream.
        self.has_shard = !self.nightmare && (lucky || self.lucid());
        Some(self.theme)
    }

    /// Take the wake door.
    pub fn wake(&mut self) {
        self.depth += 1;
        self.shard_this_dream = false;
        self.theme = DreamTheme::Awakening;
        self.has_shard = false;
        self.nightmare = false;
        self.card_dream = false;
        self.blend = None;
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
    fn card_dreams_come_spaced_out_and_never_on_a_nightmare() {
        use DreamTheme::*;
        let plan = [Garden, TheTunnel, MirrorHall, JellyfishSky, Elfworks];
        for seed in 0..200 {
            for every in [2, 3, 5] {
                let mut d = DreamDirector::new(seed);
                d.nightmare_every = every;
                d.plan_cards(&plan.map(|t| (t, None)));
                let mut hits: Vec<(u32, DreamTheme)> = Vec::new();
                let mut last_card = false;
                for _ in 0..20 {
                    let promised = d.next;
                    let t = d.descend().unwrap();
                    assert_eq!(t, promised, "the portal preview stays honest");
                    if d.card_dream {
                        assert!(!d.nightmare, "seed {seed}: card dream on a nightmare");
                        assert!(!last_card, "seed {seed}: card dreams back to back");
                        hits.push((d.depth, t));
                    }
                    last_card = d.card_dream;
                }
                assert_eq!(hits.len(), 5, "seed {seed} every {every}: {hits:?}");
                for (i, &(depth, t)) in hits.iter().enumerate() {
                    assert_eq!(t, plan[i]);
                    let due = card_depth(i);
                    assert!(
                        depth == due || depth == due + 1,
                        "{t:?} at {depth}, due {due}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_fused_card_dream_blends_and_the_waking_dream_is_never_planned() {
        use DreamTheme::*;
        let mut d = DreamDirector::new(3);
        d.nightmare_every = 5;
        d.plan_cards(&[(Awakening, None), (Garden, Some(MyceliumGrove))]);
        assert_eq!(d.cards.len(), 1);
        while d.depth < 3 {
            d.descend();
        }
        assert!(d.card_dream);
        assert_eq!(d.blend, Some(MyceliumGrove));
        d.descend();
        assert_eq!(d.blend, None);
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
    fn white_dissolve_only_comes_after_going_deeper() {
        for seed in 0..30 {
            assert!(
                !descent(seed, 300).contains(&DreamTheme::WhiteDissolve),
                "seed {seed}: reached without going deeper"
            );
        }
        let hits = (0..30)
            .filter(|&seed| {
                let mut d = DreamDirector::new(seed);
                d.go_deeper();
                (0..200).any(|_| d.descend() == Some(DreamTheme::WhiteDissolve))
            })
            .count();
        assert!(hits >= 25, "only {hits}/30 deep runs found the white");
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
        let (mut hits, mut dreams) = (0, 0);
        for _ in 0..n {
            d.descend();
            if !d.nightmare {
                dreams += 1;
                hits += d.has_shard as u32;
            }
        }
        let rate = hits as f64 / dreams as f64;
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
    fn every_fifth_dream_is_a_nightmare_without_a_shard() {
        let mut d = DreamDirector::new(2);
        for _ in 0..3 {
            d.collect_shard();
        }
        assert!(d.lucid());
        for _ in 0..30 {
            d.descend();
            assert_eq!(
                d.nightmare,
                d.depth % NIGHTMARE_EVERY == 0,
                "depth {}",
                d.depth
            );
            if d.nightmare {
                assert!(!d.has_shard, "no wake door in a nightmare");
            } else {
                assert!(d.has_shard, "lucid dreams have the wake door");
            }
        }
        d.wake();
        assert!(!d.nightmare);
    }

    #[test]
    fn shard_bonus_makes_shards_commoner() {
        let mut d = DreamDirector::new(9);
        d.shard_bonus = 0.3;
        let n = 2000;
        let (mut hits, mut dreams) = (0, 0);
        for _ in 0..n {
            d.descend();
            if !d.nightmare {
                dreams += 1;
                hits += d.has_shard as u32;
            }
        }
        let rate = hits as f64 / dreams as f64;
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

    #[test]
    fn forced_themes_come_first() {
        let mut d = DreamDirector::new(3);
        d.forced = vec![DreamTheme::Garden, DreamTheme::MirrorHall];
        d.next = DreamTheme::Garden;
        assert_eq!(d.descend(), Some(DreamTheme::Garden));
        assert_eq!(
            d.next,
            DreamTheme::MirrorHall,
            "the portal previews the forced one"
        );
        assert_eq!(d.descend(), Some(DreamTheme::MirrorHall));
        assert!(d.forced.is_empty());
        d.descend().unwrap(); // back to rolling
    }
}
