//! Every dream gets a name and a whisper, grown from its theme's grammar.
//!
//! Names come from a handful of templates ("THE {ADJ} {NOUN}", "THE {NOUN}
//! THAT {REL}", "THE {NOUN} OF {ABSTRACT}", ...) filled from very large
//! per-theme word lists. About one dream in four is a *mashup*: one slot is
//! borrowed from a different dream, so an office can be "THE FLUORESCENT
//! ORCHARD" and the garden can whisper "the printer is breathing again".
//! Awakening never mashes: waking up should feel clean.

use super::theme::DreamTheme;
use super::vocab::{vocab, Vocab, ABSTRACTS, CODAS};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

pub const MAX_NAME: usize = 40;
pub const MAX_WHISPER: usize = 56;
/// Chance a (non-Awakening) dream borrows one word from another dream.
pub const MASHUP_CHANCE: f64 = 0.25;

const DONORS: [DreamTheme; 13] = [
    DreamTheme::Lobby,
    DreamTheme::LiminalOffice,
    DreamTheme::VoidPlatforms,
    DreamTheme::Garden,
    DreamTheme::NightmareFactory,
    DreamTheme::CursedForest,
    DreamTheme::DrownedLibrary,
    DreamTheme::SkyStairs,
    DreamTheme::MirrorHall,
    DreamTheme::MyceliumGrove,
    DreamTheme::TheTunnel,
    DreamTheme::FractalCathedral,
    DreamTheme::Elfworks,
];

fn pick(list: &'static [&'static str], rng: &mut StdRng) -> &'static str {
    list.choose(rng).expect("vocab lists are non-empty (tests)")
}

/// The theme's own vocab, and (for mashups) a donor dream's.
fn sources(theme: DreamTheme, rng: &mut StdRng) -> (Vocab, Vocab) {
    let own = vocab(theme);
    if theme != DreamTheme::Awakening && rng.gen_bool(MASHUP_CHANCE) {
        let donor = *DONORS
            .iter()
            .filter(|&&d| d != theme)
            .collect::<Vec<_>>()
            .choose(rng)
            .expect("many donors");
        (own, vocab(*donor))
    } else {
        (own, vocab(theme))
    }
}

fn name_once(theme: DreamTheme, rng: &mut StdRng) -> String {
    let (own, other) = sources(theme, rng);
    // A mashup swaps exactly one slot: which one is random.
    let swap = rng.gen_range(0..3);
    let adj = pick(
        if swap == 0 {
            other.adjectives
        } else {
            own.adjectives
        },
        rng,
    );
    let noun = pick(if swap == 1 { other.nouns } else { own.nouns }, rng);
    let rel = pick(
        if swap == 2 {
            other.relatives
        } else {
            own.relatives
        },
        rng,
    );
    let abs = if rng.gen_bool(0.5) {
        pick(own.abstracts, rng)
    } else {
        pick(ABSTRACTS, rng)
    };
    match rng.gen_range(0..10) {
        0..=3 => format!("THE {adj} {noun}"),
        4..=5 => format!("THE {noun} THAT {rel}"),
        6 => format!("THE {noun} OF {abs}"),
        7 => format!("THE {adj} {noun} OF {abs}"),
        8 => format!("SOMEONE ELSE'S {noun}"),
        _ => format!("THE LAST {noun} BEFORE {abs}"),
    }
}

fn whisper_once(theme: DreamTheme, rng: &mut StdRng) -> String {
    let (own, other) = sources(theme, rng);
    let subject = pick(
        if rng.gen_bool(0.5) {
            other.subjects
        } else {
            own.subjects
        },
        rng,
    );
    let predicate = pick(own.predicates, rng);
    match rng.gen_range(0..10) {
        0..=1 => pick(own.whispers, rng).to_string(),
        2..=7 => format!("{subject} {predicate}"),
        _ => format!("{subject} {predicate}. {}", pick(CODAS, rng)),
    }
}

/// Generates until the line fits (the first few tries almost always do);
/// falls back to the theme's shortest canned line so it can never fail.
fn fitted(
    theme: DreamTheme,
    rng: &mut StdRng,
    max: usize,
    gen: fn(DreamTheme, &mut StdRng) -> String,
    fallback: fn(&Vocab) -> &'static str,
) -> String {
    for _ in 0..16 {
        let s = gen(theme, rng);
        if s.chars().count() <= max {
            return s;
        }
    }
    fallback(&vocab(theme)).to_string()
}

/// Mix the theme in, or every theme walks the same RNG path for a seed
/// (same template, same abstract noun) and the lists feel tiny.
fn theme_seed(theme: DreamTheme, seed: u64) -> u64 {
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (theme as u64 + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93)
}

pub fn dream_name(theme: DreamTheme, seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(theme_seed(theme, seed) ^ 0x0D2E_A11E);
    fitted(theme, &mut rng, MAX_NAME, name_once, |v| v.nouns[0])
}

pub fn whisper(theme: DreamTheme, seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(theme_seed(theme, seed) ^ 0x005E_C2E7);
    fitted(theme, &mut rng, MAX_WHISPER, whisper_once, |v| {
        v.whispers[0]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::theme::ALL_THEMES;
    use std::collections::HashSet;

    #[test]
    fn names_and_whispers_are_deterministic_and_well_shaped() {
        for t in ALL_THEMES {
            for s in 0..400 {
                let n = dream_name(t, s);
                assert_eq!(n, dream_name(t, s));
                assert_eq!(n, n.to_uppercase(), "{n}");
                assert!(
                    n.chars().count() <= MAX_NAME,
                    "{n} is too long for the title card"
                );
                assert!(!n.contains("  ") && !n.ends_with(' '), "{n:?}");
                let w = whisper(t, s);
                assert_eq!(w, whisper(t, s));
                assert!(!w.is_empty() && w.chars().count() <= MAX_WHISPER, "{w}");
                assert_eq!(w, w.to_lowercase(), "{w}");
            }
        }
    }

    #[test]
    fn names_and_whispers_vary_a_lot() {
        for t in ALL_THEMES {
            let names: HashSet<String> = (0..300).map(|s| dream_name(t, s)).collect();
            let whispers: HashSet<String> = (0..300).map(|s| whisper(t, s)).collect();
            assert!(
                names.len() >= 200,
                "{t:?}: only {} names / 300",
                names.len()
            );
            assert!(
                whispers.len() >= 180,
                "{t:?}: only {} whispers / 300",
                whispers.len()
            );
        }
    }

    #[test]
    fn every_template_shows_up() {
        let names: Vec<String> = ALL_THEMES
            .iter()
            .flat_map(|&t| (0..300).map(move |s| dream_name(t, s)))
            .collect();
        for needle in [" THAT ", " OF ", "SOMEONE ELSE'S ", "THE LAST ", " BEFORE "] {
            assert!(
                names.iter().any(|n| n.contains(needle)),
                "no name contains {needle:?}"
            );
        }
    }

    #[test]
    fn some_dreams_borrow_words_from_other_dreams() {
        let office_nouns: HashSet<&str> = vocab(DreamTheme::LiminalOffice)
            .nouns
            .iter()
            .copied()
            .collect();
        let garden_names: Vec<String> = (0..2000)
            .map(|s| dream_name(DreamTheme::Garden, s))
            .collect();
        let borrowed = garden_names
            .iter()
            .filter(|n| office_nouns.iter().any(|w| n.contains(&format!(" {w}"))))
            .count();
        assert!(
            borrowed > 15,
            "garden never dreamed of the office ({borrowed})"
        );
        assert!(borrowed < 600, "garden is mostly office ({borrowed})");
    }

    #[test]
    fn themes_do_not_move_in_lockstep() {
        // Same seed, different themes: templates should differ at least sometimes.
        let shape = |n: &str| {
            n.contains(" THAT ") as u8
                | (n.contains(" OF ") as u8) << 1
                | (n.contains("BEFORE") as u8) << 2
                | (n.starts_with("SOMEONE") as u8) << 3
        };
        let mut differ = 0;
        for s in 0..200 {
            let a = shape(&dream_name(DreamTheme::Garden, s));
            let b = shape(&dream_name(DreamTheme::NightmareFactory, s));
            differ += (a != b) as u32;
        }
        assert!(
            differ > 40,
            "garden and factory share a template {}/200 times",
            200 - differ
        );
    }

    #[test]
    fn awakening_never_mashes() {
        let own: HashSet<&str> = vocab(DreamTheme::Awakening).nouns.iter().copied().collect();
        for s in 0..300 {
            let n = dream_name(DreamTheme::Awakening, s);
            assert!(own.iter().any(|w| n.contains(w)), "{n}");
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        for t in ALL_THEMES {
            for s in 0..6 {
                println!(
                    "{t:?} | {} | {}",
                    dream_name(t, s * 7919),
                    whisper(t, s * 7919)
                );
            }
        }
    }
}
