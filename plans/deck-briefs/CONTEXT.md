# Deck / sort / shop: shared context for every stream

Owner's request (2026-10-23), verbatim intent:
1. **Deck**: build a deck before the run. In play you hold a hand; the hand holds ONE MORE card than before. A card that is used is discarded, then shuffled back into the deck, and the hand draws a replacement. Clean, Balatro-style animations (draw fan-out, lift on hover/use, discard flick, shuffle riffle, settle).
2. **Sorting**: booklet, merge menu (and loadout/deck-builder) can sort cards: Rarity, dream/name, card number (newest/oldest), ability. One key cycles the sort field, one flips direction.
3. **Shop**: remove the moth MERGE menu that pops up after a beaten nightmare ("moth_after_nightmare" in main.rs). Show the actual shop instead (the moth's room: `moth_shop.rs`, run upgrades for run dust). Merging stays reachable via `[m]` in the Lucid Store and the title. Add MANY more items to everything (store catalog, moth stock, ...).

## Decisions (defaults; owner said "start building")
- Hand size = `upgrades::ABILITY_SLOTS` + 1 (3 now). Keys: `shift`, `e`, and a new third key `q` (pad: add the third button next to the existing two; settings `Action`/`Row` rules in the dreamscape-dev skill).
- Deck = booklet cards chosen on the loadout screen (`lore.deck: Vec<u32>`, `#[serde(default)]`, old saves keep working; empty deck falls back to the old `loadout_cards()` rule). Deck size min 3, max `DECK_MAX = 12` (a const). The old loadout slots become the deck cap growth: `DECK_MAX` is fixed; the existing `lore.slots()` growth sets the hand-building tier is NOT used for the deck.
- Using a card's ability: card -> discard pile; the discard is shuffled back into the draw pile (random position, seeded stream `dream_seed ^ 0xDEC0_DEC0`, existing seeds must still replay); the empty hand slot draws the top card after `DRAW_DELAY` (0.35s). A card's cooldown travels with the card (it is not ready until its cooldown ends, even if redrawn).
- Cards still give passive + edge + their dream once per run as today; only the ability slotting becomes dynamic.
- Co-op and autopilot: keep the old fixed loadout behaviour (like combat did); no deck there.
- Reduced motion (`self.settings.reduced_motion`) must disable shake/flick/riffle (instant moves).

## Rules for every stream
1. Own git worktree + branch (in your brief). Do not merge, push, or touch other worktrees. The owner merges.
2. Pure logic in small modules with unit tests (no GL, no clock; `dt` in). `main.rs` only wires, inside `// === <stream> ===` ... `// === end <stream> ===` marker blocks.
3. TDD: failing test first, one commit per green task, message `<stream>: <what>`.
4. Read the `dreamscape-dev` skill gotchas before editing (array lengths, HudView fixtures, `Ctx::default()`, never `pkill dreamscape`).
5. Zero NEW compiler warnings (known: `TRAIL_KEEP` in `dream/variant.rs`, `Sentry::update`).
6. **Build etiquette** (4 cores, 7 GB, three streams run at once): `export CARGO_TARGET_DIR=<yours> CARGO_BUILD_JOBS=2`. Focused tests only: `cargo test -p dreamscape -- <filter>` takes ONE positional filter. Never run the whole suite (owner does at merge). A waiting build is expected, not hung. A blocked tool call is not a reason to stop: skip, keep working, list it as unverified.
7. Screenshots: `DREAMSCAPE_AUTOPILOT=1 DREAMSCAPE_SKIP_WARNING=1 DREAMSCAPE_SEED=5 DREAMSCAPE_SHOT=/path.png DREAMSCAPE_SHOT_AT=3 ./target/debug/dreamscape` with `DREAMSCAPE_BOOKLET=<tmp copy>` (never the real save).
8. Write `plans/deck-briefs/NOTES-<stream>.md` (what you built, files touched, numbers, what is NOT wired/verified). Final reply: 5 lines max, including a "defined but not wired" list if any.
9. Commit early and often; a rate limit can cut you off.
