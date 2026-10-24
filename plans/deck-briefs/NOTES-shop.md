# NOTES: SHOP stream (branch feat/shop-items)

## Built
- **Moth shop replaces the merge popup.** `moth_after_nightmare()` now opens `show_moth_shop` when `moth_shop::due(boss_reward, depth, sanctuary)`; not in autopilot/prologue/co-op. Called from `choose` (upgrade pick, and the pact option) and `skip_choice`, before `boss_reward` is cleared, so once per pick. Leaving ([x]/esc) returns to Playing. Merge is still on `[m]` in the Lucid Store and the title.
- **Every `SHOP_EVERY = 3` depths** the same shop opens after the pick (const in `moth_shop.rs`; never in sanctuary dreams). A nightmare on a depth divisible by 3 opens one shop, not two. Shelf is the dream's shelf, so a secret-room visit and the post-pick shop share stock.
- **moth_shop.rs**: `STOCK` 3 -> 5 (special is one of the 5), `Shelf {cards, special, rerolls}`, reroll for dust (`3 + 2n`, special stays, a bought special does not return), rotating special (best rare/mythic of 4 rolls, 60% price), `Terms {pct_off, extra_stock}` from perks. Reroll is the last card in the choice view (also key `R`); number keys now go to 7.
- **6 new run perks** (`store::PerkFx::of(&perks)`, one place): POCKET DUST (+8 start dust), HAGGLER (-25% moth prices), DEEP SHELVES (+1 shelf card), SPARE DIE (+1 pick reroll), SHRUG (+5 dust on skip), BOUNTY (x2 nightmare dust). All wired in main.rs (`// === shop ===` blocks).
- **Catalog 25 -> 56**: perks 13, eyes 12 (+6), crystals 10 (+5), frames 7 (+4), HUD inks 8 (+4), new shelf `Trinkets` = card backs (6 incl. default Dusk). New cosmetics are painted: eye irises (hud.rs), crystal colours, frame colours (`booklet_ui::style_frame`), inks, back preview (`shop_ui::card_back_pattern`, `CardBack::colors()`).
- `Stash.card_back` (`#[serde(default)]`), `Item::Back`; test `an_old_booklet_stash_loses_nothing` loads an old-shape RON and checks dust, charges, owned and equipped items.
- Store UI: shelves of >7 items scroll (`window_start`, `<` `>` arrows, `n/len` label), 6 shelves fit (store design height 600 -> 680). Choice cards with 5+ cards are taller and shrink their text.
- Dev hooks: `DREAMSCAPE_OPEN=store:<shelf>:<col>`, `DREAMSCAPE_SCREEN=moth` (autopilot, stops auto-picking).
- Screenshots: `plans/deck-briefs/shots-shop/` (moth room, six shelves).

## Prices
Typical run pays ~67 dust (`run_dust(6,3,4)`); every perk <= 50 (test), cosmetics 25-120.

## Tests added
moth_shop (shelf size/determinism, deep shelves, special, haggler, reroll, bought special, `due`), store (catalog size/prices, PerkFx, card backs, old-save migration), shop_ui (window scroll), main (reward -> shop -> leave -> Playing and never Merge; skip path; autopilot excluded; depth shop; buy/reroll spend run dust only; perks; every shelf cursor draws; fullest shelf fits).
`any_pair` is now `#[cfg(test)]` (merge_ui.rs, one line) since nothing calls it at runtime.

## NOT wired / unverified
- `CardBack` is stored and shown in the store only; the DECK stream must paint `lore/stash.card_back` (`CardBack::colors()`, `shop_ui::card_back_pattern` are ready).
- No Upgrade variants added (per brief). A nightmare whose reward pool is empty skips the shop (`open_upgrade_choice` returns early).
- A sold-out shelf closes the shop (the reroll card is unreachable then).
- New perks' start effects (POCKET DUST, SPARE DIE) are in `begin_run`, not exercised by a test (begin_run needs GL); BOUNTY is read at the portal, also untested beyond `PerkFx`.
- Full suite not run (owner does at merge); focused filters pass: shop, store, pick, upgrade_ui, audit_text.
