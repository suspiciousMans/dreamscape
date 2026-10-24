//! Hand scoring: the cards in your hand score like a poker hand, and the
//! rank multiplies the dust you EARN (never the stash, prices or odds).
//! Pure: no clock, no GL. Everything is a named const.

use crate::cards::{Attribute, Card, Rarity};
use crate::dream::DreamTheme;
use crate::upgrades::Ability;
use serde::{Deserialize, Serialize};

/// A full hand (small decks hold fewer and score on what exists).
pub const FULL_HAND: usize = 5;

// Multipliers in hundredths (integers, so rounding is exact).
pub const NONE_PCT: u32 = 100;
pub const PAIR_PCT: u32 = 110;
pub const TWO_PAIR_PCT: u32 = 120;
pub const TRIO_PCT: u32 = 130;
pub const FULL_HOUSE_PCT: u32 = 145;
pub const QUAD_PCT: u32 = 160;
pub const FIVE_PCT: u32 = 180;
pub const FLUSH_PCT: u32 = 150;
pub const RAINBOW_PCT: u32 = 135;
pub const RESONANT_PCT: u32 = 140;
/// Hard cap on any multiplier (x1.80).
pub const SCORE_MAX_PCT: u32 = 180;
/// Seconds the tag's soft pulse lasts when the rank changes.
pub const PULSE_SECS: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandRank {
    #[default]
    None,
    Pair,
    TwoPair,
    Trio,
    FullHouse,
    Quad,
    FiveOfAKind,
    Flush,
    Rainbow,
    ResonantPair,
}

impl HandRank {
    pub fn name(self) -> &'static str {
        match self {
            HandRank::None => "NONE",
            HandRank::Pair => "PAIR",
            HandRank::TwoPair => "TWO PAIR",
            HandRank::Trio => "TRIO",
            HandRank::FullHouse => "FULL HOUSE",
            HandRank::Quad => "QUAD",
            HandRank::FiveOfAKind => "FIVE OF A KIND",
            HandRank::Flush => "FLUSH",
            HandRank::Rainbow => "RAINBOW",
            HandRank::ResonantPair => "RESONANT PAIR",
        }
    }

    pub fn pct(self) -> u32 {
        let p = match self {
            HandRank::None => NONE_PCT,
            HandRank::Pair => PAIR_PCT,
            HandRank::TwoPair => TWO_PAIR_PCT,
            HandRank::Trio => TRIO_PCT,
            HandRank::FullHouse => FULL_HOUSE_PCT,
            HandRank::Quad => QUAD_PCT,
            HandRank::FiveOfAKind => FIVE_PCT,
            HandRank::Flush => FLUSH_PCT,
            HandRank::Rainbow => RAINBOW_PCT,
            HandRank::ResonantPair => RESONANT_PCT,
        };
        p.clamp(NONE_PCT, SCORE_MAX_PCT)
    }

    /// "x1.30"
    pub fn mult_label(self) -> String {
        format!("x{}.{:02}", self.pct() / 100, self.pct() % 100)
    }

    /// Full house or anything that pays at least as much.
    pub fn is_full_house_or_better(self) -> bool {
        self.pct() >= FULL_HOUSE_PCT
    }

    /// The better of two ranks by multiplier (the first wins ties).
    pub fn best(self, other: HandRank) -> HandRank {
        if other.pct() > self.pct() {
            other
        } else {
            self
        }
    }
}

/// What the score reads off one card. A tool card is wild.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandFace {
    pub theme: DreamTheme,
    pub attribute: Attribute,
    pub rarity: Rarity,
    pub ability: Ability,
    pub wild: bool,
}

impl HandFace {
    pub fn of(card: &Card) -> Self {
        HandFace {
            theme: card.theme,
            attribute: card.attribute,
            rarity: card.rarity,
            ability: crate::powers::power_of(card).active,
            wild: false,
        }
    }

    /// A tool card: counts as any one attribute that helps, once per hand.
    pub fn wild(ability: Ability) -> Self {
        HandFace {
            theme: DreamTheme::Lobby,
            attribute: Attribute::Velvet,
            rarity: Rarity::Faint,
            ability,
            wild: true,
        }
    }
}

/// Attribute group sizes among real cards, biggest first, with the first
/// wild (if any) added to the biggest group.
fn groups(faces: &[HandFace]) -> Vec<usize> {
    let mut seen: Vec<(Attribute, usize)> = Vec::new();
    for f in faces.iter().filter(|f| !f.wild) {
        match seen.iter_mut().find(|(a, _)| *a == f.attribute) {
            Some(e) => e.1 += 1,
            None => seen.push((f.attribute, 1)),
        }
    }
    let mut counts: Vec<usize> = seen.into_iter().map(|e| e.1).collect();
    counts.sort_unstable_by(|a, b| b.cmp(a));
    if faces.iter().any(|f| f.wild) {
        match counts.first_mut() {
            Some(c) => *c += 1,
            None => counts.push(1),
        }
    }
    counts
}

/// The single best rank of what is in the hand (they never stack).
pub fn classify(hand: &[HandFace]) -> HandRank {
    let hand = &hand[..hand.len().min(FULL_HAND)];
    let g = groups(hand);
    let full = hand.len() == FULL_HAND;
    let real: Vec<&HandFace> = hand.iter().filter(|f| !f.wild).collect();
    let mut best = match (g.first().copied().unwrap_or(0), g.get(1).copied().unwrap_or(0)) {
        (n, _) if n >= 5 && full => HandRank::FiveOfAKind,
        (4, _) => HandRank::Quad,
        (3, 2) => HandRank::FullHouse,
        (3, _) => HandRank::Trio,
        (2, 2) => HandRank::TwoPair,
        (2, _) => HandRank::Pair,
        _ => HandRank::None,
    };
    if full && real.len() == FULL_HAND {
        if real.iter().all(|f| f.rarity == real[0].rarity) {
            best = best.best(HandRank::Flush);
        }
        let distinct = real
            .iter()
            .enumerate()
            .all(|(i, f)| real[..i].iter().all(|g| g.ability != f.ability));
        if distinct {
            best = best.best(HandRank::Rainbow);
        }
    }
    let resonant = real.iter().enumerate().any(|(i, a)| {
        real[i + 1..]
            .iter()
            .any(|b| crate::fusion::is_resonant(a.theme, b.theme))
    });
    if resonant {
        best = best.best(HandRank::ResonantPair);
    }
    best
}

/// The best rank any hand drawn from `deck` could reach (tool cards stay
/// out). A deck under five cards is judged whole.
pub fn best_in_deck(deck: &[HandFace]) -> HandRank {
    let deck: Vec<HandFace> = deck.iter().copied().filter(|f| !f.wild).collect();
    let k = deck.len().min(FULL_HAND);
    let mut best = HandRank::None;
    if k == 0 {
        return best;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        let hand: Vec<HandFace> = idx.iter().map(|&i| deck[i]).collect();
        best = best.best(classify(&hand));
        // Next combination in lexicographic order.
        let Some(i) = (0..k).rev().find(|&i| idx[i] != i + deck.len() - k) else {
            return best;
        };
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// Scales dust earned by a rank. Round half up; a rank above NONE always
/// adds at least +1 when the base is 1 or more. Never below `base`.
pub fn scale_dust(base: u32, rank: HandRank) -> u32 {
    let scaled = (base as u64 * rank.pct() as u64 + 50) / 100;
    let scaled = scaled as u32;
    if rank != HandRank::None && base >= 1 {
        scaled.max(base + 1)
    } else {
        scaled.max(base)
    }
}

/// The tag over the hand: the rank and its multiplier, or nothing.
pub fn tag(rank: HandRank) -> Option<(&'static str, String)> {
    (rank != HandRank::None).then(|| (rank.name(), rank.mult_label()))
}

/// The summary screen's line, or nothing when no hand ever scored.
pub fn summary_line(best: HandRank) -> Option<String> {
    (best != HandRank::None).then(|| format!("best hand: {}", best.name()))
}

/// The deck builder's line.
pub fn deck_line(best: HandRank) -> String {
    format!("best hand with this deck: {}", best.name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ability::*;
    use Attribute as A;

    // Resonant pairs: Garden<->MyceliumGrove; Lobby pairs with something else.
    fn f(a: Attribute, r: Rarity, ab: Ability) -> HandFace {
        HandFace { theme: DreamTheme::Lobby, attribute: a, rarity: r, ability: ab, wild: false }
    }
    const RARITIES: [Rarity; 5] = [Rarity::Faint, Rarity::Hazy, Rarity::Vivid, Rarity::Lucid, Rarity::Fused];
    /// Mixed rarities and one ability, so only the attributes score.
    fn hand(attrs: &[Attribute]) -> Vec<HandFace> {
        attrs.iter().enumerate().map(|(i, &a)| f(a, RARITIES[i % 5], Dash)).collect()
    }

    #[test]
    fn every_attribute_shape_ranks() {
        assert_eq!(classify(&hand(&[A::Velvet, A::Static, A::Void, A::Bloom, A::Rust])), HandRank::None);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Void, A::Bloom, A::Rust])), HandRank::Pair);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Void, A::Void, A::Rust])), HandRank::TwoPair);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Velvet, A::Void, A::Rust])), HandRank::Trio);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Velvet, A::Void, A::Void])), HandRank::FullHouse);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Velvet, A::Velvet, A::Rust])), HandRank::Quad);
        assert_eq!(classify(&hand(&[A::Velvet; 5])), HandRank::FiveOfAKind);
    }

    #[test]
    fn flush_rainbow_and_resonant() {
        let flush: Vec<_> = [Dash, Dash, Dash, Dash, Dash].iter().enumerate()
            .map(|(i, &ab)| f(ATTRS[i], Rarity::Vivid, ab)).collect();
        assert_eq!(classify(&flush), HandRank::Flush);
        let rainbow: Vec<_> = [Dash, Blink, Stillness, Decoy, Rewind].iter().enumerate()
            .map(|(i, &ab)| f(ATTRS[i], [Rarity::Faint, Rarity::Hazy, Rarity::Vivid, Rarity::Lucid, Rarity::Fused][i], ab)).collect();
        assert_eq!(classify(&rainbow), HandRank::Rainbow);
        let mut res = hand(&[A::Velvet, A::Static, A::Void]);
        res[0].theme = DreamTheme::Garden;
        res[1].theme = DreamTheme::MyceliumGrove;
        assert_eq!(classify(&res), HandRank::ResonantPair);
    }

    const ATTRS: [Attribute; 5] = [A::Velvet, A::Static, A::Void, A::Bloom, A::Rust];

    #[test]
    fn the_single_best_multiplier_wins_and_nothing_stacks() {
        // Full house (1.45) vs resonant (1.40): the house.
        let mut h = hand(&[A::Velvet, A::Velvet, A::Velvet, A::Void, A::Void]);
        h[0].theme = DreamTheme::Garden;
        h[3].theme = DreamTheme::MyceliumGrove;
        assert_eq!(classify(&h), HandRank::FullHouse);
        // Pair (1.10) vs resonant (1.40): the resonant pair.
        let mut h = hand(&[A::Velvet, A::Velvet, A::Void, A::Bloom, A::Rust]);
        h[2].theme = DreamTheme::Garden;
        h[3].theme = DreamTheme::MyceliumGrove;
        assert_eq!(classify(&h), HandRank::ResonantPair);
        // A quad of one rarity: quad (1.60) beats flush (1.50).
        let h: Vec<_> = [A::Velvet, A::Velvet, A::Velvet, A::Velvet, A::Rust].iter()
            .map(|&a| f(a, Rarity::Hazy, Dash)).collect();
        assert_eq!(classify(&h), HandRank::Quad);
        // Five of a kind that is also a flush is still x1.80, not more.
        let h: Vec<_> = (0..5).map(|_| f(A::Velvet, Rarity::Hazy, Dash)).collect();
        assert_eq!(classify(&h).pct(), SCORE_MAX_PCT);
    }

    #[test]
    fn wilds_count_once_for_the_attribute_that_helps() {
        let mut h = hand(&[A::Velvet, A::Velvet, A::Velvet, A::Velvet]);
        h.push(HandFace::wild(Dash));
        assert_eq!(classify(&h), HandRank::FiveOfAKind);
        let mut h = hand(&[A::Velvet, A::Velvet, A::Void, A::Void]);
        h.push(HandFace::wild(Dash));
        assert_eq!(classify(&h), HandRank::FullHouse);
        // Two wilds: the second is inert.
        let mut h = hand(&[A::Velvet, A::Velvet, A::Void]);
        h.push(HandFace::wild(Dash));
        h.push(HandFace::wild(Blink));
        assert_eq!(classify(&h), HandRank::Trio);
        // A wild is not a flush or rainbow card.
        let mut h: Vec<_> = (0..4).map(|i| f(ATTRS[i], Rarity::Vivid, Dash)).collect();
        h.push(HandFace::wild(Dash));
        assert_eq!(classify(&h), HandRank::Pair);
        assert_eq!(classify(&[HandFace::wild(Dash)]), HandRank::None);
    }

    #[test]
    fn small_hands_score_on_what_exists_and_never_below_one() {
        assert_eq!(classify(&[]), HandRank::None);
        assert_eq!(classify(&hand(&[A::Velvet, A::Velvet, A::Velvet])), HandRank::Trio);
        // Three same is not five of a kind, four same is a quad.
        assert_eq!(classify(&hand(&[A::Velvet; 4])), HandRank::Quad);
        for r in [HandRank::None, HandRank::Pair, HandRank::Flush, HandRank::FiveOfAKind, HandRank::ResonantPair] {
            assert!(r.pct() >= NONE_PCT && r.pct() <= SCORE_MAX_PCT);
        }
        assert_eq!(HandRank::FiveOfAKind.pct(), 180);
    }

    #[test]
    fn dust_rounds_half_up_with_a_plus_one_floor() {
        assert_eq!(scale_dust(10, HandRank::None), 10);
        assert_eq!(scale_dust(0, HandRank::FiveOfAKind), 0);
        assert_eq!(scale_dust(10, HandRank::Trio), 13);
        assert_eq!(scale_dust(5, HandRank::Pair), 6, "5.5 rounds up");
        assert_eq!(scale_dust(1, HandRank::Pair), 2, "at least +1");
        assert_eq!(scale_dust(3, HandRank::Pair), 4, "3.3 -> 3 raised to +1");
        assert_eq!(scale_dust(100, HandRank::FiveOfAKind), 180);
        assert_eq!(scale_dust(7, HandRank::FullHouse), 10, "10.15 rounds down");
    }

    #[test]
    fn deck_search_finds_the_best_hand_and_skips_tools() {
        let deck = hand(&[A::Velvet, A::Void, A::Velvet, A::Rust, A::Velvet, A::Void, A::Bloom]);
        assert_eq!(best_in_deck(&deck), HandRank::FullHouse);
        let mut with_tool = hand(&[A::Velvet, A::Velvet, A::Velvet, A::Velvet]);
        with_tool.push(HandFace::wild(Dash));
        assert_eq!(best_in_deck(&with_tool), HandRank::Quad, "tool cards excluded");
        assert_eq!(best_in_deck(&[]), HandRank::None);
        let twelve: Vec<_> = hand(&(0..12).map(|i| ATTRS[i % 5]).collect::<Vec<_>>());
        assert_eq!(best_in_deck(&twelve).pct() >= FULL_HOUSE_PCT, true);
        assert_eq!(best_in_deck(&hand(&[A::Velvet, A::Velvet, A::Void])), HandRank::Pair);
    }

    #[test]
    fn text_and_order() {
        assert_eq!(HandRank::Trio.mult_label(), "x1.30");
        assert_eq!(HandRank::FiveOfAKind.mult_label(), "x1.80");
        assert!(tag(HandRank::None).is_none());
        assert_eq!(tag(HandRank::FullHouse), Some(("FULL HOUSE", "x1.45".to_string())));
        assert_eq!(summary_line(HandRank::FullHouse).unwrap(), "best hand: FULL HOUSE");
        assert!(HandRank::Flush.is_full_house_or_better());
        assert!(!HandRank::Trio.is_full_house_or_better());
        assert_eq!(HandRank::Pair.best(HandRank::Trio), HandRank::Trio);
        assert_eq!(HandRank::Flush.best(HandRank::Rainbow), HandRank::Flush);
    }
}
