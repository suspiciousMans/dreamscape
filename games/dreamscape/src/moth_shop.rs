//! The moth's room behind a cracked wall: run upgrades for dust.
//!
//! Pure. The game shows the stock on the ordinary upgrade choice screen
//! (`upgrade_ui`), rolled by `upgrades::roll_choices`, and settles a
//! purchase with [`buy`]. There is no merging here; that stays in the
//! store's `[m]` screen.
//!
//! Why dust and not shards: `director.lucidity` is progress toward waking
//! (caught dreamers lose it, GO DEEPER raises the bar), so spending a shard
//! would undo waking. Only dust earned THIS run (`bonus_dust`) can be spent
//! here, never the banked stash, so a run that is quit or lost mid-shop
//! loses nothing permanent.

use crate::upgrades::{Roll, RunUpgrades, Tier, Upgrade};

/// Dust for a common card.
pub const PRICE_COMMON: u32 = 6;
/// Dust for a rare card.
pub const PRICE_RARE: u32 = 12;
/// Dust for a mythic card.
pub const PRICE_MYTHIC: u32 = 20;
/// Most cards on the shelf (the special slot is one of them).
pub const STOCK: usize = 5;
/// Dust for the first reroll of the shelf; each one costs [`REROLL_STEP`] more.
pub const REROLL_BASE: u32 = 3;
pub const REROLL_STEP: u32 = 2;
/// Percent of the normal price the special slot costs.
pub const SPECIAL_PCT: u32 = 60;
/// Every this-many depths the moth also opens a shop after the pick.
pub const SHOP_EVERY: u32 = 3;

/// A moth line for the shelf (flagged for the user's approval).
pub const TITLE: &str = "THE MOTH'S ROOM";
pub const SUBTITLE: &str = "everything here is for dust";
pub const EMPTY: &str = "THE MOTH HAS NOTHING LEFT FOR YOU";

/// Dust price of a card of this tier; curses are never for sale.
pub fn price(tier: Tier) -> Option<u32> {
    match tier {
        Tier::Common => Some(PRICE_COMMON),
        Tier::Rare => Some(PRICE_RARE),
        Tier::Mythic => Some(PRICE_MYTHIC),
        Tier::Cursed => None,
    }
}

/// Standing terms from perks (HAGGLER, DEEP SHELVES).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Terms {
    pub pct_off: u32,
    pub extra_stock: usize,
}

impl Terms {
    pub fn from_perks(fx: &crate::store::PerkFx) -> Terms {
        Terms {
            pct_off: fx.moth_pct_off,
            extra_stock: fx.shelf_bonus,
        }
    }

    /// `base` less the standing discount, never below 1 dust.
    pub fn apply(self, base: u32) -> u32 {
        (base * (100 - self.pct_off.min(90)) / 100).max(1)
    }
}

/// Should the moth open a shop after this pick? After a beaten nightmare,
/// and on every [`SHOP_EVERY`]th depth; never in a calm (sanctuary) dream.
pub fn due(boss_reward: bool, depth: u32, sanctuary: bool) -> bool {
    !sanctuary && (boss_reward || (depth > 0 && depth % SHOP_EVERY == 0))
}

/// What is on the shelf now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shelf {
    pub cards: Vec<Upgrade>,
    /// Index into `cards` of the discounted special, if any.
    pub special: Option<usize>,
    pub rerolls: u32,
}

impl Shelf {
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.cards.len() {
            return;
        }
        self.cards.remove(index);
        self.special = match self.special {
            Some(s) if s == index => None,
            Some(s) if s > index => Some(s - 1),
            other => other,
        };
    }
}

/// Dust for the next reroll after `done` rerolls.
pub fn reroll_price(done: u32) -> u32 {
    REROLL_BASE + REROLL_STEP * done
}

fn rolled(seed: u64, run: &RunUpgrades, depth: u32, salts: std::ops::Range<u64>) -> Vec<Upgrade> {
    let mut out: Vec<Upgrade> = Vec::new();
    for k in salts {
        let cards = crate::upgrades::roll_choices(
            seed ^ 0x0007_0B0C_5E11 ^ k.wrapping_mul(0x9E37_79B9),
            run,
            Roll { depth, boss: false },
        );
        for u in cards {
            if price(u.tier()).is_some() && !out.contains(&u) {
                out.push(u);
            }
        }
    }
    out
}

/// The usual rolled cards (never curses, never one that would do nothing)
/// plus one discounted special, at most `STOCK + extra_stock`, the same for
/// the same dream and the same number of rerolls.
pub fn stock(seed: u64, run: &RunUpgrades, depth: u32, terms: Terms) -> Shelf {
    build(seed, run, depth, terms, 0)
}

fn build(seed: u64, run: &RunUpgrades, depth: u32, terms: Terms, rerolls: u32) -> Shelf {
    let size = STOCK + terms.extra_stock;
    let r = rerolls as u64;
    // The special rotates with the dream only, so a reroll keeps it.
    let specials = rolled(seed ^ 0x5BEC_1A10, run, depth, 0..4);
    let special = specials
        .iter()
        .copied()
        .max_by_key(|u| (u.tier(), price(u.tier())))
        .filter(|u| u.tier() >= Tier::Rare)
        .or_else(|| specials.first().copied());
    let mut cards = rolled(seed, run, depth, r * 8..r * 8 + 8);
    cards.retain(|u| Some(*u) != special);
    // === abilities2 === a fixed-loadout run may be offered a new ability
    let offer = ability_offer(seed, run, rerolls);
    cards.truncate(size - special.is_some() as usize - offer.is_some() as usize);
    cards.extend(offer);
    // === end abilities2 ===
    let special_at = special.map(|u| {
        cards.push(u);
        cards.len() - 1
    });
    Shelf {
        cards,
        special: special_at,
        rerolls,
    }
}

// === abilities2 ===
/// One of the new abilities, if this run can still learn it (a deck hand
/// takes its abilities from its cards, so it never can). Same for the same
/// dream and the same number of rerolls.
pub fn ability_offer(seed: u64, run: &RunUpgrades, rerolls: u32) -> Option<Upgrade> {
    let open: Vec<Upgrade> = crate::abilities2::NEW_ABILITIES
        .iter()
        .map(|&a| Upgrade::Learn(a))
        .filter(|&u| run.available(u))
        .collect();
    if open.is_empty() {
        return None;
    }
    let h = (seed ^ 0xAB11_17E5).wrapping_add(rerolls as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    Some(open[(h >> 33) as usize % open.len()])
}
// === end abilities2 ===

/// Price of `shelf.cards[index]` with your terms (the special is cheaper).
pub fn price_at(shelf: &Shelf, index: usize, terms: Terms) -> Option<u32> {
    let card = shelf.cards.get(index)?;
    let mut cost = price(card.tier())?;
    if shelf.special == Some(index) {
        cost = (cost * SPECIAL_PCT).div_ceil(100).max(1);
    }
    Some(terms.apply(cost))
}

/// Buying `shelf[index]` with `dust`: the card and its price, or why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoSuchCard,
    TooPoor,
    /// Already maxed out (or otherwise no use) since it was shelved.
    NoUse,
}

pub fn buy(
    shelf: &Shelf,
    index: usize,
    dust: u32,
    run: &RunUpgrades,
    terms: Terms,
) -> Result<(Upgrade, u32), Refusal> {
    let &card = shelf.cards.get(index).ok_or(Refusal::NoSuchCard)?;
    let cost = price_at(shelf, index, terms).ok_or(Refusal::NoSuchCard)?;
    if !run.available(card) {
        return Err(Refusal::NoUse);
    }
    if dust < cost {
        return Err(Refusal::TooPoor);
    }
    Ok((card, cost))
}

/// Pay for a fresh set of cards (the special stays). Returns the new shelf
/// and the dust it cost, or `TooPoor`.
pub fn reroll(
    shelf: &Shelf,
    seed: u64,
    run: &RunUpgrades,
    depth: u32,
    dust: u32,
    terms: Terms,
) -> Result<(Shelf, u32), Refusal> {
    let cost = reroll_price(shelf.rerolls);
    if dust < cost {
        return Err(Refusal::TooPoor);
    }
    let mut next = build(seed, run, depth, terms, shelf.rerolls + 1);
    // A special that was already bought stays gone.
    if shelf.special.is_none() {
        if let Some(i) = next.special {
            next.remove(i);
        }
    }
    Ok((next, cost))
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Terms = Terms {
        pct_off: 0,
        extra_stock: 0,
    };

    #[test]
    fn dearer_tiers_cost_more_and_curses_are_not_for_sale() {
        assert!(PRICE_COMMON < PRICE_RARE && PRICE_RARE < PRICE_MYTHIC);
        assert_eq!(price(Tier::Cursed), None);
        assert_eq!(price(Tier::Rare), Some(PRICE_RARE));
    }

    #[test]
    fn the_shelf_is_deterministic_has_no_curses_and_usually_fills() {
        let run = RunUpgrades::default();
        let (mut stocked, mut full, mut n) = (0, 0, 0);
        for seed in 0..300 {
            for depth in [2, 8, 20] {
                let shelf = stock(seed, &run, depth, T);
                assert!(shelf.cards.len() <= STOCK);
                assert_eq!(shelf, stock(seed, &run, depth, T));
                for u in &shelf.cards {
                    assert!(price(u.tier()).is_some(), "{u:?}");
                    assert!(run.available(*u), "{u:?} would do nothing");
                }
                let mut seen = shelf.cards.clone();
                seen.dedup();
                assert_eq!(seen.len(), shelf.cards.len(), "duplicates on the shelf");
                stocked += shelf.cards.len();
                full += (shelf.cards.len() == STOCK) as usize;
                n += 1;
            }
        }
        assert!(stocked > n * 4, "shelves are mostly bare: {stocked}/{n}");
        assert!(full * 10 > n * 9, "{full}/{n} shelves are full");
    }

    #[test]
    fn a_fixed_loadout_shelf_offers_a_new_ability_and_a_deck_hand_never_does() {
        let run = RunUpgrades::default();
        let mut seen = std::collections::HashSet::new();
        for seed in 0..200 {
            let offer = ability_offer(seed, &run, 0).expect("a free dreamer can learn one");
            let Upgrade::Learn(a) = offer else { panic!("{offer:?}") };
            assert!(crate::abilities2::NEW_ABILITIES.contains(&a));
            assert!(stock(seed, &run, 5, T).cards.contains(&offer), "seed {seed}");
            assert!(price_at(&stock(seed, &run, 5, T), stock(seed, &run, 5, T).cards.iter().position(|c| *c == offer).unwrap(), T).is_some());
            seen.insert(a);
        }
        assert_eq!(seen.len(), 8, "every new ability gets sold");
        let mut hand = RunUpgrades::default();
        hand.deck_slots = 3;
        assert_eq!(ability_offer(1, &hand, 0), None);
        assert!(stock(1, &hand, 5, T).cards.iter().all(|u| !u.is_ability()));
    }

    #[test]
    fn deep_shelves_add_a_card() {
        let run = RunUpgrades::default();
        let deep = Terms {
            extra_stock: 1,
            ..T
        };
        let bigger = (0..100)
            .filter(|&s| stock(s, &run, 6, deep).cards.len() == STOCK + 1)
            .count();
        assert!(bigger > 90, "{bigger}");
    }

    #[test]
    fn the_special_is_cheaper_and_survives_removal() {
        let run = RunUpgrades::default();
        let shelf = stock(3, &run, 9, T);
        let at = shelf.special.expect("a special is shelved");
        let normal = price(shelf.cards[at].tier()).unwrap();
        let special = price_at(&shelf, at, T).unwrap();
        assert!(special < normal, "{special} vs {normal}");
        let mut s2 = shelf.clone();
        s2.remove(0);
        if at > 0 {
            assert_eq!(s2.special, Some(at - 1));
            assert_eq!(s2.cards[at - 1], shelf.cards[at]);
        }
        s2.remove(s2.special.unwrap_or(0));
        assert!(s2.special.is_none() || at == 0);
    }

    #[test]
    fn the_special_rotates_between_dreams() {
        let run = RunUpgrades::default();
        let mut seen = std::collections::HashSet::new();
        for seed in 0..40 {
            let s = stock(seed, &run, 6, T);
            if let Some(i) = s.special {
                seen.insert(s.cards[i]);
            }
        }
        assert!(seen.len() >= 3, "{}", seen.len());
    }

    #[test]
    fn haggler_cuts_prices() {
        let run = RunUpgrades::default();
        let shelf = stock(1, &run, 5, T);
        let h = Terms {
            pct_off: crate::store::HAGGLER_PCT,
            ..T
        };
        for i in 0..shelf.cards.len() {
            assert!(price_at(&shelf, i, h).unwrap() < price_at(&shelf, i, T).unwrap());
            assert!(price_at(&shelf, i, h).unwrap() >= 1);
        }
    }

    #[test]
    fn rerolls_cost_more_each_time_and_change_the_cards() {
        let run = RunUpgrades::default();
        assert!(reroll_price(1) > reroll_price(0));
        let shelf = stock(7, &run, 6, T);
        assert_eq!(
            reroll(&shelf, 7, &run, 6, reroll_price(0) - 1, T),
            Err(Refusal::TooPoor)
        );
        let (next, cost) = reroll(&shelf, 7, &run, 6, 99, T).unwrap();
        assert_eq!(cost, reroll_price(0));
        assert_eq!(next.rerolls, 1);
        assert_ne!(next.cards, shelf.cards);
        assert_eq!(
            next.special.map(|i| next.cards[i]),
            shelf.special.map(|i| shelf.cards[i]),
            "the special stays"
        );
        let (third, cost2) = reroll(&next, 7, &run, 6, 99, T).unwrap();
        assert_eq!(cost2, reroll_price(1));
        assert_ne!(third.cards, next.cards);
    }

    #[test]
    fn a_bought_special_does_not_come_back_on_reroll() {
        let run = RunUpgrades::default();
        let mut shelf = stock(7, &run, 6, T);
        shelf.remove(shelf.special.unwrap());
        let (next, _) = reroll(&shelf, 7, &run, 6, 99, T).unwrap();
        assert_eq!(next.special, None);
    }

    #[test]
    fn shops_open_after_nightmares_and_every_third_depth_but_not_when_calm() {
        assert!(due(true, 1, false));
        assert!(due(false, SHOP_EVERY, false));
        assert!(due(false, 2 * SHOP_EVERY, false));
        assert!(!due(false, 1, false) && !due(false, 2, false) && !due(false, 0, false));
        assert!(!due(true, SHOP_EVERY, true), "never in a sanctuary");
    }

    #[test]
    fn buying_checks_the_purse_the_shelf_and_the_use() {
        let run = RunUpgrades::default();
        let shelf = stock(1, &run, 5, T);
        let first = shelf.cards[0];
        let cost = price_at(&shelf, 0, T).unwrap();
        assert_eq!(buy(&shelf, 0, cost, &run, T), Ok((first, cost)));
        assert_eq!(buy(&shelf, 0, cost - 1, &run, T), Err(Refusal::TooPoor));
        assert_eq!(buy(&shelf, 9, 999, &run, T), Err(Refusal::NoSuchCard));
        let mut maxed = run.clone();
        for _ in 0..20 {
            maxed.take(first);
        }
        if !maxed.available(first) {
            assert_eq!(buy(&shelf, 0, 999, &maxed, T), Err(Refusal::NoUse));
        }
    }
}
