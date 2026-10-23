# NOTES: shards (branch feat/hard-shards)
Design/constants (all in `shard_rules.rs`):
- `MIN_DEPTH` 4; `chance(depth)` = 20% at d4, +4%/depth, cap 60%; split 50/50 Guarded/Heavy. One kind per dream (never both).
- Own seeded stream (`seed ^ STREAM`), never draws from dream rng -> existing seeds replay.
- Plain when: gilded, boss/nightmare, prologue/tutorial, co-op, objective != plain shard.
- Guarded: sentry placed on a floor cell 1-2 cells from the shard, off the route, clear of spawn/portal, same level, initial beam angle toward shard. Sentries aren't solid -> route untouched. No cell -> plain.
- Heavy: `HEAVY_SECONDS` 25, speed x0.8, alert radius x1.5, cleared by next shard pickup restart/portal/new dream. HUD word "heavy" appended to the twist line.
Status: see bottom (updated at end).

## Done (2 commits on feat/hard-shards)
- Rules + 7 tests in `shard_rules.rs` (depth gating, determinism, never both, route-clear guard on >=3 layouts, alert scaling).
- Wiring in main.rs inside `// === shards ===` blocks: fields, `load_dream` (kind roll, push Sentry `dream::Special`, aim it), `complete_objective` (starts heavy), `player_speed` (x0.8), per-frame pacer alert scaling, HUD `twist` line appends "heavy".
- Small shared edits: `enemy_ai.rs` `scale_alert()`; `dream/mod.rs` re-exports `Special`.
- Full `cargo test --release`: 662 passed.
## Unverified
- Not played visually; guard sentry rotates (existing behaviour), starts aimed at shard. Heavy timer uses dream_age (a time-loop rewind could shorten it).
- No new screen_smoke state (only the existing twist line, which is already `fit`-ed; "heavy" is a short suffix).
## Knobs
`MIN_DEPTH`, `chance()`, `HEAVY_SECONDS/SPEED/ALERT`, guard distance 1-2 cells in `guard_cell`.
