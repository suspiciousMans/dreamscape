# NOTES tactics (branch feat/hard-tactics, worktree /home/_l/dreamscape-hard/tactics)
Logic: `games/dreamscape/src/tactics.rs` (pure fns + `State` of per-pacer shout cooldowns).
Constants: SHOUT_RADIUS 6 cells, SHOUT_COOLDOWN 8s, depth gates shout>=3, jester>=4, mimic/sentry>=5, pack>=6;
MIMIC_EXTRA_DELAY 3s, SENTRY_BOOST x1.5, PACK_MAX 3 within 9 cells, JESTER_ASSIST_RADIUS = sentry range.
Depth = `director.depth`. Off in co-op. Alerts use existing `EnemyAI::alert()` (still leashed, tiring, slower than player).
Wiring (main.rs `// === tactics ===`): pacer loop collects "started chasing", then shouts/packs; jester throw uses `jester_dir`;
mimic delay + sentry boost in `update_specials`; `specials::Sentry::update_scaled` added.
Status: see end of file.

## Done
All 4 mechanics wired; sentry sighting alert (2 cells) kept. Full `cargo test --release -p dreamscape`: 663 passed. Commits on feat/hard-tactics.
Unverified: live feel/tuning (no game run); co-op/hunter dreams skip tactics via hunted/hunter check; no eye-voice line added.
Knobs: constants at top of tactics.rs.
