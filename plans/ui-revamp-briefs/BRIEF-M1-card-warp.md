# Brief M1: card warp

Goal: every card art face wobbles like a melting dream, driven by its theme, readable text stays flat.

## Scope
- New `src/card_warp.rs` (pure) + `mod card_warp;` in main.rs.
- Swap the quad call for the warp mesh in: `hand.rs` (`Tilt::image`, line ~463), `booklet_ui.rs`, `reveal_ui.rs`, `merge_ui.rs`, `loadout_ui.rs`, `deck_ui.rs`. One-line swaps only; do not restyle those screens (M4 does).
- Leave alone: settings, hud, relic files, `pack_art.rs` textures.

## Types / functions
```rust
pub struct WarpParams { pub theme: DreamTheme, pub seed: u64, pub t: f32, pub hover: f32 /*0..1 eased*/, pub juice: UiJuice, pub reduced_motion: bool }
pub struct WarpGrid { pub cols: usize, pub rows: usize }          // vertices = (cols+1)*(rows+1)
pub fn grid_for(size_px: f32, juice: UiJuice) -> WarpGrid;         // 3x4 small .. 8x10 large; Off = 1x1 (plain quad)
pub fn offset(p: &WarpParams, u: f32, v: f32) -> (f32 /*du*/, f32 /*dv*/); // in 0..1 card space, |d| <= MAX_DISP
pub fn mesh_points(rect: Rect, p: &WarpParams) -> (Vec<Pos2>, Vec<Pos2 /*uv*/>, Vec<u32>); // local space, caller rotates
pub fn theme_character(theme: DreamTheme) -> Character;             // freq, amp, drift dir, swirl
pub const MAX_DISP: f32 = 0.035;   // fraction of card size at hover=1, Full
```
- Displacement = 2-octave sine/noise field: low freq slow sag (like world `uMelt`, see `assets/shaders/mesh.vert` ~line 80) + a slower swirl. Theme sets frequency, amplitude (0.4..1.0 of MAX), drift direction and swirl sign so a Melting Clockworks card sags, a Jellyfish Sky card drifts up, a Mirror Hall card shimmers. Seeded per card so two same-theme cards differ.
- hover raises amplitude by x1.8 and speed x1.5, eased by the caller via `ui_motion::Spring`.
- Edge pinning: displacement fades to 0 at the card border so frames/plates stay aligned (`edge = smoothstep(0, .12, min(u,1-u,v,1-v))`).
- `reduced_motion` => t frozen to 0 (static soft warp, no animation); `UiJuice::Off` => identity.
- Add a helper in `hand.rs` `Tilt::warped_image(...)` that rotates the returned local points via `at()` and builds the egui `Mesh` (2 triangles per cell).

## Tests
- deterministic for same `(seed,t)`; different seeds differ; `offset` bounded by `MAX_DISP` for 10k random samples; border offsets exactly 0; Off => identity points; reduced_motion => independent of `t`; grid cell count within budget; every `DreamTheme` has a `Character` (iterate `ALL_THEMES`).
- `every_screen_stays_inside_its_ui_budget` still green with a full 5-card hand + booklet page at Full juice, both resolutions. If it fails, lower `grid_for` density, not card count.
- `screen_smoke` for hand/booklet/reveal/merge/loadout.

## Approval list
None (no player text).
