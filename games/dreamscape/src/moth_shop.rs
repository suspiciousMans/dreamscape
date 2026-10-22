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
/// Most cards on the shelf.
pub const STOCK: usize = 3;

/// A moth line for the shelf (flagged for the user's approval).
pub const TITLE: &str = "THE MOTH'S ROOM";
pub const SUBTITLE: &str = "everything here is for dust. the moth is not sentimental.";
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

/// What is on the shelf: the usual rolled cards (never curses, never one
/// that would do nothing), at most [`STOCK`], the same for the same dream.
pub fn stock(seed: u64, run: &RunUpgrades, depth: u32) -> Vec<Upgrade> {
    let mut cards = crate::upgrades::roll_choices(
        seed ^ 0x0007_0B0C_5E11,
        run,
        Roll { depth, boss: false },
    );
    cards.retain(|u| price(u.tier()).is_some());
    cards.truncate(STOCK);
    cards
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
    shelf: &[Upgrade],
    index: usize,
    dust: u32,
    run: &RunUpgrades,
) -> Result<(Upgrade, u32), Refusal> {
    let &card = shelf.get(index).ok_or(Refusal::NoSuchCard)?;
    let cost = price(card.tier()).ok_or(Refusal::NoSuchCard)?;
    if !run.available(card) {
        return Err(Refusal::NoUse);
    }
    if dust < cost {
        return Err(Refusal::TooPoor);
    }
    Ok((card, cost))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dearer_tiers_cost_more_and_curses_are_not_for_sale() {
        assert!(PRICE_COMMON < PRICE_RARE && PRICE_RARE < PRICE_MYTHIC);
        assert_eq!(price(Tier::Cursed), None);
        assert_eq!(price(Tier::Rare), Some(PRICE_RARE));
    }

    #[test]
    fn the_shelf_is_small_deterministic_and_has_no_curses() {
        let run = RunUpgrades::default();
        let mut stocked = 0;
        for seed in 0..300 {
            for depth in [2, 8, 20] {
                let shelf = stock(seed, &run, depth);
                assert!(shelf.len() <= STOCK);
                assert_eq!(shelf, stock(seed, &run, depth));
                for u in &shelf {
                    assert!(price(u.tier()).is_some(), "{u:?}");
                    assert!(run.available(*u), "{u:?} would do nothing");
                }
                stocked += shelf.len();
            }
        }
        assert!(stocked > 600, "shelves are mostly bare: {stocked}");
    }

    #[test]
    fn buying_checks_the_purse_the_shelf_and_the_use() {
        let run = RunUpgrades::default();
        let shelf = stock(1, &run, 5);
        let first = shelf[0];
        let cost = price(first.tier()).unwrap();
        assert_eq!(buy(&shelf, 0, cost, &run), Ok((first, cost)));
        assert_eq!(buy(&shelf, 0, cost - 1, &run), Err(Refusal::TooPoor));
        assert_eq!(buy(&shelf, 9, 999, &run), Err(Refusal::NoSuchCard));
        let mut maxed = run.clone();
        for _ in 0..20 {
            maxed.take(first);
        }
        if !maxed.available(first) {
            assert_eq!(buy(&shelf, 0, 999, &maxed), Err(Refusal::NoUse));
        }
    }
}
