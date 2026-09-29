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
pub const ALL: [&str; 29] = [
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
            for (at, id) in [(10, "depth_10"), (20, "depth_20"), (35, "depth_35")] {
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
    fn twenty_nine_distinct_named_achievements() {
        let mut ids = ALL.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 29);
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
        b.achievements.purchases = 1;
        let mut all: Vec<&str> = check(&b, None);
        for e in [
            Event::Woke {
                depth: 12,
                caught: 0,
                fell: 0,
                abilities_used: 0,
                daily: true,
                long_at: Some(10),
            },
            Event::ShardTaken,
            Event::Depth(40),
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
