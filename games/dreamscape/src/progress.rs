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

pub const MAX_ASCENSION: u32 = 10;

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
        _ => "every dream harder than the last, from the start",
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
    pub peak_difficulty: f32,
    pub run_log: Vec<DreamRecord>,
    pub seen_kinds: Vec<EnemyKind>,
    pub stats: (u32, u32, u32, f32, u32),
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
        r.rerolls_used = self.rerolls_used;
        r
    }
}

pub const SAVE_PATH: &str = "games/dreamscape/saved_run.ron";

pub fn save_path() -> PathBuf {
    std::env::var_os("DREAMSCAPE_SAVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SAVE_PATH))
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
        }
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
            peak_difficulty: 2.5,
            run_log: vec![],
            seen_kinds: vec![EnemyKind::Stalker],
            stats: (1, 2, 0, 88.0, 0),
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
    fn saves_round_trip_and_missing_files_are_none() {
        let path = std::env::temp_dir().join(format!("ds_run_{}.ron", std::process::id()));
        let s = sample(3);
        save(&path, &s).unwrap();
        assert_eq!(load(&path), Some(s));
        clear(&path);
        assert_eq!(load(&path), None);
    }
}
