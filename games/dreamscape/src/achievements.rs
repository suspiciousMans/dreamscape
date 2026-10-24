//! Achievements, exactly as designed in `ACHIEVEMENTS.md`. Pure: the game
//! reports events, `check` says what they unlock, and unlocks are kept in
//! the booklet so they work offline and without Steam (`steam.rs` mirrors
//! them to Steam when that feature is on).

use crate::cards::{Booklet, Rarity};
use crate::dream::{DreamTheme, ALL_ENEMY_KINDS, ALL_THEMES};
use crate::upgrades::Ability;
use serde::{Deserialize, Serialize};

/// Steamworks API names, in the order of the design doc.
#[cfg(test)]
pub const ALL: [&str; 45] = [
    "wake_once",
    "first_shard",
    "first_nightmare",
    "first_card",
    "first_purchase",
    "depth_10",
    "depth_20",
    "depth_35",
    "go_deeper",
    "white_dissolve",
    "all_dreams",
    "trip_dreams",
    "all_enemies",
    "prophetic_card",
    "full_booklet",
    "beat_quake",
    "beat_swarm_mother",
    "beat_eclipse",
    "untouched_nightmare",
    "clean_run",
    "no_abilities",
    "synergy",
    "four_synergies",
    "ascension_5",
    "ascension_10",
    "daily",
    "decoy_boss",
    "rewind_loop",
    "long_stare",
    // === achievements2 ===
    "first_fusion",
    "resonant_card",
    "lucid_card",
    "cards_25",
    "cards_50",
    "ten_dreams",
    "five_enemies",
    "bare_depth_5",
    "long_depth_15",
    "nightmares_5",
    "purchases_10",
    "dust_500",
    "early_wake",
    "deep_clean",
    "depth_50",
    // === end achievements2 ===
    // === handscore ===
    "full_house",
    // === end handscore ===
];

pub const TRIP_DREAMS: [DreamTheme; 6] = [
    DreamTheme::AfterimageFields,
    DreamTheme::SynesthesiaHall,
    DreamTheme::MeltingClockworks,
    DreamTheme::JellyfishSky,
    DreamTheme::WatchingWallpaper,
    DreamTheme::WhiteDissolve,
];

pub const FULL_BOOKLET: usize = 100;
pub const STARE_SECONDS: f32 = 10.0;

// === achievements2 === thresholds
pub const CARDS_25: usize = 25;
pub const CARDS_50: usize = 50;
pub const TEN_DREAMS: usize = 10;
pub const FIVE_ENEMIES: usize = 5;
pub const BARE_DEPTH: u32 = 5;
pub const LONG_DEPTH: u32 = 15;
pub const NIGHTMARES_5: u32 = 5;
pub const PURCHASES_10: u32 = 10;
pub const DUST_HOARD: u32 = 500;
/// A short run ended in at most this many seconds of dreaming.
pub const EARLY_WAKE_SECS: u32 = 90;
/// A clean wake at least this deep.
pub const DEEP_CLEAN_DEPTH: u32 = 12;
pub const DEPTH_50: u32 = 50;
// === end achievements2 ===

/// The display name (the doc's "Name" column).
pub fn name(id: &str) -> &'static str {
    match id {
        "wake_once" => "Lucid",
        "first_shard" => "Something Solid",
        "first_nightmare" => "Face It",
        "first_card" => "Keepsake",
        "first_purchase" => "Window Shopping, Then Not",
        "depth_10" => "Under",
        "depth_20" => "Deeper Under",
        "depth_35" => "Sleep Paralysis",
        "go_deeper" => "Not Yet",
        "white_dissolve" => "Blank Page",
        "all_dreams" => "Dream Cartographer",
        "trip_dreams" => "Afterimages",
        "all_enemies" => "Field Notes",
        "prophetic_card" => "Prophecy",
        "full_booklet" => "Scrapbook",
        "beat_quake" => "Aftershock",
        "beat_swarm_mother" => "Mother's Day",
        "beat_eclipse" => "Totality",
        "untouched_nightmare" => "Didn't Even Blink",
        "clean_run" => "Sound Sleeper",
        "no_abilities" => "Bare Hands",
        "synergy" => "Better Together",
        "four_synergies" => "Harmonics",
        "ascension_5" => "Restless",
        "ascension_10" => "Insomniac",
        "daily" => "Same Time Tomorrow",
        "decoy_boss" => "Nice Try",
        "rewind_loop" => "Déjà Vu",
        "long_stare" => "It Stares Back",
        // === achievements2 ===
        "first_fusion" => "Two Become One",
        "resonant_card" => "Resonance",
        "lucid_card" => "Clear as Day",
        "cards_25" => "Pocketful",
        "cards_50" => "Shoebox",
        "ten_dreams" => "Wanderer",
        "five_enemies" => "Acquaintances",
        "bare_depth_5" => "Empty Pockets",
        "long_depth_15" => "Marathon",
        "nightmares_5" => "Regular at the Door",
        "purchases_10" => "Regular Customer",
        "dust_500" => "Dust Bunny",
        "early_wake" => "Light Sleeper",
        "deep_clean" => "Untroubled",
        "depth_50" => "Bottomless",
        // === end achievements2 ===
        "full_house" => "Full House",
        _ => "",
    }
}

/// Something that happened, as far as achievements care.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Woke up from a run.
    Woke {
        depth: u32,
        caught: u32,
        fell: u32,
        abilities_used: u32,
        daily: bool,
        /// A long run's ascension level, if it was one.
        long_at: Option<u32>,
    },
    ShardTaken,
    /// Reached this depth in a run.
    Depth(u32),
    WentDeeper,
    NightmareBeaten {
        tier: u32,
        caught_in_it: u32,
    },
    /// Synergies held right now.
    Synergies(usize),
    AbilityUsed {
        ability: Ability,
        nightmare: bool,
        theme: DreamTheme,
    },
    /// Seconds spent looking straight at a stalker, first person.
    Stare(f32),
}

/// Unlocks earned so far (API names), saved in the booklet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Unlocked {
    pub ids: Vec<String>,
    /// Lucid Store purchases ever (`first_purchase`).
    pub purchases: u32,
}

impl Unlocked {
    pub fn has(&self, id: &str) -> bool {
        self.ids.iter().any(|i| i == id)
    }

    /// Records the new ones; returns just those.
    pub fn grant(&mut self, ids: Vec<&'static str>) -> Vec<&'static str> {
        let fresh: Vec<&'static str> = ids.into_iter().filter(|id| !self.has(id)).collect();
        for &id in &fresh {
            self.ids.push(id.to_string());
        }
        fresh
    }
}

/// Everything `e` (with the booklet as it is now) unlocks.
pub fn check(b: &Booklet, e: Option<Event>) -> Vec<&'static str> {
    let mut out = state(b);
    if let Some(e) = e {
        out.extend(event(e));
    }
    out
}

/// What the booklet alone proves.
fn state(b: &Booklet) -> Vec<&'static str> {
    let c = &b.codex;
    let seen = |t: DreamTheme| c.dreams.iter().any(|(d, _)| *d == t);
    let mut out = Vec::new();
    let mut when = |yes: bool, id| {
        if yes {
            out.push(id)
        }
    };
    when(b.card_count() > 0, "first_card");
    when(c.nightmares_beaten > 0, "first_nightmare");
    when(seen(DreamTheme::WhiteDissolve), "white_dissolve");
    when(
        ALL_THEMES
            .iter()
            .all(|&t| t == DreamTheme::Awakening || seen(t))
            && seen(DreamTheme::Awakening),
        "all_dreams",
    );
    when(TRIP_DREAMS.iter().all(|&t| seen(t)), "trip_dreams");
    when(
        ALL_ENEMY_KINDS.iter().all(|k| c.enemies.contains(k)),
        "all_enemies",
    );
    when(
        b.cards().any(|c| c.rarity == Rarity::Prophetic),
        "prophetic_card",
    );
    when(b.card_count() >= FULL_BOOKLET, "full_booklet");
    when(c.ascension_unlocked > 5, "ascension_5");
    when(b.achievements.purchases > 0, "first_purchase");
    // === achievements2 ===
    when(b.cards().any(|c| matches!(c.rarity, Rarity::Fused | Rarity::Resonant)), "first_fusion");
    when(b.cards().any(|c| c.rarity == Rarity::Resonant), "resonant_card");
    when(b.cards().any(|c| matches!(c.rarity, Rarity::Lucid | Rarity::Prophetic)), "lucid_card");
    when(b.card_count() >= CARDS_25, "cards_25");
    when(b.card_count() >= CARDS_50, "cards_50");
    when(c.dreams.len() >= TEN_DREAMS, "ten_dreams");
    when(c.enemies.len() >= FIVE_ENEMIES, "five_enemies");
    when(c.bare_wake_depth >= BARE_DEPTH, "bare_depth_5");
    when(c.deepest_long >= LONG_DEPTH, "long_depth_15");
    when(c.nightmares_beaten >= NIGHTMARES_5, "nightmares_5");
    when(b.achievements.purchases >= PURCHASES_10, "purchases_10");
    when(b.stash.dust >= DUST_HOARD, "dust_500");
    when(c.fastest_wake.is_some_and(|s| s <= EARLY_WAKE_SECS), "early_wake");
    // === end achievements2 ===
    when(b.best_hand.is_full_house_or_better(), "full_house"); // === handscore ===
    out
}

fn event(e: Event) -> Vec<&'static str> {
    let mut out = Vec::new();
    match e {
        Event::Woke {
            depth,
            caught,
            fell,
            abilities_used,
            daily,
            long_at,
        } => {
            out.push("wake_once");
            if long_at.is_some_and(|l| l >= 5) {
                out.push("ascension_5");
            }
            if long_at.is_some_and(|l| l >= 10) {
                out.push("ascension_10");
            }
            if caught == 0 && fell == 0 {
                out.push("clean_run");
                // === achievements2 ===
                if depth >= DEEP_CLEAN_DEPTH {
                    out.push("deep_clean");
                }
                // === end achievements2 ===
            }
            if depth >= 10 && abilities_used == 0 {
                out.push("no_abilities");
            }
            if daily {
                out.push("daily");
            }
        }
        Event::ShardTaken => out.push("first_shard"),
        Event::Depth(d) => {
            for (at, id) in [(10, "depth_10"), (20, "depth_20"), (35, "depth_35"), (DEPTH_50, "depth_50")] {
                if d >= at {
                    out.push(id);
                }
            }
        }
        Event::WentDeeper => out.push("go_deeper"),
        Event::NightmareBeaten { tier, caught_in_it } => {
            match tier {
                2 => out.push("beat_quake"),
                3 => out.push("beat_swarm_mother"),
                t if t >= 4 => out.push("beat_eclipse"),
                _ => {}
            }
            if tier >= 2 && caught_in_it == 0 {
                out.push("untouched_nightmare");
            }
        }
        Event::Synergies(n) => {
            if n >= 1 {
                out.push("synergy");
            }
            if n >= 3 {
                out.push("four_synergies");
            }
        }
        Event::AbilityUsed {
            ability,
            nightmare,
            theme,
        } => {
            if ability == Ability::Decoy && nightmare {
                out.push("decoy_boss");
            }
            if ability == Ability::Rewind && theme == DreamTheme::MeltingClockworks {
                out.push("rewind_loop");
            }
        }
        Event::Stare(s) => {
            if s >= STARE_SECONDS {
                out.push("long_stare");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::tests::record;
    use DreamTheme::*;

    fn got(b: &Booklet, e: Event) -> Vec<&'static str> {
        check(b, Some(e))
    }

    #[test]
    fn forty_five_distinct_named_achievements() {
        let mut ids = ALL.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 45);
        for id in ALL {
            assert!(id.len() <= 28 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{id}: Steam API name");
            assert!(name(id).len() <= 28, "{id}: name fits the toast");
        }
        for id in ALL {
            assert!(!name(id).is_empty(), "{id}");
        }
    }

    #[test]
    fn every_achievement_can_unlock() {
        let mut b = Booklet::default();
        let recs: Vec<_> = (0..FULL_BOOKLET as u64)
            .map(|i| record(ALL_THEMES[i as usize % 20], i, 1, 0.1, false))
            .collect();
        b.add_run(1, &recs);
        b.runs[0].cards[0].rarity = Rarity::Prophetic;
        for t in ALL_THEMES {
            b.codex.visit(t, 3);
        }
        for k in ALL_ENEMY_KINDS {
            b.codex.meet(k);
        }
        b.codex.nightmares_beaten = 1;
        b.codex.ascension_unlocked = 10;
        b.achievements.purchases = PURCHASES_10;
        // === achievements2 ===
        b.runs[0].cards[1].rarity = Rarity::Resonant;
        b.runs[0].cards[2].rarity = Rarity::Lucid;
        b.codex.bare_wake_depth = BARE_DEPTH;
        b.codex.deepest_long = LONG_DEPTH;
        b.codex.nightmares_beaten = NIGHTMARES_5;
        b.codex.fastest_wake = Some(EARLY_WAKE_SECS);
        b.stash.dust = DUST_HOARD;
        // === end achievements2 ===
        let mut all: Vec<&str> = check(&b, None);
        for e in [
            Event::Woke {
                depth: 50,
                caught: 0,
                fell: 0,
                abilities_used: 0,
                daily: true,
                long_at: Some(10),
            },
            Event::ShardTaken,
            Event::Depth(DEPTH_50),
            Event::WentDeeper,
            Event::NightmareBeaten {
                tier: 2,
                caught_in_it: 0,
            },
            Event::NightmareBeaten {
                tier: 3,
                caught_in_it: 1,
            },
            Event::NightmareBeaten {
                tier: 4,
                caught_in_it: 1,
            },
            Event::Synergies(3),
            Event::AbilityUsed {
                ability: Ability::Decoy,
                nightmare: true,
                theme: Lobby,
            },
            Event::AbilityUsed {
                ability: Ability::Rewind,
                nightmare: false,
                theme: MeltingClockworks,
            },
            Event::Stare(11.0),
        ] {
            all.extend(got(&b, e));
        }
        for id in ALL {
            assert!(all.contains(&id), "{id} never unlocks");
        }
    }

    // === achievements2 ===
    #[test]
    fn the_new_state_achievements_need_exactly_their_threshold() {
        let mut b = Booklet::default();
        let recs: Vec<_> = (0..CARDS_50 as u64)
            .map(|i| record(ALL_THEMES[i as usize % 20], i, 1, 0.1, false))
            .collect();
        b.add_run(1, &recs[..CARDS_25 - 1]);
        let has = |b: &Booklet, id: &str| check(b, None).contains(&id);
        assert!(!has(&b, "cards_25"));
        b.add_run(2, &recs[CARDS_25 - 1..CARDS_25]);
        assert!(has(&b, "cards_25") && !has(&b, "cards_50"));
        b.add_run(3, &recs[CARDS_25..]);
        assert!(has(&b, "cards_50"));
        // Fusion and resonance are different; a lucid card counts, a faint one does not.
        for r in &mut b.runs {
            for c in &mut r.cards {
                c.rarity = Rarity::Faint;
            }
        }
        assert!(!has(&b, "first_fusion") && !has(&b, "lucid_card"));
        b.runs[0].cards[0].rarity = Rarity::Fused;
        assert!(has(&b, "first_fusion") && !has(&b, "resonant_card"));
        b.runs[0].cards[0].rarity = Rarity::Resonant;
        assert!(has(&b, "first_fusion") && has(&b, "resonant_card"));
        b.runs[0].cards[1].rarity = Rarity::Lucid;
        assert!(has(&b, "lucid_card"));
        // Codex counters, one short then enough.
        for (id, short, enough) in [
            ("bare_depth_5", BARE_DEPTH - 1, BARE_DEPTH),
            ("long_depth_15", LONG_DEPTH - 1, LONG_DEPTH),
            ("nightmares_5", NIGHTMARES_5 - 1, NIGHTMARES_5),
        ] {
            let mut x = Booklet::default();
            let set = |x: &mut Booklet, n: u32| match id {
                "bare_depth_5" => x.codex.bare_wake_depth = n,
                "long_depth_15" => x.codex.deepest_long = n,
                _ => x.codex.nightmares_beaten = n,
            };
            set(&mut x, short);
            assert!(!has(&x, id), "{id}");
            set(&mut x, enough);
            assert!(has(&x, id), "{id}");
        }
        let mut x = Booklet::default();
        x.achievements.purchases = PURCHASES_10 - 1;
        x.stash.dust = DUST_HOARD - 1;
        x.codex.fastest_wake = Some(EARLY_WAKE_SECS + 1);
        for id in ["purchases_10", "dust_500", "early_wake"] {
            assert!(!has(&x, id), "{id}");
        }
        x.achievements.purchases += 1;
        x.stash.dust += 1;
        x.codex.fastest_wake = Some(EARLY_WAKE_SECS);
        for id in ["purchases_10", "dust_500", "early_wake"] {
            assert!(has(&x, id), "{id}");
        }
    }

    #[test]
    fn dreams_and_enemies_met_are_counted() {
        let mut b = Booklet::default();
        for &t in ALL_THEMES.iter().take(TEN_DREAMS - 1) {
            b.codex.visit(t, 1);
        }
        assert!(!check(&b, None).contains(&"ten_dreams"));
        b.codex.visit(ALL_THEMES[TEN_DREAMS - 1], 1);
        assert!(check(&b, None).contains(&"ten_dreams"));
        for &k in ALL_ENEMY_KINDS.iter().take(FIVE_ENEMIES - 1) {
            b.codex.meet(k);
        }
        assert!(!check(&b, None).contains(&"five_enemies"));
        b.codex.meet(ALL_ENEMY_KINDS[FIVE_ENEMIES - 1]);
        assert!(check(&b, None).contains(&"five_enemies"));
    }

    #[test]
    fn a_deep_clean_wake_and_depth_fifty_unlock_only_when_earned() {
        let b = Booklet::default();
        let woke = |depth, caught, fell| Event::Woke { depth, caught, fell, abilities_used: 1, daily: false, long_at: None };
        assert!(got(&b, woke(DEEP_CLEAN_DEPTH, 0, 0)).contains(&"deep_clean"));
        assert!(!got(&b, woke(DEEP_CLEAN_DEPTH - 1, 0, 0)).contains(&"deep_clean"));
        assert!(!got(&b, woke(30, 1, 0)).contains(&"deep_clean"));
        assert!(!got(&b, woke(30, 0, 1)).contains(&"deep_clean"));
        assert!(got(&b, Event::Depth(DEPTH_50)).contains(&"depth_50"));
        assert!(!got(&b, Event::Depth(DEPTH_50 - 1)).contains(&"depth_50"));
    }
    // === end achievements2 ===

    #[test]
    fn nothing_unlocks_for_nothing() {
        let b = Booklet::default();
        assert!(check(&b, None).is_empty());
        assert!(got(&b, Event::Depth(9)).is_empty());
        assert!(got(&b, Event::Stare(9.9)).is_empty());
        assert!(got(
            &b,
            Event::NightmareBeaten {
                tier: 1,
                caught_in_it: 0
            }
        )
        .is_empty());
        let woke = got(
            &b,
            Event::Woke {
                depth: 12,
                caught: 1,
                fell: 0,
                abilities_used: 2,
                daily: false,
                long_at: Some(4),
            },
        );
        assert_eq!(woke, vec!["wake_once"]);
        assert!(!check(&b, Some(Event::Depth(20))).contains(&"depth_35"));
    }

    #[test]
    fn grants_are_remembered_once_and_round_trip() {
        let mut u = Unlocked::default();
        assert_eq!(
            u.grant(vec!["wake_once", "first_shard"]),
            vec!["wake_once", "first_shard"]
        );
        assert!(u.grant(vec!["wake_once"]).is_empty());
        let b: Booklet = ron::from_str("(runs: [])").unwrap();
        assert!(b.achievements.ids.is_empty(), "old booklets load");
        let text = ron::to_string(&u).unwrap();
        assert_eq!(ron::from_str::<Unlocked>(&text).unwrap(), u);
    }
}
