# NOTES: LOADSPIKES stream (branch perf/loadspikes)
No display/GPU here: numbers are CPU only, from headless tests. "dbg" = the old unoptimised test profile; "opt" = the new dev profile (below).

## 1. Spikes (dream generation, load_dream)
- New scopes (ENGINE_PROF=1): `load.total`, `load.generate`, `load.clear`, `load.music_start`, `load.textures_gen`, `load.textures_upload`, `load.spawn_tiles`, `load.spawn_foes`, `load.spawn_features`. Parent: read them from the log on a real GPU to see which one owns the 25-36 ms frames. Music is already composed on a thread.
- Finding: O(n^2) BFS. `maze` portal pick, `pick_shard` (up to 4x per dream), `secret::pick` and the nightmare arena each ran one `Grid::path` per cell. Now one `Grid::path_lens` BFS per call; results identical (golden hash over 20 themes x 12 seeds x 3 depths + nightmares unchanged: `78f8d991e17a6be7`, test `dream_golden_hash`).
- `cargo test -p dreamscape -- dream_gen_timing --ignored --nocapture` (dbg): mean generate 18.5 -> 3.4 ms; layout::generate 5.0 -> 0.37 ms; slowest dream (DrownedLibrary d12) 65 -> 12 ms. opt profile: mean 0.27 ms, layout 0.04 ms.
- Textures: 7 per dream (floor, wall, prop, shard, portal, wake, enemy) are now made on threads (`texprefetch.rs`, pixels identical to `spec.rgba()`, tested), GL uploads stay on the main thread: 11.2 -> 5.2 ms (dbg, loaded box). The cracked wall reuses the wall pixels instead of regenerating them.
- Not cacheable: no TexSpec repeats between dreams (seeds differ), so no cross-dream cache was added.

## 2. hud_view per-frame cost (`hud_view_cost`, thread CPU time, 239 cards, dbg, us/call)
| mode | before | after |
|---|---|---|
| Playing | 87 | 13 |
| Loadout | 409 | 30 |
| Merge | 402 | 33 |
| Booklet | 353 | 43 |
| Title | 157 | 66 |
- Loadout/Merge/Booklet card lists are `Arc<[Card]>` built by `viewcache::CardCache`, rebuilt only when the cards (fingerprint), the sort or the page changes. `sorting::order` no longer needs a cloned list (generic over `Borrow<Card>`).
- `mastery_levels` listed every card every frame; the screens treat a missing card as level 0, so only cards with xp are listed now. Attunement line cached on the same fingerprint.
- Left alone: `settings_snapshot` clone and the small Strings (13 us in play).

## 3. Build/test time (4 cores shared with other streams, so wall times are noisy; user time in brackets)
Slice = `-- dream:: deck:: screen_smoke` (323 tests).
| profile | game-only clean build | slice run |
|---|---|---|
| baseline (dev opt 0) | 28 s (36) | 192 s (466) |
| deps opt 3, game opt 0 | 27 s (36) | 100 s (221) |
| deps opt 3, game opt 1 | 84 s (144) | 37.5 s (77) |
| + game `debug = "line-tables-only"` | 68 s (114) | 37.3 s (76) |
- One-line edit rebuild is ~4-5 s in every variant (incremental). A cold build of all dependencies at opt 3 took 535 s once.
- Adopted in workspace `Cargo.toml`: `[profile.dev] opt-level = 1`, `[profile.dev.package."*"] opt-level = 3`, `[profile.dev.package.dreamscape] debug = "line-tables-only"`. Full suite: 1153 passed, test run 42 s (was 8-9 min).
- No `mold`/`ld.lld` installed, so no linker change. `cargo build` (debug) now gives an optimised-ish binary, faster to play.
- Not done: splitting main.rs (out of scope).

## 4. Memory
- 480 generated dreams + 5 textures each: peak RSS of the test process 9 MB, nothing retained across dreams. Spare prefetched pixels (7 x 16 KB) are freed at the end of `load_dream`. No other large retained structure found; peak RSS of a live run needs a display.
