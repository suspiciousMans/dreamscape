# Abilities 3 + "gambling" for retention: shared context
Owner request (2026-10-24): "add some more abilities with different effects" and "incorporate gambling into the game to help keep players coming back". Chosen: daily fortune card, double-or-nothing at the moth, dream tickets, wager portal, hand scoring (wave 2), plus 8 new abilities.
Same rules as `plans/deck-briefs/CONTEXT.md` and `plans/items-briefs/CONTEXT.md` (read both): own worktree + branch, pure modules + unit tests, `// === <stream> ===` marker blocks in main.rs, TDD, one commit per green task, build etiquette (`CARGO_TARGET_DIR` own, `CARGO_BUILD_JOBS=2`, focused tests only, one positional filter), no new warnings, NOTES file `plans/gamble-briefs/NOTES-<stream>.md`, 5-line final reply with "defined but not wired" list. The `dreamscape-dev` skill lists the gotchas. NOTE: a display exists in the parent session but NOT in yours; screenshots may fail: say so and rely on `screen_smoke` + `audit_text_fits_every_screen`.

## Guardrails (non-negotiable, owner-approved design)
- **Dust and cards only. Never real money, never a purchase hook.** No real-money wording anywhere.
- **Odds are shown exactly as rolled** on the screen that asks for the wager (reuse `lottery::display_odds` style).
- **Stakes are opt-in, capped, and confirmed** (two-step: choose, then confirm). Caps are named consts. Nothing is ever staked by default.
- **No near-miss theatrics, no countdown pressure, no "last chance" text.** Missing days/runs is never punished; streak bonuses are small and a gap only pauses them.
- **Outcomes are decided once and saved** at the moment of commit (seeded from its own stream, e.g. `dream_seed ^ 0x...`), so quitting and reloading cannot re-roll them. Existing seeds must still replay.
- Pity/guarantees where a dry spell is possible (follow `lottery.rs`).
- Co-op, autopilot, prologue: all new gambling is inert there.
- New player-facing text is plain and goes in your NOTES approval list.
Persisted state: new fields in the booklet save (`#[serde(default)]`), old saves load unchanged; test it.
