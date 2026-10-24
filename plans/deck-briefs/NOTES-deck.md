# NOTES: DECK stream (feat/deck)

## Design (smallest that makes the hand dynamic)
- `run.abilities` stays the list of hand slots (index = key: shift, e, q). A deck run sets `RunUpgrades::deck_slots` (>0).
  - Using a slot: `try_use` as before, then `deck_on_use`: the card goes `Deck::play(slot, cooldown)`, the slot's state is marked `spent` (cannot fire).
  - `Deck::tick` shuffles the discard back (0.30 s, random position below the top, seeded `dream_seed ^ 0xDEC0_DEC0`) and draws the top card into the empty slot after `DRAW_DELAY` 0.35 s. `update_deck` turns `Drawn` events into `RunUpgrades::deck_draw(slot, ability, card cooldown)`.
  - A used ability's running effect (Ward, Dash...) outlives its card: the old state moves to the end of `abilities` as a spent ghost until `active` ends (`run.active()` still sees it). `taken`'s `Learn(..)` entries are re-synced (`sync_learned`) so synergies follow the cards in hand.
  - Cooldown travels with the card (`Deck.cooldowns`, ticks everywhere).
- `ABILITY_SLOTS` stays 2 (co-op/autopilot/resumed runs keep the old kit); `HAND_SIZE = ABILITY_SLOTS + 1 = 3`.
- Deck run is on unless co-op/prologue/autopilot. `DREAMSCAPE_DECK=1,2,3` forces one (also with autopilot).
- Saved: `lore.deck: Vec<u32>` (`#[serde(default)]`). `Booklet::deck_numbers()` = cleaned deck, else the old `loadout_cards()` rule.

## Files
deck.rs (new, pure), deck_ui.rs (new builder screen), hand.rs (poses/riffle/stack), upgrades.rs (HAND_SIZE, `spent`, `deck_slots`, `deck_draw`, `from_hand`, `sync_learned`), settings.rs/pad.rs (Ability3: Q / pad Y; pad X is now only ability 2), cards.rs, lore.rs, hud.rs (`hand_pile`), main.rs (`// === deck ===` blocks), loadout_ui.rs/pixels.rs (`#[allow(dead_code)]` only).

## Numbers
DECK_MIN 3, DECK_MAX 12, hand 3, DRAW_DELAY 0.35, SHUFFLE_DELAY 0.30, draw anim 0.40 s (arc 44u, overshoot ~8%), flick 0.28 s, riffle 0.42 s (3 sheets), sway ±0.02 rad, ready lift 7u.

## Deliberate limits / decisions
- Passives, edges, card dreams (`plan_cards`), quirk bonuses (`card_fx`) and mastery XP come from the OPENING HAND (3 cards), not all 12 deck cards, to keep balance near the old loadout. Cards drawn later only bring their ability.
- Deck runs are not offered `Learn(ability)` picks (`available()` false) and the form's starting ability is skipped (no free slot): their abilities come only from cards. Say so if you want them back.
- Two cards with the same ability can sit in hand; `ability_used`/tints are keyed by ability so both pop together.
- A resumed (saved) run falls back to the fixed kit (deck state is not saved).
- Ascension rule 13 removes hand slots as it removed loadout slots.
- The deck builder replaces the loadout screen's slot row (`deck_ui::draw`); a deck of 1..2 cards cannot start unless you own fewer than 3 cards; empty = automatic. Opening the screen prefills the deck with what would be used.

## Defined but not wired / unverified
- `loadout_ui::draw/toggle/slot_of` are now unused (allow(dead_code)); `lore.loadout` is still used by title teaser, `loadout_cards()` fallback.
- Sorting is NOT called from deck_ui (SORT stream; wire `sorting::order` in `deck_ui::draw`/`draw_grid` at merge).
- SCREENSHOTS NOT TAKEN: the game crashes at startup in this sandbox (even the stock binary: "No such file or directory", no GPU), so the animation was verified by unit tests (timings, on-screen bounds, reduced motion) and draw-smoke tests only, not by eye.
- Full suite not run (owner does). Run here: deck, deck_ui, hand, upgrades, settings, pad, cards, lore, loadout_ui, progress, coop, screen_smoke: all green.
