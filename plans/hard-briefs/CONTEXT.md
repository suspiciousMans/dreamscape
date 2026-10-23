# Hard mode: shared context (read first)

Game: Dreamscape, Rust, `/home/_l/dreamscape` (workspace; crate `games/dreamscape`, engine in `engine/`).
Goal: make the game harder with new MECHANICS (not just bigger numbers). Six streams, each in its own git
worktree + branch off `feat/hard-contract` (commit 88dc781, which only adds empty modules `pressure.rs`,
`tactics.rs`, `shard_rules.rs`, `theme_rules.rs`, `pacts.rs` and their `mod` lines). Branches merge later
into one; keep your changes local to your own files so that works.

## Rules for every stream
- Tests first (write the failing test, then code). Put LOGIC in your own module as pure, unit-tested fns;
  keep wiring in `main.rs` / `hud.rs` / `gameplay.rs` minimal and inside a clearly marked block
  (`// === <stream> ===` ... `// === end <stream> ===`) so merges are easy. Do not edit other streams' modules.
- Gentle start: dreams at depth 0 and 1, the tutorial/prologue, co-op, boss arenas and secret rooms are
  NOT affected unless your brief says so. Difficulty ramps with depth. Never make a route impossible:
  the "patient walker" guarantee (a route to shard/portal always exists) must still hold; existing tests enforce it.
- Existing seeds must replay: new randomness uses its own seeded stream, never draws from existing ones.
- Persisted data (booklet/settings/saved run) new fields need `#[serde(default)]`.
- Any new on-screen text must fit every window size: `cargo test --release -p dreamscape audit_text` must pass
  (use `pixel_ui::fit`; add your screen/state to that test in `main.rs` `screen_smoke` if it draws new text).
- No sound, particles, or extras beyond what the mechanic needs ("playable core"). Short, dry, lowercase eye-voice text.
- Build/test: `export CARGO_TARGET_DIR=/home/_l/dreamscape/target CARGO_BUILD_JOBS=2` then
  `cargo test --release -p dreamscape <filter>`; full suite at the end: `cargo test --release`.
  The machine has 4 cores / 7 GB and other streams build too: cargo will print "Blocking waiting for file lock";
  that is expected, NOT hung. Wait. Never run more than one cargo at a time yourself.
- One commit per task in your worktree (`git add -A <your files>; git commit`). Do NOT push, do NOT merge.
  If a tool call is blocked/times out, skip it, keep working uncommitted, and say which files hold what.
- Write `/home/_l/dreamscape/plans/hard-briefs/NOTES-<stream>.md` EARLY (design decisions, constants) and
  update it at the end (what is done, how it is wired, what is unverified, tuning knobs). Your final reply: 5 lines max.
- Do not run the game window. Verify with tests (the `screen_smoke` module in `main.rs` can draw frames headless).

## Useful pointers
- `main.rs`: `DreamscapeGame` (state), `update()` (~line 7064, per-frame), `hud_view()` (builds `HudView`), dream spawn
  (`begin_run`, dream build), `choose_event`, `open_upgrade_choice`, caught/respawn logic (grep "caught", "respawn").
- `gameplay.rs`: difficulty(), enemy_speed(), chase_radius(), pressured_* fns; consts HARD_GROWTH 1.18 etc.
- `specials.rs` (stalker/mimic/sentry/drifter/jester, `Elite`), `enemy_ai.rs`, `hunter.rs`/`boss.rs`.
- `dream/variant.rs` (twists), `dream/` (themes, layouts, `DreamTheme`, `ALL_THEMES` = 20 themes).
- `upgrades.rs` (Upgrade, Curse, Tier, RunUpgrades), `events.rs`, `progress.rs` (Ascension 0..=10, `ascension_rule`).
- `summary_ui.rs` (wake summary), `hud.rs` (in-game HUD, scaled by `hud::ui_scale`).
