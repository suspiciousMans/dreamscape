# Idea: disposable cards from packs

Status: idea only. Not scoped, not approved, no briefs yet.

## Idea (owner, 2026-11-05)

Alongside the cards pulled from a pack, add a second kind of card: smaller,
different-type cards that work like disposables. Use once, then they are gone.

## Open questions for the owner

1. Where do they come from? Only packs, or also events, shops, or drops?
2. Do they go into the booklet, or are they run-only like tool cards?
   (Tool cards live in the run state and never touch `lore.deck` or the booklet.)
3. Do they take a deck slot and draw, or sit outside the deck?
4. What do they do? One-shot abilities, or something else?
5. Do they count against the pack's pull size, or are they extra?

## Notes for later

- Closest existing pattern is tool cards (`deck::TOOL_BASE` range, run-only,
  saved in `SavedRun`). A disposable type may reuse that path rather than
  invent a new system.
- Any new fields on persisted structs need `#[serde(default)]`.
- Packs are covered in `plans/PLAN-pick-a-pack.md`. Read it before designing.
