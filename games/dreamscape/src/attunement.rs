//! Attunement (roadmap 4.2): collect five cards of one attribute and the
//! dreamer is attuned to it for good. Every run then starts with one free
//! stack of that attribute's passive.
//!
//! (Named attunement rather than "resonance" so it isn't mistaken for the
//! RESONANT card rarity, which is what merging two matching dreams makes.)

use crate::cards::{Attribute, Card};
use crate::dream::{DreamTheme, ALL_THEMES};
use crate::upgrades::Upgrade;

/// Cards of one attribute needed to attune.
pub const NEEDED: usize = 5;

/// Every attribute, once.
#[cfg(test)]
pub fn all_attributes() -> Vec<Attribute> {
    let mut out: Vec<Attribute> = Vec::new();
    for &t in ALL_THEMES.iter() {
        let a = Attribute::of(t);
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

/// The passive an attunement gives: the one the attribute's dreams grant.
pub fn reward(a: Attribute) -> Upgrade {
    let theme: DreamTheme = ALL_THEMES
        .iter()
        .copied()
        .find(|&t| Attribute::of(t) == a)
        .expect("every attribute has a dream");
    crate::powers::power(theme).passive
}

/// (attribute, cards held) for every attribute with at least one card,
/// fullest first.
pub fn counts<'a>(cards: impl Iterator<Item = &'a Card>) -> Vec<(Attribute, usize)> {
    let mut out: Vec<(Attribute, usize)> = Vec::new();
    for c in cards {
        match out.iter_mut().find(|(a, _)| *a == c.attribute) {
            Some((_, n)) => *n += 1,
            None => out.push((c.attribute, 1)),
        }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

pub fn attuned(counts: &[(Attribute, usize)]) -> Vec<Attribute> {
    counts
        .iter()
        .filter(|(_, n)| *n >= NEEDED)
        .map(|(a, _)| *a)
        .collect()
}

/// The booklet's one-line summary, e.g. `ATTUNED  BLOOM +CALM MIND   next: TIDE 3/5`.
pub fn line(counts: &[(Attribute, usize)]) -> String {
    let done = attuned(counts);
    let mut parts: Vec<String> = Vec::new();
    if !done.is_empty() {
        let names: Vec<String> = done
            .iter()
            .map(|&a| format!("{} +{}", a.label(), reward(a).info().name))
            .collect();
        parts.push(format!("ATTUNED  {}", names.join(" · ")));
    }
    if let Some((a, n)) = counts.iter().find(|(_, n)| *n < NEEDED) {
        parts.push(format!("next: {} {}/{}", a.label(), n, NEEDED));
    }
    if parts.is_empty() {
        format!("attune to an attribute with {NEEDED} of its cards")
    } else {
        parts.join("   ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Booklet;

    fn card_of(theme: DreamTheme) -> Card {
        crate::cards::card_from(&crate::cards::tests::record(theme, 1, 1, 0.1, false))
    }

    #[test]
    fn every_attribute_has_a_reward_passive() {
        let attrs = all_attributes();
        assert_eq!(attrs.len(), ALL_THEMES.len());
        for a in attrs {
            let u = reward(a);
            assert!(!u.is_ability(), "{a:?} rewards an ability, not a passive");
        }
    }

    #[test]
    fn five_cards_of_one_attribute_attune_four_do_not() {
        let cards: Vec<Card> = (0..4).map(|_| card_of(DreamTheme::Garden)).collect();
        let c = counts(cards.iter());
        assert_eq!(c, vec![(Attribute::Bloom, 4)]);
        assert!(attuned(&c).is_empty());
        let mut cards = cards;
        cards.push(card_of(DreamTheme::Garden));
        assert_eq!(attuned(&counts(cards.iter())), vec![Attribute::Bloom]);
    }

    #[test]
    fn counts_are_fullest_first_and_keep_attributes_apart() {
        let mut cards: Vec<Card> = (0..2).map(|_| card_of(DreamTheme::Lobby)).collect();
        cards.extend((0..3).map(|_| card_of(DreamTheme::Garden)));
        let c = counts(cards.iter());
        assert_eq!(c[0], (Attribute::Bloom, 3));
        assert_eq!(c[1], (Attribute::Velvet, 2));
    }

    #[test]
    fn the_line_names_what_you_have_and_what_is_next() {
        assert!(line(&[]).contains("attune"));
        let c = vec![(Attribute::Bloom, 5), (Attribute::Velvet, 2)];
        let l = line(&c);
        assert!(l.contains("ATTUNED") && l.contains("BLOOM"));
        assert!(l.contains("next: VELVET 2/5"));
    }

    #[test]
    fn an_empty_booklet_attunes_to_nothing() {
        let b = Booklet::default();
        assert!(attuned(&counts(b.cards())).is_empty());
    }
}
