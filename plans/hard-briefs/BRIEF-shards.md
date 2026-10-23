# Stream: SHARDS (module `shard_rules.rs`, branch feat/hard-shards)
Mechanic: some shards are a skill check, not free pickups.
Design (decided):
- From depth >= 4, a shard may be `Guarded` or `Heavy` (seeded stream of its own; weights rise with depth; never both in one dream;
  the guaranteed-shard twist GILDED stays plain; boss/secret/prologue/tutorial/co-op stay plain).
  - Guarded: a stationary sentry (use the existing Sentry kind) is placed with the shard in its cone range, facing it.
    Being seen alerts enemies (existing sentry behaviour).
  - Heavy: after pickup you are slower (x0.8) and louder (alert radius x1.5) for HEAVY_SECONDS (25s) or until the next shard/portal;
    HUD shows a one-word status ("heavy") in the existing status area.
- Pure fns in shard_rules.rs: `kind_for(seed, depth, twists_gilded, ..) -> ShardKind`, `guard_cell(shard_cell, layout)`,
  `heavy_mults(elapsed)` with unit tests (depth gating, determinism, never both, seed replay unaffected).
- The guard must not make the shard unreachable: validate with the existing patient-walker style test for at least 3 layouts.
