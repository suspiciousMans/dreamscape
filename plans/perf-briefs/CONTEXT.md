# Performance: shared context
Owner (2026-10-24): "optimize the game as much as you can, make our code as efficient as possible". RULE ZERO: **behaviour and look stay identical** (same seeds replay, same pixels unless a brief says otherwise, full suite green). Measure, change, measure; report numbers.

## Measured baseline (Intel HD 520, 1280x720, profiling build, ENGINE_NOVSYNC=1, ms per frame)
- Normal play (autopilot): whole frame ~6.8 ms uncapped (~147 fps); vsync-bound at 60 so the game is already fine in play. Breakdown: egui paint 3.2 (tessellate + GPU), world passes 1.7, post present 1.4, egui run 0.2, hud_view 0.1, game update 0.12 (every `upd.*` < 0.02).
- Loadout screen (239-card booklet): 9.2 ms: egui draw 3.6 + tessellate 0.64, 1596 shapes -> 14.8k vertices / 16.4k triangles; hud_view 0.73 (clones all cards); world passes still 1.65 behind the menu.
- Merge screen: 11.7 ms: 10,134 shapes -> 43k vertices / 51.5k triangles; tessellate 2.5 + draw 4.5; egui run 1.1.
- First seconds of a run and dream transitions: worst frames 25-36 ms (spikes).
- The full test suite takes 8-9 min; a fresh build ~6 min (release, LTO).
Conclusion: the CPU game logic is NOT the bottleneck. Cost is (1) the egui UI, especially pixel-art drawn as one rect per pixel, (2) world pass overhead and GL call volume, (3) spikes at load time, (4) build/test time.

## Tools (already merged on branch perf/harness, which you start from)
- `engine::perf`: `perf::scope("name")` (RAII timer), `perf::count("name #", n)`, report every 300 frames. Enabled only by env `ENGINE_PROF=1`; log goes to `~/.local/share/dreamscape/dreamscape.log`. `ENGINE_NOVSYNC=1` swaps immediately (uncapped, real cost). `gl.finish()` is called after sections only when profiling is on, so GPU time is charged to the right scope.
- Build for measuring: `cargo build --profile profiling -p dreamscape --features dev` (release speed + symbols; the `dev` feature is needed for DREAMSCAPE_* switches). `plans/perf-briefs/prof.sh <name> [ENV=VAL ...]` runs 28 s of the autopilot and prints the last report (it assumes a booklet copy at `$TMPDIR/bkp.ron`).
- **You have NO display/GPU**: you cannot run the game or see GPU time. Measure in tests instead: shapes/vertices/triangles per screen (`screen_smoke` renders every screen through `hud::draw` on a headless egui context: count `output.shapes`, tessellate and time it with `std::time::Instant` over many iterations), GL-call counts, allocation counts, wall times of pure functions. The parent measures GPU time after merging, so make counts go down and keep the look identical (compare rendered shape lists / tessellated meshes before and after where you can).
- Rules: own worktree + branch, pure modules + unit tests, `// === perf ===` marker blocks in main.rs, one commit per green task, `CARGO_TARGET_DIR` own, `CARGO_BUILD_JOBS=2`, focused tests, no full suite, zero warnings, NOTES `plans/perf-briefs/NOTES-<stream>.md` with before/after numbers, final reply 5 lines. Read the `dreamscape-dev` skill.
