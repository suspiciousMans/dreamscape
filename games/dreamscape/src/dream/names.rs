//! Every dream gets a name and a whisper drawn from its theme's vocabulary.
//! Shown on the title card and written into the dream journal.

use super::theme::DreamTheme;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

struct Vocab {
    adjectives: &'static [&'static str],
    nouns: &'static [&'static str],
    whispers: &'static [&'static str],
}

fn vocab(theme: DreamTheme) -> Vocab {
    use DreamTheme::*;
    match theme {
        Lobby => Vocab {
            adjectives: &["WAITING", "VELVET", "PATIENT", "HUSHED", "PASTEL"],
            nouns: &["LOBBY", "ANTEROOM", "FOYER", "RECEPTION", "CORRIDOR"],
            whispers: &[
                "someone is expected",
                "the bell never rings",
                "take a number. any number.",
                "you have been here before",
            ],
        },
        LiminalOffice => Vocab {
            adjectives: &["FLUORESCENT", "ENDLESS", "CARPETED", "HUMMING", "VACANT"],
            nouns: &["OFFICE", "CUBICLES", "BREAKROOM", "ANNEX", "FLOOR ZERO"],
            whispers: &[
                "the printer is warm. no one printed.",
                "your desk is still here",
                "it is always 3:14 pm",
                "the hum knows your name",
            ],
        },
        VoidPlatforms => Vocab {
            adjectives: &["WEIGHTLESS", "SHATTERED", "STARLESS", "DRIFTING", "HOLLOW"],
            nouns: &[
                "ARCHIPELAGO",
                "STAIRWELL",
                "NOTHING",
                "CONSTELLATION",
                "SHORELINE",
            ],
            whispers: &[
                "don't look down. there is no down.",
                "the gaps are wider than they look",
                "something is falling with you",
                "step where the light is",
            ],
        },
        Garden => Vocab {
            adjectives: &["OVERGROWN", "BREATHING", "CANDIED", "WHISPERING", "SUNLESS"],
            nouns: &["GARDEN", "ORCHARD", "HEDGEROW", "GREENHOUSE", "MEADOW"],
            whispers: &[
                "the flowers turn to watch you",
                "it smells like a birthday",
                "the hedges moved again",
                "pick nothing",
            ],
        },
        NightmareFactory => Vocab {
            adjectives: &["RUSTED", "GRINDING", "SLEEPLESS", "FEVERED", "HUNGRY"],
            nouns: &["FACTORY", "FOUNDRY", "ASSEMBLY LINE", "BOILER ROOM", "MILL"],
            whispers: &[
                "the machines are making you",
                "keep moving. they count heartbeats.",
                "production is ahead of schedule",
                "do not become the product",
            ],
        },
        Awakening => Vocab {
            adjectives: &["MORNING", "GOLDEN", "QUIET", "WARM", "FIRST"],
            nouns: &["LIGHT", "ROOM", "WINDOW", "BREATH", "HOUR"],
            whispers: &[
                "you remember your name",
                "the alarm is ringing, softly",
                "it was only a dream. mostly.",
                "welcome back",
            ],
        },
    }
}

fn pick(list: &'static [&'static str], rng: &mut StdRng) -> &'static str {
    list.choose(rng).expect("vocab lists are non-empty (tests)")
}

pub fn dream_name(theme: DreamTheme, seed: u64) -> String {
    let v = vocab(theme);
    let mut rng = StdRng::seed_from_u64(seed ^ 0x0D2E_A11E);
    format!(
        "THE {} {}",
        pick(v.adjectives, &mut rng),
        pick(v.nouns, &mut rng)
    )
}

pub fn whisper(theme: DreamTheme, seed: u64) -> String {
    let v = vocab(theme);
    let mut rng = StdRng::seed_from_u64(seed ^ 0x005E_C2E7);
    pick(v.whispers, &mut rng).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::theme::ALL_THEMES;
    use std::collections::HashSet;

    #[test]
    fn names_and_whispers_are_deterministic_and_well_shaped() {
        for t in ALL_THEMES {
            for s in 0..50 {
                let n = dream_name(t, s);
                assert_eq!(n, dream_name(t, s));
                assert!(n.starts_with("THE "), "{n}");
                assert_eq!(n, n.to_uppercase(), "{n}");
                assert!(n.len() <= 32, "{n} is too long for the title card");
                let w = whisper(t, s);
                assert!(!w.is_empty() && w.len() <= 48, "{w}");
                assert_eq!(w, w.to_lowercase(), "{w}");
            }
        }
    }

    #[test]
    fn names_vary() {
        for t in ALL_THEMES {
            let distinct: HashSet<String> = (0..60).map(|s| dream_name(t, s)).collect();
            assert!(distinct.len() >= 8, "{t:?}: only {} names", distinct.len());
        }
    }

    #[test]
    fn each_theme_has_its_own_nouns() {
        let mut seen = HashSet::new();
        for t in ALL_THEMES {
            for n in vocab(t).nouns {
                assert!(seen.insert(*n), "{t:?} reuses noun {n}");
            }
        }
    }

    #[test]
    fn every_list_is_non_empty() {
        for t in ALL_THEMES {
            let v = vocab(t);
            assert!(
                !v.adjectives.is_empty() && !v.nouns.is_empty() && !v.whispers.is_empty(),
                "{t:?}"
            );
        }
    }
}
