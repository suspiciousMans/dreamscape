//! The deck: the cards you built before the run. A hand's worth is drawn;
//! a card that is used is discarded, shuffled back into the draw pile, and
//! the empty slot draws a replacement after a short beat.
//!
//! Pure: no clock (`dt` in), no GL, cards are booklet numbers. The animation
//! layer reads [`DeckEvent`]s and the pile sizes; `main.rs` only wires.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Fewest and most cards a deck is built from.
pub const DECK_MIN: usize = 3;
pub const DECK_MAX: usize = 12;
/// Seconds between a card being used and its slot drawing the next one.
pub const DRAW_DELAY: f32 = 0.35;
/// Seconds a used card sits in the discard before it is shuffled back in.
/// Shorter than [`DRAW_DELAY`], so a tiny deck can draw the card again.
pub const SHUFFLE_DELAY: f32 = 0.30;
/// Mixed into the dream seed for the deck's shuffles.
pub const SEED_MIX: u64 = 0xDEC0_DEC0;

// === abilities3 ===
/// How many played cards RECALL can look back through.
pub const RECALL_MEMORY: usize = 8;
/// The share of a recalled card's cooldown that is still left.
pub const RECALL_COOLDOWN_LEFT: f32 = 0.5;
// === end abilities3 ===

// === upgrades2 ===
/// How run upgrades bend the deck (`RunUpgrades::deck_perks`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeckPerks {
    /// Multiplies [`DRAW_DELAY`] (smaller = sooner).
    pub draw_mult: f32,
    /// Multiplies [`SHUFFLE_DELAY`].
    pub shuffle_mult: f32,
    /// How many cards, the first used, go straight back, ready.
    pub free_reshuffles: u32,
}

impl Default for DeckPerks {
    fn default() -> Self {
        Self {
            draw_mult: 1.0,
            shuffle_mult: 1.0,
            free_reshuffles: 0,
        }
    }
}
// === end upgrades2 ===

/// What happened, for the animation layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckEvent {
    /// `card` was drawn into hand slot `slot`.
    Drawn { slot: usize, card: u32 },
    /// `card` was used out of `slot` and went to the discard.
    Played { slot: usize, card: u32 },
    /// `card` was shuffled from the discard back into the draw pile.
    Shuffled { card: u32 },
}

/// What [`Deck::play`] hands back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Played {
    pub card: u32,
    pub slot: usize,
}

#[derive(Clone, Debug)]
pub struct Deck {
    /// The draw pile; the top is the END.
    draw: Vec<u32>,
    /// One entry per hand slot; `None` while it waits to draw.
    hand: Vec<Option<u32>>,
    /// Used cards waiting (seconds left) to be shuffled back.
    discard: Vec<(u32, f32)>,
    /// Empty slots waiting (seconds left) to draw.
    waiting: Vec<(usize, f32)>,
    /// Seconds until each card can be used again. Travels with the card.
    cooldowns: Vec<(u32, f32)>,
    rng: StdRng,
    events: Vec<DeckEvent>,
    // === upgrades2 ===
    pub perks: DeckPerks,
    free_used: u32,
    // === end upgrades2 ===
    // === abilities3 ===
    /// Cards played, oldest first (RECALL looks back through it).
    played: Vec<u32>,
    // === end abilities3 ===
}

impl Deck {
    /// A shuffled deck of these cards (repeats dropped). `seed` is the dream
    /// seed: the deck mixes in [`SEED_MIX`] itself.
    pub fn new(cards: &[u32], seed: u64) -> Self {
        let mut unique: Vec<u32> = Vec::new();
        for &c in cards {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        let mut rng = StdRng::seed_from_u64(seed ^ SEED_MIX);
        // Fisher-Yates.
        for i in (1..unique.len()).rev() {
            unique.swap(i, rng.gen_range(0..=i));
        }
        Self {
            cooldowns: unique.iter().map(|&c| (c, 0.0)).collect(),
            draw: unique,
            hand: Vec::new(),
            discard: Vec::new(),
            waiting: Vec::new(),
            rng,
            events: Vec::new(),
            perks: DeckPerks::default(),
            free_used: 0,
            played: Vec::new(),
        }
    }

    /// Sizes the hand to `hand_size` slots and draws into every empty one
    /// that the draw pile can fill. A deck smaller than the hand leaves the
    /// extra slots empty.
    pub fn draw_to_fill(&mut self, hand_size: usize) {
        self.hand.resize(hand_size, None);
        for slot in 0..self.hand.len() {
            if self.hand[slot].is_none() && !self.waiting.iter().any(|w| w.0 == slot) {
                self.draw_into(slot);
            }
        }
    }

    fn draw_into(&mut self, slot: usize) -> bool {
        match self.draw.pop() {
            Some(card) => {
                self.hand[slot] = Some(card);
                self.events.push(DeckEvent::Drawn { slot, card });
                true
            }
            None => false,
        }
    }

    /// Uses the card in `slot`: it goes to the discard carrying `cooldown`,
    /// and the slot draws again after [`DRAW_DELAY`]. `None` for an empty slot.
    pub fn play(&mut self, slot: usize, cooldown: f32) -> Option<Played> {
        let card = self.hand.get_mut(slot)?.take()?;
        if let Some(c) = self.cooldowns.iter_mut().find(|c| c.0 == card) {
            c.1 = cooldown.max(0.0);
        }
        // === upgrades2 === a free reshuffle: back at once, no cooldown
        let free = self.free_used < self.perks.free_reshuffles;
        if free {
            self.free_used += 1;
            if let Some(c) = self.cooldowns.iter_mut().find(|c| c.0 == card) {
                c.1 = 0.0;
            }
        }
        let shuffle = if free { 0.0 } else { SHUFFLE_DELAY * self.perks.shuffle_mult };
        self.discard.push((card, shuffle));
        self.waiting.push((slot, DRAW_DELAY * self.perks.draw_mult));
        // === end upgrades2 ===
        self.events.push(DeckEvent::Played { slot, card });
        self.played.push(card); // === abilities3 ===
        if self.played.len() > RECALL_MEMORY {
            self.played.remove(0);
        }
        Some(Played { card, slot })
    }

    // === abilities3 ===
    /// MULLIGAN: every card in hand goes back into the draw pile, the pile
    /// is shuffled (the deck's own stream), and every empty slot draws.
    /// Returns the slots that were thrown back, with their cards.
    pub fn mulligan(&mut self) -> Vec<(usize, u32)> {
        let mut thrown = Vec::new();
        for slot in 0..self.hand.len() {
            if let Some(card) = self.hand[slot].take() {
                self.draw.push(card);
                self.events.push(DeckEvent::Played { slot, card });
                thrown.push((slot, card));
            }
        }
        for i in (1..self.draw.len()).rev() {
            self.draw.swap(i, self.rng.gen_range(0..=i));
        }
        if let Some(&(_, card)) = thrown.first() {
            self.events.push(DeckEvent::Shuffled { card });
        }
        for slot in 0..self.hand.len() {
            if self.hand[slot].is_none() && self.draw_into(slot) {
                self.waiting.retain(|w| w.0 != slot);
            }
        }
        thrown
    }

    /// The card RECALL would bring back: the most recently played card
    /// (other than `except`, RECALL's own) that is not already in hand.
    pub fn recall_candidate(&self, except: Option<u32>) -> Option<u32> {
        self.played
            .iter()
            .rev()
            .copied()
            .find(|&c| Some(c) != except && !self.hand.contains(&Some(c)))
    }

    /// RECALL: that card leaves the discard or the draw pile and lands in
    /// the first empty slot, with `RECALL_COOLDOWN_LEFT` of its cooldown
    /// left. `None` (nothing changes) when there is no card or no free slot.
    pub fn recall(&mut self, except: Option<u32>) -> Option<(usize, u32)> {
        let card = self.recall_candidate(except)?;
        let slot = self.hand.iter().position(|h| h.is_none())?;
        self.discard.retain(|d| d.0 != card);
        self.draw.retain(|&c| c != card);
        self.waiting.retain(|w| w.0 != slot);
        self.hand[slot] = Some(card);
        if let Some(c) = self.cooldowns.iter_mut().find(|c| c.0 == card) {
            c.1 *= RECALL_COOLDOWN_LEFT;
        }
        self.events.push(DeckEvent::Drawn { slot, card });
        Some((slot, card))
    }
    // === end abilities3 ===

    /// Advances every timer: cooldowns (all cards, wherever they are), the
    /// discard's shuffle-back, and the empty slots' draws.
    pub fn tick(&mut self, dt: f32) {
        for c in &mut self.cooldowns {
            c.1 = (c.1 - dt).max(0.0);
        }
        let mut back = Vec::new();
        for d in &mut self.discard {
            d.1 -= dt;
        }
        self.discard.retain(|&(card, left)| {
            if left <= 0.0 {
                back.push(card);
                false
            } else {
                true
            }
        });
        for card in back {
            // Anywhere but the top, so a card is not drawn straight back
            // unless it is the only one left.
            let at = if self.draw.is_empty() {
                0
            } else {
                self.rng.gen_range(0..self.draw.len())
            };
            self.draw.insert(at, card);
            self.events.push(DeckEvent::Shuffled { card });
        }
        for w in &mut self.waiting {
            w.1 -= dt;
        }
        let due: Vec<usize> = self
            .waiting
            .iter()
            .filter(|w| w.1 <= 0.0)
            .map(|w| w.0)
            .collect();
        for slot in due {
            if self.draw_into(slot) {
                self.waiting.retain(|w| w.0 != slot);
            }
            // Pile empty: the slot keeps waiting for the discard to return.
        }
    }

    /// The events since the last call, oldest first.
    pub fn events(&mut self) -> Vec<DeckEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn hand(&self) -> &[Option<u32>] {
        &self.hand
    }

    pub fn draw_len(&self) -> usize {
        self.draw.len()
    }

    #[cfg(test)]
    pub fn discard_len(&self) -> usize {
        self.discard.len()
    }

    /// Seconds until `card` can be used again (0 = ready).
    pub fn cooldown(&self, card: u32) -> f32 {
        self.cooldowns
            .iter()
            .find(|c| c.0 == card)
            .map_or(0.0, |c| c.1)
    }

    /// Every card in the deck, wherever it is.
    #[cfg(test)]
    pub fn total(&self) -> usize {
        self.cooldowns.len()
    }

    /// All card numbers currently held anywhere (draw, hand, discard).
    #[cfg(test)]
    pub fn all_cards(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.draw.clone();
        v.extend(self.hand.iter().flatten());
        v.extend(self.discard.iter().map(|d| d.0));
        v
    }
}

/// What a saved deck becomes: kept in order, repeats and cards the booklet
/// no longer holds dropped, trimmed to [`DECK_MAX`].
pub fn clean(saved: &[u32], owned: impl Fn(u32) -> bool) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &n in saved {
        if owned(n) && !out.contains(&n) && out.len() < DECK_MAX {
            out.push(n);
        }
    }
    out
}

/// Adds or removes `card` from a deck being built. Returns false if it was
/// refused (the deck is full).
pub fn toggle(deck: &mut Vec<u32>, card: u32) -> bool {
    if let Some(i) = deck.iter().position(|&c| c == card) {
        deck.remove(i);
        true
    } else if deck.len() < DECK_MAX {
        deck.push(card);
        true
    } else {
        false
    }
}

/// Cards a deck needs to start a run: [`DECK_MIN`], or every card you own
/// if you own fewer.
pub fn needed(owned: usize) -> usize {
    DECK_MIN.min(owned)
}

/// A deck can start a run when it has [`needed`] cards (none at all means
/// "let the loadout rule choose").
pub fn valid(len: usize, owned: usize) -> bool {
    len == 0 || (needed(owned)..=DECK_MAX).contains(&len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut v: Vec<u32>) -> Vec<u32> {
        v.sort_unstable();
        v
    }

    // === abilities3 ===
    #[test]
    fn mulligan_redeals_the_whole_hand_and_loses_nothing() {
        for seed in 0..20 {
            let mut d = Deck::new(&[1, 2, 3, 4, 5, 6, 7, 8], seed);
            d.draw_to_fill(5);
            d.events();
            let thrown = d.mulligan();
            assert_eq!(thrown.len(), 5);
            assert!(d.hand().iter().all(|h| h.is_some()), "a fresh full hand");
            assert_eq!(sorted(d.all_cards()), vec![1, 2, 3, 4, 5, 6, 7, 8], "nothing lost or doubled");
            let ev = d.events();
            assert_eq!(ev.iter().filter(|e| matches!(e, DeckEvent::Played { .. })).count(), 5);
            assert_eq!(ev.iter().filter(|e| matches!(e, DeckEvent::Drawn { .. })).count(), 5);
            assert!(ev.iter().any(|e| matches!(e, DeckEvent::Shuffled { .. })));
        }
    }

    #[test]
    fn mulligan_after_playing_keeps_the_played_card_in_the_discard() {
        let mut d = Deck::new(&[1, 2, 3, 4, 5, 6], 4);
        d.draw_to_fill(3);
        let used = d.hand()[0].unwrap();
        d.play(0, 5.0);
        d.mulligan();
        assert_eq!(d.hand().iter().flatten().count(), 3);
        assert!(!d.hand().contains(&Some(used)), "the played card is not redealt yet");
        assert_eq!(sorted(d.all_cards()), vec![1, 2, 3, 4, 5, 6]);
        d.tick(DRAW_DELAY + 1.0);
        assert_eq!(sorted(d.all_cards()), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn recall_brings_back_the_last_played_card_without_duplicating_any() {
        for seed in 0..30 {
            for wait in [0.0_f32, 0.1, 0.5, 2.0] {
                let mut d = Deck::new(&[1, 2, 3, 4, 5, 6, 7], seed);
                d.draw_to_fill(5);
                let first = d.hand()[1].unwrap();
                d.play(1, 8.0);
                d.tick(wait);
                if d.hand().contains(&Some(first)) {
                    continue; // it was drawn straight back: nothing to recall
                }
                let recall_card = d.hand()[3].unwrap();
                d.play(3, 14.0);
                let got = d.recall(Some(recall_card));
                let (slot, card) = got.expect("a played card to recall");
                assert_eq!(card, first, "the last card before Recall itself");
                assert_eq!(d.hand()[slot], Some(first));
                assert_eq!(sorted(d.all_cards()), vec![1, 2, 3, 4, 5, 6, 7], "seed {seed} wait {wait}");
                d.tick(5.0);
                assert_eq!(sorted(d.all_cards()), vec![1, 2, 3, 4, 5, 6, 7]);
                assert_eq!(d.all_cards().iter().filter(|&&c| c == first).count(), 1);
            }
        }
    }

    #[test]
    fn recall_has_nothing_to_take_before_anything_was_played_or_when_it_is_back_in_hand() {
        let mut d = Deck::new(&[1, 2, 3], 1);
        d.draw_to_fill(3);
        assert_eq!(d.recall_candidate(None), None);
        assert_eq!(d.recall(None), None);
        d.play(0, 1.0);
        let c = d.recall(None);
        assert!(c.is_some());
        assert_eq!(d.recall_candidate(None), None, "already in hand");
    }

    #[test]
    fn a_recalled_card_keeps_half_its_cooldown() {
        let mut d = Deck::new(&[1, 2, 3, 4], 2);
        d.draw_to_fill(3);
        let c = d.hand()[0].unwrap();
        d.play(0, 10.0);
        d.recall(None);
        assert!((d.cooldown(c) - 10.0 * RECALL_COOLDOWN_LEFT).abs() < 1e-4);
    }
    // === end abilities3 ===

    #[test]
    fn a_new_deck_fills_the_hand_from_the_top() {
        let mut d = Deck::new(&[1, 2, 3, 4, 5], 7);
        assert!(d.hand().is_empty());
        d.draw_to_fill(3);
        assert_eq!(d.hand().len(), 3);
        assert!(d.hand().iter().all(|h| h.is_some()));
        assert_eq!(d.draw_len(), 2);
        let ev = d.events();
        assert_eq!(ev.len(), 3);
        assert!(ev.iter().all(|e| matches!(e, DeckEvent::Drawn { .. })));
        assert!(d.events().is_empty(), "events drain");
    }

    #[test]
    fn repeats_are_dropped_and_a_small_deck_leaves_slots_empty() {
        let mut d = Deck::new(&[4, 4, 9], 1);
        assert_eq!(d.total(), 2);
        d.draw_to_fill(3);
        assert_eq!(d.hand().iter().flatten().count(), 2);
        assert_eq!(d.hand()[2], None);
    }

    #[test]
    fn the_same_seed_shuffles_the_same_way() {
        let a = Deck::new(&[1, 2, 3, 4, 5, 6, 7, 8], 99);
        let b = Deck::new(&[1, 2, 3, 4, 5, 6, 7, 8], 99);
        assert_eq!(a.draw, b.draw);
        let c = Deck::new(&[1, 2, 3, 4, 5, 6, 7, 8], 100);
        assert_ne!(a.draw, c.draw);
    }

    #[test]
    fn a_played_card_is_discarded_then_shuffled_back_then_a_new_card_is_drawn() {
        let mut d = Deck::new(&[1, 2, 3, 4, 5], 3);
        d.draw_to_fill(3);
        d.events();
        let before = d.hand()[1].unwrap();
        let played = d.play(1, 6.0).unwrap();
        assert_eq!(played, Played { card: before, slot: 1 });
        assert_eq!(d.hand()[1], None);
        assert_eq!(d.discard_len(), 1);
        assert_eq!(d.play(1, 1.0), None, "an empty slot plays nothing");
        d.tick(SHUFFLE_DELAY + 0.01);
        assert_eq!(d.discard_len(), 0, "back in the draw pile");
        assert_eq!(d.draw_len(), 3);
        assert_eq!(d.hand()[1], None, "the draw has not come yet");
        d.tick(DRAW_DELAY - SHUFFLE_DELAY);
        let now = d.hand()[1].expect("drew");
        assert_ne!(now, before, "not the card just played");
        let ev = d.events();
        assert!(matches!(ev[0], DeckEvent::Played { slot: 1, .. }));
        assert!(matches!(ev[1], DeckEvent::Shuffled { card } if card == before));
        assert!(matches!(ev[2], DeckEvent::Drawn { slot: 1, card } if card == now));
    }

    #[test]
    fn a_deck_as_small_as_the_hand_draws_the_same_card_back() {
        let mut d = Deck::new(&[1, 2, 3], 5);
        d.draw_to_fill(3);
        let c = d.hand()[0].unwrap();
        d.play(0, 0.0);
        d.tick(DRAW_DELAY + 0.01);
        assert_eq!(d.hand()[0], Some(c));
    }

    #[test]
    fn a_cards_cooldown_travels_with_it() {
        let mut d = Deck::new(&[1, 2, 3, 4], 8);
        d.draw_to_fill(3);
        let c = d.hand()[2].unwrap();
        d.play(2, 5.0);
        assert!((d.cooldown(c) - 5.0).abs() < 1e-6);
        d.tick(2.0);
        assert!((d.cooldown(c) - 3.0).abs() < 1e-5, "ticks while out of hand");
        d.tick(10.0);
        assert_eq!(d.cooldown(c), 0.0);
        assert_eq!(d.cooldown(999), 0.0);
    }

    #[test]
    fn an_empty_draw_pile_waits_for_the_discard() {
        let mut d = Deck::new(&[1, 2, 3], 2);
        d.draw_to_fill(3);
        d.play(0, 0.0);
        d.play(1, 0.0);
        assert_eq!(d.draw_len(), 0);
        d.tick(DRAW_DELAY + 0.01);
        assert_eq!(d.hand().iter().flatten().count(), 3, "both came back");
    }

    #[test]
    fn it_never_loses_or_duplicates_a_card() {
        for seed in 0..40u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let n = rng.gen_range(1..=DECK_MAX as u32);
            let cards: Vec<u32> = (10..10 + n).collect();
            let mut d = Deck::new(&cards, seed);
            d.draw_to_fill(rng.gen_range(1..=4));
            for _ in 0..300 {
                if rng.gen_bool(0.5) {
                    let slot = rng.gen_range(0..d.hand().len());
                    d.play(slot, rng.gen_range(0.0..8.0));
                }
                d.tick(rng.gen_range(0.0..0.5));
                assert_eq!(sorted(d.all_cards()), cards, "seed {seed}");
                d.events();
            }
        }
    }

    #[test]
    fn saved_decks_are_cleaned() {
        let kept = clean(&[3, 3, 9, 5, 7], |n| n != 9);
        assert_eq!(kept, vec![3, 5, 7]);
        let many: Vec<u32> = (0..30).collect();
        assert_eq!(clean(&many, |_| true).len(), DECK_MAX);
    }

    #[test]
    fn building_a_deck_adds_removes_and_stops_at_the_max() {
        let mut d = Vec::new();
        for n in 0..DECK_MAX as u32 {
            assert!(toggle(&mut d, n));
        }
        assert!(!toggle(&mut d, 99), "full");
        assert!(toggle(&mut d, 0), "removing always works");
        assert_eq!(d.len(), DECK_MAX - 1);
        assert!(!valid(2, 20) && valid(3, 20) && valid(12, 20) && !valid(13, 20));
        assert!(valid(0, 20), "empty = automatic");
        assert!(valid(1, 1) && valid(2, 2), "a small booklet can still start");
        assert_eq!(needed(1), 1);
        assert_eq!(needed(50), DECK_MIN);
    }

    // === upgrades2 ===
    #[test]
    fn perks_speed_up_the_draw_and_the_shuffle_back() {
        let mut d = Deck::new(&[1, 2, 3, 4, 5], 3);
        d.perks = DeckPerks { draw_mult: 0.5, shuffle_mult: 0.5, free_reshuffles: 0 };
        d.draw_to_fill(3);
        d.events();
        d.play(0, 5.0).unwrap();
        d.tick(DRAW_DELAY * 0.5 + 0.01);
        assert!(d.hand()[0].is_some(), "drew at half the delay");
        assert!(d.events().iter().any(|e| matches!(e, DeckEvent::Shuffled { .. })));
    }

    #[test]
    fn the_first_free_reshuffle_comes_back_ready_and_only_once() {
        let mut d = Deck::new(&[1, 2, 3, 4, 5], 3);
        d.perks = DeckPerks { free_reshuffles: 1, ..DeckPerks::default() };
        d.draw_to_fill(3);
        d.events();
        let a = d.play(0, 9.0).unwrap().card;
        assert_eq!(d.cooldown(a), 0.0);
        d.tick(0.0);
        assert_eq!(d.discard_len(), 0, "shuffled back at once");
        let b = d.play(1, 9.0).unwrap().card;
        assert_eq!(d.cooldown(b), 9.0, "the second use pays in full");
        assert_eq!(d.discard_len(), 1);
    }
    // === end upgrades2 ===
}
