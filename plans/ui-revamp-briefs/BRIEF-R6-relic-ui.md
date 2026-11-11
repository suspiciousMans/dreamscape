# Brief R6: relic UI (pouch + case opening + shop tab)

Goal: players can spend Ash on a pull, watch a satisfying case opening, browse relics, and equip one.

## Scope
- New `src/relic_ui.rs` + `mod relic_ui;`, marker `// === relics-ui ===` in main.rs, and a new tab in `shop_ui.rs` beside Tickets/Abilities (read how those tabs are switched first).
- Wiring to existing: `retention::press_relic` (line 166, appends the pulled relic), `relics::pull/odds/cost_for`, `Stash::spend_ash/ash`, `retention::equip_relic` (R4), `relic_names::name/flavour` (N), `relic_art::RelicArtCache` (R3), `card_warp` (M1), `prompts` (M3), `ui_motion` (M0).
- Flow: build the `Relic` from a `RelicPull`: id = next, seed from the pull RNG, wear = `relics::wear_tier(seed)`, `etch_glyph`, weapon via `relic_weapon::weapon_for`, effect via `relic_effects::effect_for`, name/flavour from N with `taken` = owned names; THEN `press_relic`. Outcome is rolled once and saved at commit so quitting mid-animation cannot re-roll (see `plans/gamble-briefs/CONTEXT.md` guardrails: exact odds on screen, pity counters shown, no near-miss pressure copy, no countdown).

## Screens
- **Pull**: a chunky case (pixel_ui helpers) centred; exact odds table by rarity + foil chance + pity ("Mythic guaranteed in N pulls"); cost on the button; focused-action chip.
- **Case-opening beats** (soft): idle sway -> press (squash ease) -> lid pop (ease_out_back) -> light rises -> relic card slides up with warp and eased flip -> name types in -> hold scaled by rarity (Cracked 0.4 s .. Mythic 1.4 s) -> foil sweep if foil. Skip on any key jumps to the final card (outcome already committed). `reduced_motion` = crossfade only; `reduce_flashing` = no white flash.
- **Pouch**: scrollable list/grid, each with warped art, name, rarity, flavour, weapon icon, buff summary; Equip button + chip; equipped marker; sorted by rarity then id via a pure `order()` (map cursor through it and identify by id, never raw index).
- Settings: none.

## Tests
- Pure: `relic_from_pull` deterministic and unique ids; commit-once (re-opening screen after commit shows the same relic); beat timeline monotone; skip lands on final state; pouch order stable; cursor maps by id.
- `screen_smoke` for pull, mid-animation, pouch (empty and 30 relics) at 640x360 and 1280x720; `audit_text_fits_every_screen`; UI budget test at Full.
- Save compat: booklet without relics shows an empty pouch.

## Approval list
All new on-screen copy to `NOTES-relic-ui.md`; word lists in `NOTES-names.md` (N).
