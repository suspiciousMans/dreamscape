# NOTES: toolcards stream (feat/tool-cards)

## What it does
Deck runs: the moth's shelf sells up to 2 **tool cards** for run dust. A bought card is shuffled into the draw pile (own seeded stream `dream_seed ^ 0x700C_A4D5 ^ id`) for the rest of the run, is used/discarded/shuffled back like any card, and is never in the booklet or `lore.deck`. Fixed-loadout runs are unchanged (no tool rows).

- `deck.rs`: `TOOL_BASE = u32::MAX - 64`, `tool_id` / `tool_ability` / `is_tool`, `Deck::insert_shuffled`, `Deck::tool_cards`.
- `moth_shop.rs`: `Shelf.tools`, `Shelf::rows()` (cards then tools; reroll and coin rows follow), `tool_pool()`, `tool_offer`, `tool_price`, `buy_tool`, consts `TOOLS_OFFERED=2`, `MAX_TOOL_CARDS=2`, `TOOL_PRICE_CHEAP/MID/STRONG = 8/14/22`, `TOOL_PRICE_STEP = 4`. HAGGLER applies. Price rise = tool cards held (cards never leave the deck, so held == bought).
- `hand.rs`: `HandCard.tool`, frame `TOOL_FRAME` (teal), a "TOOL" tag on the art; no passive/edge/art (the `card: None` path draws tint bands + ability name).
- `progress::SavedRun.tool_cards` (`#[serde(default)]`).

## Finding that differs from the brief
The pool is derived as the brief says (all abilities minus any a dream or `PAIR_ABILITIES` teaches). Tether, Veil, Pulse, Bind, Mirror, Quake, Surge, Haste ARE card-reachable (dreams / resonant pairs), so they are NOT sold. The pool is the 8 with no card source: Bargain, Snare, Wager, Echo, Swap, Mulligan, Recall, Overload. Prices: cheap 8 = Swap, Mulligan, Recall; strong 22 = Overload, Bargain; mid 14 = the rest (Snare, Wager, Echo).

## Resume
A resumed run still falls back to the fixed loadout (no deck), so held tool cards cannot appear in a hand. The list is kept (`Game.tool_cards`) and written back on every save, so nothing paid for is dropped; `restore_tool_cards()` re-inserts them if a deck exists. No refund. If you want them to matter after resume, the options are a refund or resuming the deck itself.

## Player-facing text (for approval)
- Shelf tag: `<N> dust · tool card · this run only`
- Full: `no room`; already held: `already in your deck`; poor: `needs <N> dust`
- Hand tag: `TOOL`
- Row name/description: the ability's existing name and desc.

## Not verified visually
No display here: the TOOL tag/teal frame is covered by `screen_smoke` (paints a tool hand) and `audit_text_fits_every_screen`, not by a screenshot. Look at a deck run's moth shop (`DREAMSCAPE_DECK=...`) and a tool card in hand.
