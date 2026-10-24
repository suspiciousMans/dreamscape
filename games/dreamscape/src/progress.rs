//! Meta-progression that lives between runs:
//! - **Ascension** levels 0..=10: each beaten long run unlocks the next,
//!   and each level adds one more twist of the knife (cumulative).
//! - **Daily dream**: one shared seed per calendar day, with its own best.
//! - **Codex**: every dream type and enemy you've met, and your best depth
//!   in each dream type (kept in the booklet file).
//! - **Saved run**: quitting mid-run saves it so you can continue later.
//!   It is written at the start of every dream, so continuing always puts
//!   you at the start of the dream you were in.

use crate::cards::DreamRecord;
use crate::dream::{DreamDirector, DreamTheme, EnemyKind, PropKind, RunLength};
use crate::upgrades::{RunUpgrades, Upgrade};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_ASCENSION: u32 = 16;

/// Rule levels (11+) leave depths 0 and 1 alone.
pub const RULES_FROM_DEPTH: u32 = 2;
/// The eye's clock at level 15, and what each level above 14 takes off it.
pub const EYE_SECONDS: f32 = 90.0;
pub const EYE_SHRINK: f32 = 3.0;
/// Level 12: how far from the remembered spot a watcher may stand, and how close to the start it may not.
pub const MEMORY_RADIUS: f32 = 14.0;
pub const MEMORY_START_CLEAR: f32 = 6.0;

/// One level's addition, for the title screen.
pub fn ascension_rule(level: u32) -> &'static str {
    match level {
        0 => "the dream as it is",
        1 => "one more shard to wake",
        2 => "enemies 10% faster",
        3 => "one fewer card at every pick",
        4 => "twists come sooner and more often",
        5 => "a nightmare every 4th dream",
        6 => "no free reroll",
        7 => "enemies notice you from further away",
        8 => "shards are rarer",
        9 => "half the safety after a respawn",
        10 => "every dream harder than the last, from the start",
        11 => "caught: back to the start, half your lucidity",
        12 => "the dream waits where you fell",
        13 => "one fewer ability",
        14 => "shard beacons and sense, halved",
        15 => "the eye is closing: slow, and it takes a card",
        _ => "everything at once: a twist every dream",
    }
}

/// Everything an ascension level changes, cumulative up to `level`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ascension(pub u32);

impl Ascension {
    fn at(self, n: u32) -> bool {
        self.0 >= n
    }
    pub fn extra_shards(self) -> u32 {
        self.at(1) as u32
    }
    pub fn enemy_speed(self) -> f32 {
        if self.at(2) {
            1.1
        } else {
            1.0
        }
    }
    pub fn fewer_cards(self) -> usize {
        self.at(3) as usize
    }
    pub fn twist_bonus(self) -> f64 {
        if self.at(4) {
            0.15
        } else {
            0.0
        }
    }
    pub fn nightmare_every(self) -> u32 {
        if self.at(5) {
            4
        } else {
            crate::dream::NIGHTMARE_EVERY
        }
    }
    pub fn free_rerolls(self) -> bool {
        !self.at(6)
    }
    pub fn alert(self) -> f32 {
        if self.at(7) {
            1.25
        } else {
            1.0
        }
    }
    pub fn shard_penalty(self) -> f64 {
        if self.at(8) {
            0.15
        } else {
            0.0
        }
    }
    pub fn grace(self) -> f32 {
        if self.at(9) {
            0.5
        } else {
            1.0
        }
    }
    pub fn always_hard(self) -> bool {
        self.at(10)
    }
    /// Rule levels apply from `RULES_FROM_DEPTH` on.
    pub fn rules_at(self, depth: u32) -> bool {
        depth >= RULES_FROM_DEPTH
    }
    /// 11: a catch sends you to the start with half your lucidity.
    pub fn no_safe_respawn(self, depth: u32) -> bool {
        self.at(11) && self.rules_at(depth)
    }
    pub fn halve_lucidity(lucidity: u32) -> u32 {
        lucidity / 2
    }
    /// 12: the dream remembers where you fell.
    pub fn remembers(self, depth: u32) -> bool {
        self.at(12) && self.rules_at(depth)
    }
    /// 13: loadout slots after the penalty (never below 1).
    pub fn loadout_slots(self, slots: usize) -> usize {
        if self.at(13) {
            slots.saturating_sub(1).max(1)
        } else {
            slots
        }
    }
    /// 14: multiplier on shard sense and beacon strength.
    pub fn pickup_factor(self) -> f64 {
        if self.at(14) {
            0.5
        } else {
            1.0
        }
    }
    /// 15: the eye's clock for this dream, if any.
    pub fn eye_timer(self, depth: u32) -> Option<f32> {
        (self.at(15) && self.rules_at(depth))
            .then(|| (EYE_SECONDS - EYE_SHRINK * (self.0 - 14) as f32).max(30.0))
    }
    /// 16: a twist is always active.
    pub fn always_twist(self, depth: u32) -> bool {
        self.at(16) && self.rules_at(depth)
    }
}

/// 12: where a watcher stands. Picks from `walkable` points within
/// `MEMORY_RADIUS` of where you were caught, never within
/// `MEMORY_START_CLEAR` of the start. Seeded; `None` when nothing fits.
pub fn remembered_spot(
    seed: u64,
    caught: (f32, f32),
    start: (f32, f32),
    walkable: &[(f32, f32)],
) -> Option<(f32, f32)> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    let d = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let ok: Vec<(f32, f32)> = walkable
        .iter()
        .copied()
        .filter(|&p| d(p, start) >= MEMORY_START_CLEAR)
        .collect();
    let near: Vec<(f32, f32)> = ok.iter().copied().filter(|&p| d(p, caught) <= MEMORY_RADIUS).collect();
    let pool = if near.is_empty() {
        // Nothing close: take the closest few instead (still bounded by the list).
        let mut by = ok;
        by.sort_by(|a, b| d(*a, caught).total_cmp(&d(*b, caught)));
        by.truncate(3);
        by
    } else {
        near
    };
    if pool.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0xA5CE_0012);
    Some(pool[rng.gen_range(0..pool.len())])
}

/// 15: failing the clock costs the newest picked card (abilities stay).
pub fn lose_last_card(taken: &mut Vec<Upgrade>) -> Option<Upgrade> {
    let i = taken.iter().rposition(|u| !matches!(u, Upgrade::Learn(_)))?;
    Some(taken.remove(i))
}

/// 16: a seeded twist from `options` (the caller passes the ones that fit).
pub fn forced_twist<T: Copy>(seed: u64, options: &[T]) -> Option<T> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    if options.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0xA5CE_0016);
    Some(options[rng.gen_range(0..options.len())])
}

/// Days since the Unix epoch (UTC) for a unix time in seconds.
pub fn day_number(unix_secs: u64) -> u64 {
    unix_secs / 86_400
}

/// The shared seed for a day's dream.
pub fn daily_seed(day: u64) -> u64 {
    day.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDA11_D2EA
}

/// What you've met, kept forever in the booklet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Codex {
    /// (dream type, deepest you've been in one).
    pub dreams: Vec<(DreamTheme, u32)>,
    pub enemies: Vec<EnemyKind>,
    pub nightmares_beaten: u32,
    /// Highest ascension level unlocked.
    pub ascension_unlocked: u32,
    /// (day number, best depth that day).
    pub daily_best: Option<(u64, u32)>,
    /// Quickest short-run wake, in seconds of dreaming.
    pub fastest_wake: Option<u32>,
    /// Deepest wake from a long run.
    pub deepest_long: u32,
    /// Deepest wake without taking a single upgrade.
    pub bare_wake_depth: u32,
    /// The form picked on the title screen.
    pub form: crate::form::Form,
}

/// What a wake set a new best in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewRecord {
    FastestWake,
    DeepestLong,
    BareWake,
}

impl NewRecord {
    /// Dust paid for it: a bonus the first time, a little for every
    /// improvement after.
    pub fn dust(self, first: bool) -> u32 {
        let base = match self {
            NewRecord::FastestWake => 40,
            NewRecord::DeepestLong => 60,
            NewRecord::BareWake => 50,
        };
        if first {
            base
        } else {
            base / 4
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NewRecord::FastestWake => "NEW RECORD: fastest wake",
            NewRecord::DeepestLong => "NEW RECORD: deepest long run",
            NewRecord::BareWake => "NEW RECORD: deepest wake with no upgrades",
        }
    }
}

impl Codex {
    /// Notes a visit; true if it's the first time in this dream type.
    pub fn visit(&mut self, theme: DreamTheme, depth: u32) -> bool {
        match self.dreams.iter_mut().find(|(t, _)| *t == theme) {
            Some((_, best)) => {
                *best = (*best).max(depth);
                false
            }
            None => {
                self.dreams.push((theme, depth));
                true
            }
        }
    }

    pub fn meet(&mut self, kind: EnemyKind) -> bool {
        if self.enemies.contains(&kind) {
            return false;
        }
        self.enemies.push(kind);
        true
    }

    pub fn best_in(&self, theme: DreamTheme) -> Option<u32> {
        self.dreams.iter().find(|(t, _)| *t == theme).map(|d| d.1)
    }

    /// Beating a long run at your highest level unlocks the next one.
    pub fn beat_long_run(&mut self, at_level: u32) -> bool {
        if at_level >= self.ascension_unlocked && self.ascension_unlocked < MAX_ASCENSION {
            self.ascension_unlocked = at_level + 1;
            return true;
        }
        false
    }

    /// Files a wake against the records; returns the ones it broke.
    /// The first wake of a kind always sets a record.
    pub fn record_wake(
        &mut self,
        long: bool,
        seconds: u32,
        depth: u32,
        upgrades: usize,
    ) -> Vec<(NewRecord, u32)> {
        let mut broke = Vec::new();
        if !long && self.fastest_wake.map_or(true, |best| seconds < best) {
            let first = self.fastest_wake.is_none();
            self.fastest_wake = Some(seconds);
            broke.push((NewRecord::FastestWake, NewRecord::FastestWake.dust(first)));
        }
        if long && depth > self.deepest_long {
            let first = self.deepest_long == 0;
            self.deepest_long = depth;
            broke.push((NewRecord::DeepestLong, NewRecord::DeepestLong.dust(first)));
        }
        if upgrades == 0 && depth > self.bare_wake_depth {
            let first = self.bare_wake_depth == 0;
            self.bare_wake_depth = depth;
            broke.push((NewRecord::BareWake, NewRecord::BareWake.dust(first)));
        }
        broke
    }

    pub fn record_daily(&mut self, day: u64, depth: u32) {
        match self.daily_best {
            Some((d, best)) if d == day => self.daily_best = Some((d, best.max(depth))),
            _ => self.daily_best = Some((day, depth)),
        }
    }

    pub fn daily_best_for(&self, day: u64) -> Option<u32> {
        self.daily_best.filter(|(d, _)| *d == day).map(|(_, b)| b)
    }
}

/// A run in progress, saved at the start of each dream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedRun {
    pub run_seed: u64,
    pub length: RunLengthSave,
    pub ascension: u32,
    pub daily: Option<u64>,
    // Director state (rebuilt by replaying descents, then overwritten).
    pub depth: u32,
    pub lucidity: u32,
    pub shards_to_wake: u32,
    pub hard_from: Option<u32>,
    pub overdrive: u32,
    pub has_shard: bool,
    pub nightmare: bool,
    pub motif: Option<PropKindSave>,
    // The run.
    pub taken: Vec<Upgrade>,
    pub anchors_used: u32,
    pub rerolls_used: u32,
    pub perks: Vec<crate::store::Perk>,
    pub shards_this_run: u32,
    pub bonus_dust: u32,
    // === pacts ===
    #[serde(default)]
    pub pacts: Vec<crate::pacts::Pact>,
    pub peak_difficulty: f32,
    pub run_log: Vec<DreamRecord>,
    pub seen_kinds: Vec<EnemyKind>,
    pub stats: (u32, u32, u32, f32, u32),
    /// The loadout's dreams (theme, fused theme), replanned on continue.
    #[serde(default)]
    pub card_dreams: Vec<(DreamTheme, Option<DreamTheme>)>,
    /// The companion's weight, if one rode along.
    #[serde(default)]
    pub burden: Option<crate::lore::Weight>,
    /// The starting kit's difficulty factor (older saves: none).
    #[serde(default)]
    pub loadout_scale: Option<f32>,
    /// Who the dreamer was (older saves: the crystal).
    #[serde(default)]
    pub form: crate::form::Form,
    /// === risk === coin flips, owed wins and the wager portal's state.
    #[serde(default)]
    pub gamble: crate::gamble::RunState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunLengthSave {
    Short,
    Long,
}

impl From<RunLength> for RunLengthSave {
    fn from(l: RunLength) -> Self {
        match l {
            RunLength::Short => RunLengthSave::Short,
            RunLength::Long => RunLengthSave::Long,
        }
    }
}

impl From<RunLengthSave> for RunLength {
    fn from(l: RunLengthSave) -> Self {
        match l {
            RunLengthSave::Short => RunLength::Short,
            RunLengthSave::Long => RunLength::Long,
        }
    }
}

/// `PropKind` isn't serde (it's pure generator data); saved by index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropKindSave(pub usize);

impl PropKindSave {
    pub fn from_kind(k: PropKind) -> Self {
        Self(
            crate::dream::ALL_PROP_KINDS
                .iter()
                .position(|&p| p == k)
                .unwrap_or(0),
        )
    }
    pub fn kind(self) -> PropKind {
        crate::dream::ALL_PROP_KINDS[self.0.min(crate::dream::ALL_PROP_KINDS.len() - 1)]
    }
}

impl SavedRun {
    /// Rebuilds the director: the same seed replayed through the same number
    /// of descents lands on the same dream types, then the rest is restored.
    pub fn director(&self) -> DreamDirector {
        let mut d = DreamDirector::with_length(self.run_seed, self.length.into());
        d.nightmare_every = Ascension(self.ascension).nightmare_every();
        d.plan_cards(&self.card_dreams);
        for _ in 0..self.depth {
            d.descend();
        }
        d.lucidity = self.lucidity;
        d.shards_to_wake = self.shards_to_wake;
        d.hard_from = self.hard_from;
        d.overdrive = self.overdrive;
        d.has_shard = self.has_shard;
        d.nightmare = self.nightmare;
        d
    }

    pub fn upgrades(&self) -> RunUpgrades {
        let mut r = RunUpgrades::default();
        for &u in &self.taken {
            r.take(u);
        }
        r.anchors_used = self.anchors_used;
        r.burden = self.burden;
        r.rerolls_used = self.rerolls_used;
        r
    }
}

pub fn save_path() -> PathBuf {
    crate::dev::var_os("DREAMSCAPE_SAVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::save_file("saved_run.ron"))
}

pub fn load(path: &Path) -> Option<SavedRun> {
    ron::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn save(path: &Path, run: &SavedRun) -> anyhow::Result<()> {
    std::fs::write(
        path,
        ron::ser::to_string_pretty(run, ron::ser::PrettyConfig::default())?,
    )?;
    Ok(())
}

pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    #[test]
    fn wakes_file_records_and_only_better_ones_break_them() {
        use super::*;
        let kinds = |v: Vec<(NewRecord, u32)>| v.into_iter().map(|r| r.0).collect::<Vec<_>>();
        let mut c = Codex::default();
        assert_eq!(kinds(c.record_wake(false, 300, 3, 2)), vec![NewRecord::FastestWake]);
        assert!(c.record_wake(false, 300, 3, 2).is_empty());
        assert_eq!(kinds(c.record_wake(false, 250, 3, 1)), vec![NewRecord::FastestWake]);
        // Long runs track depth, never time.
        assert_eq!(kinds(c.record_wake(true, 9999, 9, 4)), vec![NewRecord::DeepestLong]);
        assert!(c.record_wake(true, 1, 9, 4).is_empty());
        // No upgrades: its own record, any run length.
        assert_eq!(
            kinds(c.record_wake(true, 1, 12, 0)),
            vec![NewRecord::DeepestLong, NewRecord::BareWake]
        );
        assert_eq!((c.bare_wake_depth, c.deepest_long), (12, 12));
    }

    #[test]
    fn the_first_record_pays_more_than_beating_it() {
        use super::*;
        let mut c = Codex::default();
        let first = c.record_wake(false, 300, 3, 2)[0].1;
        let again = c.record_wake(false, 200, 3, 2)[0].1;
        assert_eq!(first, NewRecord::FastestWake.dust(true));
        assert!(first > again && again > 0);
        for r in [NewRecord::FastestWake, NewRecord::DeepestLong, NewRecord::BareWake] {
            assert!(r.dust(true) > r.dust(false) && r.dust(false) > 0, "{r:?}");
        }
    }

    #[test]
    fn old_booklets_without_records_load_with_none() {
        let c: super::Codex = ron::from_str("(nightmares_beaten: 2)").unwrap();
        assert_eq!((c.fastest_wake, c.deepest_long, c.bare_wake_depth), (None, 0, 0));
    }

    use super::*;
    use crate::upgrades::Ability;

    #[test]
    fn ascension_rules_stack() {
        let a0 = Ascension(0);
        let a10 = Ascension(10);
        assert_eq!(a0.extra_shards(), 0);
        assert_eq!(a10.extra_shards(), 1);
        assert!(a10.enemy_speed() > 1.0 && a10.alert() > 1.0 && a10.grace() < 1.0);
        assert!(a10.nightmare_every() < a0.nightmare_every());
        assert!(!a10.free_rerolls() && a0.free_rerolls());
        assert!(a10.always_hard() && !Ascension(9).always_hard());
        assert!(Ascension(5).twist_bonus() > 0.0 && Ascension(3).twist_bonus() == 0.0);
        for l in 0..=MAX_ASCENSION {
            assert!(!ascension_rule(l).is_empty());
            assert!(ascension_rule(l).len() <= 48, "short enough to fit");
        }
    }

    #[test]
    fn rule_levels_11_to_16_stack_and_spare_the_start() {
        assert_eq!(MAX_ASCENSION, 16);
        assert!(!Ascension(10).no_safe_respawn(5) && Ascension(11).no_safe_respawn(5));
        assert!(!Ascension(11).no_safe_respawn(1), "depth 0-1 are gentle");
        assert_eq!(Ascension::halve_lucidity(5), 2);
        assert!(!Ascension(11).remembers(5) && Ascension(12).remembers(5));
        assert_eq!(Ascension(12).loadout_slots(3), 3);
        assert_eq!(Ascension(13).loadout_slots(3), 2);
        assert_eq!(Ascension(16).loadout_slots(1), 1);
        assert_eq!(Ascension(13).pickup_factor(), 1.0);
        assert_eq!(Ascension(14).pickup_factor(), 0.5);
        assert_eq!(Ascension(14).eye_timer(5), None);
        assert_eq!(Ascension(15).eye_timer(5), Some(87.0));
        assert_eq!(Ascension(16).eye_timer(5), Some(84.0));
        assert_eq!(Ascension(16).eye_timer(1), None);
        assert!(!Ascension(15).always_twist(5) && Ascension(16).always_twist(5));
        assert!(Ascension(16).no_safe_respawn(2) && Ascension(16).remembers(2));
    }

    #[test]
    fn remembered_spot_is_seeded_bounded_and_off_the_start() {
        let walk: Vec<(f32, f32)> = (0..30).map(|i| (i as f32 * 2.0, 0.0)).collect();
        let start = (0.0, 0.0);
        for seed in 0..50 {
            let p = remembered_spot(seed, (30.0, 0.0), start, &walk).unwrap();
            assert!(p.0 >= MEMORY_START_CLEAR);
            assert!((p.0 - 30.0).abs() <= MEMORY_RADIUS);
            assert_eq!(Some(p), remembered_spot(seed, (30.0, 0.0), start, &walk));
        }
        // caught on the start cell: still never spawns there
        let p = remembered_spot(1, start, start, &walk).unwrap();
        assert!(p.0 >= MEMORY_START_CLEAR);
        assert_eq!(remembered_spot(1, start, start, &[(1.0, 0.0)]), None);
        assert_eq!(remembered_spot(1, start, start, &[]), None);
    }

    #[test]
    fn eye_failure_costs_newest_card_not_abilities() {
        let mut t = vec![Upgrade::ShardSense, Upgrade::Learn(Ability::Ward), Upgrade::WideEyes];
        assert_eq!(lose_last_card(&mut t), Some(Upgrade::WideEyes));
        assert_eq!(lose_last_card(&mut t), Some(Upgrade::ShardSense));
        assert_eq!(lose_last_card(&mut t), None);
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn forced_twist_is_seeded() {
        let o = [1, 2, 3, 4];
        assert_eq!(forced_twist(9, &o), forced_twist(9, &o));
        assert_eq!(forced_twist::<u8>(9, &[]), None);
        assert!((0..20).map(|s| forced_twist(s, &o).unwrap()).collect::<std::collections::HashSet<_>>().len() > 1);
    }

    #[test]
    fn beating_long_runs_unlocks_levels_one_at_a_time() {
        let mut c = Codex::default();
        assert!(c.beat_long_run(0));
        assert_eq!(c.ascension_unlocked, 1);
        assert!(
            !c.beat_long_run(0),
            "replaying a lower level unlocks nothing"
        );
        for l in 1..20 {
            c.beat_long_run(l);
        }
        assert_eq!(c.ascension_unlocked, MAX_ASCENSION);
    }

    #[test]
    fn daily_seed_is_stable_per_day_and_best_resets_daily() {
        let day = day_number(1_760_000_000);
        assert_eq!(day, day_number(1_760_000_000 + 3600));
        assert_ne!(daily_seed(day), daily_seed(day + 1));
        let mut c = Codex::default();
        c.record_daily(day, 5);
        c.record_daily(day, 3);
        assert_eq!(c.daily_best_for(day), Some(5));
        c.record_daily(day + 1, 2);
        assert_eq!(c.daily_best_for(day + 1), Some(2));
        assert_eq!(c.daily_best_for(day), None);
    }

    #[test]
    fn the_codex_remembers_firsts_and_bests() {
        let mut c = Codex::default();
        assert!(c.visit(DreamTheme::Garden, 3));
        assert!(!c.visit(DreamTheme::Garden, 7));
        assert!(!c.visit(DreamTheme::Garden, 2));
        assert_eq!(c.best_in(DreamTheme::Garden), Some(7));
        assert!(c.meet(EnemyKind::Mimic));
        assert!(!c.meet(EnemyKind::Mimic));
        let text = ron::to_string(&c).unwrap();
        assert_eq!(ron::from_str::<Codex>(&text).unwrap(), c);
        assert_eq!(ron::from_str::<Codex>("()").unwrap(), Codex::default());
    }

    fn sample(depth: u32) -> SavedRun {
        SavedRun {
            run_seed: 77,
            length: RunLengthSave::Long,
            ascension: 2,
            daily: None,
            depth,
            lucidity: 2,
            shards_to_wake: 6,
            hard_from: Some(0),
            overdrive: 0,
            has_shard: true,
            nightmare: false,
            motif: Some(PropKindSave::from_kind(PropKind::Tree)),
            taken: vec![
                Upgrade::SwiftFeet,
                Upgrade::Learn(Ability::Dash),
                Upgrade::Learn(Ability::Blink),
                Upgrade::Learn(Ability::Phase),
            ],
            anchors_used: 0,
            rerolls_used: 1,
            perks: vec![],
            shards_this_run: 2,
            bonus_dust: 9,
            pacts: vec![],
            peak_difficulty: 2.5,
            run_log: vec![],
            seen_kinds: vec![EnemyKind::Stalker],
            stats: (1, 2, 0, 88.0, 0),
            card_dreams: Vec::new(),
            burden: None,
            loadout_scale: None,
            form: Default::default(),
            gamble: Default::default(),
        }
    }

    #[test]
    fn a_saved_run_rebuilds_the_same_director_and_upgrades() {
        let s = sample(7);
        let mut live = DreamDirector::with_length(77, RunLength::Long);
        for _ in 0..7 {
            live.descend();
        }
        let d = s.director();
        assert_eq!(
            (d.depth, d.theme, d.next),
            (live.depth, live.theme, live.next)
        );
        assert_eq!(d.dream_seed(), live.dream_seed());
        assert_eq!(d.lucidity, 2);
        let r = s.upgrades();
        let held: Vec<Ability> = r.abilities.iter().map(|a| a.ability).collect();
        assert_eq!(held, vec![Ability::Blink, Ability::Phase]);
        assert_eq!(r.rerolls_used, 1);
        assert_eq!(s.motif.unwrap().kind(), PropKind::Tree);
    }

    #[test]
    fn a_continued_run_keeps_its_card_dreams() {
        use crate::dream::DreamTheme::*;
        let mut s = sample(7);
        s.card_dreams = vec![(Garden, None), (MirrorHall, Some(TheTunnel))];
        let mut live = DreamDirector::with_length(77, RunLength::Long);
        live.nightmare_every = Ascension(2).nightmare_every();
        live.plan_cards(&s.card_dreams);
        for _ in 0..7 {
            live.descend();
        }
        let d = s.director();
        assert_eq!((d.theme, d.next), (live.theme, live.next));
        assert_eq!(d.cards, live.cards);
    }

    #[test]
    fn saves_round_trip_and_missing_files_are_none() {
        let path = std::env::temp_dir().join(format!("ds_run_{}.ron", std::process::id()));
        let s = sample(3);
        save(&path, &s).unwrap();
        assert_eq!(load(&path), Some(s));
        clear(&path);
        assert_eq!(load(&path), None);
    }
}
