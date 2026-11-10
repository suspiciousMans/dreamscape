# Brief R3: relic art

Goal: relic textures (wear mask, etch glyph, foil sweep) cached and ready for the warp, using the shared `TexSpec` system.

## Scope
- New `src/relic_art.rs` + `mod relic_art;`. Reads `pack_art` / `TexSpec` (`crate::dream::{DreamTheme, TexSpec, Pattern, TEX_SIZE}`) and `relics::{wear_tier, etch_glyph, ETCH_GLYPHS, WearTier}`; mirror the cache style in `pack_art.rs:19` and `booklet_ui.rs:15` (`HashMap<(DreamTheme,u64), egui::TextureHandle>`).
- Leave alone: UI screens (R6), warp (M1), names.

## Functions
- `pub fn base_spec(theme, seed) -> TexSpec` (reuse `theme.spec()`/`pack_art::body_spec`).
- `pub fn rgba(relic: &Relic) -> Vec<u8>`: base texture, then (1) wear: spatter/noise on one palette slot and dimming scaled by `WearTier` (Clean 0 .. Ruined heavy), (2) etch: 1 of `ETCH_GLYPHS` procedural marks (lines/dots/arcs on the pixel grid) in the accent colour, low alpha, (3) rarity frame colours. Deterministic from `seed`.
- `pub fn foil_sweep(t: f32, u: f32, v: f32, tilt: (f32,f32)) -> f32` pure highlight 0..1 for the painter; follows hover tilt (reuse the `worth.rs` foil shimmer if one exists; grep `foil`).
- `pub struct RelicArtCache`: `get(ctx, &Relic) -> TextureId`, bounded size.
- Weapon icon: `pub fn weapon_icon_rgba(shape: Shape, tint, size) -> Vec<u8>` low-res silhouette for the pouch row (flat-shaded 2D projection of the mesh vertices from `dream::build_mesh`, painted with run-merged rects; no GL).

## Tests
- Same relic -> identical bytes; different seeds differ; Clean vs Ruined differ in a measurable pixel fraction; etch glyph present (nonzero accent pixels) for every glyph id; `foil_sweep` in 0..1 and moves with t; cache returns the same handle twice; weapon icon non-empty for all 11 shapes.

## Approval list
None.
