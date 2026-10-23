# Stream B: pack art (P4 + P5 polish). Worktree: /home/_l/dreamscape-wt/b  Branch: feat/pick-a-pack-b
Files you own: src/pack_art.rs; extracting the moth sprite from src/shop_ui.rs into a shared fn (e.g. `pub(crate) fn paint_moth(...)`
in shop_ui.rs or a small new module, keeping the shopkeeper drawing identical); exposing `hud::paint_eye` as `pub(crate)` if needed.
Do NOT edit main.rs, reveal_ui.rs, packs.rs (contract), lottery.rs, cards.rs.

Replace the skeleton `draw_pack` body (keep its signature and `PACK_SIZE`, `PackArt`):
- Foil wrapper: rounded rect, crimped zigzag top and bottom edges, foil highlight band that sweeps with `time` (scaled by `motion`; motion 0 => static).
- Body texture: `dream::texture::generate(pattern, palette, bands, seed)` using the theme's own `ThemeSpec` (`theme.spec().patterns`, floor/wall colours) so every theme
  looks different with zero new image files. Cache the uploaded texture in `PackArt` keyed by (theme, seed). Same upload approach as `booklet_ui::CardArt::texture` (NEAREST).
- Emblem (centre): Eye = reuse `hud::paint_eye`; Moth = extracted shopkeeper sprite; Door, Shard, Crescent = simple painter shapes (pixel-ish, matches the game's look).
  Emblem sits on a dark medallion so it reads on any texture.
- Pips on the crimp = `pips` (card count, clamp 1..=5).
- Tilt: skew/offset the wrapper and shift the foil band by `tilt * motion`.
- No rarity tells on the wrapper.
Tests: headless draw smoke for every `DreamTheme` x every `Emblem` at two rect sizes (shape count > some floor, no panic); texture cache reuse (same key twice => one texture);
motion=0 draws identical shapes at two different `time` values; pips clamped. Look at existing smoke tests in main.rs `screen_smoke` and booklet_ui tests for the egui `ctx.run` pattern.
Visual check: render and eyeball is not possible for you; instead assert geometry (emblem inside medallion inside rect). If a PNG dump is easy via egui shapes -> skip, not required.
