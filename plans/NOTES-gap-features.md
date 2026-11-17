# NOTES — gap features (variants + attunement combos)

Closes the three feature gaps left after the 2026-11-12 gap-closure plan's first
pass: the two missing twists (ROADMAP 3.4) and the two-attunement combos
(triage item F2 / plan S2d). Docs drift (S5) and the refactors (S7) are *not*
part of this: this pass is features only.

## Shipped

### S4a — KALEIDOSCOPE
- `kaleido.rs` (new, pure): `strength(twist, reduce_flashing, reduced_motion)`,
  `fold(uv)`, `shaped(uv, strength)`. The frame folds into four mirrored
  quadrants about the centre.
- Rendered in `games/dreamscape/assets/shaders/post_composite.frag` via a new
  optional `uMirror` uniform, uploaded by `engine::renderer::PostParams::mirror`
  (`engine/src/renderer/post.rs`). Shader profiles without `uMirror` no-op, the
  same contract as `uTintColor`/`uTintStrength`.
- Off at 0 under **either** `reduce_flashing` or `reduced_motion`; the twist is
  purely visual, so gameplay, route and enemies are untouched.
- `fits`: never on the calm dreams (Lobby, Awakening), like every other twist.
- Wired in `main.rs` by one extra field in the single `PostParams` literal the
  composite call builds (`mirror: kaleido::strength(...)`); no other call site
  changed. `ALL_VARIANTS` goes 12 → 13 here, and to 14 with Breathing.

### S4b — BREATHING
- `breathing.rs` (new, pure): `Breathe` component, `route_points(dream)`,
  `clearance(at, route)`, `amp_for(clearance)`, `period_for`/`phase_for`
  (position-hashed, so a wall breathes the same way every load), and
  `offset(amp, period, phase, age)`.
- Only walls at least `CLEAR_CELLS` (2) cells from every route point move, and
  they lean at most `AMPLITUDE` (`CELL * 0.5`, 1.5 units). The written guarantee:
  a moving wall always keeps `2 * CELL - 0.5 * CELL` = one and a half cells of
  clearance from the route, so no breath can pinch a passage shut. Route walls
  never move at all.
- **The collider sways with the wall.** `engine::physics::step` collects
  `(transform.position, collider)` for every collider each frame, so a static
  wall's shape follows its live `Transform` — a swaying wall can nudge a
  dreamer who stands within reach of it. That is why the clearance rule is the
  safety property, not "it is only drawn": a breathing wall's centre stays at
  least one and a half cells from the route and its face at least one cell,
  even at the end of its breath. Enemy *steering* is unaffected, because
  `foe_walls` and pathfinding are snapshotted from `Block::pos` at spawn.
- Ticked beside the existing `Spin`/`Bob` systems, in `// === breathing ===`
  marker blocks in `main.rs` (spawn + tick).
- `fits`: not on the layouts with no walls (PlatformChain, Spiral, Terraces).
- Tests: `breathing_never_pinches_a_route_shut` generates every theme × 30
  seeds and checks the clearance bound for every breathing wall, plus that no
  route point can be given an amplitude at all (`amp_for(0) == 0`). Offset
  tests: zero at amp 0, bounded by amp on both axes, periodic, horizontal only.

### S2d — two-attunement combos
- `attunement.rs`: `Combo { a, b, name, line, gift }`, six curated pairs in
  `COMBOS`, `combo(attuned) -> Option<&'static Combo>`.
- A combo grants **one** extra passive on top of the two attunements (pushed
  into the same `gifts` vector in `begin_run`, so `KIT_PASSIVES` still caps the
  kit), and the booklet's attunement line gains `COMBO  <NAME>`.
- Every gift is a passive from `upgrades2`/generic picks, asserted in a test to
  be neither an ability nor either attunement's own reward, so a combo is a new
  lever rather than a third stack of something the pair already gives.
- Tests: ordering-independent lookup, one-attribute and empty cases, no pair
  listed twice, gift assertions, and the booklet line naming the combo only
  when both halves are attuned.

## Not done here
- **Docs drift (S5)**: README still claims "228 tests", "Fourteen dream types"
  and "ten ascension". ROADMAP 3.4 now says all six twists are done; the rest of
  the drift is untouched.
- **S7 refactors**, **S8 playtest/screenshots**: not started.

## Verified (Linux VM, Rust 1.100, vendored SDL2 + ALSA built locally)
- `cargo test -p dreamscape` — **1374 passed, 0 failed, 7 ignored**.
- `screen_smoke::audit_text_fits_every_screen` and
  `screen_smoke::every_screen_stays_inside_its_ui_budget` — both pass with the
  `COMBO  <NAME>` line and the two new twist labels in play.
- `cargo clippy -p dreamscape --all-targets -- -D clippy::correctness` — clean.
  (One pre-existing correctness error in `death_fx.rs` blocked this gate; it
  was `6.28` where clippy wants `TAU`, fixed here.)
- The two sway tests in `breathing.rs` failed on the first run and caught a real
  bug: the sideways term used a different angular frequency, so a wall's lean
  neither repeated on its period nor stayed inside `AMPLITUDE`. Both are fixed
  and covered.

## Unverified — read this first
- **No screen was looked at.** The sandbox has no display, so the kaleidoscope
  fold and the wall sway have never been rendered. The evidence is the pure
  tests, the shader reading and the fact that the file still passes the UI
  gates; treat both twists as unseen until a screenshot exists.
- **The shader was compiled by a GLSL validator.** `post_composite.frag` was
  built with `glslangValidator -S frag` (glslang built from source): exit 0, no
  diagnostics. That covers syntax and types; it does not cover whether the fold
  *looks* right.
- **The other three games were not compiled.** `PostParams` gained a field, so
  `sandbox`, `walkaround_demo` and `mushroom_man` got `..Default::default()` in
  their literals; their builds fail in this sandbox on a pre-existing
  `wayland-sys` system dependency, unrelated to the edit.
- **Balance is unfelt.** Combos add one passive to a kit that already carries
  two attunement passives; that is exactly the pile-up triage F2 warned about.
  Keep the feature behind a playtest before tuning numbers further.

## For approval (all new player-facing text)
- Twist labels + blurbs (HUD/summary): `KALEIDOSCOPIC` — "the dream is folded
  in four"; `BREATHING` — "the walls are breathing".
- Combo names + lines (booklet + run log):
  - `RIPTIDE` — "the garden drinks the sea" (BLOOM + TIDE)
  - `RUSTED CLOCK` — "the machines keep their own time" (RUST + HOUR)
  - `THE LONG FALL` — "there is no floor under the floor" (VOID + ABYSS)
  - `WHITE NOISE` — "every sound has a shape" (STATIC + CHORD)
  - `MIRROR GAZE` — "something is looking back" (GLASS + GAZE)
  - `MYCELIUM` — "the way grows behind you" (SPORE + TRACE)
- Booklet prefix: `COMBO  <NAME>` on the existing attunement line.

## Knobs
- Fold: `kaleido::FOLDED` (1.0), and the `0.5 + abs(uv - 0.5)` fold itself.
- Sway: `breathing::AMPLITUDE` (half a cell), `CLEAR_CELLS` (2),
  `PERIOD_FAST`/`PERIOD_SLOW` (5/9 s), `SIDE_LEAN` (0.7).
- Combos: `attunement::COMBOS` (pairs and gifts).
- Markers: `// === breathing ===`, and `kaleido.rs`'s one call site in
  `main.rs`'s composite block.
