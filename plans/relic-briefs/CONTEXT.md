# Relic System — Shared Context

## Decisions (locked)
- Ash is hard to earn: +1 per dream (+1 bonus per 3 depth, max +2); +3–5 per nightmare.
- Pull costs per rarity: Cracked 40, Worn 60, Pristine 80, Mythic 120 Ash.
- One equipped weapon at a time.
- Moth shop tab for UI.
- Mythic weapons get a tiny orbiting cosmetic mesh.
- Relic names/flavour text go through an approval list before merge.

## Save compatibility
- `Stash` gains `ash: u32` with `#[serde(default)]`.
- `Booklet` gains `relics: Vec<Relic>` and `relic_lottery: RelicState`, both `#[serde(default)]`.
- Old saves without these fields load as empty Ash and empty relic pouch.

## Pure-module rule
- `relics.rs` is pure: no GL, no clock, no global RNG. Every result is a function of `(state, seed, featured)`.
- Tests live in the same file under `#[cfg(test)]`.

## Seeded randomness
- Use `mix(seed, salt)` and `percent_roll(seed, salt)` from `worth.rs`. Do not import `rand` directly in relic logic.
- Draw order is fixed: rarity → theme → foil → wear → etch → weapon shape.

## Texture
- Base `TexSpec` comes from the same `theme.spec()` palette/pattern system as cards and packs.
- Wear mask and etch glyph are computed at draw time in `relic_art.rs` from `seed`.
- Foil sweep reuses the existing foil logic.

## Weapon shapes
- The 11 procedural shapes in `dream/meshes.rs` are the only weapon models.
- Tint comes from the relic's theme accent (`theme.spec().accents[0]`).
- Scale is 0.15–0.35 world units, offset to the dreamer's right hand.

## Sound
- New `Sound` variants live in `sounds.rs`. The exact names/indices are decided in R5; R1/R2 do not touch audio.

## UI
- `relic_ui.rs` is a new module under `// === relics ===` in `main.rs`.
- The moth shop tab is added in R6; R1/R2 do not touch UI files.
