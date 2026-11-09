# Plan: Scraps (disposable cards from packs)

Status: decisions locked by delegation (owner: "build all of them", 2026-11-09). The five open questions in `IDEA-disposable-cards.md` were answered with the closest-existing-pattern defaults below; the owner can reverse any of them.

## Idea
Alongside the cards pulled from a pack, a second, smaller kind of card that works like a disposable: use once, then it is gone. Called **scraps** in code and UI so they are not confused with dream cards or tool cards.

## Decisions (defaults chosen, reversible)
1. **Source:** opening a pack. Every opened pack adds exactly one scrap, extra to the pack's cards (does not count against the pull size, never shown as a dream card). Later waves may add moth/events as sources; not in this plan.
2. **Persistence:** scraps are kept in the booklet (`Booklet.satchel`, counts per kind) until used. They are not dream cards: no booklet number, never in `lore.deck`, never merged.
3. **In a run:** the loadout lets the player bring up to `SATCHEL_CARRY` (2) scraps. Each is shuffled into the draw pile as a run-only card in a reserved id range (`deck::SCRAP_BASE`.., below `TOOL_BASE`, never colliding with booklet numbers or tool ids). Playing one **exhausts** it: it leaves the deck for the run AND is removed from the satchel. Unplayed ones return to the satchel at run end.
4. **Effects:** six one-shot effects built from existing levers only: Mend (refill nerve), Lull (calm enemies for a few seconds), Lantern (reveal the nearest shard), Windfall (bonus dust, paid at run end), Safe Step (cancel the next hit), Redraw (discard the hand and draw a fresh one).
5. **Size of the pull:** extra, see 1. Rarity: scraps have none; the kind is rolled once at pack open, seeded from the run seed, and saved at commit so quitting cannot re-roll.
6. **Scope exclusions:** co-op, autopilot and resumed old-format runs carry no scraps (same rule as the deck). No gambling surface.

## Closest existing patterns
- `deck.rs` tool cards (`TOOL_BASE`, `tool_id`, `is_tool`, run-only, saved in `SavedRun::deck`).
- `main.rs::open_pack` (hook for the extra roll), `reveal_ui.rs` (show the scrap after the last card).
- `loadout_ui.rs` (pick which scraps to carry).
- Save compat: new `Booklet` field with `#[serde(default)]`.

## Data model
```rust
pub enum ScrapKind { Mend, Lull, Lantern, Windfall, SafeStep, Redraw }   // scraps.rs, pure
pub struct Satchel { pub counts: Vec<(ScrapKind, u32)> }                    // Booklet.satchel, serde(default)
```

## Milestones (one stream, S1 -> S4 in order)
| # | Piece | Touches |
|---|---|---|
| S1 | `scraps.rs` pure: kinds, `roll_scrap(seed)`, `Satchel` (add/spend/carry), tests | new file, `cards.rs` (field) |
| S2 | Deck integration: `SCRAP_BASE` ids, shuffle-in, exhaust on play, run save/resume | `deck.rs`, `main.rs` marker block |
| S3 | Effects wired to existing levers (nerve, enemy calm, shard reveal, dust, hit cancel, redraw) | `main.rs` marker block, `combat.rs`/`nerve.rs` as needed |
| S4 | UI: pack reveal shows the scrap, loadout picks carried scraps, small card in hand, `screen_smoke` at 640x360 and 1280x720 | `reveal_ui.rs`, `loadout_ui.rs`, `hand.rs` |

## Tests
Pure roll is deterministic per seed; satchel spend refuses at zero; old save loads with an empty satchel; scrap ids never collide with booklet numbers or tool ids; exhaust removes the card from the pile and the satchel; unplayed scraps return; `every_screen_stays_inside_its_ui_budget` and `audit_text_fits_every_screen` pass.

## Approval list
Scrap names, one-line descriptions and the pack-reveal text are player-facing: collect them under "for approval" in `plans/scrap-briefs/NOTES-scraps.md`.
