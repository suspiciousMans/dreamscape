# NOTES: smallfixes
- Card back: `HandPile.back` (None for the default Dusk back = unchanged). Non-default backs paint on the stack layers, riffle sheets and the first 40% of a draw flight (`hand::BACK_FLIGHT`); reduced motion unchanged. `shop_ui::card_back_cell` is the shared pattern rule.
- Sold-out shelf: shop stays open with an EMPTY row (index 0, not for sale), reroll, coin, leave. Existing line reused.
- Gamble persistence: `gamble::RunState.wager_start` / `visit_flips` (Option, serde default), stamped in `save_run`, consumed by `load_dream`. Old saves fall back to current dust / 0 flips.

## For approval (new player-facing text)
- Shop row name "EMPTY", tag "nothing for sale" (desc is the existing "THE MOTH HAS NOTHING LEFT FOR YOU").
