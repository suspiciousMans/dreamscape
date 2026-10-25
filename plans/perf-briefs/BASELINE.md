# Performance baseline and how to keep it

Measured 2026-10-24 on the dev laptop (Intel HD 520, 1280x720, `--profile profiling`, `ENGINE_NOVSYNC=1`). ms per frame (GPU time is charged to each section because `ENGINE_PROF` calls `glFinish` between them).

| Scene | Before the perf pass | Now |
|---|---|---|
| Normal play | 6.6 | 5.8 |
| Loadout (239 cards) | 9.2 | 6.9 |
| Merge | 11.7 | 6.8 |
| Game `update()` | 0.13 | 0.07 |
| Dream generation (mean) | 18.5 | 3.4 (0.5 in the test profile) |
| Test suite run | 8-9 min | ~46 s |

Vsync caps play at 60 fps (16.7 ms), so play has about 60% headroom here. Where the time goes in play: egui draw ~2.7 (the vignette and grain overlays are ~1.5 of it), world passes ~1.3 (about 570 draw calls), post present ~1.4 (trail + composite), everything else < 0.5.

## Guards (tests that fail if a change regresses)
- `every_screen_stays_inside_its_ui_budget`: max egui shapes/triangles per screen. A failure means a painter went back to one shape per pixel. Use the run-merged mesh helpers in `pixel_ui.rs`, or raise the number on purpose with a measurement.
- `dream_generation_stays_inside_its_time_budget`: mean < 5 ms, worst < 25 ms (about 10x headroom). A failure usually means a per-cell search crept back in (one BFS per cell was the old O(n^2)).
- A golden hash of generated dreams (loadspikes) proves optimisations did not change a dream.
- `ShaderVariantCache::ALL_FLAGS` is prewarmed at start-up: a shader variant compiled mid-play cost a 55 ms hitch on a cold Mesa cache. Adding a shader flag bit means extending `ALL_FLAGS` (a test checks it).

## How to measure
1. `cargo build --profile profiling -p dreamscape --features dev` (the dev switches need the feature).
2. `ENGINE_PROF=1 ENGINE_NOVSYNC=1` plus the usual `DREAMSCAPE_*` switches (see `plans/perf-briefs/prof.sh`, which runs the autopilot 28 s, or `PROF_SECS=100`). Reports go to `~/.local/share/dreamscape/dreamscape.log` every 300 frames: per-section avg/worst ms and counters (egui shapes/vertices/triangles, world draws/uniform/texture/VAO skips).
3. `ENGINE_PROF=1` also logs any frame over 25 ms as `spike ...` with its biggest sections (`ENGINE_SPIKE_MS=19` to lower the bar). Frames near 19-33 ms with a big `present` are vsync misses caused by the profiler's own `glFinish`, not game cost.
4. `MESA_SHADER_CACHE_DISABLE=true` reproduces a first-launch (cold shader cache) run.
5. Test numbers: `cargo test -p dreamscape -- ui_cost_table --nocapture` prints shapes/vertices/triangles/tessellation per screen.

## Rules of thumb
- The game logic is ~2% of a frame; do not optimise it before measuring. The GPU and egui are the budget.
- Every full-screen blended layer costs ~0.5 ms on this GPU; a new overlay needs a measured reason.
- Pixel art goes through `pixel_ui` run-merged meshes or cached textures, never one rect per pixel.
- New blocks/enemies that add draw calls need a batching plan (about 570 draws today = ~1 ms).
- Do not change the look for speed without asking the owner. Known approved look change: icon/fill seams removed (cleaner pixel art).

## Not done yet
- Fold vignette + grain into the post-process pass (~1.5 ms, vignette would be a smooth circle instead of a 48-gon).
- Merge static blocks into one mesh per texture (~1 ms).
- Test on weaker hardware (Steam Deck, older GPUs).
