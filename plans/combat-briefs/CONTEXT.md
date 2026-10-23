# Combat: shared context for every stream

Read `plans/2026-10-23-combat-system-plan.md` first (the design). Decisions made by the owner:
- Combat **replaces** the old rules everywhere (no CLASSIC switch). Co-op and the autopilot stay immune/off for now.
- Enemies **unravel** (no gore). New enemy names (Tooth, Weeper, Lullaby, Moths, Mask, Leech) are approved.
- Strike key is **F**, controller **B** (X/Y are ability 2).
- Nerve: 3 start, 5 max, +1 per portal, 1.2s of invulnerability after a hit. No new achievements.

## Already built (branch `feat/combat`, commit `529050e`)
- `games/dreamscape/src/nerve.rs`: pure `Nerve` (hit, heal, iframes, shove). `damage_from(source: &str)`.
- `games/dreamscape/src/combat.rs`: pure `Strike`, `in_arc`, `Ctx`/`damage`, `Kind` (enemy kinds) with `base_hp`, `Foe`, `Outcome`, `loot`, `unravel_scale`.
- `main.rs` (all inside `// === combat ===` blocks):
  - `threat_touching()` returns the *name* of what touches you (`"patrol"`, `"hunter"`, `"boss attack"`, `"special enemy"`, `"gate"`, `"beat tile"`). The fail-states block calls `take_nerve_hit(by)`; only the last nerve point is a catch.
  - `update_combat(dt)`, `resolve_strike()`, `unravel(entity, pos)`; fields `foes`, `stunned` (pacers), `staggered` (specials), `unraveling`, `motes`, `bonus_dust`.
  - `Pacer { entity, ai: EnemyAI, elite, a, b, split }` in `self.enemies`; `SpecialEnemy { entity, kind: EnemyKind, ai: SpecialAI }` in `self.specials`.
- HUD hearts, eye flinch, hand (cards) all exist. Sounds: `sounds.rs` `Sound` enum + `synth` (add variants to `ALL_SOUNDS`, update its length).

## Rules that apply to every stream
1. Work only in your own git worktree and branch (given in your brief). **Do not merge, push, or touch other worktrees.** The owner (the main session) merges.
2. Keep new logic in **pure modules** (no GL, no clock; `dt` in, decisions out) with unit tests for every number. `main.rs` only wires. Put wiring in `// === combat ===` ... `// === end combat ===` blocks like the existing ones.
3. TDD per task and one commit per green task, message `combat: <what>`.
4. Chasers stay slower than the player; every attack is telegraphed (0.4–0.8s, reuse the `boss::TELEGRAPH` idea: a scale pulse or tint change); nothing may wall in the route. Existing tests `every_route_is_physically_traversable`, `a_patient_player_is_never_walled_in*` must stay green.
5. Reduced motion (`self.settings.reduced_motion`) must be honoured by any new flash or shake.
6. Co-op: new enemies simply do not spawn when `self.coop_active()`.
7. Zero **new** compiler warnings (two pre-existing ones are known: `TRAIL_KEEP` in `dream/variant.rs`, `Sentry::update`).
8. **Build etiquette** (the machine has 4 cores and 7 GB): use `export CARGO_TARGET_DIR=<the target dir in your brief> CARGO_BUILD_JOBS=2`. Run **focused** tests only (`cargo test -p dreamscape -- <filter>`), never the whole suite (about 8 minutes; the owner runs it at merge). Never `pkill dreamscape`: it kills other people's test binaries.
9. Screenshots: `DREAMSCAPE_AUTOPILOT=1 DREAMSCAPE_SKIP_WARNING=1 DREAMSCAPE_SEED=5 DREAMSCAPE_SHOT=/path.png DREAMSCAPE_SHOT_AT=3 ./target/debug/dreamscape` (set `DREAMSCAPE_BOOKLET=<tmp file>`: never the real save). Add a dev switch `DREAMSCAPE_ENEMY=<Name>` that forces your enemy into every dream, matching existing `DREAMSCAPE_SPECIAL`. `DREAMSCAPE_STRIKE=<secs>` makes the autopilot swing.
10. Write `plans/combat-briefs/NOTES-<stream>.md` (what you built, files touched, numbers, anything the merger must know). Final reply: **5 lines max**.
