# NOTES: pacts (branch feat/hard-pacts)
Design: `pacts.rs` pure logic. 8 pacts, bonus 0.15..0.35 each, additive, cap +1.0 on the dust multiplier.
Constants: PACT_CHANCE 0.12 (own seed stream 0x9AC75EED), MIN_DEPTH 3, OWED_DUST 10, enemies x1.08, player x0.95, alert x1.25.
Deviations: "bright eyes" (brief: ambiguous, would reduce darkness) replaced by "sleepless" (skip pays no dust).
"short fuse" = +1 shard_to_wake (director.shards_to_wake) at the moment it is taken.
Pact card replaces one upgrade card (needs >=3 upgrades offered); not at boss rewards, prologue, co-op. Same pact stays offered across rerolls.
Status: DONE, full `cargo test --release` green (666+ tests). Commits 70355f4 (module), 82cdd9b (wiring).
Wiring (main.rs blocks `// === pacts ===`): fields `pacts`/`pact_offer`; open_upgrade_choice (thin air truncation, roll, pact card replaces last upgrade); choose (index==options.len()); take_pact; reroll/skip gates; enemy_speed, alert, player_speed multipliers; dust_multiplier (+dust_bonus added to run.dust()); summary rows "pact" + dust line; SavedRun.pacts (serde default, progress.rs).
Unverified: real play feel; pact seed = dream_seed ^ depth (rerolls keep same pact); owed LoseCard drops last non-ability card.
Knobs: pacts.rs consts + bonus() table.
