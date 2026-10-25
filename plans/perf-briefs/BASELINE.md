# Performance baseline and how to keep it

Measured 2026-10-24 on the dev laptop (Intel HD 520, 1280x720, `--profile profiling`, `ENGINE_NOVSYNC=1`). ms per frame (GPU time is charged to each section because `ENGINE_PROF` calls `glFinish` between them).

| Scene | Before the perf pass | Now |
|---|---|---|
| Normal play | 6.6 | 4.3 |
| Loadout (239 cards) | 9.2 | 6.9 |
| Merge | 11.7 | 6.8 |
| Game `update()` | 0.13 | 0.07 |
| Dream generation (mean) | 18.5 | 3.4 (0.5 in the test profile) |
| Test suite run | 8-9 min | ~46 s |

Vsync caps play at 60 fps (16.7 ms), so play has about 75% headroom here. Where the time goes in play now: post present ~2.1 (trail + composite, which now also draws the vignette and film grain), world passes ~1.3 (about 20 instanced draws; the GPU time is the same as with 570 separate draws, so it is raster/clear cost, not call overhead), egui draw ~0.7, everything else < 0.4.

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

## Done since the first pass
- The vignette and film grain are drawn by the composite shader (`uOv*` uniforms, `ScreenOverlay`), not as two full-screen egui layers: play 5.8 -> 4.3 ms. The vignette ring table is exactly the old mesh's vertex alphas (a test proves it); the only difference is the old 48-sided polygon is now a true circle (0.2% of the radius).
- World blocks are drawn instanced (`INSTANCED` shader variant, `engine::mesh::InstanceBuffer`, `renderstate::Batcher`): about 570 draws became about 20 and ~1,100 GL calls a frame disappeared. Pixels match the non-instanced renderer to within 2/255 on static scenes. The measured GPU time did not change on the HD 520, so this mostly helps CPU-bound machines.

## Not done yet
- Test on weaker hardware (Steam Deck, older GPUs): nothing here can emulate it.
- The composite pass now costs ~2.1 ms: the trail pass and the composite are two full-screen passes; the trail is skipped when off, so a cheaper composite (fewer texture reads) is the next lever.
- Profiling note: `glFinish` has about 0.5-1 ms of its own latency on this GPU, so a section reading ~1 ms may be mostly that floor.
