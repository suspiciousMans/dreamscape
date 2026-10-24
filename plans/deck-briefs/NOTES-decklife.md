# NOTES: DECKLIFE (branch feat/decklife)

## Built
1. **Passives/dreams follow the deck.** `RunUpgrades::from_hand(hand, deck)`: passives from every deck card (deck order, `KIT_PASSIVES` cap, fused both, resonant doubled), ability slots from the hand. `plan_cards` gets every deck card (one dream each, `card_depth(i)`). New `DreamscapeGame::deck_cards`.
2. **Edges follow the hand.** `deck_edges()` reads the cards in hand, first 2 distinct edges by slot; recomputed in `update_deck` (every tick) and `deck_on_use`. Fixed loadout, co-op, autopilot unchanged.
3. **Builder shows the cap.** `RunUpgrades::deck_passives(cards) -> (active, benched per card)`; deck builder prints "N of 10 passives active" (+ "this card's is benched" for the highlighted card) under the meter note, and benched pips draw dim.
4. **Resumed runs keep the deck.** `deck::DeckSave` (draw order, hand, discard, waiting, cooldowns, played, free reshuffles, seed, epoch) in `SavedRun::deck` (`#[serde(default)]`). `resume_deck` rebuilds `deck`, `hand_cards`, `run.abilities`, edges and tints. Old saves (no `deck`) resume on the fixed kit as before. `restore_tool_cards` removed (tool cards ride in the deck save; `SavedRun::tool_cards` kept for old saves/compat).

## Decisions / limits
- The RNG stream cannot be saved: a restored deck re-seeds with `seed ^ SEED_MIX ^ epoch`. Hand and draw order are exact; LATER shuffles are deterministic but differ from an unbroken run.
- Spent ghost abilities (effects mid-run) are not saved; a resumed run starts them over.
- Mastery/quirk effects (`refresh_card_effects`) still read the opening hand (resume: the hand at save time), not the whole deck: only passives, edges, dreams changed.
- Detail line is in the deck-builder note row, not inside the card panel (panel fits exactly).

## Tests
deck.rs save/restore (ron round trip, property cycle with restores, deterministic stream); upgrades.rs deck passives/cap; main.rs `deck_edges_*`, dreams once each, save->resume identical hand/draw, old save loads. `deck` filter 51 pass; `screen_smoke` 145 pass (includes audit_text_fits).

## For approval (new player-facing text)
- Deck builder: "N of 10 passives active"; "this card's is benched".

## Not verified
No display: builder note line and dim pips unseen (parent should screenshot `DREAMSCAPE_SCREEN=loadout` with a >10-passive deck).
