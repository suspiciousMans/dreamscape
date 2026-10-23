# Pick-a-pack: shared context (read first)

Plan (source of truth): /home/_l/dreamscape/plans/PLAN-pick-a-pack.md. Read it.
Crate: games/dreamscape (binary `dreamscape`). Relevant files: src/cards.rs, src/lottery.rs,
src/reveal_ui.rs, src/booklet_ui.rs, src/hud.rs, src/shop_ui.rs, src/main.rs (`open_pack`, `save_journal`, `mod screen_smoke`).

## Contract (already committed as skeletons on branch feat/pick-a-pack)
- `src/packs.rs`: `Emblem`, `PackOffer { theme, emblem, seed, cards: Vec<Recalled>, memory_chances: Vec<f32> }`,
  `offer_packs(log, owned, boost) -> Vec<PackOffer>`, `Emblem::for_theme`. Bodies are `todo!()` (stream A fills).
- `src/pack_art.rs`: `PackArt`, `PACK_SIZE`, `draw_pack(ctx, p, rect, theme, emblem, seed, pips, time, motion, tilt, art)`.
  Body is a grey stub (stream B fills).
- Do NOT change those signatures or type shapes. If you truly must, say so in your final summary.

## Decisions (from the user, fixed)
Replaces the end-of-run reveal. Unpicked packs lost. No cost to pick. Packs never drop Fused/Resonant.
No sound, no extras beyond the plan ("truly just the core").

## Working rules
- You work in your own git worktree (path in your brief), on your own branch. Never touch /home/_l/dreamscape (main checkout) or other worktrees.
- Build/test with: `cd <your worktree> && CARGO_TARGET_DIR=/home/_l/dreamscape/target CARGO_BUILD_JOBS=2 cargo test -p dreamscape <filter>`
  (shared target dir; cargo serialises on its lock, so a build may wait. That's expected, be patient, don't kill it.)
  Machine: 4 cores / 7 GB. Never run more than one cargo at a time yourself. No `cargo clean`.
- Failing-first TDD. One commit per task (conventional message, e.g. `feat(packs): ...`). Don't commit crash-*.txt files.
- Write progress notes early to <worktree>/NOTES-<stream>.md (uncommitted) in case you're cut off.
- Save-format: any persisted field needs `#[serde(default)]`.
- Honour reduced motion (`motion()==0`) and reduce-flashing in anything animated.
- Final summary: what you built, commit SHAs, test names + real pass/fail output, anything not done. Facts only.
