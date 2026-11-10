# UI revamp: shared context (read first, then your BRIEF)

Plan: `plans/PLAN-ui-revamp.md`. Game crate: `games/dreamscape` (read the `dreamscape-dev` gotchas if you have them: build/test cmds below).

## Locked decisions
- Feel: soft, dreamy, smooth. Hit-stop ~40 ms, slow-mo >= 0.75x, gentle shake. "Balatro" = easing, hover spring, tilt, tick-up numbers, never arcade punch.
- Setting `ui_juice: Off | Soft | Full` (default Soft). Every new effect takes a `UiJuice` + `reduced_motion` + `reduce_flashing` and degrades: Off = identity (no motion), reduced_motion = no continuous motion and no particles, reduce_flashing = no flashes/strobes.
- Card warp: slow liquid wobble from the card's dream theme, same strength on every card, stronger on hover/selection. Only the ART warps; name plate, rank, text stay flat. Applies in hand, booklet, reveal, merge, loadout, deck builder, relic screens.
- Death: dreamer dissolves (noise threshold + glowing burn edge in theme colour + drifting particles), then fade to Summary. Run-ending death = full length (~1.8 s); mid-run catch = short (0.4 s). First person: held hands/weapon dissolve + ember fade of the view.
- Prompts: context-only chips (pulse/bob) only when an action is available/required; `Required` (dims the rest) only for blocking choices (upgrade pick, event, wake door). Menus: chip on the focused action only. Glyph text only from `pad::button_label` / `settings::key_for`.
- Relics: first-person held weapon, lower-right, ~15 deg tilt toward centre, slow idle sway. Same warp family for relic art. Names from `relic_names.rs` grammar.

## Rules for every stream
- Pure logic in small modules with unit tests (no GL, no clock, no global RNG). `main.rs` only wires, inside `// === name ===` ... `// === end name ===` markers; one `mod` line each.
- Seeded randomness: `crate::worth::{mix, percent_roll}` (`pub(crate)`). Never `rand` in pure logic you add (names.rs uses `rand::StdRng` already; follow it only inside names).
- Save compat: every new field on a persisted struct gets `#[serde(default)]`. Old `settings.ron`/booklet must load unchanged (write the test).
- UI painters use the `pixel_ui` run-merged mesh helpers, not one shape per pixel. `every_screen_stays_inside_its_ui_budget` (main.rs ~17634) caps shapes/triangles: check it at 640x360 and 1280x720 with a full hand and at Full juice.
- New `Action`/settings `Row`: bump `ALL_ACTIONS` length and the `rows().len()` assert. New `Sound`: bump `ALL_SOUNDS`. Adding a `HudView`/`HandCard` field means updating the `screen_smoke` fixtures. New `Upgrade`: not needed here.
- New pub fn used only by tests: `#[cfg(test)]`. API for a later stream: one-line `#[allow(dead_code)] // <milestone>` naming it. Zero new warnings.
- Text: new player-facing strings go in `plans/ui-revamp-briefs/NOTES-<stream>.md` under "for approval". Names/ability descs are labels, not approval items.
- Do NOT edit the files owned by another stream (the BRIEF lists yours). For shared hub files (`main.rs`, `hud.rs`, `settings.rs`) touch only your marker block.
- No display in your sandbox: the game exits at startup, so say screens are "unseen"; the parent will screenshot. Pure timings and `screen_smoke` are your evidence.

## Build / test (each stream, its own worktree)
```
cd /home/_l/dreamscape-ui/<stream>      # a `git worktree` on branch feat/ui-<stream>
export CARGO_TARGET_DIR=/home/_l/dreamscape-ui/target-<stream> CARGO_BUILD_JOBS=2
cargo build -p dreamscape
cargo test -p dreamscape -- <focused filter> 2>&1 | grep -E "^test result|FAILED|^---- " | cut -c1-200
```
Full suite only at the end, in the background with notify (`cargo test -p dreamscape`, ~1-9 min). Never `pkill dreamscape` (use `pkill -x`). Commit per green task on your branch. Do not push or merge. Report facts only: files changed, tests added, test result lines, anything unseen.

## Dispatch waves
- Wave 0 (parent, sequential): M0 `ui_motion` + `ui_juice` setting, and the shared type additions below, on `feat/ui-revamp`.
- Wave 1 (parallel): M1 warp, M2 death, M3 prompts, R4 buffs, N names. Worktrees cut from `feat/ui-revamp` after wave 0.
- Wave 2 (parallel, after M1+M3 merged): M4a-e screen groups, R3 relic art, R5 weapons.
- Wave 3: R6 relic UI (after R3, M1). Then parent: R7 polish, R8 bug round, screenshots.

## Shared types the parent lands in wave 0 (do not redefine)
- `ui_motion::UiJuice { Off, Soft, Full }` with `scale(self) -> f32` (0.0/1.0/1.4), `Settings.ui_juice`.
- `ui_motion` easing fns: `smoothstep`, `ease_out_back(t)`, `ease_out_cubic(t)`, `Spring { pos, vel }` with `step(target, dt, stiffness, damping)`, `tick_up(from, to, t) -> i64`.
- `relic_types::Weapon` gains `#[serde(default)] tint: [u8;3], trail: u8, clash: u8` (R5 owns their meaning).
- `relic_types::RelicEffect` gains `#[serde(default)] max_nerve: u32` (R4 wires it).
- Corrections to the plan/brief typos found in pre-flight: `Shape` lives in `crate::dream::{Shape, ALL_SHAPES}` (11 variants: Cube, Octahedron, Orb, Cylinder, Cone, Torus, Arch, Cloud, Crescent, Island, Stairs); `hand.rs` draws card faces through a private `image()` that rotates points by the tilt, so the warp must produce local-space points and let `at()` rotate them; hud mode enum is `hud::Mode` (Warning, Title, Playing, Paused, Journal, Reveal, PackPick, Store, Booklet, Choice, Summary, Settings, Codex, Note, Memories, Loadout, Merge, ...).
