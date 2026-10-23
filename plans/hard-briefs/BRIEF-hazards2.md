# Stream: HAZARDS-2 (finish wiring; branch feat/hard-hazards2, worktree /home/_l/dreamscape-hard/hazards2, based on merged feat/hard-mode)
Read CONTEXT.md and NOTES-hazards.md first. `theme_rules.rs` already defines 8 rule kinds with pure logic; only RisingTide, StickyFloor,
GustWind (speed/push), shard reset and the title-card rule line are wired. WIRE THE REST into the running game, keeping the
`// === hazards ===` marker blocks in main.rs: ShrinkingFloor (outer cells actually crumble using the existing crumble/tile machinery, never route
cells or their margin), DimmingLight (reduce visibility/tunnel-vision radius over time, reset on shard), Silence/hearing (enemies notice dash/blink/
ability use within a radius: alert via existing EnemyAI::alert), DriftingFog (existing fog pockets move slowly), BeatFloor (hazard tiles on the beat:
reuse the existing beat/music tempo and crumble tiles; crossing on the off-beat is safe). Each needs a test for its effect fn plus a headless
screen_smoke frame test where one exists. Keep depth>=3 gating, strength ramp, and patient-walker/route tests green. Commit per rule. Do not push/merge.
Final reply 5 lines max; update NOTES-hazards.md "wired" list.
