# Plan: pick-a-pack (replaces the end-of-run reveal)

Drafted 2026-10-19 from a brainstorm with the user. Decisions are the user's;
the code facts below come from reading `reveal_ui.rs`, `cards.rs`, `lottery.rs`,
`main.rs::open_pack`, `booklet_ui.rs`, `shop_ui.rs`.

## Decisions (from the user)

1. **Replaces** the current reveal. Wake -> pick a pack -> open it. No second reveal.
2. **Unpicked packs are lost.** No stash.
3. **Picking costs nothing.**
4. **Packs never drop Fused or Resonant.** Those stay earned (moth merge, blended dreams).
5. **Art:** Balatro-style wrappers. Each pack has its own art, built from the
   eye / the moth / other game imagery, reusing existing textures.

## What exists today (so we change the right things)

- `main.rs::open_pack` calls `cards::recall(&run_log, boost)`: **one card per
  dream in the run**, each rolled "remembered or faded" (`memory_chance`). Then
  `shape_first_pack` and `lottery::apply_pack_pity`, then `Mode::Reveal`.
- A card's **rarity comes from the dream's score** (`Rarity::from_score`), not a
  random table. It is already decided when the run ends.
- `reveal_ui` already has the flip, the holds before the best card, halos, and
  adaptive layout (`pack_layout`). All of it is reusable per opened pack.
- Card art is procedural (`TexSpec` -> `texture::generate(pattern, palette, ..)`),
  cached by seed in `CardArt`. No pack art exists. `assets/textures/` holds only a
  placeholder, so "reuse textures" means reusing the generator and theme palettes.
- The eye is painter-drawn (`hud.rs::paint_eye`); the moth is pixel art in
  `shop_ui.rs`. Both can be drawn as emblems with no new image files.

## Design

**Offer.** After the summary, show up to 3 sealed packs. A pack = one dream
theme from the run, containing that run's dreams of that theme (cap 5).
Themes are picked deepest-first. Fewer than 3 themes -> fewer packs. The
waking dream (Awakening) is always in the pack for its theme, as today.

**What the player sees before picking** (honest, nothing hidden or adjusted):
theme name, how many cards, and the chance each card is remembered
(`memory_chance`). It does **not** show a rarity table, because rarity isn't
rolled; it comes from how they played. The surprise is the flip.

**Open.** The chosen pack's cards go through the existing flip (dullest first,
best last). Everything in the opened pack is pressed into the booklet
(`Booklet::press`), faded cards still pay `fade_dust`. Unchosen packs are
dropped: no dust, no cards.

**Pity.** `apply_pack_pity` runs on the *opened* pack only, and only advances
when a pack is opened. The pity counter is shown on the pick screen.

**First-run safety.** `shape_first_pack` must hold across the offer: on an
empty booklet the Garden pack (else any playable theme) is always offered, so a
new player can't pick their way into a pack with nothing to build a loadout from.

**Never Fused/Resonant.** Packs are built from `card_from` on run records.
Records with `blend` set are the only route to Fused, which is a dream the
player already earned. Test below enforces nothing else injects them.

## Pack art (procedural, no new image files)

A `pack_art.rs` that paints a foil wrapper with egui's painter:

- **Wrapper:** rounded rect with crimped (zigzag) top and bottom edges, a foil
  highlight band that sweeps with `v.time` (respecting reduced motion).
- **Body texture:** `texture::generate(theme pattern, theme palette, bands, seed)`
  using the theme's own `ThemeSpec.patterns` and colours. Same cache as `CardArt`.
- **Emblem:** a centre glyph from a small set, chosen by theme, drawn with the
  painter: the Eye (reuse `paint_eye`), the Moth (extract the shopkeeper sprite
  from `shop_ui.rs` into a shared fn), plus a couple more (door, shard, crescent).
  Different packs get different emblem + palette; same pieces, recombined.
- **Rarity tells:** none on the wrapper (rarity is not predetermined by theme).
  Wrapper trim shows pack size only (pips on the crimp).
- **Hover/tilt:** pack tilts toward cursor/stick (matches juice plan A6).

## Work, in order

| # | Piece | Touches | Done when |
|---|---|---|---|
| P1 | `PackOffer` type + `offer_packs(run_log, owned, boost) -> Vec<PackOffer>` | `cards.rs` (or new `packs.rs`) | Pure fn; unit tests below |
| P2 | Pick screen + `Mode::PackPick` | `reveal_ui.rs`, `hud.rs`, `main.rs` | Left/right/1-3/Return pick; pad works; falls through to existing `Mode::Reveal` with the chosen pack |
| P3 | `open_pack` takes the chosen offer; pity only on that pack | `main.rs`, `lottery.rs` | Existing reveal tests still pass |
| P4 | `pack_art.rs` wrapper + emblems | new file, `shop_ui.rs` (extract moth), `hud.rs` | Headless draw smoke test for every theme and emblem |
| P5 | Polish: foil sweep, tilt, open tear animation | `pack_art.rs`, `reveal_ui.rs` | Reduced motion and reduce-flashing honoured |

P1 -> P2/P3 are sequential. P4 can run in parallel with P1-P3 (it only needs a theme).

## Tests

- 3 offered packs have distinct themes; <3 themes -> fewer packs, never zero.
- First pack on an empty booklet always includes a playable pack (Garden if present).
- Chosen pack's cards are exactly the run's dreams of that theme (cap 5); unchosen
  cards never reach `Booklet::press`.
- 10k random runs: no pack ever contains `Fused` or `Resonant` unless the source
  record has `blend`.
- Pity counter advances only on the opened pack; unchosen packs don't touch it.
- Odds shown on the pick screen equal `memory_chance` exactly.
- Draw smoke test for every theme x emblem, at small and large screen sizes.
- Old saves load (no new persisted fields without `#[serde(default)]`).

## Open questions (flag before building, not decided)

1. **Pack size variance.** Theme packs mean a theme visited once is a 1-card pack
   next to a 4-card pack. That's a real choice, but 1-card packs feel flat. Option:
   pad small packs to a minimum of 2 from the nearest-attribute dreams. Default
   here: no padding.
2. **A run that only visited one theme** gets one pack, so no choice. Accept, or
   split by depth into early/late packs? Default: accept.
3. **Faded cards.** Still roll "remembered", still pay dust when faded. With a
   pick-one flow, a faded card in your chosen pack may feel bad. Default: keep as is.
4. **Co-op (`coop_game.rs`):** does each player pick their own pack? Not covered here.
